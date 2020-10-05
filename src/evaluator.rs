pub mod object;
pub mod env;
pub mod builtins;

use crate::ast::*;
use crate::evaluator::env::*;
use crate::evaluator::object::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
            }
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
            if statement == Statement::Blank {
                continue;
            }
            match self.eval_statement(statement) {
                Some(Object::Error(msg)) => return Some(Object::Error(msg)),
                obj => result = obj,
            }
        }

        result
    }

    fn eval_block_statement(&mut self, block: BlockStatement) -> Option<Object> {
        for statement in block {
            if statement == Statement::Blank {
                continue;
            }
            match self.eval_statement(statement) {
                Some(o) => match o {
                    Object::Error(msg) => return Some(Self::error(msg)),
                    Object::Continue(n) => return Some(Object::Continue(n)),
                    Object::Break(n) => return Some(Object::Break(n)),
                    _ => (),
                },
                None => (),
            };
        }
        None
    }

    fn eval_statement(&mut self, statement: Statement) -> Option<Object> {
        match statement {
            Statement::Dim(i, e) => {
                let value = match self.eval_expression(e) {
                    Some(o) => o,
                    None => return None
                };
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    let Identifier(name) = i;
                    self.env.borrow_mut().set(name, &value);
                    None
                }
            },
            Statement::Public(i, e) => {
                let value = match self.eval_expression(e) {
                    Some(o) => o,
                    None => return None
                };
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    let Identifier(name) = i;
                    self.env.borrow_mut().set_global(name, &value);
                    None
                }
            },
            Statement::Const(i, e) => {
                let value = match self.eval_expression(e) {
                    Some(o) => o,
                    None => return None
                };
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    let Identifier(name) = i;
                    self.env.borrow_mut().set_global(name, &value);
                    None
                }
            },
            Statement::HashTbl(i, o) => {
                match o {
                    HashOption::CaseCare => (),
                    HashOption::Sort => (),
                    HashOption::None => (),
                };
                let hash = HashMap::new();
                let value = Object::Hash(hash);
                let Identifier(name) = i;
                self.env.borrow_mut().set(name, &value);
                None
            },
            Statement::Print(e) => {
                match self.eval_expression(e) {
                    Some(o) => {
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
                let Identifier(name) = name;
                let func = Object::Function(params, body, Rc::clone(&self.env));
                self.env.borrow_mut().set(name, &func);
                None
            },
            Statement::Procedure {name, params, body} => {
                let Identifier(name) = name;
                let func = Object::Procedure(params, body, Rc::clone(&self.env));
                self.env.borrow_mut().set(name, &func);
                None
            },
            _ => None
        }
    }

    fn eval_if_line_statement(&mut self, condition: Expression, consequence: Statement, alternative: Option<Statement>) -> Option<Object> {
        let cond = match self.eval_expression(condition) {
            Some(o) => o,
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
            Some(o) => o,
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
            Some(o) => o,
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
                            Some(o) => o,
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
            Some(o) => o,
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
            if statement == Statement::Blank {
                continue;
            };
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
        self.env.borrow_mut().set(var.clone(), &Object::Num(counter as f64));
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
                            self.env.borrow_mut().set(var.clone(), &Object::Num(counter as f64));
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
            self.env.borrow_mut().set(var.clone(), &Object::Num(counter as f64));
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
                    Object::Hash(h) => h.keys().map(|key| key.clone()).collect::<Vec<Object>>(),
                    _ => return Some(Self::error(format!("for-in requires array, hashtable, string, or collection")))
                }
            },
            None => return Some(Self::error(format!("syntax error")))
        };

        for o in col_obj {
            self.env.borrow_mut().set(var.clone(), &o);
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

    fn eval_expression(&mut self, expression: Expression) -> Option<Object> {
        match expression {
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
                let left = self.eval_expression(*l);
                let right = self.eval_expression(*r);
                if left.is_some() && right.is_some() {
                    Some(self.eval_infix_expression(i, left.unwrap(), right.unwrap()))
                } else {
                    None
                }
            },
            Expression::Index(l, i) => {
                let left = self.eval_expression(*l);
                let index = self.eval_expression(*i);
                if left.is_some() && index.is_some() {
                    Some(self.eval_index_expression(left.unwrap(), index.unwrap()))
                } else {
                    None
                }
            },
            Expression::Function {params, body} => {
                Some(Object::Function(params, body, Rc::clone(&self.env)))
            },
            Expression::Procedure {params, body} => {
                Some(Object::Procedure(params, body, Rc::clone(&self.env)))
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
                    self.eval_assign_expression(*l, value);
                    None
                }
            },
            Expression::Ternary {condition, consequence, alternative} => {
                self.eval_ternary_expression(*condition, *consequence, *alternative)
            }
        }
    }

    fn eval_identifier(&mut self, identifier: Identifier) -> Object {
        let Identifier(name) = identifier;

        match self.env.borrow_mut().get(name.clone()) {
            Some(o) => o,
            None => Object::Error(String::from(format!("identifier not found: {}", name)))
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
            Object::Hash(ref h) => match index {
                Object::Num(_) |
                Object::Bool(_) |
                Object::String(_) => match h.get(&index) {
                    Some(o) => o.clone(),
                    None => Object::Empty
                },
                Object::Error(_) => index,
                _ => Self::error(format!("invalid key:{}", index))
            },
            _ => Self::error(format!("3 unknown operator: {} {}", left, index))
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
                self.env.borrow_mut().set(name, &value);
                None
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
                match env.get(name.clone()) {
                    Some(o) => {
                        match o {
                            Object::Array(a) => {
                                let mut arr = a.clone();
                                match index {
                                    Object::Num(n) => {
                                        let i = n as usize;
                                        if i < arr.len() {
                                            arr[i] = value;
                                            env.set(name, &Object::Array(arr));
                                        }
                                    },
                                    _ => return Some(Self::error(format!("invalid index: {}", index)))
                                };
                            },
                            Object::Hash(h) => {
                                let mut hash = h.clone();
                                hash.entry(index).or_insert_with(|| value);
                                env.set(name, &Object::Hash(hash));
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
        match left {
            Object::Num(l) => {
                match right {
                    Object::Num(n) => {
                        self.eval_infix_number_expression(infix, l, n)
                    },
                    Object::String(_) => {
                        self.eval_infix_string_expression(infix, left, right)
                    },
                    Object::Bool(b) => {
                        self.eval_infix_number_expression(infix, l, b as i64 as f64)
                    },
                    _ => {
                        Self::error(format!("mismatched type: {} {} {}", left, infix, right))
                    }
                }
            },
            Object::String(_) => {
                match right {
                    Object::Num(_) | Object::String(_) | Object::Bool(_) => (),
                    _ => return Self::error(format!("mismatched type: {} {} {}", left, infix, right))
                };
                self.eval_infix_string_expression(infix, left, right)
            },
            Object::Bool(l) => match right {
                Object::Bool(b) => self.eval_infix_logical_operator_expression(infix, l, b),
                Object::String(_) => self.eval_infix_string_expression(infix, left, right),
                Object::Num(n) => self.eval_infix_number_expression(infix, l as i64 as f64, n),
                _ => Self::error(format!("mismatched type: {} {} {}", left, infix, right))
            },
            _ => Self::error(format!("bad operator: {} {} {}", left, infix, right))
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

    fn eval_infix_string_expression(&mut self, infix: Infix, left: Object, right: Object) -> Object {
        match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
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

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>) -> Object {
        let args = args.iter().map(
            |e| self.eval_expression(e.clone()).unwrap()
        ).collect::<Vec<_>>();

        let mut is_proc = false;
        let (params, body, env) = match self.eval_expression(*func) {
            Some(Object::Function(p, b, e)) => (p, b, e),
            Some(Object::Procedure(p, b, e)) => {
                is_proc = true;
                (p, b, e)
            },
            Some(Object::Builtin(expected_param_len, f)) => {
                if expected_param_len < 0 || expected_param_len > args.len() as i32 {
                    return f(args);
                } else {
                    return Self::error(format!(
                        "too much arguments ({}). max count of arguments should be {}",
                        args.len(), expected_param_len
                    ));
                }
            },
            Some(o) => return Self::error(format!(
                "{} is not a function", o
            )),
            None => return Object::Empty,
        };

        if params.len() != args.len() {
            return Self::error(format!(
                "length of arguments should be {}, not {}",
                params.len(),
                args.len()
            ));
        }

        let current_env = Rc::clone(&self.env);
        let mut scoped_env = Env::new_with_outer(Rc::clone(&env));
        let list = params.iter().zip(args.iter());
        for (_, (identifier, o)) in list.enumerate() {
            let Identifier(name) = identifier.clone();
            scoped_env.set(name, o);
        }

        self.env = Rc::new(RefCell::new(scoped_env));
        let object = self.eval_block_statement(body);
        let result = match self.env.borrow_mut().get("result".to_string()) {
            Some(o) => o,
            None => if is_proc {
                Object::Empty
            } else {
                Object::Error("no result found".to_string())
            }
        };
        self.env = current_env;

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

}

#[cfg(test)]
mod tests {
    use crate::evaluator::builtins::init_builtins;
    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_test(input: &str, expected: Option<Object>) {
        assert_eq!(eval(input), expected);
    }

    fn eval(input: &str) -> Option<Object> {
        let mut e = Evaluator::new(Rc::new(
            RefCell::new(Env::from(init_builtins()))
        ));
        let result = e.eval(
            Parser::new(Lexer::new(input)).parse()
        );
        // println!("{:?}", e);
        result
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
            eval_test(input, expected);
        }
    }

    #[test]
    fn test_string_concat() {
        let test_cases = vec![
            (r#""hoge" + "fuga""#, Some(Object::String("hogefuga".to_string()))),
            (r#""hoge" + 100"#, Some(Object::String("hoge100".to_string()))),
            (r#"400 + "fuga""#, Some(Object::String("400fuga".to_string()))),
            (r#""hoge" + TRUE"#, Some(Object::String("hogeTrue".to_string()))),
            (r#""hoge" + FALSE"#, Some(Object::String("hogeFalse".to_string()))),
            (r#"TRUE + "hoge""#, Some(Object::String("Truehoge".to_string()))),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected);
        }
    }

    #[test]
    fn test_assign_variable() {
        let input = r#"
dim hoge = 1
hoge = 2
hoge
        "#;
        eval_test(input, Some(Object::Num(2.0)));
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
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected);
        }
    }

    #[test]
    fn test_assign_array() {
        let input = r#"
dim hoge[] = 1,3,5
hoge[0] = "hoge"
hoge[0]
        "#;
        eval_test(input, Some(Object::String("hoge".to_string())));
    }

    #[test]
    fn test_assign_array_literal() {
        let input = r#"
hoge = [1,3,5]
hoge[0] = 2
hoge[0]
        "#;
        eval_test(input, Some(Object::Num(2.0)));
    }

    #[test]
    fn test_public() {
        let input = r#"
public hoge = 1
hoge
        "#;
        eval_test(input, Some(Object::Num(1.0)));
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
        ])));
    }

    #[test]
    fn test_print() {
        let input = r#"
hoge = "print test"
print hoge
        "#;
        eval_test(input, None);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
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
            eval_test(input, expected);
        }
    }

    #[test]
    fn test_function() {
        let test_cases = vec![
            (
                r#"
a = hoge(1, 2)
// print a

function hoge(x, y)
　result = x + fuga(y)
fend
function fuga(n)
　result = n * 2
fend

a
                "#,
                Some(Object::Num(5.0))
            ),
            (
                r#"
a = 1
hoge(5)

procedure hoge(n)
    a = a + 10
fend

a
                "#,
                Some(Object::Num(1.0))
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
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected);
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
            eval_test(input, expected);
        }
    }
}