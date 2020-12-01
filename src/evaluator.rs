pub mod object;
pub mod env;
pub mod builtins;

use crate::ast::*;
use crate::evaluator::env::*;
use crate::evaluator::object::*;
use crate::parser::Parser;
use crate::lexer::Lexer;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::rc::Rc;

const HASH_CASECARE: u32 = 0x1000;
const HASH_SORT: u32 = 0x2000;
const MODULE_FUNC_PREFIX: &str = "`";

#[derive(Debug)]
pub struct  Evaluator {
    env: Rc<RefCell<Env>>,
}

impl Evaluator {
    pub fn new(env: Rc<RefCell<Env>>) -> Self {
        Evaluator {env}
    }

    fn is_truthy(obj: Object) -> bool {
        match obj {
            Object::Empty | Object::Bool(false) => false,
            Object::Num(n) => {
                n != 0.0
            },
            Object::Handle(h) => {
                h != std::ptr::null_mut()
            },
            _ => true
        }
    }

    fn error(msg: String) -> Object {
        Object::Error(msg)
    }

    fn is_error(obj: &Object) -> bool {
        match obj {
            Object::Error(_) => true,
            _ => false
        }
    }

    pub fn eval(&mut self, program: Program) -> Option<Object> {
        let mut result = None;

        for statement in program {
            match self.eval_statement(statement) {
                Some(o) => match o {
                    Object::Exit => return Some(Object::Exit),
                    Object::Error(msg) => return Some(Object::Error(msg)),
                    _ => result = Some(o),
                },
                None => ()
            }
        }
        result
    }

    fn eval_block_statement(&mut self, block: BlockStatement) -> Option<Object> {
        for statement in block {
            match self.eval_statement(statement) {
                Some(o) => match o {
                    Object::Error(_) |
                    Object::Continue(_) |
                    Object::Break(_) |
                    Object::Exit => return Some(o),
                    _ => (),
                },
                None => (),
            };
        }
        None
    }

    fn eval_definition_statement(&mut self, identifier: Identifier, expression: Expression) -> (String, Object) {
        let Identifier(name) = identifier;
        let obj = match self.eval_expression(expression) {
            Some(o) => o,
            None => Self::error(format!("syntax error on definition"))
        };
        (name, obj)
    }

    fn eval_hahtbl_definition_statement(&mut self, identifier: Identifier, hashopt: Option<Expression>) -> (String, Object) {
        let Identifier(name) = identifier;
        let opt = match hashopt {
            Some(e) => match self.eval_expression(e) {
                Some(o) => {
                    if Self::is_error(&o) {
                        return (name, o);
                    } else {
                        match o {
                            Object::Num(n) => n as u32,
                            _ => return (name, Self::error(format!("invalid hashtbl option: {}", o)))
                        }
                    }
                },
                None => return (name, Self::error(format!("syntax error")))
            },
            None => 0
        };
        let casecare = (opt & HASH_CASECARE) > 0;
        let obj = if (opt & HASH_SORT) > 0 {
            let hash = BTreeMap::new();
            Object::SortedHash(hash, casecare)
        } else {
            let hash = HashMap::new();
            Object::Hash(hash, casecare)
        };
        (name, obj)
    }

