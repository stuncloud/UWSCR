pub mod object;
// pub mod env;
pub mod environment;
pub mod builtins;

use crate::ast::*;
// use crate::evaluator::env::*;
use crate::evaluator::environment::*;
use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::logging::{out_log, LogType};

use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;

use num_traits::FromPrimitive;
use regex::Regex;
use serde_json;

#[derive(PartialEq, Debug, Clone)]
pub struct UError {
    //pos: Position
    title: String,
    msg: String,
    sub_msg: Option<String>
}

impl UError {
    pub fn new(title: String, msg: String, sub_msg: Option<String>) -> Self {
        UError{title, msg, sub_msg}
    }
}

impl From<BuiltinError> for UError {
    fn from(e: BuiltinError) -> Self {
        match e {
            BuiltinError::FunctionError(m, s) => UError::new("function error".into(), m, s),
            BuiltinError::ArgumentError(m, s) => UError::new("argument error".into(), m, s),
        }
    }
}

impl fmt::Display for UError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.sub_msg.is_some() {
            // write!(f, "[{}] {}: {} ({})", self.pos, self.title, self.msg, self.sub_msg.clone().unwrap())
            write!(f, "{}: {} ({})", self.title, self.msg, self.sub_msg.clone().unwrap())
        } else {
            // write!(f, "[{}] {}: {})", self.pos, self.title, self.msg)
            write!(f, "{}: {}", self.title, self.msg)
        }
    }
}

type EvalResult<T> = Result<T, UError>;

#[derive(Debug)]
pub struct  Evaluator {
    env: Rc<RefCell<Environment>>,
    instance_id: u32,
}

