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
use crate::settings::usettings_singleton;

use std::fmt;
use std::borrow::Cow;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::sync::{Arc, Mutex};

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
    pub fn new(title: &str, msg: &str, sub_msg: Option<&str>) -> Self {
        UError{
            title: title.to_string(),
            msg: msg.to_string(),
            sub_msg: sub_msg.map(|s| s.to_string())
        }
    }
}

impl fmt::Display for UError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.sub_msg {
            Some(ref sub) => write!(f, "{}: {} [{}]", self.title, self.msg, sub),
            None => write!(f, "{}: {}", self.title, self.msg)
        }
    }
}

type EvalResult<T> = Result<T, UError>;

#[derive(Debug, Clone)]
pub struct  Evaluator {
    env: Environment,
    instance_id: Arc<Mutex<u32>>,
}

impl Evaluator {
    pub fn new(env: Environment) -> Self {
        Evaluator {env, instance_id: Arc::new(Mutex::new(0))}
    }

    fn is_truthy(obj: Object) -> bool {
        match obj {
            Object::Empty | Object::Bool(false) => false,
            Object::Num(n) => {
                n != 0.0
            },
            Object::Handle(h) => ! h.is_null(),
            _ => true
        }
    }

    fn new_instance_id(&mut self) -> u32 {
        let mut instance_id = self.instance_id.lock().unwrap();
        *instance_id += 1;
        instance_id.clone()
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
                    Object::ExitExit(n) => {
                        std::process::exit(n);
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
                    Object::Exit |
                    Object::ExitExit(_) => return Ok(Some(o)),
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
                    "Error on hashtbl definition",
                    &format!("invalid hashtbl option: {}", o),
                    None
                ))
            },
            None => 0
        };
        let sort = (opt & HashTblEnum::HASH_SORT as u32) > 0;
        let casecare = (opt & HashTblEnum::HASH_CASECARE as u32) > 0;
        let hashtbl = HashTbl::new(sort, casecare);
        Ok((name, Object::HashTbl(Arc::new(Mutex::new(hashtbl)))))
    }

    fn eval_print_statement(&mut self, expression: Expression) -> EvalResult<Option<Object>> {
        let obj = self.eval_expression(expression)?;
        out_log(&format!("{}", obj), LogType::Print);
        println!("{}", obj);
        Ok(None)
    }

    fn set_option_settings(&self, opt: OptionSetting) {
        let singleton = usettings_singleton(None);
        let mut usettings = singleton.0.lock().unwrap();
        match opt {
            OptionSetting::Explicit(b) => usettings.options.explicit = b,
            OptionSetting::SameStr(b) => usettings.options.same_str = b,
            OptionSetting::OptPublic(b) => usettings.options.opt_public = b,
            OptionSetting::OptFinally(b) => usettings.options.opt_finally = b,
            OptionSetting::SpecialChar(_) => {},
            OptionSetting::ShortCircuit(b) => usettings.options.short_circuit = b,
            OptionSetting::NoStopHotkey(b) => usettings.options.no_stop_hot_key = b,
            OptionSetting::TopStopform(_) => {},
            OptionSetting::FixBalloon(b) => usettings.options.fix_balloon = b,
            OptionSetting::Defaultfont(ref s) => {
                if let Object::String(s) = self.expand_string(s.clone(), true) {
                    usettings.options.default_font = s.clone()
                }
            },
            OptionSetting::Position(x, y) => {
                usettings.options.position.left = x;
                usettings.options.position.top = y;
            },
            OptionSetting::Logpath(ref s) => {
                if let Object::String(s) = self.expand_string(s.clone(), true) {
                    let mut path = PathBuf::from(&s);
                    if path.is_dir() {
                        path.push("uwscr.log");
                    }
                    env::set_var("UWSCR_LOG_FILE", path.as_os_str());
                    usettings.options.log_path = Some(s);
                }
            },
            OptionSetting::Loglines(n) => {
                env::set_var("UWSCR_LOG_LINES", &n.to_string());
                usettings.options.log_lines = n as u32;
            },
            OptionSetting::Logfile(n) => {
                let n = if n < 0 || n > 4 {1} else {n};
                env::set_var("UWSCR_LOG_TYPE", n.to_string());
                usettings.options.log_file = n as u8;
            },
            OptionSetting::Dlgtitle(ref s) => {
                if let Object::String(ref s) = self.expand_string(s.clone(), true) {
                    env::set_var("UWSCR_DEFAULT_TITLE", s.as_str());
                    usettings.options.dlg_title = Some(s.to_string());
                }
            },
        }
    }

    fn eval_statement(&mut self, statement: Statement) -> EvalResult<Option<Object>> {
        match statement {
            Statement::Option(opt) => {
                self.set_option_settings(opt);
                Ok(None)
            },
            Statement::Dim(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_local(name, value)?;
                }
                Ok(None)
            },
            Statement::Public(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_public(name, value)?;
                }
                Ok(None)
            },
            Statement::Const(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_const(name, value)?;
                }
                Ok(None)
            },
            Statement::TextBlock(i, s) => {
                let Identifier(name) = i;
                let value = self.eval_literal(s)?;
                self.env.define_const(name, value)?;
                Ok(None)
            },
            Statement::HashTbl(v) => {
                for (i, hashopt, is_public) in v {
                    let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, hashopt)?;
                    if is_public {
                        self.env.define_public(name, hashtbl)?;
                    } else {
                        self.env.define_local(name, hashtbl)?;
                    }
                }
                Ok(None)
            },
            Statement::Print(e) => self.eval_print_statement(e),
            Statement::Call(block, args) => {
                let params_str = Expression::Literal(Literal::Array(args));
                let arguments = vec![
                    (Some(params_str.clone()), self.eval_expression(params_str)?)
                ];
                let call_res = self.invoke_user_function(
                    vec![
                        Expression::Params(Params::Identifier(Identifier("PARAM_STR".into())))
                    ],
                    arguments,
                    block,
                    true,
                    None,
                    None,
                    false
                )?;
                Ok(Some(call_res))
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
            Statement::Function {name, params, body, is_proc, is_async} => {
                let Identifier(fname) = name;
                let func = self.eval_funtcion_definition_statement(&fname, params, body, is_proc, is_async)?;
                self.env.define_function(fname, func)?;
                Ok(None)
            },
            Statement::Module(i, block) => {
                let Identifier(name) = i;
                let module = self.eval_module_statement(&name, block, false)?;
                self.env.define_module(name.clone(), module)?;
                // コンストラクタがあれば実行する
                let module = self.env.get_module(&name);
                if let Some(Object::Module(m)) = module {
                    let constructor = m.lock().unwrap().get_constructor();
                    match constructor {
                        Some(o) => {
                            self.invoke_function_object(o, vec![])?;
                        },
                        None => {}
                    }
                };
                Ok(None)
            },
            Statement::Class(i, block) => {
                let Identifier(name) = i;
                let class = Object::Class(name.clone(), block);
                self.env.define_class(name.clone(), class)?;
                Ok(None)
            },
            Statement::With(o_e, block) => if let Some(e) = o_e {
                let s = self.eval_block_statement(block);
                if let Expression::Identifier(Identifier(name)) = e {
                    if name.find("@with_tmp_").is_some() {
                        self.eval_instance_assignment(&Expression::Identifier(Identifier(name.clone())), &Object::Nothing)?;
                        self.env.remove_variable(name);
                    }
                }
                s
            } else {
                self.eval_block_statement(block)
            },
            Statement::Enum(name, uenum) => self.eval_enum_statement(name, uenum),
            Statement::Thread(e) => self.eval_thread_statement(e),
            Statement::Try {trys, except, finally} => self.eval_try_statement(trys, except, finally),
            Statement::Exit => Ok(Some(Object::Exit)),
            Statement::ExitExit(n) => Ok(Some(Object::ExitExit(n))),
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
                let case_obj = self.eval_expression(e)?;
                if self.is_equal(&case_obj, &select_obj)? {
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
                    Object::Exit |
                    Object::ExitExit(_) |
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
                        "Syntax error on For",
                        &format!("for {} = {}", var, s),
                        None
                    ))
                }
            },
            o => return Err(UError::new(
                "Syntax error on For",
                &format!("for {} = {}", var, o),
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
                        "Syntax error on For",
                        &format!("for {} = {} to {}", var, counter, s),
                        None
                    ))
                }
            },
            o => return Err(UError::new(
                "Syntax error on For",
                &format!("for {} = {} to {}", var, counter, o),
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
                                "Syntax error on For",
                                &format!("for {} = {} to {} step {}", var, counter, counter_end, s),
                                None
                            ))
                        }
                    },
                    o => return Err(UError::new(
                        "Syntax error on For",
                        &format!("for {} = {} to {} step {}", var, counter, counter_end, o),
                        None
                    )),
                }
            },
            None => 1
        };
        if step == 0 {
            return Err(UError::new(
                "Syntax error on For".into(),
                "step can not be 0".into(),
                None
            ));
        }
        self.env.assign(var.clone(), Object::Num(counter as f64))?;
        loop {
            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                break;
            }
            match self.eval_loopblock_statement(block.clone())? {
                Some(o) => match o {
                        Object::Continue(n) => if n > 1 {
                            return Ok(Some(Object::Continue(n - 1)));
                        } else {
                            counter += step;
                            self.env.assign(var.clone(), Object::Num(counter as f64))?;
                            continue;
                        },
                        Object::Break(n) => if n > 1 {
                            return Ok(Some(Object::Break(n - 1)));
                        } else {
                            break;
                        },
                        o => return Ok(Some(o))
                },
                _ => ()
            };
            counter += step;
            self.env.assign(var.clone(), Object::Num(counter as f64))?;
        }
        Ok(None)
    }

    fn eval_for_in_statement(&mut self, loopvar: Identifier, collection: Expression, block: BlockStatement) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let col_obj = match self.eval_expression(collection)? {
            Object::Array(a) => a,
            Object::String(s) => s.chars().map(|c| Object::String(c.to_string())).collect::<Vec<Object>>(),
            Object::HashTbl(h) => h.lock().unwrap().keys(),
            _ => return Err(UError::new(
                "For-In error",
                &format!("for-in requires array, hashtable, string, or collection"),
                None
            ))
        };

        for o in col_obj {
            self.env.assign(var.clone(), o)?;
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
                None => {},
                o => return Ok(o),
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
                None => {},
                o => return Ok(o),
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
                None => {},
                o => return Ok(o),
            };
            if self.eval_loop_flg_expression(expression.clone())? {
                break;
            }
        }
        Ok(None)
    }

    fn eval_funtcion_definition_statement(&mut self, name: &String, params: Vec<Expression>, body: Vec<Statement>, is_proc: bool, is_async: bool) -> EvalResult<Object> {
        for statement in body.clone() {
            match statement {
                Statement::Function{name: _, params: _, body: _, is_proc: _, is_async: _}  => {
                    return Err(UError::new(
                        "Function defining error",
                        &format!("nested definition of function/procedure is not allowed"),
                        None
                    ))
                },
                _ => {},
            };
        }
        if is_async {
            Ok(Object::AsyncFunction(name.clone(), params, body, is_proc, None))
        } else {
            Ok(Object::Function(name.clone(), params, body, is_proc, None))
        }
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
                    let value = self.eval_literal(s)?;
                    module.add(name, value, Scope::Const);
                },
                Statement::HashTbl(v) => {
                    for (i, opt, is_pub) in v {
                        let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                        let scope = if is_pub {Scope::Public} else {Scope::Local};
                        module.add(name, hashtbl, scope);
                    }
                },
                Statement::Function{name: i, params, body, is_proc, is_async} => {
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
                            Statement::HashTbl(v) => {
                                for (i, opt, is_pub) in v {
                                    if is_pub {
                                        let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                                        module.add(name, hashtbl, Scope::Public);
                                    }
                                }
                            },
                            Statement::Function{name: _, params: _, body: _, is_proc: is_proc2, is_async: _}  => {
                                let in_func = if is_proc2{"procedure"}else{"function"};
                                let out_func = if is_proc{"procedure"}else{"function"};
                                return Err(UError::new(
                                    &format!("Nested {}", in_func),
                                    &format!("you can not define {} in {}", in_func, out_func),
                                    None
                                ));
                            },
                            _ => new_body.push(statement),
                        };
                    }
                    module.add(
                        func_name.clone(),
                        if is_async {
                            Object::AsyncFunction(func_name, params, new_body, is_proc, None)
                        } else {
                            Object::Function(func_name, params, new_body, is_proc, None)
                        },
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
        let m = Arc::new(Mutex::new(module));
        m.lock().unwrap().set_module_reference_to_member_functions(Arc::clone(&m));
        if is_instance {
            Ok(Object::Instance(Arc::clone(&m), 0))
        } else {
            Ok(Object::Module(Arc::clone(&m)))
        }
    }

    fn eval_try_statement(&mut self, trys: BlockStatement, except: Option<BlockStatement>, finally: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let obj = match self.eval_block_statement(trys) {
            Ok(opt) => opt,
            Err(e) => {
                self.env.set_try_error_messages(
                    format!("{}", e),
                    format!("")
                );
                if except.is_some() {
                    self.eval_block_statement(except.unwrap())?
                } else {
                    None
                }
            },
        };
        let opt_finally = {
            let singleton = usettings_singleton(None);
            let usettings = singleton.0.lock().unwrap();
            usettings.options.opt_finally
        };
        if ! opt_finally {
            // OPTFINALLYでない場合でexit、exitexitなら終了する
            match obj {
                Some(Object::Exit) |
                Some(Object::ExitExit(_)) => return Ok(obj),
                _ => {}
            }
        }
        if finally.is_some() {
            self.eval_block_statement(finally.unwrap())?;
        }
        Ok(obj)
    }

    fn eval_enum_statement(&mut self, name: String, uenum: UEnum) -> EvalResult<Option<Object>> {
        self.env.define_const(name, Object::Enum(uenum))?;
        Ok(None)
    }

    fn eval_thread_statement(&mut self, expression: Expression) -> EvalResult<Option<Object>> {
        if let Expression::FuncCall{func, args, is_await: _} = expression {
            let mut thread_self = Evaluator {
                env: Environment {
                    current: Arc::new(Mutex::new(Layer {
                        local: Vec::new(),
                        outer: None,
                    })),
                    global: Arc::clone(&self.env.global)
                },
                instance_id: Arc::clone(&self.instance_id),
            };
            thread::spawn(move || {
                std::panic::set_hook(Box::new(|panic_info|{
                    let mut e = panic_info.to_string();
                    let v = e.rmatch_indices("', s").collect::<Vec<_>>();
                    if v.len() > 0 {
                        let i = v[0].0;
                        e.truncate(i);
                    }
                    e = e.replace("panicked at '", "");
                    eprintln!("Error occured on thread> {}", e);
                }));
                let result = thread_self.eval_function_call_expression(func, args, false);
                if result.is_err() {
                    panic!("{}", result.unwrap_err());
                }
            });
        }
        Ok(None)
    }


    fn eval_expression(&mut self, expression: Expression) -> EvalResult<Object> {
        let obj: Object = match expression {
            Expression::Identifier(i) => self.eval_identifier(i)?,
            Expression::Array(v, index_list) => {
                match index_list.len() {
                    0 => {
                        return Err(UError::new(
                            "Array Error",
                            "Size or dimension must be specified",
                            None
                        ));
                    },
                    1 => {
                        let e = index_list[0].clone();
                        let size = match self.eval_expression(e)? {
                            Object::Num(n) => n as usize + 1,
                            Object::Empty => v.len(),
                            o => return Err(UError::new(
                                "Array error",
                                &format!("invalid index: {}", o),
                                None
                            )),
                        };
                        let mut array = vec![];
                        for e in v {
                            array.push(self.eval_expression(e)?);
                        }
                        array.resize(size, Object::Empty);
                        Object::Array(array)
                    },
                    _ => {
                        // 2次元以上
                        let mut array = vec![];
                        let mut sizes = vec![];
                        let mut i = 1;
                        for index in index_list {
                            match self.eval_expression(index)? {
                                Object::Num(n) => sizes.push(n as usize),
                                Object::Empty => if i > 1 {
                                    return Err(UError::new(
                                        "Array error",
                                        "no array size can be omitted except for the first []",
                                        None
                                    ));
                                } else {
                                    sizes.push(usize::MAX);
                                },
                                o => return Err(UError::new(
                                    "Array error",
                                    &format!("invalid index: {}", o),
                                    None
                                )),
                            }
                            i += 1;
                        }
                        for e in v {
                            array.push(self.eval_expression(e)?);
                        }
                        // 各次元サイズを格納した配列の順序を反転
                        sizes.reverse();

                        let l = array.len();
                        let actual_size = sizes.clone().into_iter().map(
                            // 最大添字を配列サイズにする
                            |n| if n == usize::MAX {n} else {n + 1}
                        ).reduce(|a, mut b| {
                            if b == usize::MAX {
                                // 値が省略された場合は実際のサイズを算出
                                b = (l / a) as usize + (if l % a == 0 {0} else {1});
                            }
                            match a.checked_mul(b) {
                                Some(n) => n,
                                None => 0,
                            }
                        }).unwrap();

                        if actual_size == 0 {
                            return Err(UError::new(
                                "Array error",
                                "total size of array is out of bounds",
                                None
                            ));
                        }
                        array.resize(actual_size, Object::Empty);
                        for size in sizes {
                            // 低い方から処理
                            let mut tmp = array;
                            tmp.reverse();
                            array = vec![];
                            loop {
                                let mut dimension = vec![];
                                for _ in 0..=size {
                                    let o = tmp.pop();
                                    if o.is_some() {
                                        dimension.push(o.unwrap());
                                    } else {
                                        break;
                                    }
                                }
                                array.push(Object::Array(dimension));
                                if tmp.len() == 0 {
                                    break;
                                }
                            }
                        }
                        array.pop().unwrap()
                    },
                }
            },
            Expression::Literal(l) => self.eval_literal(l)?,
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
                let outer_local = self.env.get_local_copy();
                Object::AnonFunc(params, body, Arc::new(Mutex::new(outer_local)), is_proc)
            },
            Expression::FuncCall {func, args, is_await} => {
                self.eval_function_call_expression(func, args, is_await)?
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
                "Expression evaluation error",
                &format!("bad expression: {}", p),
                None
            )),
            Expression::UObject(json) => {
                // 文字列展開する
                if let Object::String(s) = self.expand_string(json, true) {
                    match serde_json::from_str::<serde_json::Value>(s.as_str()) {
                        Ok(v) => Object::UObject(Arc::new(Mutex::new(v))),
                        Err(e) => return Err(UError::new(
                            "Json parse error",
                            &format!("{}", s),
                            Some(&format!("Error message: {}", e))
                        )),
                    }
                } else {
                    Object::Empty
                }
            },
        };
        Ok(obj)
    }

    fn eval_identifier(&mut self, identifier: Identifier) -> EvalResult<Object> {
        let Identifier(name) = identifier;
        // let env = self.env.lock().unwrap();
        let obj = match self.env.get_variable(&name, true) {
            Some(o) => o,
            None => match self.env.get_function(&name) {
                Some(o) => o,
                None => match self.env.get_module(&name) {
                    Some(o) => o,
                    None => match self.env.get_class(&name) {
                        Some(o) => o,
                        None => return Err(UError::new(
                            "Identifier not found",
                            &format!("{}", name),
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
                "Prefix - error",
                &format!("Not an number {}", right),
                None
            ))
        }
    }

    fn eval_plus_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        if let Object::Num(n) = right {
            Ok(Object::Num(n))
        } else {
            Err(UError::new(
                "Prefix + error",
                &format!("Not an number {}", right),
                None
            ))
        }
    }

    fn eval_index_expression(&mut self, left: Object, index: Object, hash_enum: Option<Object>) -> EvalResult<Object> {
        let obj = match left.clone() {
            Object::Array(ref a) => if hash_enum.is_some() {
                return Err(UError::new(
                    "Invalid index",
                    &format!("{}[{}, {}]", left, index, hash_enum.unwrap()),
                    None
                ));
            } else if let Object::Num(i) = index {
                self.eval_array_index_expression(a.clone(), i as i64)?
            } else {
                return Err(UError::new(
                    "Invalid index",
                    &format!("{}[{}]", left, index),
                    None
                ))
            },
            Object::HashTbl(h) => {
                let mut hash = h.lock().unwrap();
                let (key, i) = match index.clone(){
                    Object::Num(n) => (n.to_string(), Some(n as usize)),
                    Object::Bool(b) => (b.to_string(), None),
                    Object::String(s) => (s, None),
                    _ => return Err(UError::new(
                        "Invalid key",
                        &format!("{}", index),
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
                                    "Invalid index",
                                    &format!("{}[{}, {}]", left, key, n),
                                    None
                                ));
                            },
                            HashTblEnum::HASH_VAL => if i.is_some() {
                                hash.get_value(i.unwrap())
                            } else {
                                return Err(UError::new(
                                    "Invalid index",
                                    &format!("{}[{}, {}]", left, key, n),
                                    None
                                ));
                            },
                            _ => return Err(UError::new(
                                "Invalid index",
                                &format!("{}[{}, {}]", left, index, n),
                                None
                            ))
                        }
                    } else {
                        return Err(UError::new(
                            "Invalid index",
                            &format!("invalid index: {}[{}, {}]", left, index, hash_enum.unwrap()),
                            None
                        ));
                    }
                } else {
                    hash.get(key)
                }
            },
            Object::UObject(u) => if hash_enum.is_some() {
                return Err(UError::new(
                    "Invalid index",
                    &format!("{}[{}, {}]", left, index, hash_enum.unwrap()),
                    None
                ));
            } else {
                let v = u.lock().unwrap().clone();
                let (value, pointer) = match index {
                    Object::String(ref s) => {
                        (v.get(s), format!("/{}", s))
                    },
                    Object::Num(n) => {
                        (v.get(n as usize), format!("/{}", n))
                    },
                    _ => {
                        return Err(UError::new(
                            "Invalid index",
                            &format!("{}[{}]", left, index),
                            None
                        ));
                    }
                };
                if value.is_some() {
                    self.eval_uobject(&value.unwrap(), Arc::clone(&u), pointer)?
                } else {
                    return Err(UError::new(
                        "Index out of bound",
                        &format!("{}[{}]", left, index),
                        None
                    ));
                }
            },
            Object::UChild(u, p) => if hash_enum.is_some() {
                return Err(UError::new(
                    "Invalid index",
                    &format!("{}[{}, {}]", left, index, hash_enum.unwrap()),
                    None
                ));
            } else {
                let v = u.lock().unwrap().pointer(p.as_str()).unwrap_or(&serde_json::Value::Null).clone();
                let (value, pointer) = match index {
                    Object::String(ref s) => {
                        (v.get(s), format!("{}/{}", p, s))
                    },
                    Object::Num(n) => {
                        (v.get(n as usize), format!("{}/{}", p, n))
                    },
                    _ => {
                        return Err(UError::new(
                            "Invalid index",
                            &format!("{}[{}]", left, index),
                            None
                        ));
                    }
                };
                if value.is_some() {
                    self.eval_uobject(&value.unwrap(), Arc::clone(&u), pointer)?
                } else {
                    return Err(UError::new(
                        "Index out of bound",
                        &format!("{}[{}]", left, index),
                        None
                    ));
                }
            },
            o => return Err(UError::new(
                "Not an Array or Hashtable",
                &format!("{}", o),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: i64) -> EvalResult<Object> {
        let max = (array.len() as i64) - 1;
        if index < 0 || index > max {
            return Err(UError::new(
                "Index out of bound",
                &format!("{}", index),
                None
            ));
        }
        let obj = array.get(index as usize).map_or(Object::Empty, |o| o.clone());
        Ok(obj)
    }

    fn eval_assign_expression(&mut self, left: Expression, value: Object) -> EvalResult<Object> {
        let assigned_value = value.clone();
        self.eval_instance_assignment(&left, &value)?;
        let mut is_in_scope_auto_disposable = true;
        let instance = match value {
            Object::Instance(_, _) => Some(value.clone()),
            _ => None,
        };
        match left {
            Expression::Identifier(ident) => {
                let Identifier(name) = ident;
                // let mut env = self.env.lock().unwrap();
                if let Some(Object::This(m)) = self.env.get_variable(&"this".into(), true) {
                    // moudele/classメンバであればその値を更新する
                    m.lock().unwrap().assign(&name, value.clone(), None)?;
                    is_in_scope_auto_disposable = false;
                }
                is_in_scope_auto_disposable = ! self.env.assign(name, value)? && is_in_scope_auto_disposable;
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
                        let obj = self.env.get_variable(&name, true);
                        match obj {
                            Some(o) => {
                                match o {
                                    Object::Array(a) => {
                                        let mut arr = a.clone();
                                        match index {
                                            Object::Num(n) => {
                                                let i = n as usize;
                                                if let Some(Object::This(m)) = self.env.get_variable(&"this".into(), true) {
                                                    // moudele/classメンバであればその値を更新する
                                                    m.lock().unwrap().assign(&name, value.clone(), Some(index))?;
                                                    is_in_scope_auto_disposable = false;
                                                }
                                                if i < arr.len() {
                                                    arr[i] = value;
                                                    is_in_scope_auto_disposable = ! self.env.assign(name, Object::Array(arr))?;
                                                }
                                            },
                                            _ => return Err(UError::new(
                                                "Invalid index",
                                                &format!("{} is not valid index", index),
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
                                                "Invalid key",
                                                &format!("{} is not valid key", index),
                                                None
                                            ))
                                        };
                                        let mut hash = h.lock().unwrap();
                                        hash.insert(key, value);
                                    },
                                    _ => return Err(UError::new(
                                        "Invalid index call",
                                        &format!("{} is neither array nor hashtbl", name),
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
                                        m.lock().unwrap().assign(&name, value, Some(index))?;
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
                                    match v.lock().unwrap().get_mut(name.as_str()) {
                                        Some(serde_json::Value::Array(a)) => *a.get_mut(i).unwrap() = Self::object_to_serde_value(value)?,
                                        Some(_) => return Err(UError::new(
                                            "UObject error",
                                            &format!("{} is not an array", name),
                                            None
                                        )),
                                        None => return Err(UError::new(
                                            "UObject error",
                                            &format!("{} not found", name),
                                            None
                                        )),
                                    };
                                }
                            } else {
                                return Err(UError::new(
                                    "UObject error",
                                    &format!("invalid index: {}", index),
                                    None
                                ));
                            },
                            Object::UChild(u, p) => if let Object::Num(n) = index {
                                if let Expression::Identifier(Identifier(name)) = *right {
                                    let i = n as usize;
                                    match u.lock().unwrap().pointer_mut(p.as_str()).unwrap().get_mut(name.as_str()) {
                                        Some(serde_json::Value::Array(a)) => *a.get_mut(i).unwrap() = Self::object_to_serde_value(value)?,
                                        Some(_) => return Err(UError::new(
                                            "UObject error",
                                            &format!("{} is not an array", name),
                                            None
                                        )),
                                        None => return Err(UError::new(
                                            "UObject error",
                                            &format!("{} not found", name),
                                            None
                                        )),
                                    };
                                }
                            } else {
                                return Err(UError::new(
                                    "UObject error",
                                    &format!("invalid index: {}", index),
                                    None
                                ));
                            },
                            o => return Err(UError::new(
                                "Error on . operator",
                                &format!("not module or object: {}", o),
                                None
                            ))
                        }
                    },
                    _ => return Err(UError::new(
                        "Assignment error",
                        &format!("syntax error on assignment: {:?}", *arr),
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
                            m.lock().unwrap().assign_public(&member_name, value, None)?;
                            is_in_scope_auto_disposable = false;
                        },
                        _ => return Err(UError::new(
                            "Assignment error",
                            &format!("syntax error on assignment"),
                            None
                        )),
                    }
                },
                Object::This(m) => {
                    let mut module = m.lock().unwrap();
                    if let Expression::Identifier(Identifier(member)) = *right {
                        module.assign(&member, value, None)?;
                    } else {
                        return Err(UError::new(
                            "Invalid member call",
                            &format!("member not found on {}", module.name()),
                            None
                        ));
                    }
                },
                Object::Global => if let Expression::Identifier(Identifier(name)) = *right {
                    is_in_scope_auto_disposable = ! self.env.assign_public(name, value)?;
                } else {
                    return Err(UError::new(
                        "Error on assignment",
                        "global variable not found",
                        None
                    ))
                },
                Object::UObject(v) => if let Expression::Identifier(Identifier(name)) = *right {
                    match v.lock().unwrap().get_mut(name.as_str()) {
                        Some(mut_v) => *mut_v = Self::object_to_serde_value(value)?,
                        None => return Err(UError::new(
                            "UObject",
                            &format!("{} not found", name),
                            None
                        ))
                    }
                } else {
                    return Err(UError::new(
                        "UObject",
                        &format!("error on assignment"),
                        None
                    ));
                },
                Object::UChild(u, p) => if let Expression::Identifier(Identifier(name)) = *right {
                    match u.lock().unwrap().pointer_mut(p.as_str()).unwrap().get_mut(name.as_str()) {
                        Some(mut_v) => *mut_v = Self::object_to_serde_value(value)?,
                        None => return Err(UError::new(
                            "UObject",
                            &format!("{} not found", name),
                            None
                        ))
                    }
                } else {
                    return Err(UError::new(
                        "UObject",
                        &format!("error on assignment"),
                        None
                    ));
                },
                o => return Err(UError::new(
                    "Error on . operator",
                    &format!("not module or object: {}", o),
                    None
                ))
            },
            _ => return Err(UError::new(
                "Invalid assignment",
                &format!("not an variable: {:?}", left),
                None
            ))
        }
        if ! is_in_scope_auto_disposable {
            // スコープ内自動破棄対象じゃないインスタンスはグローバルに移す
            if let Some(Object::Instance(ref ins, id)) = instance {
                self.env.set_instances(Arc::clone(ins), id, true);
                self.env.remove_variable(format!("@INSTANCE{}", id));
                self.env.remove_from_instances(id);
            }
        }
        Ok(assigned_value)
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
            Object::Array(mut a) => if infix == Infix::Plus {
                a.push(right);
                Ok(Object::Array(a))
            } else {
                self.eval_infix_misc_expression(infix, left, right)
            },
            Object::UObject(v) => {
                let value = v.lock().unwrap().clone();
                let left = self.eval_uobject(&value, Arc::clone(&v), "/".into())?;
                self.eval_infix_expression(infix, left, right)
            },
            Object::UChild(v, p) => {
                let value = v.lock().unwrap().pointer(p.as_str()).unwrap_or(&serde_json::Value::Null).clone();
                let left = self.eval_uobject(&value, Arc::clone(&v), p)?;
                self.eval_infix_expression(infix, left, right)
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
                    "Infix error",
                    &format!("mismatched type: {} {} {}", left, infix, right),
                    None
                ))
            },
            Infix::Equal => Object::Bool(format!("{}", left) == format!("{}", right)),
            Infix::NotEqual => Object::Bool(format!("{}", left) != format!("{}", right)),
            _ => return Err(UError::new(
                "Infix error",
                &format!("mismatched type: {} {} {}", left, infix, right),
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
            Infix::Divide => match right as i64 {
                0 => Object::Num(0.0), // 0除算は0を返す
                _ => Object::Num(left / right),
            },
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
                "Infix error",
                &format!("you can not assign variable in expression: {} {} {}", left, infix, right),
                None
            ))
        };
        match obj {
            Object::Num(n) => if ! n.is_finite() {
                // 無限またはNaNはエラーにする
                Err(UError::new(
                    "calculation error",
                    &format!("result value is not valid number: {}", n),
                    None
                ))
            } else {
                Ok(Object::Num(n))
            },
            o => Ok(o)
        }
    }

    fn eval_infix_string_expression(&mut self, infix: Infix, left: String, right: String) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                "Infix error",
                &format!("bad operator: {} {} {}", left, infix, right),
                None
            ))
        };
        Ok(obj)
    }

    fn eval_infix_empty_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(self.is_equal(&left, &right)?),
            Infix::NotEqual => Object::Bool(! self.is_equal(&left, &right)?),
            _ => return Err(UError::new(
                "Infix error",
                &format!("bad operator: {} {} {}", left, infix, right),
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

    fn eval_literal(&mut self, literal: Literal) -> EvalResult<Object> {
        let obj = match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::ExpandableString(value) => self.expand_string(value, true),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects)?,
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
            Literal::NaN => Object::Num(f64::NAN),
            Literal::TextBlock(text, is_ex) => if is_ex {
                Object::ExpandableTB(text)
            } else {
                self.expand_string(text, false)
            },
        };
        Ok(obj)
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
                    self.env.get_variable(&text.into(), false).map(|o| format!("{}", o).into())
                } else {
                    continue;
                },
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    fn eval_array_literal(&mut self, objects: Vec<Expression>) -> EvalResult<Object> {
        let mut arr = vec![];
        for e in objects {
            arr.push(self.eval_expression(e.clone())?);
        }
        Ok(Object::Array(arr))
    }

    fn eval_expression_for_func_call(&mut self, expression: Expression) -> EvalResult<Object> {
        // 関数定義から探してなかったら変数を見る
        match expression {
            Expression::Identifier(i) => {
                let Identifier(name) = i;
                match self.env.get_function(&name) {
                    Some(o) => Ok(o),
                    None => match self.env.get_class(&name) {
                        Some(o) => Ok(o),
                        None => match self.env.get_variable(&name, true) {
                            Some(o) => Ok(o),
                            None => return Err(UError::new(
                                "Invalid Identifier",
                                &format!("function not found: {}", &name),
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

    fn invoke_task(&mut self, func: Object, arguments: Vec<(Option<Expression>, Object)>) -> Object {
        // task用のselfを作る
        let mut task_self = Evaluator {
            env: Environment {
                current: Arc::new(Mutex::new(Layer {
                    local: Vec::new(),
                    outer: None,
                })),
                global: Arc::clone(&self.env.global)
            },
            instance_id: Arc::clone(&self.instance_id),
        };
        // 関数を非同期実行し、UTaskを返す
        let handle = thread::spawn(move || {
            task_self.invoke_function_object(func, arguments)
        });
        let task = UTask {
            handle: Arc::new(Mutex::new(Some(handle))),
        };
        Object::Task(task)
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
                        &format!("Eval parse error[{}]:", &errors.len()),
                        &parse_errors,
                        None
                    ));
                }
                self.eval(program)?.map_or(Object::Empty, |o| o)
            },
            Object::SpecialFuncResult(t) => match t {
                SpecialFuncResultType::GetEnv => {
                    self.env.get_env()
                },
                SpecialFuncResultType::ListModuleMember(name) => {
                    self.env.get_module_member(&name)
                },
                SpecialFuncResultType::BuiltinConstName(e) => {
                    if let Some(Expression::Identifier(Identifier(name))) = e {
                        self.env.get_name_of_builtin_consts(&name)
                    } else {
                        Object::Empty
                    }
                },
                SpecialFuncResultType::Task(func, arguments) => {
                    self.invoke_task(*func, arguments)
                },
            },
            _ => result
        };
        Ok(obj)
    }

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>, is_await: bool) -> EvalResult<Object> {
        type Argument = (Option<Expression>, Object);
        let mut arguments: Vec<Argument> = vec![];
        for arg in args {
            arguments.push((Some(arg.clone()), self.eval_expression(arg)?));
        }

        let func_object = self.eval_expression_for_func_call(*func)?;
        match func_object {
            Object::DestructorNotFound => return Ok(Object::Empty),
            Object::Function(_, params, body, is_proc, obj) => return self.invoke_user_function(params, arguments, body, is_proc, None, obj, false),
            Object::AsyncFunction(_, _,_, _, _) => {
                let task = self.invoke_task(func_object, arguments);
                let result = if is_await {
                    if let Object::Task(t) = task {
                        let mut handle = t.handle.lock().unwrap();
                        match handle.take().unwrap().join() {
                            Ok(res) => res,
                            Err(e) => {
                                Err(UError::new(
                                    "Task error",
                                    "task ended incorrectly",
                                    Some(&format!("{:?}", e))
                                ))
                            }
                        }
                    } else {
                        Ok(task)
                    }
                } else {
                    Ok(task)
                };
                return result;
            },
            Object::AnonFunc(params, body, o, is_proc) => return self.invoke_user_function(params, arguments, body, is_proc, Some(o), None, false),
            Object::BuiltinFunction(name, expected_param_len, f) => {
                if expected_param_len >= arguments.len() as i32 {
                    let res = f(BuiltinFuncArgs::new(name, arguments))?;
                    return self.builtin_func_result(res);
                } else {
                    let l = arguments.len();
                    return Err(UError::new(
                        "Too many arguments",
                        &format!(
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
                if let Object::Instance(ins, _) = instance {
                    let constructor = match ins.lock().unwrap().get_function(&name) {
                        Ok(o) => o,
                        Err(_) => return Err(UError::new(
                            "Constructor not found",
                            &format!("you must define procedure {}()", &name),
                            None
                        ))
                    };
                    if let Object::Function(_, params, body, _, _) = constructor {
                        return self.invoke_user_function(params, arguments, body, true, None, Some(Arc::clone(&ins)), true);
                    } else {
                        return Err(UError::new(
                            "Syntax Error",
                            &format!("{} is not valid constructor", &name),
                            None
                        ));
                    }
                } else {
                    return Err(UError::new(
                        "Syntax Error",
                        &format!("{} is not a class", &name),
                        None
                    ));
                }
            },
            o => return Err(UError::new(
                "Not a function",
                &format!("{}", o),
                None
            )),
        };
    }

    fn invoke_function_object(&mut self, object: Object, arguments: Vec<(Option<Expression>, Object)>) -> EvalResult<Object> {
        match object {
            Object::Function(_, params, body, is_proc, module_reference) |
            Object::AsyncFunction(_, params, body, is_proc, module_reference)=> {
                return self.invoke_user_function(params, arguments, body, is_proc, None, module_reference, false);
            },
            o => Err(UError::new(
                "Syntax Error",
                &format!("not a function: {}", o),
                None
            ))
        }
    }

    fn invoke_user_function(
        &mut self,
        mut params: Vec<Expression>,
        mut arguments: Vec<(Option<Expression>, Object)>,
        body: Vec<Statement>,
        is_proc: bool,
        anon_outer: Option<Arc<Mutex<Vec<NamedObject>>>>,
        module_reference: Option<Arc<Mutex<Module>>>,
        is_class_instance: bool
    ) -> EvalResult<Object> {
        let org_param_len = params.len();
        if params.len() > arguments.len() {
            arguments.resize(params.len(), (None, Object::EmptyParam));
        } else if params.len() < arguments.len() {
            params.resize(arguments.len(), Expression::Params(Params::VariadicDummy));
        }

        if anon_outer.is_some() {
            let clone_outer = anon_outer.clone().unwrap();
            let outer_local = clone_outer.lock().unwrap();
            self.env.copy_scope(outer_local.clone());
        } else {
            self.env.new_scope();
        }
        let list = params.into_iter().zip(arguments.into_iter());
        let mut variadic = vec![];
        let mut variadic_name = String::new();
        let mut reference = vec![];
        for (_, (e, (arg_e, o))) in list.enumerate() {
            let param = match e {
                Expression::Params(p) => p,
                _ => return Err(UError::new(
                    "Invalid parameter",
                    &format!("bad parameter: {:?}", e),
                    None
                ))
            };
            let (name, value) = match param {
                Params::Identifier(i) => {
                    let Identifier(name) = i;
                    if arg_e.is_none() {
                        return Err(UError::new(
                            "argument required",
                            &format!("{}", name),
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
                            "Invalid argument",
                            &format!("{}", name),
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
                            "Invalid argument",
                            &format!("{}", name),
                            None
                        )),
                    }
                },
                Params::WithDefault(i, default) => {
                    let Identifier(name) = i;
                    if Object::EmptyParam.is_equal(&o) {
                        (name, self.eval_expression(*default)?)
                    } else {
                        (name, o)
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
                            "Too many arguments",
                            &format!("should be less than or equal to {}", org_param_len),
                            None
                        ))
                    }
                    variadic.push(o.clone());
                    continue;
                }
            };
            if variadic.len() == 0 {
                self.env.define_local(name, value)?;
            }
        }
        if variadic.len() > 0 {
            self.env.define_local(variadic_name, Object::Array(variadic))?;
        }

        match module_reference {
            Some(ref m) => {
                self.env.set_module_private_member(m);
            },
            None => {},
        };

        if ! is_proc {
            // resultにEMPTYを入れておく
            self.env.assign("result".into(), Object::Empty)?;
        }

        // 関数実行
        match self.eval_block_statement(body) {
            Ok(_) => {},
            Err(e) => {
                // 関数ブロックでエラーが発生した場合は、関数の実行事態ががなかったことになる
                // - 戻り値を返さない
                // - 参照渡しされた変数は更新されない
                // - 関数内で作られたインスタンスを自動破棄しない

                // スコープを戻す
                self.env.restore_scope(None);
                return Err(e);
            }
        }

        // 戻り値
        let result = if is_class_instance {
            match module_reference {
                Some(ref m) => Object::Instance(Arc::clone(m), self.new_instance_id()),
                None => return Err(UError::new(
                    "Syntax error".into(),
                    "failed to create new instance".into(),
                    None
                )),
            }
        } else if is_proc {
            Object::Empty
        } else {
            match self.env.get_variable(&"result".to_string(), true) {
                Some(o) => o,
                None => Object::Empty
            }
        };
        // 参照渡し
        let mut ref_values = vec![];
        let mut do_not_dispose = vec![];
        for (p_name, _) in reference.clone() {
            let obj = self.env.get_variable(&p_name, true).unwrap();
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
        {
            self.env.restore_scope(anon_outer);
        }

        for ((_, e), o) in reference.iter().zip(ref_values.iter()) {
            // Expressionが代入可能な場合のみ代入処理を行う
            match e {
                Expression::Identifier(_) |
                Expression::Index(_, _, _) |
                Expression::DotCall(_, _) => {
                    self.eval_assign_expression(e.clone(), o.clone())?;
                    // 参照渡しでインスタンスを帰す場合は自動破棄対象とする
                    match o {
                        Object::Instance(ref ins, id) => {
                            self.env.set_instances(Arc::clone(ins), *id, false);
                        },
                        _ => {},
                    }
                },
                _ => {},
            };
        };

        // 戻り値がインスタンスなら自動破棄されるようにしておく
        match result {
            Object::Instance(ref ins, id) => {
                self.env.set_instances(Arc::clone(ins), id, false);
            },
            _ => {},
        }

        Ok(result)
    }

    fn auto_dispose_instances(&mut self, refs: Vec<String>, include_global: bool) {
        let ins_list = self.env.get_instances();
        for ins_name in ins_list {
            if ! refs.contains(&ins_name) {
                let obj = self.env.get_tmp_instance(&ins_name, false).unwrap_or(Object::Empty);
                if let Object::Instance(ins, _) = obj {
                    let destructor = ins.lock().unwrap().get_destructor();
                    if destructor.is_some() {
                        self.invoke_function_object(destructor.unwrap(), vec![]).ok();
                    }
                    ins.lock().unwrap().dispose();
                }
            }
        }
        if include_global {
            let ins_list = self.env.get_global_instances();
            for ins_name in ins_list {
                let obj = self.env.get_tmp_instance(&ins_name, true).unwrap_or(Object::Empty);
                if let Object::Instance(ins, _) = obj {
                    {
                        let destructor = ins.lock().unwrap().get_destructor();
                        if destructor.is_some() {
                            self.invoke_function_object(destructor.unwrap(), vec![]).ok();
                        }
                    }
                    ins.lock().unwrap().dispose();
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
            Expression::FuncCall{func:_, args:_, is_await:_} |
            Expression::DotCall(_, _) |
            Expression::UObject(_) => {
                self.eval_expression(left)?
            },
            _ => return Err(UError::new(
                "Error on . operator",
                &format!("invalid expression: {:?}", left),
                None
            )),
        };
        match instance {
            Object::Module(m) |
            Object::Instance(m, _) => {
                let module = m.lock().unwrap(); // Mutex<Module>をロック
                match right {
                    Expression::Identifier(i) => {
                        let Identifier(member_name) = i;
                        if module.is_local_member(&member_name) {
                            if let Some(Object::This(this)) = self.env.get_variable(&"this".into(), true) {
                                if this.try_lock().is_err() {
                                    // ロックに失敗した場合、上でロックしているMutexと同じだと判断
                                    // なので自分のモジュールメンバの値を返す
                                    return module.get_member(&member_name);
                                }
                            }
                            Err(UError::new(
                                "Access denied",
                                &format!("you can not access to {}.{}", module.name(), member_name),
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
                        "Error on . operator",
                        "member does not exist",
                        None
                    )),
                }
            },
            Object::This(m) => {
                let module = m.lock().unwrap();
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
                        "Function not found",
                        &format!("member not found on {}", module.name()),
                        None
                    ))
                }
            },
            Object::Global => {
                if let Expression::Identifier(Identifier(g_name)) = right {
                    self.env.get_global(&g_name, is_func)
                } else {
                    Err(UError::new(
                        "Global",
                        &format!("not an identifier ({:?})", right),
                        None
                    ))
                }
            },
            Object::Class(name, _) => Err(UError::new(
                "Invalid Class call",
                &format!("{0} can not be called directly; try {0}() to create instance", name),
                None
            )),
            Object::UObject(u) => if let Expression::Identifier(Identifier(key)) = right {
                match u.lock().unwrap().get(key.as_str()) {
                    Some(v) => self.eval_uobject(v, Arc::clone(&u), format!("/{}", key)),
                    None => Err(UError::new(
                        "UObject",
                        &format!("{} not found", key),
                        None
                    )),
                }
            } else {
                Err(UError::new(
                    "UObject",
                    &format!("not an identifier ({:?})", right),
                    None
                ))
            },
            Object::UChild(u,p) => if let Expression::Identifier(Identifier(key)) = right {
                match u.lock().unwrap().pointer(p.as_str()).unwrap().get(key.as_str()) {
                    Some(v) => self.eval_uobject(v, Arc::clone(&u), format!("{}/{}", p, key)),
                    None => Err(UError::new(
                        "UObject",
                        &format!("{} not found", key),
                        None
                    ))
                }
            } else {
                Err(UError::new(
                    "UObject",
                    &format!("not an identifier ({:?})", right),
                    None
                ))
            },
            Object::Enum(e) => if let Expression::Identifier(Identifier(member)) = right {
                if let Some(n) = e.get(&member) {
                    Ok(Object::Num(n))
                } else {
                    Err(UError::new(
                        "Invalid Enum member",
                        &format!("{}.{} is not defined for", &e.name, member),
                        None
                    ))
                }
            } else {
                Err(UError::new(
                    "Invalid Enum member",
                    &format!("not an identifier ({:?})", right),
                    None
                ))
            },
            o => Err(UError::new(
                ". operator not supported",
                &format!("{}", o),
                None
            )),
        }
    }

    // UObject
    fn eval_uobject(&self, v: &serde_json::Value, top: Arc<Mutex<serde_json::Value>>, pointer: String) -> EvalResult<Object> {
        let o = match v {
            serde_json::Value::Null => Object::Null,
            serde_json::Value::Bool(b) => Object::Bool(*b),
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(f) => Object::Num(f),
                None => return Err(UError::new(
                    "UObject error",
                    &format!("can not convert {} to number", n),
                    None
                )),
            },
            serde_json::Value::String(s) => {
                self.expand_string(s.clone(), true)
            },
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
            Object::UObject(u) => u.lock().unwrap().clone(),
            Object::UChild(u, p) => u.lock().unwrap().pointer(p.as_str()).unwrap().clone(),
            _ => return Err(UError::new(
                "UObject error",
                &format!("can not convert {} to uobject", o),
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
            if m.lock().unwrap().is_disposed() {
                return Ok(());
            }
            // Nothingが代入される場合は明示的にデストラクタを実行及びdispose()
            match new_value {
                Object::Nothing => {
                    let mut ins = m.lock().unwrap();
                    let destructor = ins.get_destructor();
                    if destructor.is_some() {
                        self.invoke_function_object(destructor.unwrap(), vec![])?;
                    }
                    ins.dispose();
                },
                _ => {},
            }
        }
        Ok(())
    }

    fn is_equal(&mut self, obj1: &Object, obj2: &Object) -> EvalResult<bool> {
        let r = self.eval_infix_expression(Infix::Equal, obj1.clone(), obj2.clone())?;
        Ok(Self::is_truthy(r))
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_test(input: &str, expected: Result<Option<Object>, UError>, ast: bool) {
        let mut e = Evaluator::new(Environment::new(vec![]));
        let program = Parser::new(Lexer::new(input)).parse();
        if ast {
            println!("{:?}", program);
        }
        let result = e.eval(program);
        match expected {
            Ok(expected_obj) => match result {
                Ok(result_obj) => if result_obj.is_some() && expected_obj.is_some() {
                    let left = result_obj.unwrap();
                    let right = expected_obj.unwrap();
                    if ! left.is_equal(&right) {
                        panic!("\nresult: {:?}\nexpected: {:?}\n\n{}", left, right, input);
                    }
                } else if result_obj.is_some() || expected_obj.is_some() {
                    // どちらかがNone
                    panic!("\nresult: {:?}\nexpected: {:?}\n\n{}", result_obj, expected_obj, input);
                },
                Err(_) => panic!("this test should be ok:\n{}", input),
            },
            Err(expected_err) => match result {
                Ok(_) => panic!("this test should occure error:\n{}", input),
                Err(result_err) => assert_eq!(result_err, expected_err),
            },
        }
    }


    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        let mut e = Evaluator::new(Environment::new(vec![]));
        let program = Parser::new(Lexer::new(input)).parse();
        match e.eval(program) {
            Ok(_) => e,
            Err(err) => panic!("{}", err)
        }
    }

    //
    fn eval_test_with_env(e: &mut Evaluator, input: &str, expected: Result<Option<Object>, UError>) {
        let program = Parser::new(Lexer::new(input)).parse();
        let result = e.eval(program);
        match expected {
            Ok(expected_obj) => match result {
                Ok(result_obj) => if result_obj.is_some() && expected_obj.is_some() {
                    let left = result_obj.unwrap();
                    let right = expected_obj.unwrap();
                    if ! left.is_equal(&right) {
                        panic!("\nresult: {:?}\nexpected: {:?}\n\n{}", left, right, input);
                    }
                } else if result_obj.is_some() || expected_obj.is_some() {
                    // どちらかがNone
                    panic!("\nresult: {:?}\nexpected: {:?}\n\n{}", result_obj, expected_obj, input);
                },
                Err(_) => panic!("this test should be ok:\n{}", input),
            },
            Err(expected_err) => match result {
                Ok(_) => panic!("this test should occure error:\n{}", input),
                Err(result_err) => assert_eq!(result_err, expected_err),
            },
        }
    }


    #[test]
    fn test_num_expression() {
        let test_cases = vec![
            ("5", Ok(Some(Object::Num(5.0)))),
            ("10", Ok(Some(Object::Num(10.0)))),
            ("-5", Ok(Some(Object::Num(-5.0)))),
            ("-10", Ok(Some(Object::Num(-10.0)))),
            ("1.23", Ok(Some(Object::Num(1.23)))),
            ("-1.23", Ok(Some(Object::Num(-1.23)))),
            ("+(-5)", Ok(Some(Object::Num(-5.0)))),
            ("1 + 2 + 3 - 4", Ok(Some(Object::Num(2.0)))),
            ("2 * 3 * 4", Ok(Some(Object::Num(24.0)))),
            ("-3 + 3 * 2 + -3", Ok(Some(Object::Num(0.0)))),
            ("5 + 3 * -2", Ok(Some(Object::Num(-1.0)))),
            ("6 / 3 * 2 + 1", Ok(Some(Object::Num(5.0)))),
            ("1.2 + 2.4", Ok(Some(Object::Num(3.5999999999999996)))),
            ("1.2 * 3", Ok(Some(Object::Num(3.5999999999999996)))),
            ("2 * (5 + 10)", Ok(Some(Object::Num(30.0)))),
            ("3 * 3 * 3 + 10", Ok(Some(Object::Num(37.0)))),
            ("3 * (3 * 3) + 10", Ok(Some(Object::Num(37.0)))),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", Ok(Some(Object::Num(50.0)))),
            ("1 + TRUE", Ok(Some(Object::Num(2.0)))),
            ("1 + false", Ok(Some(Object::Num(1.0)))),
            ("TRUE + 1", Ok(Some(Object::Num(2.0)))),
            ("5 mod 3", Ok(Some(Object::Num(2.0)))),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
    }

    #[test]
    fn test_string_infix() {
        let test_cases = vec![
            (r#""hoge" + "fuga""#, Ok(Some(Object::String("hogefuga".to_string())))),
            (r#""hoge" + 100"#, Ok(Some(Object::String("hoge100".to_string())))),
            (r#"400 + "fuga""#, Ok(Some(Object::String("400fuga".to_string())))),
            (r#""hoge" + TRUE"#, Ok(Some(Object::String("hogeTrue".to_string())))),
            (r#""hoge" + FALSE"#, Ok(Some(Object::String("hogeFalse".to_string())))),
            (r#"TRUE + "hoge""#, Ok(Some(Object::String("Truehoge".to_string())))),
            (r#""hoge" = "hoge""#, Ok(Some(Object::Bool(true)))),
            (r#""hoge" == "hoge""#, Ok(Some(Object::Bool(true)))),
            (r#""hoge" == "fuga""#, Ok(Some(Object::Bool(false)))),
            (r#""hoge" == "HOGE""#, Ok(Some(Object::Bool(false)))),
            (r#""hoge" == 1"#, Ok(Some(Object::Bool(false)))),
            (r#""hoge" != 1"#, Ok(Some(Object::Bool(true)))),
            (r#""hoge" <> 1"#, Ok(Some(Object::Bool(true)))),
            (r#""hoge" <> "hoge"#, Ok(Some(Object::Bool(false)))),
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
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
dim HOGE = 2
hoge
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
dim hoge = 2
dim hoge = 3
                "#,
                Err(UError::new(
                    "Error on definition",
                    &format!("HOGE is already defined."),
                    None
                ))
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
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
hashtbl hoge
hoge["test"] = 2
hoge["TEST"]
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
hashtbl hoge
hoge[1.23] = 2
hoge[1.23]
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
hashtbl hoge
hoge[FALSE] = 2
hoge[FALSE]
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
hashtbl hoge = HASH_CASECARE
hoge["abc"] = 1
hoge["ABC"] = 2
hoge["abc"] + hoge["ABC"]
                "#,
                Ok(Some(Object::Num(3.0)))
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
                Ok(Some(Object::String("cdba".to_string())))
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
                Ok(Some(Object::String("hoge".to_string())))
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
        eval_test(input, Ok(Some(Object::String("hoge".to_string()))), false);
    }

    #[test]
    fn test_assign_array_literal() {
        let input = r#"
hoge = [1,3,5]
hoge[0] = 2
hoge[0]
        "#;
        eval_test(input, Ok(Some(Object::Num(2.0))), false);
    }

    #[test]
    fn test_public() {
        let input = r#"
public hoge = 1
hoge
        "#;
        eval_test(input, Ok(Some(Object::Num(1.0))), false);
    }

    #[test]
    fn test_array_definition() {
        let test_cases = vec![
            (
                r#"
                dim hoge[3] = 1,2
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Num(1.0),
                    Object::Num(2.0),
                    Object::Empty,
                    Object::Empty,
                ])))
            ),
            (
                r#"
                dim hoge[2][2] = 1,2,3, 4,5,6, 7
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Num(1.0),
                        Object::Num(2.0),
                        Object::Num(3.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(4.0),
                        Object::Num(5.0),
                        Object::Num(6.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(7.0),
                        Object::Empty,
                        Object::Empty,
                    ]),
                ])))
            ),
            (
                r#"
                dim hoge[2, 2] = 1,2,3, 4,5,6, 7
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Num(1.0),
                        Object::Num(2.0),
                        Object::Num(3.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(4.0),
                        Object::Num(5.0),
                        Object::Num(6.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(7.0),
                        Object::Empty,
                        Object::Empty,
                    ]),
                ])))
            ),
            (
                r#"
                // 省略
                dim hoge[, 2] = 1,2,3, 4,5,6, 7
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Num(1.0),
                        Object::Num(2.0),
                        Object::Num(3.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(4.0),
                        Object::Num(5.0),
                        Object::Num(6.0),
                    ]),
                    Object::Array(vec![
                        Object::Num(7.0),
                        Object::Empty,
                        Object::Empty,
                    ]),
                ])))
            ),
            (
                r#"
                // 多次元
                dim hoge[1][1][1] = 0,1, 2,3, 4,5, 6,7
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(0.0),
                                Object::Num(1.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(2.0),
                                Object::Num(3.0),
                            ]
                        ),
                    ]),
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(4.0),
                                Object::Num(5.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(6.0),
                                Object::Num(7.0),
                            ]
                        ),
                    ]),
                ])))
            ),
            (
                r#"
                // 省略
                dim hoge[][1][1] = 0,1, 2,3, 4,5, 6,7
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(0.0),
                                Object::Num(1.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(2.0),
                                Object::Num(3.0),
                            ]
                        ),
                    ]),
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(4.0),
                                Object::Num(5.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(6.0),
                                Object::Num(7.0),
                            ]
                        ),
                    ]),
                ])))
            ),
            (
                r#"
                // EMPTY埋め
                dim hoge[1][1][1] = 0,1, 2,3, 4
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(0.0),
                                Object::Num(1.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(2.0),
                                Object::Num(3.0),
                            ]
                        ),
                    ]),
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(4.0),
                                Object::Empty,
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Empty,
                                Object::Empty,
                            ]
                        ),
                    ]),
                ])))
            ),
            (
                r#"
                // 省略+EMPTY埋め
                dim hoge[][1][1] = 0,1, 2,3, 4,5, 6
                hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(0.0),
                                Object::Num(1.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(2.0),
                                Object::Num(3.0),
                            ]
                        ),
                    ]),
                    Object::Array(vec![
                        Object::Array(
                            vec![
                                Object::Num(4.0),
                                Object::Num(5.0),
                            ]
                        ),
                        Object::Array(
                            vec![
                                Object::Num(6.0),
                                Object::Empty,
                            ]
                        ),
                    ]),
                ])))
            ),
        ];
        let error_cases = vec![
            (
                format!(r#"
                // usize超え
                dim hoge[{}][1]
                hoge
                "#, usize::MAX),
                Err(UError::new(
                    "Array error".into(),
                    "total size of array is out of bounds".into(),
                    None
                ))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false)
        }
        for (input, expected) in error_cases {
            eval_test(&input, expected, false)
        }
    }

    #[test]
    fn test_print() {
        let input = r#"
hoge = "print test"
print hoge
        "#;
        eval_test(input, Ok(Some(Object::String("print test".into()))), false);
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
Ok(                Some(Object::Num(4.0)))
            ),
            (
                r#"
for i = 0 to 2
    i = 10
next
i
                "#,
                Ok(Some(Object::Num(3.0)))
            ),
            (
                r#"
for i = 0 to 5 step 2
next
i
                "#,
                Ok(Some(Object::Num(6.0)))
            ),
            (
                r#"
for i = 5 to 0 step -2
next
i
                "#,
                Ok(Some(Object::Num(-1.0)))
            ),
            (
                r#"
for i = "0" to "5" step "2"
next
i
                "#,
                Ok(Some(Object::Num(6.0)))
            ),
            (
                r#"
for i = 0 to "5s"
next
                "#,
                Err(UError::new(
                    "Syntax error on For",
                    &format!("for i = 0 to 5s"),
                    None
                ))
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
                Ok(Some(Object::Num(1.0)))
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
                Ok(Some(Object::Num(1.0)))
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
Ok(                Some(Object::Num(15.0)))
            ),
            (
                r#"
a = ""
for c in "hoge"
    a = c + a
next
a
                "#,
                Ok(Some(Object::String("egoh".to_string())))
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
                Ok(Some(Object::Num(6.0)))
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
                Ok(Some(Object::Num(6.0)))
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
                Ok(Some(Object::Num(0.0)))
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
                Ok(Some(Object::Num(0.0)))
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
                Ok(Some(Object::Num(4.0)))
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
                Ok(Some(Object::Num(0.0)))
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
                Ok(Some(Object::Num(-1.0)))
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
                Ok(Some(Object::String("a is true".to_string())))
            ),
            (
                r#"
if 1 < 0 then a = "a is true" else a = "a is false"
a
                "#,
                Ok(Some(Object::String("a is false".to_string())))
            ),
            (
                r#"
if true then print "test sucseed!" else print "should not be printed"
                "#,
                Ok(None)
            ),
            (
                r#"
a = 1
if false then a = 5
a
                "#,
                Ok(Some(Object::Num(1.0)))
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
Ok(                Some(Object::String("a is true".to_string())))
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
                Ok(Some(Object::String("a is false".to_string())))
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
                Ok(Some(Object::String("test sucseed!".to_string())))
            ),
            (
                r#"
a = 1
if false then
    a = 5
endif
a
                "#,
                Ok(Some(Object::Num(1.0)))
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
                Ok(Some(Object::String("test1 sucseed!".to_string())))
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
                Ok(Some(Object::String("test2 sucseed!".to_string())))
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
                Ok(Some(Object::String("test3 sucseed!".to_string())))
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
                Ok(Some(Object::String("test4 sucseed!".to_string())))
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
                Ok(Some(Object::String("test1 sucseed!".to_string())))
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
                Ok(Some(Object::String("test2 sucseed!".to_string())))
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
                Ok(Some(Object::String("test3 sucseed!".to_string())))
            ),
            (
                r#"
select 6
    default
        a = "test4 sucseed!"
selend
a
                "#,
                Ok(Some(Object::String("test4 sucseed!".to_string())))
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
                Ok(Some(Object::String("test5 sucseed!".to_string())))
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
                Ok(Some(Object::Num(5.0)))
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
                Ok(Some(Object::Num(5.0)))
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
                Ok(Some(Object::Num(5.0)))
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
                Ok(Some(Object::Num(7.0)))
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
                Ok(Some(Object::Num(2.0)))
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
                Ok(Some(Object::Num(5.0)))
            ),
            (
                r#"
hoge(5)

function hoge(n)
    // no result
fend
                "#,
                Ok(Some(Object::Empty))
            ),
            (
                r#"
a = hoge(5)
a == 5

procedure hoge(n)
    result = n
fend
                "#,
                Ok(Some(Object::Bool(false)))
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
                Ok(Some(Object::String("should not be over written".to_string())))
            ),
            (
                r#"
f  = function(x, y)
    result = x + y
fend

f(5, 10)
                "#,
                Ok(Some(Object::Num(15.0)))
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
                Ok(Some(Object::Num(15.0)))
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
                Ok(Some(Object::String("testing closure".to_string())))
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
                Ok(Some(Object::String("done".to_string())))
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
                Ok(Some(Object::Num(4.0)))
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
                Ok(Some(Object::Num(1.0)))
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
                Err(UError::new(
                    "Error on definition",
                    &format!("DIM_AND_DIM is already defined."),
                    None
                ))
            ),
            (
                r#"
public pub_and_const = 1
const pub_and_const = 2
                "#,
                Err(UError::new(
                    "Error on definition",
                    &format!("PUB_AND_CONST is already defined."),
                    None
                ))
            ),
            (
                r#"
const const_and_const = 1
const const_and_const = 2
                "#,
                Err(UError::new(
                    "Error on definition",
                    &format!("CONST_AND_CONST is already defined."),
                    None
                ))
            ),
            (
                r#"
public public_and_public = 1
public public_and_public = 2
                "#,
                Ok(None)
            ),
            (
                r#"
hashtbl hash_and_hash
hashtbl hash_and_hash
                "#,
                Err(UError::new(
                    "Error on definition",
                    &format!("HASH_AND_HASH is already defined."),
                    None
                ))
            ),
            (
                r#"
function func_and_func()
fend
function func_and_func()
fend
                "#,
                Err(UError::new(
                    "Function defining error",
                    &format!("FUNC_AND_FUNC is already defined."),
                    None
                ))
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
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = "hoge"
a += "fuga"
a
                "#,
                Ok(Some(Object::String("hogefuga".to_string())))
            ),
            (
                r#"
a = 5
a -= 3
a
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = 2
a *= 5
a
                "#,
                Ok(Some(Object::Num(10.0)))
            ),
            (
                r#"
a = 10
a /= 5
a
                "#,
                Ok(Some(Object::Num(2.0)))
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
        eval_test(input, Ok(Some(Object::Num(11.0))), false)
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
                Ok(Some(Object::String("script local".to_string())))
            ),
            (
                r#"
                v += " 1"
                v
                "#,
                Ok(Some(Object::String("script local 1".to_string())))
            ),
            (
                "p",
                Ok(Some(Object::String("public".to_string())))
            ),
            (
                r#"
                p += " 1"
                p
                "#,
                Ok(Some(Object::String("public 1".to_string())))
            ),
            (
                "c",
                Ok(Some(Object::String("const".to_string())))
            ),
            (
                "func()",
                Ok(Some(Object::String("function".to_string())))
            ),
            (
                "f",
                Ok(Some(Object::String("variable".to_string())))
            ),
            (
                "f()",
                Ok(Some(Object::String("function".to_string())))
            ),
            (
                "get_p()",
                Ok(Some(Object::String("public 1".to_string())))
            ),
            (
                "get_c()",
                Ok(Some(Object::String("const".to_string())))
            ),
            (
                "get_v()",
                Err(UError::new("Identifier not found","v",None))
            ),
            (
                "M.v",
                Err(UError::new("Access denied","you can not access to M.v",None))
            ),
            (
                "M.p",
                Ok(Some(Object::String("module public".to_string())))
            ),
            (
                "M.c",
                Ok(Some(Object::String("module const".to_string())))
            ),
            (
                "M.func()",
                Ok(Some(Object::String("module function".to_string())))
            ),
            (
                "M.get_v()",
                Ok(Some(Object::String("module local".to_string())))
            ),
            (
                "M.get_this_v()",
                Ok(Some(Object::String("module local".to_string())))
            ),
            (
                "M.get_m_v()",
                Ok(Some(Object::String("module local".to_string())))
            ),
            (
                "M.get_p()",
                Ok(Some(Object::String("module public".to_string())))
            ),
            (
                "M.get_outer_p2()",
                Ok(Some(Object::String("public 2".to_string())))
            ),
            (
                "M.inner_func()",
                Ok(Some(Object::String("module function".to_string())))
            ),
            (
                "M.outer_func()",
                Ok(Some(Object::String("function".to_string())))
            ),
            (
                "M.set_a(5)",
                Ok(Some(Object::Num(5.0)))
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
                Ok(Some(Object::Num(3.0)))
            ),
            (
                "hoge('abc')",
                Ok(Some(Object::String("abc".to_string())))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test_with_env(&mut e, input, expected);
        }
    }
}