    fn eval_statement(&mut self, statement: Statement) -> Option<Object> {
        match statement {
            Statement::Dim(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e);
                    if Self::is_error(&value) {
                        return Some(value);
                    } else {
                        match self.env.borrow_mut().define_local(name, &value) {
                            Ok(()) => (),
                            Err(err) => return Some(err),
                        }
                    }
                }
                None
            },
            Statement::Public(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e);
                    if Self::is_error(&value) {
                        return Some(value);
                    } else {
                        match self.env.borrow_mut().define_public(name, &value) {
                            Ok(()) => (),
                            Err(err) => return Some(err),
                        }
                    }
                }
                None
            },
            Statement::Const(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e);
                    if Self::is_error(&value) {
                        return Some(value);
                    } else {
                        match self.env.borrow_mut().define_const(name, &value) {
                            Ok(()) => (),
                            Err(err) => return Some(err),
                        }
                    }
                }
                None
            },
            Statement::HashTbl(i, hashopt, is_public) => {
                let (name, hashtbl) = self.eval_hahtbl_definition_statement(i, hashopt);
                if Self::is_error(&hashtbl) {
                    return Some(hashtbl);
                }
                if is_public {
                    match self.env.borrow_mut().define_public(name, &hashtbl) {
                        Ok(()) => None,
                        Err(err) => Some(err),
                    }
                } else {
                    match self.env.borrow_mut().define_local(name, &hashtbl) {
                        Ok(()) => None,
                        Err(err) => Some(err),
                    }
                }
            },
            Statement::Print(e) => {
                match self.eval_expression(e) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        println!("{}", o);
                        None
                    },
                    None => None
                }
            },
            Statement::Call(s) => {
                println!("{}", s);
                None
            },
            Statement::DefDll(s) => {
                println!("{}", s);
                None
            },
            Statement::Expression(e) => self.eval_expression(e),
            Statement::For {loopvar, from, to, step, block} => {
                self.eval_for_statement(loopvar, from, to, step, block)
            },
            Statement::ForIn {loopvar, collection, block} => {
                self.eval_for_in_statement(loopvar, collection, block)
            },
            Statement::While(e, b) => self.eval_while_statement(e, b),
            Statement::Repeat(e, b) => self.eval_repeat_statement(e, b),
            Statement::Continue(n) => Some(Object::Continue(n)),
            Statement::Break(n) => Some(Object::Break(n)),
            Statement::IfSingleLine {condition, consequence, alternative} => {
                self.eval_if_line_statement(condition, *consequence, *alternative)
            },
            Statement::If {condition, consequence, alternative} => {
                self.eval_if_statement(condition, consequence, alternative)
            },
            Statement::ElseIf {condition, consequence, alternatives} => {
                self.eval_elseif_statement(condition, consequence, alternatives)
            },
            Statement::Select {expression, cases, default} => {
                self.eval_select_statement(expression, cases, default)
            },
            Statement::Function {name, params, body} => {
                let Identifier(fname) = name;
                let func = self.eval_funtcion_definition_statement(&fname, params, body, false, None);
                if Self::is_error(&func) {
                    return Some(func);
                }
                match self.env.borrow_mut().define_function(fname, &func) {
                    Ok(()) => None,
                    Err(err) => Some(err),
                }
            },
            Statement::Procedure {name, params, body} => {
                let Identifier(pname) = name;
                let func = self.eval_funtcion_definition_statement(&pname, params, body, true, None);
                if Self::is_error(&func) {
                    return Some(func);
                }
                match self.env.borrow_mut().define_function(pname, &func) {
                    Ok(()) => None,
                    Err(err) => Some(err),
                }
            },
            Statement::ModuleFunction {module_name, name, params, body} => {
                let func = self.eval_funtcion_definition_statement(&name, params, body, false, Some(&module_name));
                if Self::is_error(&func) {
                    return Some(func);
                }
                match self.env.borrow_mut().define_function(format!("{}.{}", module_name, name), &func) {
                    Ok(()) => None,
                    Err(err) => Some(err),
                }
            },
            Statement::ModuleProcedure {module_name, name, params, body} => {
                let func = self.eval_funtcion_definition_statement(&name, params, body, true, Some(&module_name));
                if Self::is_error(&func) {
                    return Some(func);
                }
                match self.env.borrow_mut().define_function(format!("{}.{}", module_name, name), &func) {
                    Ok(()) => None,
                    Err(err) => Some(err),
                }
            },
            Statement::Module(i, block) => {
                let Identifier(name) = i;
                let module = self.eval_module_statement(&name, block, false);
                if Self::is_error(&module) {
                    return Some(module);
                }
                let result = self.env.borrow_mut().define_module(name.clone(), &module);
                match result {
                    Ok(()) => {
                        let constructor_name = format!("{}.{}", name.clone(), name.clone());
                        if ! self.env.borrow_mut().does_function_exists(&constructor_name) {
                            return None;
                        }
                        let constructor = self.eval_function_call_expression(
                            Box::new(Expression::Identifier(Identifier(constructor_name))),
                            vec![]
                        );
                        if Self::is_error(&constructor) {
                            Some(constructor)
                        } else {
                            None
                        }
                    },
                    Err(err) => Some(err),
                }
            },
            Statement::Exit => Some(Object::Exit),
        }
    }

    fn eval_if_line_statement(&mut self, condition: Expression, consequence: Statement, alternative: Option<Statement>) -> Option<Object> {
        let cond = match self.eval_expression(condition) {
            Some(o) => if Self::is_error(&o) {
                return Some(o);
            } else {
                o
            },
            None => return Some(Self::error(format!("syntax error")))
        };
        if Self::is_truthy(cond) {
            self.eval_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_statement(s),
                None => None
            }
        }
    }

    fn eval_if_statement(&mut self, condition: Expression, consequence: BlockStatement, alternative: Option<BlockStatement>) -> Option<Object> {
        let cond = match self.eval_expression(condition) {
            Some(o) => if Self::is_error(&o) {
                return Some(o);
            } else {
                o
            },
            None => return Some(Self::error(format!("syntax error")))
        };
        if Self::is_truthy(cond) {
            self.eval_block_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_block_statement(s),
                None => None
            }
        }
    }

    fn eval_elseif_statement(&mut self, condition: Expression, consequence: BlockStatement, alternatives: Vec<(Option<Expression>, BlockStatement)>) -> Option<Object> {
        let cond = match self.eval_expression(condition) {
            Some(o) => if Self::is_error(&o) {
                return Some(o);
            } else {
                o
            },
            None => return Some(Self::error(format!("syntax error")))
        };
        if Self::is_truthy(cond) {
            return self.eval_block_statement(consequence);
        } else {
            for (altcond, block) in alternatives {
                match altcond {
                    Some(e) => {
                        // elseif
                        let cond = match self.eval_expression(e) {
                            Some(o) => if Self::is_error(&o) {
                                return Some(o);
                            } else {
                                o
                            },
                            None => return Some(Self::error(format!("syntax error")))
                        };
                        if Self::is_truthy(cond) {
                            return self.eval_block_statement(block);
                        }
                    },
                    None => {
                        // else
                        return self.eval_block_statement(block);
                    }
                }
            }
        }
        None
    }

    fn eval_select_statement(&mut self, expression: Expression, cases: Vec<(Vec<Expression>, BlockStatement)>, default: Option<BlockStatement>) -> Option<Object> {
        let select_obj = match self.eval_expression(expression) {
            Some(o) => if Self::is_error(&o) {
                return Some(o);
            } else {
                o
            },
            None => return Some(Self::error(format!("syntax error")))
        };
        for (case_exp, block) in cases {
            for e in case_exp {
                match self.eval_expression(e) {
                    Some(o) => {
                        if o == select_obj {
                            return self.eval_block_statement(block);
                        }
                    },
                    None => return Some(Self::error(format!("syntax error")))
                }
            }
        }
        match default {
            Some(b) => self.eval_block_statement(b),
            None => None
        }
    }

    fn eval_loopblock_statement(&mut self, block: BlockStatement) -> Option<Object> {
        for statement in block {
            match self.eval_statement(statement) {
                Some(o) => if Self::is_error(&o) {
                    return Some(o);
                } else {
                    match o {
                        Object::Continue(n) => return Some(Object::Continue(n)),
                        Object::Break(n) => return Some(Object::Break(n)),
                        _ => ()
                    }
                },
                None => ()
            };
        }
        None
    }

    fn eval_for_statement(&mut self,loopvar: Identifier, from: Expression, to: Expression, step: Option<Expression>, block: BlockStatement) -> Option<Object> {
        let Identifier(var) = loopvar;
        let mut counter = match self.eval_expression(from) {
            Some(o) => match o {
                Object::Num(n) => n as i64,
                Object::Bool(b) => if b {1} else {0},
                Object::String(s) => {
                    match s.parse::<i64>() {
                        Ok(i) => i,
                        Err(_) => return Some(Self::error(format!("syntax error: for {} = {}", var, s)))
                    }
                },
                _ => return Some(Self::error(format!("syntax error: for {} = {}", var, o))),
            },
            None => return Some(Self::error(format!("{} should start with number", var))),
        };
        let counter_end = match self.eval_expression(to) {
            Some(o) => match o {
                Object::Num(n) => n as i64,
                Object::Bool(b) => if b {1} else {0},
                Object::String(s) => {
                    match s.parse::<i64>() {
                        Ok(i) => i,
                        Err(_) => return Some(Self::error(format!("syntax error: for {} = {} to {}", var, counter, s)))
                    }
                },
                _ => return Some(Self::error(format!("syntax error: for {} = {} to {}", var, counter, o))),
            },
            None => return Some(Self::error(format!("{} should end with number", var))),
        };
        let step = match step {
            Some(e) => {
                match self.eval_expression(e) {
                    Some(o) => match o {
                        Object::Num(n) => n as i64,
                        Object::Bool(b) => b as i64,
                        Object::String(s) => {
                            match s.parse::<i64>() {
                                Ok(i) => i,
                                Err(_) => return Some(Self::error(format!("syntax error: for {} = {} to {} step {}", var, counter, counter_end, s))),
                            }
                        },
                        _ => return Some(Self::error(format!("syntax error: for {} = {} to {} step {}", var, counter, counter_end, o))),
                    },
                    None => return Some(Self::error(format!("step should be number"))),
                }
            },
            None => 1
        };
        match self.env.borrow_mut().assign(var.clone(), &Object::Num(counter as f64)) {
            Ok(()) => (),
            Err(err) => return Some(err)
        };
        loop {
            match self.eval_loopblock_statement(block.clone()) {
                Some(o) => if Self::is_error(&o) {
                    return Some(o);
                } else {
                    match o {
                        Object::Continue(n) => if n > 1 {
                            return Some(Object::Continue(n - 1));
                        } else {
                            counter += step;
                            match self.env.borrow_mut().assign(var.clone(), &Object::Num(counter as f64)) {
                                Ok(()) => (),
                                Err(err) => return Some(err)
                            };
                            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                                break;
                            }
                            continue;
                        },
                        Object::Break(n) => if n > 1 {
                            return Some(Object::Break(n - 1));
                        } else {
                            break;
                        },
                        _ => ()
                    }
                },
                _ => ()
            };
            counter += step;
            match self.env.borrow_mut().assign(var.clone(), &Object::Num(counter as f64)) {
                Ok(()) => (),
                Err(err) => return Some(err)
            };
            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                break;
            }
        }
        None
    }

    fn eval_for_in_statement(&mut self, loopvar: Identifier, collection: Expression, block: BlockStatement) -> Option<Object> {
        let Identifier(var) = loopvar;
        let col_obj = match self.eval_expression(collection) {
            Some(o) => {
                match o {
                    Object::Error(m) => return Some(Self::error(m)),
                    Object::Array(a) => a,
                    Object::String(s) => s.chars().map(|c| Object::String(c.to_string())).collect::<Vec<Object>>(),
                    Object::Hash(h, _) => h.keys().map(|key| Object::String(key.clone())).collect::<Vec<Object>>(),
                    Object::SortedHash(h, _) => h.keys().map(|key| Object::String(key.clone())).collect::<Vec<Object>>(),
                    _ => return Some(Self::error(format!("for-in requires array, hashtable, string, or collection")))
                }
            },
            None => return Some(Self::error(format!("syntax error")))
        };

        for o in col_obj {
            match self.env.borrow_mut().assign(var.clone(), &o) {
                Ok(()) => (),
                Err(err) => return Some(err)
            };
            match self.eval_loopblock_statement(block.clone()) {
                Some(Object::Error(m)) => return Some(Self::error(m)),
                Some(Object::Continue(n)) => if n > 1 {
                    return Some(Object::Continue(n - 1));
                } else {
                    continue;
                },
                Some(Object::Break(n)) => if n > 1 {
                    return Some(Object::Break(n - 1));
                } else {
                    break;
                },
                _ => ()
            }
        }
        None
    }

    fn eval_loop_expression(&mut self, expression: Expression) -> Result<bool, Object> {
        match self.eval_expression(expression) {
            Some(o) => Ok(Self::is_truthy(o)),
            None => Err(Self::error(format!("syntax error")))
        }
    }

    fn eval_while_statement(&mut self, expression: Expression, block: BlockStatement) -> Option<Object> {
        let mut flg = match self.eval_loop_expression(expression.clone()) {
            Ok(b) => b,
            Err(e) => return Some(e)
        };
        while flg {
            match self.eval_loopblock_statement(block.clone()) {
                Some(o) => if Self::is_error(&o) {
                    return Some(o);
                } else {
                    match o {
                        Object::Continue(n) => if n > 1{
                            return Some(Object::Continue(n - 1));
                        } else {
                            flg = match self.eval_loop_expression(expression.clone()) {
                                Ok(b) => b,
                                Err(e) => return Some(e)
                            };
                            continue;
                        },
                        Object::Break(n) => if n > 1 {
                            return Some(Object::Break(n - 1));
                        } else {
                            break;
                        },
                        _ => ()
                    }
                },
                _ => ()
            };
            flg = match self.eval_loop_expression(expression.clone()) {
                Ok(b) => b,
                Err(e) => return Some(e)
            };
        }
        None
    }

    fn eval_repeat_statement(&mut self, expression: Expression, block: BlockStatement) -> Option<Object> {
        loop {
            match self.eval_loopblock_statement(block.clone()) {
                Some(o) => if Self::is_error(&o) {
                    return Some(o);
                } else {
                    match o {
                        Object::Continue(n) => if n > 1 {
                            return Some(Object::Continue(n - 1));
                        } else {
                            continue;
                        },
                        Object::Break(n) => if n > 1 {
                            return Some(Object::Break(n - 1));
                        } else {
                            break;
                        },
                        _ => ()
                    }
                },
                _ => ()
            };
            match self.eval_loop_expression(expression.clone()) {
                Ok(b) => if b {
                    break;
                },
                Err(e) => return Some(e)
            };
        }
        None
    }

    fn eval_funtcion_definition_statement(&mut self, name: &String, params: Vec<Identifier>, body: Vec<Statement>, is_proc: bool, module_name: Option<&String>) -> Object {
        for statement in body.clone() {
            match statement {
                Statement::Public(_) | Statement::Const(_) => {
                    match self.eval_statement(statement) {
                        Some(Object::Error(msg)) => return Self::error(msg),
                        _ => {},
                    };
                },
                Statement::Function{name: _, params: _, body: _} |
                Statement::Procedure{name: _, params: _, body: _} => {
                    return Self::error(format!("nested definition of function/procedure is not allowed"));
                },
                _ => {},
            };
        }
        let func = match module_name {
            Some(s) => if is_proc {
                Object::ModuleProcedure(s.clone(), name.clone(), params, body, Rc::clone(&self.env))
            } else {
                Object::ModuleFunction(s.clone(), name.clone(), params, body, Rc::clone(&self.env))
            },
            None => if is_proc {
                Object::Procedure(name.clone(), params, body, Rc::clone(&self.env))
            } else {
                Object::Function(name.clone(), params, body, Rc::clone(&self.env))
            }
        };
        func
    }

    fn eval_module_statement(&mut self, name: &String, block: BlockStatement, is_class: bool) -> Object {
        let mut members = HashMap::new();
        for statement in block {
            match statement {
                Statement::Dim(vec) => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = match self.eval_expression(e) {
                            Some(o) => if Self::is_error(&o) {
                                return o
                            } else {
                                o
                            },
                            None => Object::Empty
                        };
                        members.insert(member_name.clone(), value.clone());
                        members.insert(format!("this.{}", &member_name), value.clone());
                        members.insert(format!("{}.{}", name, &member_name), value);
                    }
                },
                Statement::Public(vec) |
                Statement::Const(vec)  => {
                    for (i, _) in vec {
                        let Identifier(member_name) = i;
                        let global_name = format!("{}.{}", name, member_name);
                        members.insert(member_name.clone(), Object::GlobalMember(global_name.clone()));
                        members.insert(format!("this.{}", member_name), Object::GlobalMember(global_name));
                    }
                },
                Statement::Function{name: i, params: _, body: _} |
                Statement::Procedure{name: i, params: _, body: _} => {
                    let Identifier(member_name) = i;
                    let global_name = format!("{}.{}", name, member_name);
                    members.insert(format!("{}{}", MODULE_FUNC_PREFIX, &member_name), Object::GlobalMember(global_name.clone()));
                    members.insert(format!("{}this.{}", MODULE_FUNC_PREFIX, &member_name), Object::GlobalMember(global_name));
                },
                _ => return Self::error(format!("invalid statement"))
            }
        }
        if is_class {
            Self::error(format!("class is not supported"))
        } else {
            Object::Module(name.clone(), members)
        }
    }

    fn eval_expression(&mut self, expression: Expression) -> Option<Object> {
        let some_obj = match expression {
            Expression::Identifier(i) => Some(self.eval_identifier(i)),
            Expression::Array(v, s) => {
                let capacity = match self.eval_expression(*s) {
                    Some(o) => {
                        if Self::is_error(&o) {
                            return Some(o);
                        }
                        match o {
                            Object::Num(n) => n as usize + 1,
                            Object::Empty => v.len(),
                            _ => return Some(Self::error(format!("invalid index: {}", o)))
                        }
                    },
                    None => return None
                };
                let mut array = Vec::with_capacity(capacity);
                for e in v {
                    array.push(self.eval_expression(e).unwrap());
                }
                while array.len() < capacity {
                    array.push(Object::Empty);
                }
                Some(Object::Array(array))
            },
            Expression::Literal(l) => Some(self.eval_literal(l)),
            Expression::Prefix(p, r) => if let Some(right) = self.eval_expression(*r) {
                Some(self.eval_prefix_expression(p, right))
            } else {
                None
            },
            Expression::Infix(i, l, r) => {
                let left = match self.eval_expression(*l) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                let right = match self.eval_expression(*r) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                Some(self.eval_infix_expression(i, left, right))
            },
            Expression::Index(l, i) => {
                let left = match self.eval_expression(*l) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                let index = match self.eval_expression(*i) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                Some(self.eval_index_expression(left, index))
            },
            Expression::AnonymusFunction {params, body} => {
                Some(Object::AnonFunc(params, body, Rc::clone(&self.env)))
            },
            Expression::AnonymusProcedure {params, body} => {
                Some(Object::AnonProc(params, body, Rc::clone(&self.env)))
            },
            Expression::FuncCall {func, args} => {
                Some(self.eval_function_call_expression(func, args))
            },
            Expression::Assign(l, r) => {
                let value = match self.eval_expression(*r) {
                    Some(o) => o,
                    None => return None
                };
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    self.eval_assign_expression(*l, value)
                }
            },
            Expression::CompoundAssign(l, r, i) => {
                let left = match self.eval_expression(*l.clone()) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                let right = match self.eval_expression(*r) {
                    Some(o) => if Self::is_error(&o) {
                        return Some(o);
                    } else {
                        o
                    },
                    None => return None
                };
                // let left = self.eval_expression(*l.clone());
                // let right = self.eval_expression(*r);
                let value= self.eval_infix_expression(i, left, right);
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    self.eval_assign_expression(*l, value)
                }
            },
            Expression::Ternary {condition, consequence, alternative} => {
                self.eval_ternary_expression(*condition, *consequence, *alternative)
            },
            Expression::DotCall(l, r) => {
                Some(self.eval_dotcall_expression(*l, *r))
            },
        };
        match some_obj {
            Some(Object::GlobalMember(s)) => {
                self.eval_expression(Expression::Identifier(Identifier(s)))
            },
            _ => some_obj
        }
    }

    fn eval_identifier(&mut self, identifier: Identifier) -> Object {
        let Identifier(name) = identifier;
        let mut env = self.env.borrow_mut();
        match env.get_variable(&name) {
            Some(o) => match o {
                Object::BuiltinConst(bc) => *bc,
                _ => o
            },
            None => match env.get_func(&name) {
                Some(o) => o,
                None => match env.get_module(&name) {
                    Some(o) => o,
                    None => Object::Error(String::from(format!("identifier not found: {}", name)))
                }
            }
        }
    }

    fn eval_prefix_expression(&mut self, prefix: Prefix, right: Object) -> Object {
        match prefix {
            Prefix::Not => self.eval_not_operator_expression(right),
            Prefix::Minus => self.eval_minus_operator_expression(right),
            Prefix::Plus => self.eval_plus_operator_expression(right),
        }
    }

    fn eval_not_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Bool(true) => Object::Bool(false),
            Object::Bool(false) => Object::Bool(true),
            Object::Empty => Object::Bool(true),
            Object::Num(n) => {
                Object::Bool(n == 0.0)
            },
            _ => Object::Bool(false)
        }
    }

    fn eval_minus_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Num(n) => Object::Num(-n),
            _ => Self::error(format!("1 unknown operator: -{}", right))
        }
    }

    fn eval_plus_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Num(n) => Object::Num(n),
            _ => Self::error(format!("2 unknown operator: +{}", right))
        }
    }

    fn eval_index_expression(&mut self, left: Object, index: Object) -> Object {
        match left {
            Object::Array(ref a) => if let Object::Num(i) = index {
                self.eval_array_index_expression(a.clone(), i as i64)
            } else {
                Self::error(format!("imvalid index: {}[{}]", left, index))
            },
            Object::Hash(ref h, casecare) => {
                let key = match index {
                    Object::Num(n) => n.to_string(),
                    Object::Bool(b) => b.to_string(),
                    Object::String(s) => if casecare {
                        s
                    } else {
                        s.to_ascii_uppercase()
                    },
                    Object::Error(_) => return index,
                    _ => return Self::error(format!("invalid hash key:{}", index))
                };
                match h.get(&key) {
                    Some(o) => o.clone(),
                    None => Object::Empty
                }
            },
            Object::SortedHash(ref h, casecare) => {
                let key = match index {
                    Object::Num(n) => n.to_string(),
                    Object::Bool(b) => b.to_string(),
                    Object::String(s) => if casecare {
                        s
                    } else {
                        s.to_ascii_uppercase()
                    },
                    Object::Error(_) => return index,
                    _ => return Self::error(format!("invalid hash key:{}", index))
                };
                match h.get(&key) {
                    Some(o) => o.clone(),
                    None => Object::Empty
                }
            },
            _ => Self::error(format!("not array or hashtable: {}", left))
        }
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: i64) -> Object {
        let max = (array.len() as i64) - 1;
        if index < 0 || index > max {
            return Self::error(format!("index out of bounds: {}", index));
        }

        match array.get(index as usize) {
            Some(o) => o.clone(),
            None => Object::Empty
        }
    }

    fn eval_assign_expression(&mut self, left: Expression, value: Object) -> Option<Object> {

        match left {
            Expression::Identifier(i) => {
                let Identifier(name) = i;
                match self.env.borrow_mut().assign(name, &value) {
                    Ok(()) => None,
                    Err(err) => Some(err)
                }
            },
            Expression::Index(n, i) => {
                let name = match *n {
                    Expression::Identifier(i) => {
                        let Identifier(n) = i;
                        n
                    },
                    _ => return None
                };
                let index = match self.eval_expression(*i) {
                    Some(o) => o,
                    None => return None
                };
                let mut env = self.env.borrow_mut();
                match env.get_variable(&name) {
                    Some(o) => {
                        match o {
                            Object::Array(a) => {
                                let mut arr = a.clone();
                                match index {
                                    Object::Num(n) => {
                                        let i = n as usize;
                                        if i < arr.len() {
                                            arr[i] = value;
                                            match env.assign(name, &Object::Array(arr)) {
                                                Ok(()) => (),
                                                Err(err) => return Some(err)
                                            };
                                        }
                                    },
                                    _ => return Some(Self::error(format!("invalid index: {}", index)))
                                };
                            },
                            Object::Hash(h, casecare) => {
                                let key = match index {
                                    Object::Num(n) => n.to_string(),
                                    Object::Bool(b) => b.to_string(),
                                    Object::String(s) => if casecare {
                                        s
                                    } else {
                                        s.to_ascii_uppercase()
                                    },
                                    _ => return Some(Self::error(format!("invalid hash key: {}", index)))
                                };
                                let mut hash = h.clone();
                                hash.entry(key).or_insert_with(|| value);
                                match env.assign(name, &Object::Hash(hash, casecare)) {
                                    Ok(()) => (),
                                    Err(err) => return Some(err)
                                };
                            },
                            Object::SortedHash(h, casecare) => {
                                let key = match index {
                                    Object::Num(n) => n.to_string(),
                                    Object::Bool(b) => b.to_string(),
                                    Object::String(s) => if casecare {
                                        s
                                    } else {
                                        s.to_ascii_uppercase()
                                    },
                                    _ => return Some(Self::error(format!("invalid hash key: {}", index)))
                                };
                                let mut hash = h.clone();
                                hash.entry(key).or_insert_with(|| value);
                                match env.assign(name, &Object::SortedHash(hash, casecare)) {
                                    Ok(()) => (),
                                    Err(err) => return Some(err)
                                };
                            },
                            _ => return Some(Self::error(format!("not an array or hashtable: {}", name)))
                        };
                    },
                    None => return None
                };
                None
            },
            _ => None
        }
    }

    fn eval_infix_expression(&mut self, infix: Infix, left: Object, right: Object) -> Object {
        match left.clone() {
            Object::Num(n1) => {
                match right {
                    Object::Num(n) => {
                        self.eval_infix_number_expression(infix, n1, n)
                    },
                    Object::String(s) => {
                        if infix == Infix::Plus {
                            self.eval_infix_string_expression(infix, n1.to_string(), s.clone())
                        } else {
                            match s.parse::<f64>() {
                                Ok(n2) => self.eval_infix_number_expression(infix, n1, n2),
                                Err(_) => self.eval_infix_string_expression(infix, n1.to_string(), s.clone())
                            }
                        }
                    },
                    Object::Empty => self.eval_infix_number_expression(infix, n1, 0.0),
                    Object::Bool(b) => self.eval_infix_number_expression(infix, n1, b as i64 as f64),
                    _ => self.eval_infix_misc_expression(infix, left, right),
                }
            },
            Object::String(s1) => {
                match right {
                    Object::String(s2) => self.eval_infix_string_expression(infix, s1.clone(), s2.clone()),
                    Object::Num(n) => {
                        if infix == Infix::Plus {
                            self.eval_infix_string_expression(infix, s1.clone(), n.to_string())
                        } else {
                            match s1.parse::<f64>() {
                                Ok(n2) => self.eval_infix_number_expression(infix, n2, n),
                                Err(_) => self.eval_infix_string_expression(infix, s1.clone(), n.to_string())
                            }
                        }
                    },
                    Object::Bool(b) => self.eval_infix_string_expression(infix, s1.clone(), b.to_string()),
                    Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                    _ => self.eval_infix_misc_expression(infix, left, right)
                }
            },
            Object::Bool(l) => match right {
                Object::Bool(b) => self.eval_infix_logical_operator_expression(infix, l, b),
                Object::String(s) => self.eval_infix_string_expression(infix, l.to_string(), s.clone()),
                Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                Object::Num(n) => self.eval_infix_number_expression(infix, l as i64 as f64, n),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Empty => match right {
                Object::Num(n) => self.eval_infix_number_expression(infix, 0.0, n),
                Object::String(_) => self.eval_infix_empty_expression(infix, left, right),
                Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                _ => self.eval_infix_misc_expression(infix, left, right)
            }
            _ => self.eval_infix_misc_expression(infix, left, right)
        }
    }

    fn eval_infix_misc_expression(&mut self, infix: Infix, left: Object, right: Object) -> Object {
        match infix {
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => Self::error(format!("mismatched type: {} {} {}", left, infix, right)),
        }
    }

    fn eval_infix_number_expression(&mut self, infix: Infix, left: f64, right: f64) -> Object {
        match infix {
            Infix::Plus => Object::Num(left + right),
            Infix::Minus => Object::Num(left - right),
            Infix::Multiply => Object::Num(left * right),
            Infix::Divide => Object::Num(left / right),
            Infix::Mod => Object::Num(left % right),
            Infix::LessThan => Object::Bool(left < right),
            Infix::LessThanEqual => Object::Bool(left <= right),
            Infix::GreaterThan => Object::Bool(left > right),
            Infix::GreaterThanEqual => Object::Bool(left >= right),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            Infix::And => Object::Num((left as i64 & right as i64) as f64),
            Infix::Or => Object::Num((left as i64 | right as i64) as f64),
            Infix::Xor => Object::Num((left as i64 ^ right as i64) as f64),
            Infix::Assign => Self::error(format!("you can not assign variable in expression: {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_string_expression(&mut self, infix: Infix, left: String, right: String) -> Object {
        match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => Self::error(format!("bad operator: {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_empty_expression(&mut self, infix: Infix, left: Object, right: Object) -> Object {
        match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => Self::error(format!("bad operator: {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_logical_operator_expression(&mut self, infix: Infix, left: bool, right: bool) -> Object {
        match infix {
            Infix::And => Object::Bool(left && right),
            Infix::Or => Object::Bool(left || right),
            _ => self.eval_infix_number_expression(infix, left as i64 as f64, right as i64 as f64)
        }
    }

    fn eval_literal(&mut self, literal: Literal) -> Object {
        match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects),
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
        }
    }

    fn eval_array_literal(&mut self, objects: Vec<Expression>) -> Object {
        Object::Array(
            objects.iter().map(
                |e| self.eval_expression(e.clone()).unwrap()
            ).collect::<Vec<_>>()
        )
    }

    fn eval_expression_for_func_call(&mut self, expression: Expression) -> Option<Object> {
        // 関数定義から探してなかったら変数を見る
        match expression {
            Expression::Identifier(i) => {
                let Identifier(name) = i;
                let mut env = self.env.borrow_mut();
                // モジュールローカルから関数を探す
                match env.get_variable(&format!("{}{}", MODULE_FUNC_PREFIX, &name)) {
                    Some(o) => Some(o),
                    None => match env.get_func(&name.replace("global.", "")) {
                        Some(o) => Some(o),
                        None => match env.get_func(&name) {
                            Some(o) => Some(o),
                            None => match env.get_variable(&name) {
                                Some(o) => Some(o),
                                None => Some(Object::Error(format!("function not found: {}", name)))
                            }
                        }
                    }
                }
            },
            _ => self.eval_expression(expression)
        }
    }

    fn builtin_func_result(&mut self, result: Object) -> Object {
        match result {
            Object::Eval(s) => {
                let mut parser = Parser::new(Lexer::new(&s));
                let program = parser.parse();
                let errors = parser.get_errors();
                if errors.len() > 0 {
                    let mut eval_parse_error = format!("eval parse error[{}]:", errors.len());
                    for err in errors {
                        eval_parse_error = format!("{}, {}", eval_parse_error, err);
                    }
                    return Self::error(eval_parse_error);
                }
                match self.eval(program) {
                    Some(o) => o,
                    None => Object::Empty
                }
            },
            Object::Debug(t) => match t {
                DebugType::PrintEnv(s) => {
                    self.env.borrow_mut().print_env(s);
                    Object::Empty
                },
            },
            _ => result
        }
    }

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>) -> Object {
        let arguments = args.iter().map(
            |e| self.eval_expression(e.clone()).unwrap()
        ).collect::<Vec<_>>();

        let mut is_proc = false;
        let mut copy_scope = false;
        let mut module_name = String::new();
        let (params, body, env) = match self.eval_expression_for_func_call(*func) {
            Some(o) => match o {

                Object::Function(_, p, b, e) => (p, b, e),
                Object::Procedure(_, p, b, e) => {
                    is_proc = true;
                    (p, b, e)
                },
                Object::ModuleFunction(m, _, p, b, e) => {
                    module_name = m;
                    (p, b, e)
                },
                Object::ModuleProcedure(m, _, p, b, e) => {
                    module_name = m;
                    is_proc = true;
                    (p, b, e)
                },
                Object::AnonFunc(p, b, e) => {
                    copy_scope = true;
                    (p, b, e)
                },
                Object::AnonProc(p, b, e) => {
                    copy_scope = true;
                    is_proc = true;
                    (p, b, e)
                },
                Object::BuiltinFunction(expected_param_len, f) => {
                    if expected_param_len >= arguments.len() as i32 {
                        let func_result = f(arguments);
                        return self.builtin_func_result(func_result);
                    } else {
                        let l = arguments.len();
                        return Self::error(format!(
                            "{} argument{} were given, should be {}{}",
                            l, if l > 1 {"s"} else {""}, expected_param_len, if l > 1 {" (or less)"} else {""}
                        ));
                    }
                },
                Object::GlobalMember(s) => {
                    return self.eval_function_call_expression(
                        Box::new(Expression::Identifier(Identifier(s))),
                        args
                    );
                },
                Object::Error(err) => return Object::Error(err),
                _ => return Self::error(format!(
                    "{} is not a function", o
                )),
            },
            None => return Object::Empty,
        };

        if params.len() != arguments.len() {
            return Self::error(format!(
                "length of arguments should be {}, not {}",
                params.len(),
                arguments.len()
            ));
        }

        let current_env = Rc::clone(&self.env);
        let mut scoped_env = if copy_scope {
            Env::copy_scope(Rc::clone(&env))
        } else {
            Env::new_scope(Rc::clone(&env))
        };
        let list = params.iter().zip(arguments.iter());
        for (_, (identifier, o)) in list.enumerate() {
            let Identifier(name) = identifier.clone();
            scoped_env.set_function_params(name, o);
        }

        if module_name.len() > 0 {
            scoped_env.set_module_private_member(&module_name);
        }

        self.env = Rc::new(RefCell::new(scoped_env));
        let object = self.eval_block_statement(body);
        let result = if is_proc {
            Object::Empty
        } else {
            match self.env.borrow_mut().get_variable(&"result".to_string()) {
                Some(o) => o,
                None => Object::Empty
            }
        };
        let updated_public = self.env.borrow_mut().get_public_scope();
        let mut new = HashMap::new();
        if module_name.len() > 0 {
            let mut env = self.env.borrow_mut();
            match env.get_module(&module_name) {
                Some(Object::Module(_, map)) => {
                    for (k, _) in map {
                        match env.get_variable(&k) {
                            Some(o) => new.insert(k, o),
                            None => None,
                        };
                    }
                },
                _ => (),
            };
        };
        self.env = current_env;
        self.env.borrow_mut().set_public_scope(updated_public);
        // update module private members
        if new.len() > 0 {
            self.env.borrow_mut().update_module(&module_name, &Object::Module(module_name.clone(), new));
        }

        match object {
            Some(o) => if Self::is_error(&o) {
                o
            } else {
                result
            },
            None => result
        }
    }

    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> Option<Object> {
        let condition = match self.eval_expression(condition) {
            Some(c) => c,
            None => return None
        };
        if Self::is_error(&condition) {
            return Some(condition);
        }
        if Self::is_truthy(condition) {
            self.eval_expression(consequence)
        } else {
            self.eval_expression(alternative)
        }
    }

    fn eval_dotcall_expression(&mut self, left: Expression, right: Expression) -> Object {
        let instance = match left {
            Expression::Identifier(_) |
            Expression::Index(_, _) |
            Expression::FuncCall{func:_, args:_} |
            Expression::DotCall(_, _)=> {
                self.eval_expression(left)
            },
            _ => return Self::error(format!("bad operator"))
        };
        match instance {
            Some(o) => match o {
                Object::Module(name, map) => {
                    match right {
                        Expression::Identifier(i) => {
                            let Identifier(member) = i;
                            match self.eval_expression(Expression::Identifier(Identifier(format!("{}.{}", name, member)))) {
                                Some(o) => if Self::is_error(&o) {
                                    if map.contains_key(&member) {
                                        Self::error(format!("you can not access to {}.{}", name, member))
                                    } else {
                                        o
                                    }
                                } else {
                                    o
                                },
                                None => Self::error(format!(". operator: syntax error"))
                            }
                        },
                        _ => Self::error(format!("member does not exist."))
                    }
                },
                Object::Error(_) => return o,
                _ => Self::error(format!(". operator not supported"))
            },
            None => Self::error(format!(". operator: syntax error"))
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::evaluator::builtins::init_builtins;
    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_test(input: &str, expected: Option<Object>, ast: bool) {
        assert_eq!(eval(input, ast), expected);
    }

    fn eval(input: &str, ast: bool) -> Option<Object> {
        let (f, c) = init_builtins();
        let mut e = Evaluator::new(Rc::new(
            RefCell::new(Env::from_builtin(f, c))
        ));
        let program = Parser::new(Lexer::new(input)).parse();
        if ast {
            println!("{:?}", program);
        }
        let result = e.eval(program);
        result
    }

    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        let (f, c) = init_builtins();
        let mut e = Evaluator::new(Rc::new(
            RefCell::new(Env::from_builtin(f, c))
        ));
        let program = Parser::new(Lexer::new(input)).parse();
        e.eval(program);
        e
    }

    //
    fn eval_test_with_env(e: &mut Evaluator, input: &str, expected: Option<Object>) {
        let program = Parser::new(Lexer::new(input)).parse();
        assert_eq!(e.eval(program), expected)
    }


    #[test]
    fn test_num_expression() {
        let test_cases = vec![
            ("5", Some(Object::Num(5.0))),
            ("10", Some(Object::Num(10.0))),
            ("-5", Some(Object::Num(-5.0))),
            ("-10", Some(Object::Num(-10.0))),
            ("1.23", Some(Object::Num(1.23))),
            ("-1.23", Some(Object::Num(-1.23))),
            ("+(-5)", Some(Object::Num(-5.0))),
            ("1 + 2 + 3 - 4", Some(Object::Num(2.0))),
            ("2 * 3 * 4", Some(Object::Num(24.0))),
            ("-3 + 3 * 2 + -3", Some(Object::Num(0.0))),
            ("5 + 3 * -2", Some(Object::Num(-1.0))),
            ("6 / 3 * 2 + 1", Some(Object::Num(5.0))),
            ("1.2 + 2.4", Some(Object::Num(3.5999999999999996))),
            ("1.2 * 3", Some(Object::Num(3.5999999999999996))),
            ("2 * (5 + 10)", Some(Object::Num(30.0))),
            ("3 * 3 * 3 + 10", Some(Object::Num(37.0))),
            ("3 * (3 * 3) + 10", Some(Object::Num(37.0))),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", Some(Object::Num(50.0))),
            ("1 + TRUE", Some(Object::Num(2.0))),
            ("1 + false", Some(Object::Num(1.0))),
            ("TRUE + 1", Some(Object::Num(2.0))),
            ("5 mod 3", Some(Object::Num(2.0))),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_string_infix() {
        let test_cases = vec![
            (r#""hoge" + "fuga""#, Some(Object::String("hogefuga".to_string()))),
            (r#""hoge" + 100"#, Some(Object::String("hoge100".to_string()))),
            (r#"400 + "fuga""#, Some(Object::String("400fuga".to_string()))),
            (r#""hoge" + TRUE"#, Some(Object::String("hogeTrue".to_string()))),
            (r#""hoge" + FALSE"#, Some(Object::String("hogeFalse".to_string()))),
            (r#"TRUE + "hoge""#, Some(Object::String("Truehoge".to_string()))),
            (r#""hoge" = "hoge""#, Some(Object::Bool(true))),
            (r#""hoge" == "hoge""#, Some(Object::Bool(true))),
            (r#""hoge" == "fuga""#, Some(Object::Bool(false))),
            (r#""hoge" == "HOGE""#, Some(Object::Bool(false))),
            (r#""hoge" == 1"#, Some(Object::Bool(false))),
            (r#""hoge" != 1"#, Some(Object::Bool(true))),
            (r#""hoge" <> 1"#, Some(Object::Bool(true))),
            (r#""hoge" <> "hoge"#, Some(Object::Bool(false))),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_assign_variable() {
        let test_cases = vec![
            (
                r#"
dim hoge = 1
hoge = 2
hoge
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
dim HOGE = 2
hoge
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
dim hoge = 2
dim hoge = 3
                "#,
                Some(Object::Error("HOGE is already defined.".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_assign_hashtbl() {
        let test_cases = vec![
            (
                r#"
hashtbl hoge
hoge["test"] = 2
hoge["test"]
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
hashtbl hoge
hoge["test"] = 2
hoge["TEST"]
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
hashtbl hoge
hoge[1.23] = 2
hoge[1.23]
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
hashtbl hoge
hoge[FALSE] = 2
hoge[FALSE]
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
hashtbl hoge = HASH_CASECARE
hoge["abc"] = 1
hoge["ABC"] = 2
hoge["abc"] + hoge["ABC"]
                "#,
                Some(Object::Num(3.0))
            ),
            (
                r#"
hashtbl hoge = HASH_CASECARE or HASH_SORT
hoge["abc"] = "a"
hoge["ABC"] = "b"
hoge["000"] = "c"
hoge["999"] = "d"

a = ""
for key in hoge
    a = a + hoge[key]
next
a
                "#,
                Some(Object::String("cdba".to_string()))
            ),
            (
                r#"
public hashtbl hoge
hoge["a"] = "hoge"

function f(key)
    result = hoge[key]
fend

f("a")
                "#,
                Some(Object::String("hoge".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_assign_array() {
        let input = r#"
dim hoge[] = 1,3,5
hoge[0] = "hoge"
hoge[0]
        "#;
        eval_test(input, Some(Object::String("hoge".to_string())), false);
    }

    #[test]
    fn test_assign_array_literal() {
        let input = r#"
hoge = [1,3,5]
hoge[0] = 2
hoge[0]
        "#;
        eval_test(input, Some(Object::Num(2.0)), false);
    }

    #[test]
    fn test_public() {
        let input = r#"
public hoge = 1
hoge
        "#;
        eval_test(input, Some(Object::Num(1.0)), false);
    }

    #[test]
    fn test_array_definition() {
        let input = r#"
dim hoge[3] = 1, 2
hoge
        "#;
        eval_test(input, Some(Object::Array(vec![
            Object::Num(1.0),
            Object::Num(2.0),
            Object::Empty,
            Object::Empty,
        ])), false);
    }

    #[test]
    fn test_print() {
        let input = r#"
hoge = "print test"
print hoge
        "#;
        eval_test(input, None, false);
    }

    #[test]
    fn test_for() {
        let test_cases = vec![
            (
                r#"
for i = 0 to 3
next
i
                "#,
                Some(Object::Num(4.0))
            ),
            (
                r#"
for i = 0 to 2
    i = 10
next
i
                "#,
                Some(Object::Num(3.0))
            ),
            (
                r#"
for i = 0 to 5 step 2
next
i
                "#,
                Some(Object::Num(6.0))
            ),
            (
                r#"
for i = 5 to 0 step -2
next
i
                "#,
                Some(Object::Num(-1.0))
            ),
            (
                r#"
for i = "0" to "5" step "2"
next
i
                "#,
                Some(Object::Num(6.0))
            ),
            (
                r#"
for i = 0 to "5s"
next
                "#,
                Some(Object::Error("syntax error: for i = 0 to 5s".to_string()))
            ),
            (
                r#"
a = 1
for i = 0 to 3
    continue
    a = a  + 1
next
a
                "#,
                Some(Object::Num(1.0))
            ),
            (
                r#"
a = 1
for i = 0 to 20
    break
    a = 5
next
a
                "#,
                Some(Object::Num(1.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }


    #[test]
    fn test_forin() {
        let test_cases = vec![
            (
                r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
    a = a + n
next
a
                "#,
                Some(Object::Num(15.0))
            ),
            (
                r#"
a = ""
for c in "hoge"
    a = c + a
next
a
                "#,
                Some(Object::String("egoh".to_string()))
            ),
            (
                r#"
hashtbl hoge
hoge[1] = 1
hoge[2] = 2
hoge[3] = 3
a = 0
for key in hoge
    a = a + hoge[key]
next
a
                "#,
                Some(Object::Num(6.0))
            ),
            (
                r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
    a = a + n
    if n = 3 then break
next
a
                "#,
                Some(Object::Num(6.0))
            ),
            (
                r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
    continue
    a = a + n
next
a
                "#,
                Some(Object::Num(0.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }


    #[test]
    fn test_while() {
        let test_cases = vec![
            (
                r#"
a = 5
while a > 0
    a = a -1
wend
a
                "#,
                Some(Object::Num(0.0))
            ),
            (
                r#"
a = 0
while a < 3
    a = a + 1
    continue
    a = a + 1
wend
while true
    a = a + 1
    break
    a = a + 1
wend
a
                "#,
                Some(Object::Num(4.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_repeat() {
        let test_cases = vec![
            (
                r#"
a = 5
repeat
    a = a - 1
until a < 1
a
                "#,
                Some(Object::Num(0.0))
            ),
            (
                r#"
a = 2
repeat
    a = a - 1
    if a < 0 then break else continue
until false
a
                "#,
                Some(Object::Num(-1.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_if_1line() {
        let test_cases = vec![
            (
                r#"
if true then a = "a is true" else a = "a is false"
a
                "#,
                Some(Object::String("a is true".to_string()))
            ),
            (
                r#"
if 1 < 0 then a = "a is true" else a = "a is false"
a
                "#,
                Some(Object::String("a is false".to_string()))
            ),
            (
                r#"
if true then print "test sucseed!" else print "should not be printed"
                "#,
                None
            ),
            (
                r#"
a = 1
if false then a = 5
a
                "#,
                Some(Object::Num(1.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_if() {
        let test_cases = vec![
            (
                r#"
if true then
    a = "a is true"
else
    a = "a is false"
endif
a
                "#,
                Some(Object::String("a is true".to_string()))
            ),
            (
                r#"
if 0 then
    a = "a is true"
else
    a = "a is false"
endif
a
                "#,
                Some(Object::String("a is false".to_string()))
            ),
            (
                r#"
if true then
    a = "test sucseed!"
else
    a = "should not get this message"
endif
a
                "#,
                Some(Object::String("test sucseed!".to_string()))
            ),
            (
                r#"
a = 1
if false then
    a = 5
endif
a
                "#,
                Some(Object::Num(1.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_elseif() {
        let test_cases = vec![
            (
                r#"
if false then
    a = "should not get this message"
elseif true then
    a = "test1 sucseed!"
endif
a
                "#,
                Some(Object::String("test1 sucseed!".to_string()))
            ),
            (
                r#"
if false then
    a = "should not get this message"
elseif false then
    a = "should not get this message"
elseif true then
    a = "test2 sucseed!"
endif
a
                "#,
                Some(Object::String("test2 sucseed!".to_string()))
            ),
            (
                r#"
if false then
    a = "should not get this message"
elseif false then
    a = "should not get this message"
else
    a = "test3 sucseed!"
endif
a
                "#,
                Some(Object::String("test3 sucseed!".to_string()))
            ),
            (
                r#"
if true then
    a = "test4 sucseed!"
elseif true then
    a = "should not get this message"
else
    a = "should not get this message"
endif
a
                "#,
                Some(Object::String("test4 sucseed!".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_select() {
        let test_cases = vec![
            (
                r#"
select 1
    case 1
        a = "test1 sucseed!"
    case 2
        a = "should not get this message"
    default
        a = "should not get this message"
selend
a
                "#,
                Some(Object::String("test1 sucseed!".to_string()))
            ),
            (
                r#"
select 3
    case 1
        a = "should not get this message"
    case 2, 3
        a = "test2 sucseed!"
    default
        a = "should not get this message"
selend
a
                "#,
                Some(Object::String("test2 sucseed!".to_string()))
            ),
            (
                r#"
select 6
    case 1
        a = "should not get this message"
    case 2, 3
        a = "should not get this message"
    default
        a = "test3 sucseed!"
selend
a
                "#,
                Some(Object::String("test3 sucseed!".to_string()))
            ),
            (
                r#"
select 6
    default
        a = "test4 sucseed!"
selend
a
                "#,
                Some(Object::String("test4 sucseed!".to_string()))
            ),
            (
                r#"
select true
    case 1 = 2
        a = "should not get this message"
    case 2 = 2
        a = "test5 sucseed!"
selend
a
                "#,
                Some(Object::String("test5 sucseed!".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_block_in_loopblock() {
        let test_cases = vec![
            (
                r#"
a = 0
while true
    select a
        case 5
            break
            a = a + 1
        default
            a = a + 1
    selend
    if a >= 6 then break
wend
a
                "#,
                Some(Object::Num(5.0))
            ),
            (
                r#"
a = 0
while true
    if a = 5 then
        break
        a = a + 1
    else
        a = a + 1
    endif
    if a >= 6 then break
wend
a
                "#,
                Some(Object::Num(5.0))
            ),
            (
                r#"
a = 1
while a < 5
    while TRUE
        a = a + 1
        continue 2
    wend
wend
a
                "#,
                Some(Object::Num(5.0))
            ),
            (
                r#"
a = 1
for i = 0 to 5
    for j = 0 to 5
        a = a + 1
        continue 2
    next
next
a
                "#,
                Some(Object::Num(7.0))
            ),
            (
                r#"
a = 1
repeat
    repeat
        a = a + 1
        break 2
    until false
until false
a
                "#,
                Some(Object::Num(2.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_function() {
        let test_cases = vec![
            (
                r#"
a = hoge(1, 2)
a

function hoge(x, y)
　result = x + fuga(y)
fend
function fuga(n)
　result = n * 2
fend
                "#,
                Some(Object::Num(5.0))
            ),
            (
                r#"
hoge(5)

function hoge(n)
    // no result
fend
                "#,
                Some(Object::Empty)
            ),
            (
                r#"
a = hoge(5)
a == 5

procedure hoge(n)
    result = n
fend
                "#,
                Some(Object::Bool(false))
            ),
            (
                r#"
a = 'should not be over written'
hoge(5)
a

procedure hoge(n)
    a = n
fend
                "#,
                Some(Object::String("should not be over written".to_string()))
            ),
            (
                r#"
f  = function(x, y)
    result = x + y
fend

f(5, 10)
                "#,
                Some(Object::Num(15.0))
            ),
            (
                r#"
a = 1
p = procedure(x, y)
    a = x + y
fend

p(5, 10)
a
                "#,
                Some(Object::Num(1.0))
            ),
            (
                r#"
closure = test_closure("testing ")
closure("closure")

function test_closure(s)
    result = function(s2)
        result = s + s2
    fend
fend
                "#,
                Some(Object::String("testing closure".to_string()))
            ),
            (
                r#"
recursive(5)

function recursive(n)
    if n = 0 then
        result = "done"
    else
        result = recursive(n - 1)
    endif
fend
                "#,
                Some(Object::String("done".to_string()))
            ),
            (
                r#"
hoge(2, fuga)

function hoge(x, func)
    result = func(x)
fend
function fuga(n)
    result = n * 2
fend
                "#,
                Some(Object::Num(4.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }
    #[test]
    fn test_comment() {
        let test_cases = vec![
            (
                r#"
a = 1
// a = a + 2
a
                "#,
                Some(Object::Num(1.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_multiple_definitions() {
        let test_cases = vec![
            (
                r#"
dim dim_and_dim = 1
dim dim_and_dim = 2
                "#,
                Some(Object::Error("DIM_AND_DIM is already defined.".to_string()))
            ),
            (
                r#"
public pub_and_const = 1
const pub_and_const = 2
                "#,
                Some(Object::Error("PUB_AND_CONST is already defined.".to_string()))
            ),
            (
                r#"
const const_and_const = 1
const const_and_const = 2
                "#,
                Some(Object::Error("CONST_AND_CONST is already defined.".to_string()))
            ),
            (
                r#"
public public_and_public = 1
public public_and_public = 2
                "#,
                None
            ),
            (
                r#"
hashtbl hash_and_hash
hashtbl hash_and_hash
                "#,
                Some(Object::Error("HASH_AND_HASH is already defined.".to_string()))
            ),
            (
                r#"
function func_and_func()
fend
function func_and_func()
fend
                "#,
                Some(Object::Error("FUNC_AND_FUNC is already defined.".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_compound_assign() {
        let test_cases = vec![
            (
                r#"
a = 1
a += 1
a
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
a = "hoge"
a += "fuga"
a
                "#,
                Some(Object::String("hogefuga".to_string()))
            ),
            (
                r#"
a = 5
a -= 3
a
                "#,
                Some(Object::Num(2.0))
            ),
            (
                r#"
a = 2
a *= 5
a
                "#,
                Some(Object::Num(10.0))
            ),
            (
                r#"
a = 10
a /= 5
a
                "#,
                Some(Object::Num(2.0))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_public_in_function() {
        let input = r#"
hoge = a + b()

function b()
    public a = 5
    result = 6
fend

hoge
        "#;
        eval_test(input, Some(Object::Num(11.0)), false)
    }

    #[test]
    fn test_scope() {
        let definition = r#"
dim v = "script local"
public p = "public"
public p2 = "public 2"
const c = "const"

dim f = "variable"
function f()
    result = "function"
fend

function func()
    result = "function"
fend

function get_p()
    result = p
fend

function get_c()
    result = c
fend

function get_v()
    result = v
fend

module M
    dim v = "module local"
    public p = "module public"
    const c = "module const"

    function func()
        result = "module function"
    fend

    function get_v()
        result = v
    fend

    function get_this_v()
        result = this.v
    fend

    function get_p()
        result = p
    fend

    function get_outer_p2()
        result = p2
    fend

    function inner_func()
        result = func()
    fend

    function outer_func()
        result = global.func()
    fend
endmodule
        "#;
        let mut e = eval_env(definition);
        let test_cases = vec![
            (
                "v",
                Some(Object::String("script local".to_string()))
            ),
            (
                r#"
                v += " 1"
                v
                "#,
                Some(Object::String("script local 1".to_string()))
            ),
            (
                "p",
                Some(Object::String("public".to_string()))
            ),
            (
                r#"
                p += " 1"
                p
                "#,
                Some(Object::String("public 1".to_string()))
            ),
            (
                "c",
                Some(Object::String("const".to_string()))
            ),
            (
                "func()",
                Some(Object::String("function".to_string()))
            ),
            (
                "f",
                Some(Object::String("variable".to_string()))
            ),
            (
                "f()",
                Some(Object::String("function".to_string()))
            ),
            (
                "get_p()",
                Some(Object::String("public 1".to_string()))
            ),
            (
                "get_c()",
                Some(Object::String("const".to_string()))
            ),
            (
                "get_v()",
                Some(Object::Error("identifier not found: v".to_string()))
            ),
            (
                "M.v",
                Some(Object::Error("identifier not found: M.v".to_string()))
            ),
            (
                "M.p",
                Some(Object::String("module public".to_string()))
            ),
            (
                "M.c",
                Some(Object::String("module const".to_string()))
            ),
            (
                "M.func()",
                Some(Object::String("module function".to_string()))
            ),
            (
                "M.get_v()",
                Some(Object::String("module local".to_string()))
            ),
            (
                "M.get_this_v()",
                Some(Object::String("module local".to_string()))
            ),
            (
                "M.get_p()",
                Some(Object::String("module public".to_string()))
            ),
            (
                "M.get_outer_p2()",
                Some(Object::String("public 2".to_string()))
            ),
            (
                "M.inner_func()",
                Some(Object::String("module function".to_string()))
            ),
            (
                "M.outer_func()",
                Some(Object::String("function".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test_with_env(&mut e, input, expected);
        }
    }

    #[test]
    fn test_hoge() {
        let input1 = r#"
function hoge(n)
    result = n
fend
        "#;
        let mut e = eval_env(input1);
        let test_cases = vec![
            (
                "hoge(3)",
                Some(Object::Num(3.0))
            ),
            (
                "hoge('abc')",
                Some(Object::String("abc".to_string()))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test_with_env(&mut e, input, expected);
        }
    }
}