impl Evaluator {
    pub fn new(env: Rc<RefCell<Environment>>) -> Self {
        Evaluator {env, instance_id: 0}
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

    fn new_instance_id(&mut self) -> u32 {
        self.instance_id += 1;
        self.instance_id
    }

    pub fn eval(&mut self, program: Program) -> EvalResult<Option<Object>> {
        let mut result = None;

        for statement in program {
            match self.eval_statement(statement)? {
                Some(o) => match o {
                    Object::Exit => {
                        result = Some(Object::Exit);
                        break;
                    },
                    _ => result = Some(o),
                },
                None => ()
            }
        }
        self.auto_dispose_instances(vec![], true);
        Ok(result)
    }

    fn eval_block_statement(&mut self, block: BlockStatement) -> EvalResult<Option<Object>> {
        for statement in block {
            match self.eval_statement(statement)? {
                Some(o) => match o {
                    Object::Continue(_) |
                    Object::Break(_) |
                    Object::Exit => return Ok(Some(o)),
                    _ => (),
                },
                None => (),
            };
        }
        Ok(None)
    }

    fn eval_definition_statement(&mut self, identifier: Identifier, expression: Expression) -> EvalResult<(String, Object)> {
        let Identifier(name) = identifier;
        let obj = self.eval_expression(expression)?;
        Ok((name, obj))
    }

    fn eval_hashtbl_definition_statement(&mut self, identifier: Identifier, hashopt: Option<Expression>) -> EvalResult<(String, Object)> {
        let Identifier(name) = identifier;
        let opt = match hashopt {
            Some(e) => match self.eval_expression(e)? {
                Object::Num(n) => n as u32,
                o => return Err(UError::new(
                    "Error on hashtbl definition".into(),
                    format!("invalid hashtbl option: {}", o),
                    None
                ))
            },
            None => 0
        };
        let sort = (opt & HashTblEnum::HASH_SORT as u32) > 0;
        let casecare = (opt & HashTblEnum::HASH_CASECARE as u32) > 0;
        let hashtbl = HashTbl::new(sort, casecare);
        Ok((name, Object::HashTbl(Rc::new(RefCell::new(hashtbl)))))
    }

    fn eval_print_statement(&mut self, expression: Expression) -> EvalResult<Option<Object>> {
        let obj = self.eval_expression(expression)?;
        out_log(&format!("{}", obj), LogType::Print);
        println!("{}", obj);
        Ok(None)
    }

    fn eval_statement(&mut self, statement: Statement) -> EvalResult<Option<Object>> {
        match statement {
            Statement::Dim(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.borrow_mut().define_local(name, value)?;
                }
                Ok(None)
            },
            Statement::Public(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.borrow_mut().define_public(name, value)?;
                }
                Ok(None)
            },
            Statement::Const(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.borrow_mut().define_const(name, value)?;
                }
                Ok(None)
            },
            Statement::TextBlock(i, s) => {
                let Identifier(name) = i;
                let value = self.eval_literal(s);
                self.env.borrow_mut().define_const(name, value)?;
                Ok(None)
            },
            Statement::HashTbl(i, hashopt, is_public) => {
                let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, hashopt)?;
                if is_public {
                    self.env.borrow_mut().define_public(name, hashtbl)?;
                } else {
                    self.env.borrow_mut().define_local(name, hashtbl)?;
                }
                Ok(None)
            },
            Statement::Print(e) => self.eval_print_statement(e),
            Statement::Call(s) => {
                println!("{}", s);
                Ok(None)
            },
            Statement::DefDll{name: _, params:_, ret_type: _, path: _} => {
                Ok(None)
            },
            Statement::Expression(e) => Ok(Some(self.eval_expression(e)?)),
            Statement::For {loopvar, from, to, step, block} => {
                self.eval_for_statement(loopvar, from, to, step, block)
            },
            Statement::ForIn {loopvar, collection, block} => {
                self.eval_for_in_statement(loopvar, collection, block)
            },
            Statement::While(e, b) => self.eval_while_statement(e, b),
            Statement::Repeat(e, b) => self.eval_repeat_statement(e, b),
            Statement::Continue(n) => Ok(Some(Object::Continue(n))),
            Statement::Break(n) => Ok(Some(Object::Break(n))),
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
            Statement::Function {name, params, body, is_proc} => {
                let Identifier(fname) = name;
                let func = self.eval_funtcion_definition_statement(&fname, params, body, is_proc)?;
                self.env.borrow_mut().define_function(fname, func)?;
                Ok(None)
            },
            Statement::Module(i, block) => {
                let Identifier(name) = i;
                let module = self.eval_module_statement(&name, block, false)?;
                self.env.borrow_mut().define_module(name.clone(), module)?;
                // コンストラクタがあれば実行する
                let module = self.env.borrow().get_module(&name);
                if let Some(Object::Module(m)) = module {
                    if m.borrow().has_constructor() {
                        self.eval_function_call_expression(
                            Box::new(Expression::DotCall(
                                Box::new(Expression::Identifier(Identifier(name.clone()))),
                                Box::new(Expression::Identifier(Identifier(name))),
                            )),
                            vec![]
                        )?;
                    }
                };
                Ok(None)
            },
            Statement::Class(i, block) => {
                let Identifier(name) = i;
                let class = Object::Class(name.clone(), block);
                self.env.borrow_mut().define_class(name.clone(), class)?;
                Ok(None)
            },
            Statement::With(o_e, block) => if let Some(e) = o_e {
                let s = self.eval_block_statement(block);
                self.eval_instance_assignment(&e, &Object::Nothing)?;
                if let Expression::Identifier(Identifier(name)) = e {
                    self.env.borrow_mut().remove_variable(name);
                }
                s
            } else {
                self.eval_block_statement(block)
            },
            Statement::Exit => Ok(Some(Object::Exit)),
        }
    }

    fn eval_if_line_statement(&mut self, condition: Expression, consequence: Statement, alternative: Option<Statement>) -> EvalResult<Option<Object>> {
        if Self::is_truthy(self.eval_expression(condition)?) {
            self.eval_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_if_statement(&mut self, condition: Expression, consequence: BlockStatement, alternative: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        if Self::is_truthy(self.eval_expression(condition)?) {
            self.eval_block_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_block_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_elseif_statement(&mut self, condition: Expression, consequence: BlockStatement, alternatives: Vec<(Option<Expression>, BlockStatement)>) -> EvalResult<Option<Object>> {
        if Self::is_truthy(self.eval_expression(condition)?) {
            return self.eval_block_statement(consequence);
        } else {
            for (altcond, block) in alternatives {
                match altcond {
                    Some(cond) => {
                        // elseif
                        if Self::is_truthy(self.eval_expression(cond)?) {
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
        Ok(None)
    }

    fn eval_select_statement(&mut self, expression: Expression, cases: Vec<(Vec<Expression>, BlockStatement)>, default: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let select_obj = self.eval_expression(expression)?;
        for (case_exp, block) in cases {
            for e in case_exp {
                if self.eval_expression(e)? == select_obj {
                    return self.eval_block_statement(block);
                }
            }
        }
        match default {
            Some(b) => self.eval_block_statement(b),
            None => Ok(None)
        }
    }

    fn eval_loopblock_statement(&mut self, block: BlockStatement) -> EvalResult<Option<Object>> {
        for statement in block {
            if let Some(o) = self.eval_statement(statement)? {
                match o {
                    Object::Continue(_)|
                    Object::Break(_) => return Ok(Some(o)),
                    _ => ()
                }
            };
        }
        Ok(None)
    }

    fn eval_for_statement(&mut self,loopvar: Identifier, from: Expression, to: Expression, step: Option<Expression>, block: BlockStatement) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let mut counter = match self.eval_expression(from)? {
            Object::Num(n) => n as i64,
            Object::Bool(b) => if b {1} else {0},
            Object::String(s) => {
                match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => return Err(UError::new(
                        "Syntax error on For".into(),
                        format!("for {} = {}", var, s),
                        None
                    ))
                }
            },
            o => return Err(UError::new(
                "Syntax error on For".into(),
                format!("for {} = {}", var, o),
                None
            )),
        };
        let counter_end = match self.eval_expression(to)? {
            Object::Num(n) => n as i64,
            Object::Bool(b) => if b {1} else {0},
            Object::String(s) => {
                match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => return Err(UError::new(
                        "Syntax error on For".into(),
                        format!("for {} = {} to {}", var, counter, s),
                        None
                    ))
                }
            },
            o => return Err(UError::new(
                "Syntax error on For".into(),
                format!("for {} = {} to {}", var, counter, o),
                None
            )),
        };
        let step = match step {
            Some(e) => {
                match self.eval_expression(e)? {
                    Object::Num(n) => n as i64,
                    Object::Bool(b) => b as i64,
                    Object::String(s) => {
                        match s.parse::<i64>() {
                            Ok(i) => i,
                            Err(_) => return Err(UError::new(
                                "Syntax error on For".into(),
                                format!("for {} = {} to {} step {}", var, counter, counter_end, s),
                                None
                            ))
                        }
                    },
                    o => return Err(UError::new(
                        "Syntax error on For".into(),
                        format!("for {} = {} to {} step {}", var, counter, counter_end, o),
                        None
                    )),
                }
            },
            None => 1
        };
        self.env.borrow_mut().assign(var.clone(), Object::Num(counter as f64))?;
        loop {
            match self.eval_loopblock_statement(block.clone())? {
                Some(o) => match o {
                        Object::Continue(n) => if n > 1 {
                            return Ok(Some(Object::Continue(n - 1)));
                        } else {
                            counter += step;
                            self.env.borrow_mut().assign(var.clone(), Object::Num(counter as f64))?;
                            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                                break;
                            }
                            continue;
                        },
                        Object::Break(n) => if n > 1 {
                            return Ok(Some(Object::Break(n - 1)));
                        } else {
                            break;
                        },
                        _ => ()
                },
                _ => ()
            };
            counter += step;
            self.env.borrow_mut().assign(var.clone(), Object::Num(counter as f64))?;
            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                break;
            }
        }
        Ok(None)
    }

    fn eval_for_in_statement(&mut self, loopvar: Identifier, collection: Expression, block: BlockStatement) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let col_obj = match self.eval_expression(collection)? {
            Object::Array(a) => a,
            Object::String(s) => s.chars().map(|c| Object::String(c.to_string())).collect::<Vec<Object>>(),
            Object::HashTbl(h) => h.borrow().keys(),
            _ => return Err(UError::new(
                "For-In error".into(),
                format!("for-in requires array, hashtable, string, or collection"),
                None
            ))
        };

        for o in col_obj {
            self.env.borrow_mut().assign(var.clone(), o)?;
            match self.eval_loopblock_statement(block.clone())? {
                Some(Object::Continue(n)) => if n > 1 {
                    return Ok(Some(Object::Continue(n - 1)));
                } else {
                    continue;
                },
                Some(Object::Break(n)) => if n > 1 {
                    return Ok(Some(Object::Break(n - 1)));
                } else {
                    break;
                },
                _ => ()
            }
        }
        Ok(None)
    }

    fn eval_loop_flg_expression(&mut self, expression: Expression) -> Result<bool, UError> {
        Ok(Self::is_truthy(self.eval_expression(expression)? ))
    }

    fn eval_while_statement(&mut self, expression: Expression, block: BlockStatement) -> EvalResult<Option<Object>> {
        let mut flg = self.eval_loop_flg_expression(expression.clone())?;
        while flg {
            match self.eval_loopblock_statement(block.clone())? {
                Some(Object::Continue(n)) => if n > 1{
                    return Ok(Some(Object::Continue(n - 1)));
                } else {
                    flg = self.eval_loop_flg_expression(expression.clone())?;
                    continue;
                },
                Some(Object::Break(n)) => if n > 1 {
                    return Ok(Some(Object::Break(n - 1)));
                } else {
                    break;
                },
                _ => ()
            };
            flg = self.eval_loop_flg_expression(expression.clone())?;
        }
        Ok(None)
    }

    fn eval_repeat_statement(&mut self, expression: Expression, block: BlockStatement) -> EvalResult<Option<Object>> {
        loop {
            match self.eval_loopblock_statement(block.clone())? {
                Some(Object::Continue(n)) => if n > 1 {
                    return Ok(Some(Object::Continue(n - 1)));
                } else {
                    continue;
                },
                Some(Object::Break(n)) => if n > 1 {
                    return Ok(Some(Object::Break(n - 1)));
                } else {
                    break;
                },
                _ => ()
            };
            if self.eval_loop_flg_expression(expression.clone())? {
                break;
            }
        }
        Ok(None)
    }

    fn eval_funtcion_definition_statement(&mut self, name: &String, params: Vec<Expression>, body: Vec<Statement>, is_proc: bool) -> EvalResult<Object> {
        for statement in body.clone() {
            match statement {
                Statement::Function{name: _, params: _, body: _, is_proc: _}  => {
                    return Err(UError::new(
                        "Function defining error".into(),
                        format!("nested definition of function/procedure is not allowed"),
                        None
                    ))
                },
                _ => {},
            };
        }
        Ok(Object::Function(name.clone(), params, body, is_proc, None))
    }

    fn eval_module_statement(&mut self, module_name: &String, block: BlockStatement, is_instance: bool) -> EvalResult<Object> {
        let mut module = Module::new(module_name.to_string());
        for statement in block {
            match statement {
                Statement::Dim(vec) => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        module.add(member_name, value, Scope::Local);
                    }
                },
                Statement::Public(vec) => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        module.add(member_name, value, Scope::Public);
                    }
                },
                Statement::Const(vec)  => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        module.add(member_name, value, Scope::Const);
                    }
                },
                Statement::TextBlock(i, s) => {
                    let Identifier(name) = i;
                    let value = self.eval_literal(s);
                    module.add(name, value, Scope::Const);
                },
                Statement::HashTbl(i, opt, is_pub) => {
                    let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                    let scope = if is_pub {Scope::Public} else {Scope::Local};
                    module.add(name, hashtbl, scope);
                },
                Statement::Function{name: i, params, body, is_proc} => {
                    let Identifier(func_name) = i;
                    let mut new_body = Vec::new();
                    for statement in body.clone() {
                        match statement {
                            Statement::Public(vec) => {
                                for (i, e) in vec {
                                    let Identifier(member_name) = i;
                                    let value = self.eval_expression(e)?;
                                    module.add(member_name, value, Scope::Public);
                                }
                            },
                            Statement::Const(vec) => {
                                for (i, e) in vec {
                                    let Identifier(member_name) = i;
                                    let value = self.eval_expression(e)?;
                                    module.add(member_name, value, Scope::Const);
                                }
                            },
                            Statement::HashTbl(i, opt, is_pub) => {
                                if is_pub {
                                    let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                                    module.add(name, hashtbl, Scope::Public);
                                }
                            },
                            Statement::Function{name: _, params: _, body: _, is_proc: is_proc2}  => {
                                let in_func = if is_proc2{"procedure"}else{"function"};
                                let out_func = if is_proc{"procedure"}else{"function"};
                                return Err(UError::new(
                                    format!("Nested {}", in_func),
                                    format!("you can not define {} in {}", in_func, out_func),
                                    None
                                ));
                            },
                            _ => new_body.push(statement),
                        };
                    }
                    module.add(
                        func_name.clone(),
                        Object::Function(
                            func_name, params, new_body, is_proc,
                            None
                        ),
                        Scope::Function,
                    );
                },
                _ => return Err(UError::new(
                    "Invalid statement".into(),
                    "".into(),
                    None
                ))
            }
        }
        let rc = Rc::new(RefCell::new(module));
        rc.borrow_mut().set_rc_to_functions(Rc::clone(&rc));
        if is_instance {
            Ok(Object::Instance(Rc::clone(&rc), 0))
        } else {
            Ok(Object::Module(Rc::clone(&rc)))
        }
    }

    fn eval_expression(&mut self, expression: Expression) -> EvalResult<Object> {
        let obj: Object = match expression {
            Expression::Identifier(i) => self.eval_identifier(i)?,
            Expression::Array(v, s) => {
                let capacity = match self.eval_expression(*s)? {
                    Object::Num(n) => n as usize + 1,
                    Object::Empty => v.len(),
                    o => return Err(UError::new(
                        "Array error".into(),
                        format!("invalid index: {}", o),
                        None
                    )),
                };
                let mut array = Vec::with_capacity(capacity);
                for e in v {
                    array.push(self.eval_expression(e)?);
                }
                while array.len() < capacity {
                    array.push(Object::Empty);
                }
                Object::Array(array)
            },
            Expression::Literal(l) => self.eval_literal(l),
            Expression::Prefix(p, r) => {
                let right = self.eval_expression(*r)?;
                self.eval_prefix_expression(p, right)?
            },
            Expression::Infix(i, l, r) => {
                let left = self.eval_expression(*l)?;
                let right = self.eval_expression(*r)?;
                self.eval_infix_expression(i, left, right)?
            },
            Expression::Index(l, i, h) => {
                let left = self.eval_expression(*l)?;
                let index = self.eval_expression(*i)?;
                let hash_enum = if h.is_some() {
                    Some(self.eval_expression(h.unwrap())?)
                } else {
                    None
                };
                self.eval_index_expression(left, index, hash_enum)?
            },
            Expression::AnonymusFunction {params, body, is_proc} => {
                let outer_local = self.env.borrow_mut().get_local_copy();
                Object::AnonFunc(params, body, outer_local, is_proc)
            },
            Expression::FuncCall {func, args} => {
                self.eval_function_call_expression(func, args)?
            },
            Expression::Assign(l, r) => {
                let value = self.eval_expression(*r)?;
                self.eval_assign_expression(*l, value)?
            },
            Expression::CompoundAssign(l, r, i) => {
                let left = self.eval_expression(*l.clone())?;
                let right = self.eval_expression(*r)?;
                let value= self.eval_infix_expression(i, left, right)?;
                self.eval_assign_expression(*l, value)?
            },
            Expression::Ternary {condition, consequence, alternative} => {
                self.eval_ternary_expression(*condition, *consequence, *alternative)?
            },
            Expression::DotCall(l, r) => {
                self.eval_dotcall_expression(*l, *r, false)?
            },
            Expression::Params(p) => return Err(UError::new(
                "Expression evaluation error".into(),
                format!("bad expression: {}", p),
                None
            )),
            Expression::UObject(v) => Object::UObject(Rc::new(RefCell::new(v))),
        };
        Ok(obj)
    }

    fn eval_identifier(&mut self, identifier: Identifier) -> EvalResult<Object> {
        let Identifier(name) = identifier;
        let env = self.env.borrow();
        let obj = match env.get_variable(&name) {
            Some(o) => o,
            None => match env.get_function(&name) {
                Some(o) => o,
                None => match env.get_module(&name) {
                    Some(o) => o,
                    None => match env.get_class(&name) {
                        Some(o) => o,
                        None => return Err(UError::new(
                            "Identifier not found".into(),
                            format!("{}", name),
                            None
                        ))
                    }
                }
            }
        };
        Ok(obj)
    }

    fn eval_prefix_expression(&mut self, prefix: Prefix, right: Object) -> EvalResult<Object> {
        match prefix {
            Prefix::Not => self.eval_not_operator_expression(right),
            Prefix::Minus => self.eval_minus_operator_expression(right),
            Prefix::Plus => self.eval_plus_operator_expression(right),
        }
    }

    fn eval_not_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        let obj = match right {
            Object::Bool(true) => Object::Bool(false),
            Object::Bool(false) => Object::Bool(true),
            Object::Empty => Object::Bool(true),
            Object::Num(n) => {
                Object::Bool(n == 0.0)
            },
            _ => Object::Bool(false)
        };
        Ok(obj)
    }

    fn eval_minus_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        if let Object::Num(n) = right {
            Ok(Object::Num(-n))
        } else {
            Err(UError::new(
                "Prefix - error".into(),
                format!("Not an number {}", right),
                None
            ))
        }
    }

    fn eval_plus_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        if let Object::Num(n) = right {
            Ok(Object::Num(n))
        } else {
            Err(UError::new(
                "Prefix + error".into(),
                format!("Not an number {}", right),
                None
            ))
        }
    }

    fn eval_index_expression(&mut self, left: Object, index: Object, hash_enum: Option<Object>) -> EvalResult<Object> {
        let obj = match left.clone() {
            Object::Array(ref a) => if hash_enum.is_some() {
                return Err(UError::new(
                    "Invalid index".into(),
                    format!("{}[{}, {}]", left, index, hash_enum.unwrap()),
                    None
                ));
            } else if let Object::Num(i) = index {
                self.eval_array_index_expression(a.clone(), i as i64)?
            } else {
                return Err(UError::new(
                    "Invalid index".into(),
                    format!("{}[{}]", left, index),
                    None
                ))
            },
            Object::HashTbl(h) => {
                let mut hash = h.borrow_mut();
                let (key, i) = match index.clone(){
                    Object::Num(n) => (n.to_string(), Some(n as usize)),
                    Object::Bool(b) => (b.to_string(), None),
                    Object::String(s) => (s, None),
                    _ => return Err(UError::new(
                        "Invalid key".into(),
                        format!("{}", index),
                        None
                    ))
                };
                if hash_enum.is_some() {
                    if let Object::Num(n) = hash_enum.clone().unwrap() {
                        match FromPrimitive::from_f64(n).unwrap_or(HashTblEnum::HASH_UNKNOWN) {
                            HashTblEnum::HASH_EXISTS => hash.check(key),
                            HashTblEnum::HASH_REMOVE => hash.remove(key),
                            HashTblEnum::HASH_KEY => if i.is_some() {
                                hash.get_key(i.unwrap())
                            } else {
                                return Err(UError::new(
                                    "Invalid index".into(),
                                    format!("{}[{}, {}]", left, key, n),
                                    None
                                ));
                            },
                            HashTblEnum::HASH_VAL => if i.is_some() {
                                hash.get_value(i.unwrap())
                            } else {
                                return Err(UError::new(
                                    "Invalid index".into(),
                                    format!("{}[{}, {}]", left, key, n),
                                    None
                                ));
                            },
                            _ => return Err(UError::new(
                                "Invalid index".into(),
                                format!("{}[{}, {}]", left, index, n),
                                None
                            ))
                        }
                    } else {
                        return Err(UError::new(
                            "Invalid index".into(),
                            format!("invalid index: {}[{}, {}]", left, index, hash_enum.unwrap()),
                            None
                        ));
                    }
                } else {
                    hash.get(key)
                }
            },
            Object::UChild(u, p) => if hash_enum.is_some() {
                return Err(UError::new(
                    "Invalid index".into(),
                    format!("imvalid index: {}[{}, {}]", left, index, hash_enum.unwrap()),
                    None
                ));
            } else if let Object::Num(n) = index {
                let i = n as usize;
                match u.borrow().pointer(p.as_str()).unwrap().get(i) {
                    Some(v) => Self::eval_uobject(v, Rc::clone(&u), format!("{}/{}", p, i))?,
                    None => return Err(UError::new(
                        "Index out of bound".into(),
                        format!("{}[{}]", left, i),
                        None
                    ))
                }
            } else {
                return Err(UError::new(
                    "Invalid index".into(),
                    format!("imvalid index: {}[{}]", left, index),
                    None
                ));
            },
            o => return Err(UError::new(
                "Not an Array or Hashtable".into(),
                format!("{}", o),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: i64) -> EvalResult<Object> {
        let max = (array.len() as i64) - 1;
        if index < 0 || index > max {
            return Err(UError::new(
                "Index out of bound".into(),
                format!("{}", index),
                None
            ));
        }
        let obj = array.get(index as usize).map_or(Object::Empty, |o| o.clone());
        Ok(obj)
    }

    fn eval_assign_expression(&mut self, left: Expression, value: Object) -> EvalResult<Object> {
        self.eval_instance_assignment(&left, &value)?;
        let mut is_in_scope_auto_disposable = true;
        let instance = match value {
            Object::Instance(_, _) => Some(value.clone()),
            _ => None,
        };
        match left {
            Expression::Identifier(ident) => {
                let Identifier(name) = ident;
                let mut env = self.env.borrow_mut();
                if let Some(Object::This(m)) = env.get_variable(&"this".into()) {
                    // moudele/classメンバであればその値を更新する
                    m.borrow_mut().assign(&name, value.clone(), None)?;
                    is_in_scope_auto_disposable = false;
                }
                is_in_scope_auto_disposable = ! env.assign(name, value)? && is_in_scope_auto_disposable;
            },
            Expression::Index(arr, i, h) => {
                if h.is_some() {
                    return Err(UError::new(
                        "Error on assignment".into(),
                        "comma on index".into(),
                        None
                    ));
                }
                let index = self.eval_expression(*i)?;
                match *arr {
                    Expression::Identifier(ident) => {
                        let Identifier(name) = ident;
                        let obj = self.env.borrow().get_variable(&name);
                        match obj {
                            Some(o) => {
                                match o {
                                    Object::Array(a) => {
                                        let mut arr = a.clone();
                                        match index {
                                            Object::Num(n) => {
                                                let i = n as usize;
                                                if let Some(Object::This(m)) = self.env.borrow().get_variable(&"this".into()) {
                                                    // moudele/classメンバであればその値を更新する
                                                    m.borrow_mut().assign(&name, value.clone(), Some(index))?;
                                                    is_in_scope_auto_disposable = false;
                                                }
                                                if i < arr.len() {
                                                    arr[i] = value;
                                                    is_in_scope_auto_disposable = ! self.env.borrow_mut().assign(name, Object::Array(arr))?;
                                                }
                                            },
                                            _ => return Err(UError::new(
                                                "Invalid index".into(),
                                                format!("{} is not valid index", index),
                                                None
                                            ))
                                        };
                                    },
                                    Object::HashTbl(h) => {
                                        let key = match index {
                                            Object::Num(n) => n.to_string(),
                                            Object::Bool(b) => b.to_string(),
                                            Object::String(s) => s,
                                            _ => return Err(UError::new(
                                                "Invalid key".into(),
                                                format!("{} is not valid key", index),
                                                None
                                            ))
                                        };
                                        let mut hash = h.borrow_mut();
                                        hash.insert(key, value);
                                    },
                                    _ => return Err(UError::new(
                                        "Invalid index call".into(),
                                        format!("{} is neither array nor hashtbl", name),
                                        None
                                    ))
                                };
                            },
                            None => {}
                        };
                    },
                    Expression::DotCall(left, right) => {
                        match self.eval_expression(*left)? {
                            Object::Module(m) |
                            Object::Instance(m, _) |
                            Object::This(m) => {
                                match *right {
                                    Expression::Identifier(Identifier(name)) => {
                                        m.borrow_mut().assign(&name, value, Some(index))?;
                                        is_in_scope_auto_disposable = false;
                                    },
                                    _ => return Err(UError::new(
                                        "Error on assignment".into(),
                                        "syntax error".into(),
                                        None
                                    ))
                                }
                            },
                            // Value::Array
                            Object::UObject(v) => if let Object::Num(n) = index {
                                if let Expression::Identifier(Identifier(name)) = *right {
                                    let i = n as usize;
                                    match v.borrow_mut().get_mut(name.as_str()) {
                                        Some(serde_json::Value::Array(a)) => *a.get_mut(i).unwrap() = Self::object_to_serde_value(value)?,
                                        Some(_) => return Err(UError::new(
                                            "UObject error".into(),
                                            format!("{} is not an array", name),
                                            None
                                        )),
                                        None => return Err(UError::new(
                                            "UObject error".into(),
                                            format!("{} not found", name),
                                            None
                                        )),
                                    };
                                }
                            } else {
                                return Err(UError::new(
                                    "UObject error".into(),
                                    format!("invalid index: {}", index),
                                    None
                                ));
                            },
                            Object::UChild(u, p) => if let Object::Num(n) = index {
                                if let Expression::Identifier(Identifier(name)) = *right {
                                    let i = n as usize;
                                    match u.borrow_mut().pointer_mut(p.as_str()).unwrap().get_mut(name.as_str()) {
                                        Some(serde_json::Value::Array(a)) => *a.get_mut(i).unwrap() = Self::object_to_serde_value(value)?,
                                        Some(_) => return Err(UError::new(
                                            "UObject error".into(),
                                            format!("{} is not an array", name),
                                            None
                                        )),
                                        None => return Err(UError::new(
                                            "UObject error".into(),
                                            format!("{} not found", name),
                                            None
                                        )),
                                    };
                                }
                            } else {
                                return Err(UError::new(
                                    "UObject error".into(),
                                    format!("invalid index: {}", index),
                                    None
                                ));
                            },
                            o => return Err(UError::new(
                                "Error on . operator".into(),
                                format!("not module or object: {}", o),
                                None
                            ))
                        }
                    },
                    _ => return Err(UError::new(
                        "Assignment error".into(),
                        format!("syntax error on assignment: {:?}", *arr),
                        None
                    ))
                };
            },
            Expression::DotCall(left, right) => match self.eval_expression(*left)? {
                Object::Module(m) |
                Object::Instance(m, _) => {
                    match *right {
                        Expression::Identifier(i) => {
                            let Identifier(member_name) = i;
                            m.borrow_mut().assign_public(&member_name, value, None)?;
                            is_in_scope_auto_disposable = false;
                        },
                        _ => return Err(UError::new(
                            "Assignment error".into(),
                            format!("syntax error on assignment"),
                            None
                        )),
                    }
                },
                Object::This(m) => {
                    let mut module = m.borrow_mut();
                    if let Expression::Identifier(Identifier(member)) = *right {
                        module.assign(&member, value, None)?;
                    } else {
                        return Err(UError::new(
                            "Invalid member call".into(),
                            format!("member not found on {}", module.name()),
                            None
                        ));
                    }
                },
                Object::Global => if let Expression::Identifier(Identifier(name)) = *right {
                    is_in_scope_auto_disposable = ! self.env.borrow_mut().assign_public(name, value)?;
                } else {
                    return Err(UError::new(
                        "Error on assignment".into(),
                        "global variable not found".into(),
                        None
                    ))
                },
                Object::UObject(v) => if let Expression::Identifier(Identifier(name)) = *right {
                    match v.borrow_mut().get_mut(name.as_str()) {
                        Some(mut_v) => *mut_v = Self::object_to_serde_value(value)?,
                        None => return Err(UError::new(
                            "UObject".into(),
                            format!("{} not found", name),
                            None
                        ))
                    }
                } else {
                    return Err(UError::new(
                        "UObject".into(),
                        format!("error on assignment"),
                        None
                    ));
                },
                Object::UChild(u, p) => if let Expression::Identifier(Identifier(name)) = *right {
                    match u.borrow_mut().pointer_mut(p.as_str()).unwrap().get_mut(name.as_str()) {
                        Some(mut_v) => *mut_v = Self::object_to_serde_value(value)?,
                        None => return Err(UError::new(
                            "UObject".into(),
                            format!("{} not found", name),
                            None
                        ))
                    }
                } else {
                    return Err(UError::new(
                        "UObject".into(),
                        format!("error on assignment"),
                        None
                    ));
                },
                o => return Err(UError::new(
                    "Error on . operator".into(),
                    format!("not module or object: {}", o),
                    None
                ))
            },
            _ => return Err(UError::new(
                "Invalid assignment".into(),
                format!("not an variable: {:?}", left),
                None
            ))
        }
        if ! is_in_scope_auto_disposable {
            // スコープ内自動破棄対象じゃないインスタンスはグローバルに移す
            if let Some(Object::Instance(ref rc, id)) = instance {
                self.env.borrow_mut().set_instances(Rc::clone(rc), id, true);
                self.env.borrow_mut().remove_variable(format!("@INSTANCE{}", id));
                self.env.borrow_mut().remove_from_instances(id);
            }
        }
        Ok(Object::Empty)
    }

    fn eval_infix_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
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
                    Object::Version(v) => self.eval_infix_number_expression(infix, n1, v.parse()),
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
                    Object::Bool(_) => self.eval_infix_string_expression(infix, s1.clone(), format!("{}", right)),
                    Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                    Object::Version(v) => self.eval_infix_string_expression(infix, s1, v.to_string()),
                    _ => self.eval_infix_string_expression(infix, s1.clone(), format!("{}", right))
                }
            },
            Object::Bool(l) => match right {
                Object::Bool(b) => self.eval_infix_logical_operator_expression(infix, l, b),
                Object::String(s) => self.eval_infix_string_expression(infix, format!("{}", left), s.clone()),
                Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                Object::Num(n) => self.eval_infix_number_expression(infix, l as i64 as f64, n),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Empty => match right {
                Object::Num(n) => self.eval_infix_number_expression(infix, 0.0, n),
                Object::String(_) => self.eval_infix_empty_expression(infix, left, right),
                Object::Empty => self.eval_infix_empty_expression(infix, left, right),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Version(v1) => match right {
                Object::Version(v2) => self.eval_infix_number_expression(infix, v1.parse(), v2.parse()),
                Object::Num(n) => self.eval_infix_number_expression(infix, v1.parse(), n),
                Object::String(s) => self.eval_infix_string_expression(infix, v1.to_string(), s.clone()),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            _ => self.eval_infix_misc_expression(infix, left, right)
        }
    }

    fn eval_infix_misc_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => if let Object::String(s) = right {
                Object::String(format!("{}{}", left, s.clone()))
            } else {
                return Err(UError::new(
                    "Infix error".into(),
                    format!("mismatched type: {} {} {}", left, infix, right),
                    None
                ))
            },
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                "Infix error".into(),
                format!("mismatched type: {} {} {}", left, infix, right),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_infix_number_expression(&mut self, infix: Infix, left: f64, right: f64) -> EvalResult<Object> {
        let obj = match infix {
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
            Infix::Assign => return Err(UError::new(
                "Infix error".into(),
                format!("you can not assign variable in expression: {} {} {}", left, infix, right),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_infix_string_expression(&mut self, infix: Infix, left: String, right: String) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                "Infix error".into(),
                format!("bad operator: {} {} {}", left, infix, right),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_infix_empty_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                "Infix error".into(),
                format!("bad operator: {} {} {}", left, infix, right),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_infix_logical_operator_expression(&mut self, infix: Infix, left: bool, right: bool) -> EvalResult<Object> {
        let obj = match infix {
            Infix::And => Object::Bool(left && right),
            Infix::Or => Object::Bool(left || right),
            _ => self.eval_infix_number_expression(infix, left as i64 as f64, right as i64 as f64)?
        };
        Ok(obj)
    }

    fn eval_literal(&mut self, literal: Literal) -> Object {
        match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::ExpandableString(value) => self.expand_string(value, true),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects),
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
            Literal::TextBlock(text, is_ex) => if is_ex {
                Object::ExpandableTB(text)
            } else {
                self.expand_string(text, false)
            },
        }
    }

    fn expand_string(&self, string: String, expand_var: bool) -> Object {
        let re = Regex::new("<#([^>]+)>").unwrap();
        let mut new_string = string.clone();
        for cap in re.captures_iter(string.as_str()) {
            let expandable = cap.get(1).unwrap().as_str();
            let rep_to: Option<Cow<str>> = match expandable.to_ascii_uppercase().as_str() {
                "CR" => Some("\r\n".into()),
                "TAB" => Some("\t".into()),
                "DBL" => Some("\"".into()),
                text => if expand_var {
                    self.env.borrow().get_variable(&text.into()).map(|o| format!("{}", o).into())
                } else {
                    continue;
                },
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    fn eval_array_literal(&mut self, objects: Vec<Expression>) -> Object {
        Object::Array(
            objects.iter().map(
                |e| self.eval_expression(e.clone()).unwrap()
            ).collect::<Vec<_>>()
        )
    }

    fn eval_expression_for_func_call(&mut self, expression: Expression) -> EvalResult<Object> {
        // 関数定義から探してなかったら変数を見る
        match expression {
            Expression::Identifier(i) => {
                let Identifier(name) = i;
                let env = self.env.borrow();
                match env.get_function(&name) {
                    Some(o) => Ok(o),
                    None => match env.get_class(&name) {
                        Some(o) => Ok(o),
                        None => match env.get_variable(&name) {
                            Some(o) => Ok(o),
                            None => return Err(UError::new(
                                "Invalid Identifier".into(),
                                format!("function not found: {}", &name),
                                None
                            )),
                        }
                    }
                }
            },
            Expression::DotCall(left, right) => Ok(
                self.eval_dotcall_expression(*left, *right, true)?
            ),
            _ => Ok(self.eval_expression(expression)?)
        }
    }

    fn builtin_func_result(&mut self, result: Object) -> EvalResult<Object> {
        let obj = match result {
            Object::Eval(s) => {
                let mut parser = Parser::new(Lexer::new(&s));
                let program = parser.parse();
                let errors = parser.get_errors();
                if errors.len() > 0 {
                    let mut parse_errors = String::new();
                    for pe in &errors {
                        if parse_errors.len() > 0 {
                            parse_errors = format!("{}, {}", parse_errors, pe);
                        } else {
                            parse_errors = format!("{}", pe);
                        }
                    }
                    return Err(UError::new(
                        format!("Eval parse error[{}]:", &errors.len()),
                        parse_errors,
                        None
                    ));
                }
                self.eval(program)?.map_or(Object::Empty, |o| o)
            },
            Object::Debug(t) => match t {
                DebugType::GetEnv => {
                    self.env.borrow().get_env()
                },
                DebugType::ListModuleMember(name) => {
                    self.env.borrow_mut().get_module_member(&name)
                },
                DebugType::BuiltinConstName(e) => {
                    if let Some(Expression::Identifier(Identifier(name))) = e {
                        self.env.borrow().get_name_of_builtin_consts(&name)
                    } else {
                        Object::Empty
                    }
                }
            },
            _ => result
        };
        Ok(obj)
    }

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>) -> EvalResult<Object> {
        type Argument = (Option<Expression>, Object);
        let mut arguments: Vec<Argument> = vec![];
        for arg in args {
            arguments.push((Some(arg.clone()), self.eval_expression(arg)?));
        }

        let (
            mut params,
            body,
            is_proc,
            anon_outer,
            rc_module,
            is_class_instance,
        ) = match self.eval_expression_for_func_call(*func)? {
            Object::DestructorNotFound => return Ok(Object::Empty),
            Object::Function(_, p, b, is_proc, obj) => (p, b, is_proc, None, obj, false),
            Object::AnonFunc(p, b, o, is_proc) =>  (p, b, is_proc, Some(o), None, false),
            Object::BuiltinFunction(name, expected_param_len, f) => {
                if expected_param_len >= arguments.len() as i32 {
                    let res = f(BuiltinFuncArgs::new(name, arguments))?;
                    return self.builtin_func_result(res);
                } else {
                    let l = arguments.len();
                    return Err(UError::new(
                        "Too many arguments".into(),
                        format!(
                            "{} argument{} were given, should be {}{}",
                            l, if l > 1 {"s"} else {""}, expected_param_len, if l > 1 {" (or less)"} else {""}
                        ),
                        None
                    ));
                }
            },
            // class constructor
            Object::Class(name, block) => {
                let instance = self.eval_module_statement(&name, block, true)?;
                if let Object::Instance(rc, _) = instance {
                    let constructor = match rc.borrow().get_function(&name) {
                        Ok(o) => o,
                        Err(_) => return Err(UError::new(
                            "Constructor not found".into(),
                            format!("you must define procedure {}()", &name),
                            None
                        ))
                    };
                    if let Object::Function(_, p, b, _, _) = constructor {
                        (p, b, false, None, Some(Rc::clone(&rc)), true)
                    } else {
                        return Err(UError::new(
                            "Syntax Error".into(),
                            format!("{} is not valid constructor", &name),
                            None
                        ));
                    }
                } else {
                    return Err(UError::new(
                        "Syntax Error".into(),
                        format!("{} is not a class", &name),
                        None
                    ));
                }
            },
            o => return Err(UError::new(
                "Not a function".into(),
                format!("{}", o),
                None
            )),
        };
        let org_param_len = params.len();
        if params.len() > arguments.len() {
            arguments.resize(params.len(), (None, Object::Empty));
        } else if params.len() < arguments.len() {
            params.resize(arguments.len(), Expression::Params(Params::VariadicDummy));
        }

        if anon_outer.is_some() {
            self.env.borrow_mut().copy_scope(anon_outer.unwrap());
        } else {
            self.env.borrow_mut().new_scope();
        }
        let list = params.into_iter().zip(arguments.into_iter());
        let mut variadic = vec![];
        let mut variadic_name = String::new();
        let mut reference = vec![];
        for (_, (e, (arg_e, o))) in list.enumerate() {
            let param = match e {
                Expression::Params(p) => p,
                _ => return Err(UError::new(
                    "Invalid parameter".into(),
                    format!("bad parameter: {:?}", e),
                    None
                ))
            };
            let (name, value) = match param {
                Params::Identifier(i) => {
                    let Identifier(name) = i;
                    if arg_e.is_none() {
                        return Err(UError::new(
                            "argument required".into(),
                            format!("{}", name),
                            None
                        ));
                    }
                    (name, o.clone())
                },
                Params::Reference(i) => {
                    let Identifier(name) = i.clone();
                    let e = arg_e.unwrap();
                    match e {
                        Expression::Array(_, _) |
                        Expression::Assign(_, _) |
                        Expression::CompoundAssign(_, _, _) |
                        Expression::Params(_) => return Err(UError::new(
                            "Invalid argument".into(),
                            format!("{}", name),
                            None
                        )),
                        _ => reference.push((name.clone(), e))
                    };
                    (name, o.clone())
                },
                Params::Array(i, b) => {
                    let Identifier(name) = i;
                    let e = arg_e.unwrap();
                    match e {
                        Expression::Identifier(_) |
                        Expression::Index(_, _, _) |
                        Expression::DotCall(_, _) => {
                            if b {
                                reference.push((name.clone(), e));
                            }
                            (name, o.clone())
                        },
                        Expression::Literal(Literal::Array(_)) => {
                            (name, o.clone())
                        },
                        _ => return Err(UError::new(
                            "Invalid argument".into(),
                            format!("{}", name),
                            None
                        )),
                    }
                },
                Params::WithDefault(i, default) => {
                    let Identifier(name) = i;
                    if o == Object::Empty {
                        (name, self.eval_expression(*default)?)
                    } else {
                        (name, o.clone())
                    }
                },
                Params::Variadic(i) => {
                    let Identifier(name) = i;
                    variadic_name = name.clone();
                    variadic.push(o.clone());
                    continue;
                },
                Params::VariadicDummy => {
                    if variadic.len() < 1 {
                        return Err(UError::new(
                            "Too many arguments".into(),
                            format!("should be less than or equal to {}", org_param_len),
                            None
                        ))
                    }
                    variadic.push(o.clone());
                    continue;
                }
            };
            if variadic.len() == 0 {
                self.env.borrow_mut().define_local(name, value)?;
            }
        }
        if variadic.len() > 0 {
            self.env.borrow_mut().define_local(variadic_name, Object::Array(variadic))?;
        }

        match rc_module {
            Some(ref m) => {
                self.env.borrow_mut().set_module_private_member(m);
            },
            None => {},
        };

        if ! is_proc {
            // resultにEMPTYを入れておく
            self.env.borrow_mut().assign("result".into(), Object::Empty)?;
        }

        // 関数実行
        self.eval_block_statement(body)?;

        // 戻り値
        let result = if is_class_instance {
            match rc_module {
                Some(ref rc) => Object::Instance(Rc::clone(rc), self.new_instance_id()),
                None => return Err(UError::new(
                    "Syntax error".into(),
                    "failed to create new instance".into(),
                    None
                )),
            }
        } else if is_proc {
            Object::Empty
        } else {
            match self.env.borrow_mut().get_variable(&"result".to_string()) {
                Some(o) => o,
                None => Object::Empty
            }
        };
        // 参照渡し
        let mut ref_values = vec![];
        let mut do_not_dispose = vec![];
        for (p_name, _) in reference.clone() {
            let obj = self.env.borrow_mut().get_variable(&p_name).unwrap();
            match obj {
                Object::Instance(_, id) => do_not_dispose.push(format!("@INSTANCE{}", id)),
                _ => {},
            }
            ref_values.push(obj);
        }
        match result {
            Object::Instance(_, id) => do_not_dispose.push(format!("@INSTANCE{}", id)),
            _ => {},
        }

        // このスコープのインスタンスを破棄
        self.auto_dispose_instances(do_not_dispose, false);

        // 関数スコープを抜ける
        self.env.borrow_mut().restore_scope();

        for ((_, e), o) in reference.iter().zip(ref_values.iter()) {
            // Expressionが代入可能な場合のみ代入処理を行う
            match e {
                Expression::Identifier(_) |
                Expression::Index(_, _, _) |
                Expression::DotCall(_, _) => {
                    self.eval_assign_expression(e.clone(), o.clone())?;
                    // 参照渡しでインスタンスを帰す場合は自動破棄対象とする
                    match o {
                        Object::Instance(ref rc, id) => {
                            self.env.borrow_mut().set_instances(Rc::clone(rc), *id, false);
                        },
                        _ => {},
                    }
                },
                _ => {},
            };
        };

        // 戻り値がインスタンスなら自動破棄されるようにしておく
        match result {
            Object::Instance(ref rc, id) => {
                self.env.borrow_mut().set_instances(Rc::clone(rc), id, false);
            },
            _ => {},
        }

        Ok(result)
    }

    fn auto_dispose_instances(&mut self, refs: Vec<String>, include_global: bool) {
        let ins_list = self.env.borrow_mut().get_instances();
        for ins_name in ins_list {
            if ! refs.contains(&ins_name) {
                let obj = self.env.borrow_mut().get_tmp_instance(&ins_name, false).unwrap_or(Object::Empty);
                if let Object::Instance(ins, _) = obj {
                    let destructor = Expression::DotCall(
                        Box::new(Expression::Identifier(Identifier(ins_name))),
                        Box::new(Expression::Identifier(Identifier(format!("_{}_", ins.borrow().name())))),
                    );
                    self.eval_function_call_expression(Box::new(destructor), vec![]).ok();
                    ins.borrow_mut().dispose();
                }
            }
        }
        if include_global {
            let ins_list = self.env.borrow_mut().get_global_instances();
            for ins_name in ins_list {
                let obj = self.env.borrow_mut().get_tmp_instance(&ins_name, false).unwrap_or(Object::Empty);
                if let Object::Instance(ins, _) = obj {
                    let destructor = Expression::DotCall(
                        Box::new(Expression::Identifier(Identifier(ins_name))),
                        Box::new(Expression::Identifier(Identifier(format!("_{}_", ins.borrow().name())))),
                    );
                    self.eval_function_call_expression(Box::new(destructor), vec![]).ok();
                    ins.borrow_mut().dispose();
                }
            }
        }
    }

    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> EvalResult<Object> {
        let condition = self.eval_expression(condition)?;
        if Self::is_truthy(condition) {
            self.eval_expression(consequence)
        } else {
            self.eval_expression(alternative)
        }
    }

    fn eval_dotcall_expression(&mut self, left: Expression, right: Expression, is_func: bool) -> EvalResult<Object> {
        let instance = match left {
            Expression::Identifier(_) |
            Expression::Index(_, _, _) |
            Expression::FuncCall{func:_, args:_} |
            Expression::DotCall(_, _) |
            Expression::UObject(_) => {
                self.eval_expression(left)?
            },
            _ => return Err(UError::new(
                "Error on . operator".into(),
                format!("invalid expression: {:?}", left),
                None
            )),
        };
        match instance {
            Object::Module(m) |
            Object::Instance(m, _) => {
                let module = m.borrow();
                match right {
                    Expression::Identifier(i) => {
                        let Identifier(member_name) = i;
                        if module.is_local_member(&member_name) {
                            if let Some(Object::This(m)) = self.env.borrow().get_variable(&"this".into()) {
                                if module.name() == m.borrow().name() {
                                    return module.get_member(&member_name);
                                }
                            }
                            Err(UError::new(
                                "Access denied".into(),
                                format!("you can not access to {}.{}", module.name(), member_name),
                                None
                            ))
                        } else if is_func {
                            module.get_function(&member_name)
                        } else {
                            match module.get_public_member(&member_name) {
                                Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true)),
                                res => res
                            }
                        }
                    },
                    _ => Err(UError::new(
                        "Error on . operator".into(),
                        "member does not exist".into(),
                        None
                    )),
                }
            },
            Object::This(m) => {
                let module = m.borrow();
                if let Expression::Identifier(i) = right {
                    let Identifier(member_name) = i;
                    if is_func {
                        module.get_function(&member_name)
                    } else {
                        match module.get_member(&member_name) {
                            Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true)),
                            res => res
                        }
                    }
                } else {
                    Err(UError::new(
                        "Function not found".into(),
                        format!("member not found on {}", module.name()),
                        None
                    ))
                }
            },
            Object::Global => {
                if let Expression::Identifier(Identifier(g_name)) = right {
                    self.env.borrow().get_global(&g_name, is_func)
                } else {
                    Err(UError::new(
                        "Global".into(),
                        format!("not an identifier ({:?})", right),
                        None
                    ))
                }
            },
            Object::Class(name, _) => Err(UError::new(
                "Invalid Class call".into(),
                format!("{0} can not be called directly; try {0}() to create instance", name),
                None
            )),
            Object::UObject(u) => if let Expression::Identifier(Identifier(key)) = right {
                match u.borrow().get(key.as_str()) {
                    Some(v) => Self::eval_uobject(v, Rc::clone(&u), format!("/{}", key)),
                    None => Err(UError::new(
                        "UObject".into(),
                        format!("{} not found", key),
                        None
                    )),
                }
            } else {
                Err(UError::new(
                    "UObject".into(),
                    format!("not an identifier ({:?})", right),
                    None
                ))
            },
            Object::UChild(u,p) => if let Expression::Identifier(Identifier(key)) = right {
                match u.borrow().pointer(p.as_str()).unwrap().get(key.as_str()) {
                    Some(v) => Self::eval_uobject(v, Rc::clone(&u), format!("{}/{}", p, key)),
                    None => Err(UError::new(
                        "UObject".into(),
                        format!("{} not found", key),
                        None
                    ))
                }
            } else {
                Err(UError::new(
                    "UObject".into(),
                    format!("not an identifier ({:?})", right),
                    None
                ))
            },
            o => Err(UError::new(
                ". operator not supported".into(),
                format!("{}", o),
                None
            )),
        }
    }

    // UObject
    fn eval_uobject(v: &serde_json::Value, top: Rc<RefCell<serde_json::Value>>, pointer: String) -> EvalResult<Object> {
        let o = match v {
            serde_json::Value::Null => Object::Null,
            serde_json::Value::Bool(b) => Object::Bool(*b),
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(f) => Object::Num(f),
                None => return Err(UError::new(
                    "UObject error".into(),
                    format!("can not convert {} to number", n),
                    None
                )),
            },
            serde_json::Value::String(s) => Object::String(s.clone()),
            serde_json::Value::Array(_) |
            serde_json::Value::Object(_) => Object::UChild(top, pointer),
        };
        Ok(o)
    }

    fn object_to_serde_value(o: Object) -> EvalResult<serde_json::Value> {
        let v = match o {
            Object::Null => serde_json::Value::Null,
            Object::Bool(b) => serde_json::Value::Bool(b),
            Object::Num(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
            Object::String(ref s) => serde_json::Value::String(s.clone()),
            Object::UObject(u) => u.borrow().clone(),
            Object::UChild(u, p) => u.borrow().pointer(p.as_str()).unwrap().clone(),
            _ => return Err(UError::new(
                "UObject error".into(),
                format!("can not convert {} to uobject", o),
                None
            )),
        };
        Ok(v)
    }

    fn eval_instance_assignment(&mut self, left: &Expression, new_value: &Object) -> EvalResult<()> {
        let old_value = match self.eval_expression(left.clone()) {
            Ok(o) => o,
            Err(_) => return Ok(())
        };
        if let Object::Instance(ref m, _) = old_value {
            // 既に破棄されてたらなんもしない
            if m.borrow().is_disposed() {
                return Ok(());
            }
            // Nothingが代入される場合は明示的にデストラクタを実行及びdispose()
            if new_value == &Object::Nothing {
                let destructor = Expression::DotCall(
                    Box::new(left.clone()),
                    Box::new(Expression::Identifier(Identifier(format!("_{}_", m.borrow().name())))),
                );
                self.eval_function_call_expression(Box::new(destructor), vec![])?;
                m.borrow_mut().dispose();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_test(input: &str, expected: Option<Object>, ast: bool) {
        match eval(input, ast) {
            Ok(o) => assert_eq!(o, expected),
            Err(err) => panic!("{}\n{}", err, input)
        }
    }

    fn eval(input: &str, ast: bool) -> EvalResult<Option<Object>> {
        let mut e = Evaluator::new(Rc::new(RefCell::new(
            Environment::new(vec![])
        )));
        let program = Parser::new(Lexer::new(input)).parse();
        if ast {
            println!("{:?}", program);
        }
        e.eval(program)
    }

    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        let mut e = Evaluator::new(Rc::new(RefCell::new(
            Environment::new(vec![])
        )));
        let program = Parser::new(Lexer::new(input)).parse();
        match e.eval(program) {
            Ok(_) => e,
            Err(err) => panic!("{}", err)
        }
    }

    //
    fn eval_test_with_env(e: &mut Evaluator, input: &str, expected: Option<Object>) {
        let program = Parser::new(Lexer::new(input)).parse();
        match e.eval(program) {
            Ok(o) => assert_eq!(o, expected),
            Err(err) => panic!("{}", err)
        }
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
//             (
//                 r#"
// dim hoge = 2
// dim hoge = 3
//                 "#,
//                 Some(Object::Error("HOGE is already defined.".to_string()))
//             ),
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
        eval_test(input, Some(Object::Empty), false);
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
//             (
//                 r#"
// for i = 0 to "5s"
// next
//                 "#,
//                 Some(Object::Error("syntax error: for i = 0 to 5s".to_string()))
//             ),
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
//             (
//                 r#"
// dim dim_and_dim = 1
// dim dim_and_dim = 2
//                 "#,
//                 Some(Object::Error("DIM_AND_DIM is already defined.".to_string()))
//             ),
//             (
//                 r#"
// public pub_and_const = 1
// const pub_and_const = 2
//                 "#,
//                 Some(Object::Error("PUB_AND_CONST is already defined.".to_string()))
//             ),
//             (
//                 r#"
// const const_and_const = 1
// const const_and_const = 2
//                 "#,
//                 Some(Object::Error("CONST_AND_CONST is already defined.".to_string()))
//             ),
            (
                r#"
public public_and_public = 1
public public_and_public = 2
                "#,
                None
            ),
//             (
//                 r#"
// hashtbl hash_and_hash
// hashtbl hash_and_hash
//                 "#,
//                 Some(Object::Error("HASH_AND_HASH is already defined.".to_string()))
//             ),
//             (
//                 r#"
// function func_and_func()
// fend
// function func_and_func()
// fend
//                 "#,
//                 Some(Object::Error("FUNC_AND_FUNC is already defined.".to_string()))
//             ),
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

    function get_m_v()
        result = M.v
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

    dim a = 1
    function get_a()
        result = a
    fend
    function set_a(n)
        a = n
        result = get_a()
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
            // (
            //     "get_v()",
            //     Some(Object::Error("identifier not found: v".to_string()))
            // ),
            // (
            //     "M.v",
            //     Some(Object::Error("you can not access to M.v".to_string()))
            // ),
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
                "M.get_m_v()",
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
            (
                "M.set_a(5)",
                Some(Object::Num(5.0))
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