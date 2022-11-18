pub mod object;
// pub mod env;
pub mod environment;
pub mod builtins;
pub mod def_dll;
pub mod com_object;
pub mod devtools_protocol;

use crate::ast::*;
use crate::evaluator::environment::*;
use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::def_dll::*;
use crate::evaluator::com_object::*;
use crate::evaluator::devtools_protocol::{Browser, Element, ElementProperty};
use crate::error::UWSCRErrorTitle;
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::gui::{LogPrintWin, UWindow, Balloon};
use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::logging::{out_log, LogType};
use crate::settings::*;
use crate::winapi::{attach_console,free_console,show_message,FORCE_WINDOW_MODE};
use windows::{
    Win32::System::{
        Com::{
            // COINIT_APARTMENTTHREADED,
            // COINIT_MULTITHREADED,
            IDispatch,
            // CoInitializeEx, CoUninitialize,
        },
    },
};

use std::borrow::Cow;
use std::env;
use std::mem;
use std::path::PathBuf;
use std::thread;
use std::sync::{Arc, Mutex};
use std::ffi::c_void;
use std::panic;
use std::io::{stdout, Write, BufWriter};

use num_traits::FromPrimitive;
use regex::Regex;
use serde_json;
use libffi::middle::{Cif, CodePtr, Type};
use once_cell::sync::OnceCell;
use serde_json::Value;

pub static LOGPRINTWIN: OnceCell<Mutex<LogPrintWin>> = OnceCell::new();

type EvalResult<T> = Result<T, UError>;

#[derive(Debug, Clone)]
pub struct  Evaluator {
    pub env: Environment,
    pub ignore_com_err: bool,
    pub com_err_flg: bool,
    lines: Vec<String>,
    pub balloon: Option<Balloon>,
}

impl Evaluator {
    pub fn clear(&mut self) {
        self.env.clear();
    }

    pub fn new(env: Environment) -> Self {
        Evaluator {
            env,
            ignore_com_err: false,
            com_err_flg: false,
            lines: vec![],
            balloon: None,
        }
    }

    pub fn start_logprint_win(mut visible: bool) -> Result<(), Vec<String>> {
        if let Some(&true) = FORCE_WINDOW_MODE.get() {
            visible = true;
        }
        thread::spawn(move || {
            let mut counter = 0;
            let lp = loop {
                match LogPrintWin::new(visible) {
                    Ok(lp) => break lp,
                    Err(_e) => {
                        counter += 1;
                        #[cfg(debug_assertions)]
                        println!("\u{001b}[31m[debug] {_e}\u{001b}[0m");
                        if counter > 10 {
                            panic!("Failed to create logprint win");
                        }
                    },
                }
            };
            let lp2 = lp.clone();
            LOGPRINTWIN.get_or_init(move || Mutex::new(lp));
            lp2.message_loop().ok();
        });
        let now = std::time::Instant::now();
        let limit = std::time::Duration::from_millis(100);
        while LOGPRINTWIN.get().is_none() {
            if now.elapsed() > limit {
                return Err(vec![
                    UError::new(UErrorKind::InitializeError, UErrorMessage::FailedToInitializeLogPrintWindow).to_string()
                ]);
            } else {
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        Ok(())
    }
    pub fn stop_logprint_win() {
        if let Some(m) = LOGPRINTWIN.get() {
            m.lock().unwrap().close();
        }
    }

    pub fn get_line(&self, row: usize) -> EvalResult<String> {
        if row < 1 || row > self.lines.len(){
            Err(UError::new(
                UErrorKind::EvaluatorError,
                UErrorMessage::InvalidErrorLine(row)
            ))
        } else {
            Ok(self.lines[row - 1].to_string())
        }
    }

    pub fn eval(&mut self, program: Program, clear: bool) -> EvalResult<Option<Object>> {
        let mut result = None;
        let Program(program_block, mut lines) = program;
        self.lines.append(&mut lines);
        for statement in program_block {
            let res = stacker::maybe_grow(2 * 1024 * 1024, 20*1024*1024, || {
                self.eval_statement(statement)
            });
            match res {
                Ok(opt) => match opt {
                    Some(o) => match o {
                        Object::Exit => {
                            result = Some(Object::Exit);
                            break;
                        },
                        _ => result = Some(o),
                    },
                    None => ()
                },
                Err(e) => if let UErrorKind::ExitExit(n) = e.kind {
                    std::process::exit(n);
                } else {
                    return Err(e);
                }
            }
        }
        if clear {
            self.clear();
        }

        Ok(result)
    }

    fn eval_block_statement(&mut self, block: BlockStatement) -> EvalResult<Option<Object>> {
        for statement in block {
            match self.eval_statement(statement) {
                Ok(result) => match result {
                    Some(o) => match o {
                        Object::Continue(_) |
                        Object::Break(_) |
                        Object::Exit => return Ok(Some(o)),
                        _ => (),
                    },
                    None => (),
                },
                Err(e) => {
                    return Err(e);
                }
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
                    UErrorKind::HashtblError,
                    UErrorMessage::InvalidHashtblOption(o),
                ))
            },
            None => 0
        };
        let sort = (opt & HashTblEnum::HASH_SORT as u32) > 0;
        let casecare = (opt & HashTblEnum::HASH_CASECARE as u32) > 0;
        let hashtbl = HashTbl::new(sort, casecare);
        Ok((name, Object::HashTbl(Arc::new(Mutex::new(hashtbl)))))
    }

    fn eval_hash_sugar_statement(&mut self, hash: HashSugar) -> EvalResult<()> {
        let name = hash.name.0;
        let opt = match hash.option {
            Some(e) => match self.eval_expression(e)? {
                Object::Num(n) => n as u32,
                o => return Err(UError::new(
                    UErrorKind::HashtblError,
                    UErrorMessage::InvalidHashtblOption(o),
                ))
            },
            None => 0
        };
        let sort = (opt & HashTblEnum::HASH_SORT as u32) > 0;
        let casecare = (opt & HashTblEnum::HASH_CASECARE as u32) > 0;
        let mut hashtbl = HashTbl::new(sort, casecare);
        for (name_expr, val_expr) in hash.members {
            let name = if let Expression::Literal(Literal::ExpandableString(s)) = name_expr {
                self.expand_string(s, true, false).to_string()
            } else if let Expression::Literal(Literal::String(s)) = name_expr {
                s
            } else {
                name_expr.to_string()
            };
            let value = self.eval_expression(val_expr)?;
            hashtbl.insert(name, value);
        }
        let object = Object::HashTbl(Arc::new(Mutex::new(hashtbl)));
        if hash.is_public {
            self.env.define_public(&name, object)?;
        } else {
            self.env.define_local(&name, object)?;
        }
        Ok(())
    }

    fn eval_print_statement(&mut self, expression: Expression) -> EvalResult<Option<Object>> {
        let obj = self.eval_expression(expression)?;
        out_log(&format!("{}", obj), LogType::Print);
        if let Some(lp) = LOGPRINTWIN.get() {
            lp.lock().unwrap().print(&obj.to_string());
        }
        if ! *FORCE_WINDOW_MODE.get().unwrap_or(&false) {
            let out = stdout();
            let mut out = BufWriter::new(out.lock());
            writeln!(out, "{}", obj)?;
        }
        Ok(None)
    }

    fn set_option_settings(&self, opt: OptionSetting) {
        let mut usettings = USETTINGS.lock().unwrap();
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
                if let Object::String(s) = self.expand_string(s.clone(), true, false) {
                    let mut name_size = s.split(",");
                    let name = name_size.next().unwrap();
                    let size = name_size.next().unwrap_or("15").parse::<i32>().unwrap_or(15);
                    usettings.options.default_font = DefaultFont::new(name, size);
                }
            },
            OptionSetting::Position(x, y) => {
                usettings.options.position.left = x;
                usettings.options.position.top = y;
            },
            OptionSetting::Logpath(ref s) => {
                if let Object::String(s) = self.expand_string(s.clone(), true, false) {
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
                if let Object::String(ref s) = self.expand_string(s.clone(), true, false) {
                    env::set_var("UWSCR_DEFAULT_TITLE", s.as_str());
                    usettings.options.dlg_title = Some(s.to_string());
                }
            },
            OptionSetting::AllowIEObj(b) => usettings.options.allow_ie_object = b,
        }
    }

    fn eval_statement(&mut self, statement: StatementWithRow) -> EvalResult<Option<Object>> {
        let StatementWithRow { statement, row, line, script_name } = statement;
        let result = self.eval_statement_inner(statement);
        if self.ignore_com_err {
            match result {
                Ok(r) => Ok(r),
                Err(mut e) => if e.is_com_error {
                    self.com_err_flg = true;
                    Ok(None)
                } else {
                    if ! e.line.has_row() {
                        e.set_line(row, line, script_name);
                    }
                    Err(e)
                }

            }
        } else {
            match result {
                Ok(r) => Ok(r),
                Err(mut e) => {
                    if ! e.line.has_row() {
                        e.set_line(row, line, script_name);
                    }
                    Err(e)
                }
            }
        }
    }

    fn eval_statement_inner(&mut self, statement: Statement) -> EvalResult<Option<Object>> {
        match statement {
            Statement::Option(opt) => {
                self.set_option_settings(opt);
                Ok(None)
            },
            Statement::Dim(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_local(&name, value)?;
                }
                Ok(None)
            },
            Statement::Public(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_public(&name, value)?;
                }
                Ok(None)
            },
            Statement::Const(vec) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    self.env.define_const(&name, value)?;
                }
                Ok(None)
            },
            Statement::TextBlock(i, s) => {
                let Identifier(name) = i;
                let value = self.eval_literal(s, false)?;
                self.env.define_const(&name, value)?;
                Ok(None)
            },
            Statement::HashTbl(v) => {
                for (i, hashopt, is_public) in v {
                    let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, hashopt)?;
                    if is_public {
                        self.env.define_public(&name, hashtbl)?;
                    } else {
                        self.env.define_local(&name, hashtbl)?;
                    }
                }
                Ok(None)
            },
            Statement::Hash(hash) => {
                self.eval_hash_sugar_statement(hash)?;
                Ok(None)
            },
            Statement::Print(e) => self.eval_print_statement(e),
            Statement::Call(block, args) => {
                let Program(body, _) = block;
                let params = vec![
                    FuncParam::new(Some("PARAM_STR".into()), ParamKind::Identifier)
                ];
                let params_str = Expression::Literal(Literal::Array(args));
                let arguments = vec![
                    (Some(params_str.clone()), self.eval_expression(params_str)?)
                ];
                let func = Function {
                    name: None,
                    params,
                    body,
                    is_proc: true,
                    module: None,
                    outer: None,
                };
                let result = match func.invoke(self, arguments, false) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                };
                result
            },
            Statement::DefDll{name, params, ret_type, path} => {
                let func = self.eval_def_dll_statement(&name, &path, params, ret_type)?;
                self.env.define_dll_function(&name, func)?;
                Ok(None)
            },
            Statement::Struct(identifier, members) => {
                let name = identifier.0;
                let s = self.eval_struct_statement(&name, members)?;
                self.env.define_struct(&name, s)?;
                Ok(None)
            }
            Statement::Expression(e) => Ok(Some(self.eval_expression(e)?)),
            Statement::For {loopvar, from, to, step, block, alt} => {
                self.eval_for_statement(loopvar, from, to, step, block, alt)
            },
            Statement::ForIn {loopvar, collection, block, alt} => {
                self.eval_for_in_statement(loopvar, collection, block, alt)
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
                self.env.define_function(&fname, func)?;
                Ok(None)
            },
            Statement::Module(i, block) => {
                let Identifier(name) = i;
                let module = self.eval_module_statement(&name, block)?;
                self.env.define_module(&name, Object::Module(module))?;
                // コンストラクタがあれば実行する
                let module = self.env.get_module(&name);
                if let Some(Object::Module(m)) = module {
                    let maybe_constructor = if let Some(f) = m.lock().unwrap().get_constructor() {
                        Some(f)
                    } else {
                        None
                    };
                    if let Some(f) = maybe_constructor {
                        f.invoke(self, vec![], false)?;
                    }
                };
                Ok(None)
            },
            Statement::Class(i, block) => {
                let Identifier(name) = i;
                let class = Object::Class(name.clone(), block);
                self.env.define_class(&name, class)?;
                Ok(None)
            },
            Statement::With(o_e, block) => if let Some(e) = o_e {
                let s = self.eval_block_statement(block);
                if let Expression::Identifier(Identifier(name)) = e {
                    if name.find("@with_tmp_").is_some() {
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
            Statement::ExitExit(n) => Err(UError::exitexit(n)),
            Statement::ComErrIgn => {
                self.ignore_com_err = true;
                self.com_err_flg = false;
                Ok(None)
            },
            Statement::ComErrRet => {
                self.ignore_com_err = false;
                Ok(None)
            },
        }
    }

    fn eval_if_line_statement(&mut self, condition: Expression, consequence: StatementWithRow, alternative: Option<StatementWithRow>) -> EvalResult<Option<Object>> {
        if self.eval_expression(condition)?.is_truthy() {
            self.eval_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_if_statement(&mut self, condition: Expression, consequence: BlockStatement, alternative: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        if self.eval_expression(condition)?.is_truthy() {
            self.eval_block_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_block_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_elseif_statement(&mut self, condition: Expression, consequence: BlockStatement, alternatives: Vec<(Option<Expression>, BlockStatement)>) -> EvalResult<Option<Object>> {
        if self.eval_expression(condition)?.is_truthy() {
            return self.eval_block_statement(consequence);
        } else {
            for (altcond, block) in alternatives {
                match altcond {
                    Some(cond) => {
                        // elseif
                        if self.eval_expression(cond)?.is_truthy() {
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
                if case_obj.is_equal(&select_obj) {
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
            match self.eval_statement(statement) {
                Ok(opt) => if let Some(o) = opt {
                    match o {
                        Object::Continue(_) |
                        Object::Break(_) |
                        Object::Exit => return Ok(Some(o)),
                        _ => (),
                    }
                },
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(None)
    }

    fn eval_for_statement(&mut self,loopvar: Identifier, from: Expression, to: Expression, step: Option<Expression>, block: BlockStatement, alt: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let mut counter = match self.eval_expression(from)? {
            Object::Num(n) => n as i64,
            Object::Bool(b) => if b {1} else {0},
            Object::String(s) => {
                match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => return Err(UError::new(
                        UErrorKind::SyntaxError,
                        UErrorMessage::ForError(format!("for {} = {}", var, s)),
                    ))
                }
            },
            o => return Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForError(format!("for {} = {}", var, o)),
            )),
        };
        let counter_end = match self.eval_expression(to)? {
            Object::Num(n) => n as i64,
            Object::Bool(b) => if b {1} else {0},
            Object::String(s) => {
                match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => return Err(UError::new(
                        UErrorKind::SyntaxError,
                        UErrorMessage::ForError(format!("for {} = {} to {}", var, counter, s)),
                    ))
                }
            },
            o => return Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForError(format!("for {} = {} to {}", var, counter, o)),
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
                                UErrorKind::SyntaxError,
                                UErrorMessage::ForError(format!("for {} = {} to {} step {}", var, counter, counter_end, s)),
                            ))
                        }
                    },
                    o => return Err(UError::new(
                        UErrorKind::SyntaxError,
                        UErrorMessage::ForError(format!("for {} = {} to {} step {}", var, counter, counter_end, o)),
                    )),
                }
            },
            None => 1
        };
        if step == 0 {
            return Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForError("step can not be 0".into()),
            ));
        }
        self.env.assign(var.clone(), Object::Num(counter as f64))?;
        let broke = loop {
            if step > 0 && counter > counter_end || step < 0 && counter < counter_end {
                break false;
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
                            break true;
                        },
                        o => return Ok(Some(o))
                },
                _ => ()
            };
            counter += step;
            self.env.assign(var.clone(), Object::Num(counter as f64))?;
        };
        if ! broke && alt.is_some() {
            let block = alt.unwrap();
            self.eval_block_statement(block)?;
        }
        Ok(None)
    }

    fn eval_for_in_statement(&mut self, loopvar: Identifier, collection: Expression, block: BlockStatement, alt: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let col_obj = match self.eval_expression(collection)? {
            Object::Array(a) => a,
            Object::String(s) => s.chars().map(|c| Object::String(c.to_string())).collect::<Vec<Object>>(),
            Object::HashTbl(h) => h.lock().unwrap().keys(),
            Object::ByteArray(arr) => arr.iter().map(|n| Object::Num(*n as f64)).collect(),
            _ => return Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForInError
            ))
        };

        let mut broke = false;
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
                    broke = true;
                    break;
                },
                None => {},
                o => return Ok(o),
            }
        }
        if ! broke && alt.is_some() {
            let block = alt.unwrap();
            self.eval_block_statement(block)?;
        }
        Ok(None)
    }

    fn eval_loop_flg_expression(&mut self, expression: Expression) -> Result<bool, UError> {
        Ok(self.eval_expression(expression)?.is_truthy())
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

    fn eval_funtcion_definition_statement(&mut self, name: &String, params: Vec<FuncParam>, body: BlockStatement, is_proc: bool, is_async: bool) -> EvalResult<Object> {
        for statement in &body {
            match statement.statement {
                Statement::Function{name: _, params: _, body: _, is_proc: _, is_async: _}  => {
                    return Err(UError::new(
                        UErrorKind::FuncDefError,
                        UErrorMessage::NestedDefinition
                    ))
                },
                _ => {},
            };
        }
        let func = Function {
            name: Some(name.into()),
            params,
            body,
            is_proc,
            module: None,
            outer: None,
        };
        if is_async {
            Ok(Object::AsyncFunction(func))
        } else {
            Ok(Object::Function(func))
        }
    }

    fn eval_def_dll_statement(&mut self, name: &str, dll_path: &str, params: Vec<DefDllParam>, ret_type: DllType) -> EvalResult<Object> {
        Ok(Object::DefDllFunction(name.into(), dll_path.into(), params, ret_type))
    }

    fn eval_struct_statement(&mut self, name: &str, members: Vec<(String, DllType)>) -> EvalResult<Object> {
        let mut total_size = 0;
        for (_, t) in &members {
            match t {
                DllType::Int |
                DllType::Long |
                DllType::Bool => total_size += mem::size_of::<i32>(),
                DllType::Uint |
                DllType::Dword => total_size += mem::size_of::<u32>(),
                DllType::Word |
                DllType::Wchar => total_size += mem::size_of::<u16>(),
                DllType::Byte |
                DllType::Char => total_size += mem::size_of::<u8>(),
                DllType::Float => total_size += mem::size_of::<f32>(),
                DllType::Double => total_size += mem::size_of::<f64>(),
                DllType::Longlong => total_size += mem::size_of::<i64>(),
                DllType::Void => {},
                _ => total_size += mem::size_of::<usize>(),
            }
        }
        Ok(Object::Struct(name.into(), total_size, members))
    }

    fn new_ustruct(&self, name: &str, size: usize, members: Vec<(String, DllType)>, address: Option<usize>) -> EvalResult<Object> {
        let mut ustruct = UStruct::new(&name);
        for (n, t) in members {
            let o = match &t {
                DllType::String |
                DllType::Wstring |
                DllType::Pchar |
                DllType::PWchar => Object::Null,
                DllType::Unknown(s) => {
                    match self.env.get_struct(s) {
                        Some(o) => match o {
                            Object::Struct(name, size, members) => {
                                let o = self.new_ustruct(&name, size, members, None)?;
                                ustruct.add_struct(n, o, t);
                                continue;
                            },
                            _ => return Err(UError::new(
                                UErrorKind::StructDefError,
                                UErrorMessage::IsNotStruct(s.to_string()),
                            )),
                        },
                        None => return Err(UError::new(
                            UErrorKind::StructDefError,
                            UErrorMessage::StructNotDefined(s.to_string())
                        ))
                    }
                },
                _ => Object::Num(0.0)
            };
            ustruct.add(n, o, t)?;
        }
        if address.is_some() {
            ustruct.from_pointer(address.unwrap(), false);
        }
        Ok(Object::UStruct(name.to_string(), size, Arc::new(Mutex::new(ustruct))))
    }

    fn eval_module_statement(&mut self, module_name: &String, block: BlockStatement) -> EvalResult<Arc<Mutex<Module>>> {
        self.env.new_scope();
        let mut module = Module::new(module_name.to_string());
        for statement in block {
            match statement.statement {
                Statement::Dim(vec) => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        self.env.define_module_variable(&member_name, value.clone())?;
                        module.add(member_name, value, ContainerType::Variable);
                    }
                },
                Statement::Public(vec) => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        self.env.define_module_public(&member_name, value.clone())?;
                        module.add(member_name, value, ContainerType::Public);
                    }
                },
                Statement::Const(vec)  => {
                    for (i, e) in vec {
                        let Identifier(member_name) = i;
                        let value = self.eval_expression(e)?;
                        self.env.define_module_const(&member_name, value.clone())?;
                        module.add(member_name, value, ContainerType::Const);
                    }
                },
                Statement::TextBlock(i, s) => {
                    let Identifier(name) = i;
                    let value = self.eval_literal(s, false)?;
                    self.env.define_module_const(&name, value.clone())?;
                    module.add(name, value, ContainerType::Const);
                },
                Statement::HashTbl(v) => {
                    for (i, opt, is_pub) in v {
                        let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                        let container_type = if is_pub {
                            self.env.define_module_public(&name, hashtbl.clone())?;
                            ContainerType::Public
                        } else {
                            self.env.define_module_variable(&name, hashtbl.clone())?;
                            ContainerType::Variable
                        };
                        module.add(name, hashtbl, container_type);
                    }
                },
                Statement::Function{name: i, params, body, is_proc, is_async} => {
                    let Identifier(func_name) = i;
                    let mut new_body = Vec::new();
                    for statement in body {
                        match statement.statement {
                            Statement::Public(vec) => {
                                for (i, e) in vec {
                                    let Identifier(member_name) = i;
                                    let value = self.eval_expression(e)?;
                                    self.env.define_module_public(&member_name, value.clone())?;
                                    module.add(member_name, value, ContainerType::Public);
                                }
                            },
                            Statement::Const(vec) => {
                                for (i, e) in vec {
                                    let Identifier(member_name) = i;
                                    let value = self.eval_expression(e)?;
                                    self.env.define_module_const(&member_name, value.clone())?;
                                    module.add(member_name, value, ContainerType::Const);
                                }
                            },
                            Statement::HashTbl(v) => {
                                for (i, opt, is_pub) in v {
                                    if is_pub {
                                        let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                                        self.env.define_module_public(&name, hashtbl.clone())?;
                                        module.add(name, hashtbl, ContainerType::Public);
                                    }
                                }
                            },
                            Statement::TextBlock(i, s) => {
                                let Identifier(name) = i;
                                let value = self.eval_literal(s, false)?;
                                self.env.define_module_const(&name, value.clone())?;
                                module.add(name, value, ContainerType::Const);
                            },
                            Statement::Function{name: _, params: _, body: _, is_proc: _, is_async: _}  => {
                                return Err(UError::new(
                                    UErrorKind::FuncDefError,
                                    UErrorMessage::NestedDefinition
                                ));
                            },
                            _ => new_body.push(statement),
                        };
                    }
                    let func = Function {
                        name: Some(func_name.clone()),
                        params,
                        body: new_body,
                        is_proc,
                        module: None,
                        outer: None,
                    };
                    let func_obj = if is_async {
                        Object::AsyncFunction(func)
                    } else {
                        Object::Function(func)
                    };
                    self.env.define_module_function(&func_name, func_obj.clone())?;
                    module.add(
                        func_name,
                        func_obj,
                        ContainerType::Function,
                    );
                },
                _ => return Err(UError::new(
                    UErrorKind::SyntaxError,
                    UErrorMessage::Unknown,
                ))
            }
        }
        self.env.restore_scope(&None);
        let m = Arc::new(Mutex::new(module));
        {
            let mut module = m.lock().unwrap();
            module.set_module_reference_to_member_functions(Arc::clone(&m));

        }
        Ok(m)
    }

    fn eval_try_statement(&mut self, try_block: BlockStatement, except: Option<BlockStatement>, finally: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let opt_finally = {
            let usettings = USETTINGS.lock().unwrap();
            usettings.options.opt_finally
        };
        let obj = match self.eval_block_statement(try_block) {
            Ok(opt) => opt,
            Err(e) => if let UErrorKind::ExitExit(_) = e.kind {
                if opt_finally && finally.is_some() {
                    self.eval_block_statement(finally.unwrap())?;
                }
                return Err(e)
            } else {
                self.env.set_try_error_messages(
                    e.to_string(),
                    e.get_line().to_string()
                );
                if except.is_some() {
                    self.eval_block_statement(except.unwrap())?
                } else {
                    None
                }
            },
        };
        if ! opt_finally {
            // OPTFINALLYでない場合でexit、exitexitなら終了する
            match obj {
                Some(Object::Exit) => return Ok(obj),
                _ => {}
            }
        }
        if finally.is_some() {
            self.eval_block_statement(finally.unwrap())?;
        }
        Ok(obj)
    }

    fn eval_enum_statement(&mut self, name: String, uenum: UEnum) -> EvalResult<Option<Object>> {
        self.env.define_const(&name, Object::Enum(uenum))?;
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
                ignore_com_err: false,
                com_err_flg: false,
                lines: self.lines.clone(),
                balloon: None,
            };
            thread::spawn(move || {
                // このスレッドでのCOMを有効化
                if let Err(_) = com_object::com_initialize() {
                    panic!("Failed to initialize COM on new thread");
                }
                let old_hook = panic::take_hook();
                let uerror = Arc::new(Mutex::new(None::<UError>));
                let uerror2 = uerror.clone();
                panic::set_hook(Box::new(move |panic_info|{
                    let maybe_uerror = uerror2.lock().unwrap();
                    attach_console();
                    match &*maybe_uerror {
                        Some(e) => if let UErrorKind::ExitExit(n) = e.kind {
                            std::process::exit(n);
                        } else {
                            let err = e.to_string();
                            out_log(&err, LogType::Error);
                            let title = UWSCRErrorTitle::RuntimeError.to_string();
                            show_message(&err, &title, true);
                        },
                        None => {
                            let err = panic_info.to_string();
                            out_log(&err, LogType::Panic);
                            show_message(&err, "Panic on thread", true);
                        },
                    }
                    free_console();
                    std::process::exit(0);
                }));
                let result = thread_self.eval_function_call_expression(func, args, false);
                if let Err(e) = result {
                    {
                        let mut m = uerror.lock().unwrap();
                        *m = Some(e);
                    }
                    panic!("");
                } else {
                    panic::set_hook(old_hook);
                }
                com_object::com_uninitialize();
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
                            UErrorKind::ArrayError,
                            UErrorMessage::NoSizeSpecified,
                        ));
                    },
                    1 => {
                        let e = index_list[0].clone();
                        let size = match self.eval_expression(e)? {
                            Object::Num(n) => (n + 1.0) as usize,
                            Object::Empty => v.len(),
                            o => return Err(UError::new(
                                UErrorKind::ArrayError,
                                UErrorMessage::InvalidIndex(o),
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
                                        UErrorKind::ArrayError,
                                        UErrorMessage::ArraySizeOmitted,
                                    ));
                                } else {
                                    sizes.push(usize::MAX);
                                },
                                o => return Err(UError::new(
                                    UErrorKind::ArrayError,
                                    UErrorMessage::InvalidIndex(o),
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
                                UErrorKind::ArrayError,
                                UErrorMessage::InvalidArraySize,
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
            Expression::Literal(l) => self.eval_literal(l, false)?,
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
                let left = match *l {
                    Expression::DotCall(l, r) => {
                        self.eval_dotcall_expression(*l, *r, false, true)?
                    },
                    e => self.eval_expression(e)?
                };
                let index = self.eval_expression(*i)?;
                let hash_enum = if h.is_some() {
                    Some(self.eval_expression(h.unwrap())?)
                } else {
                    None
                };
                self.get_index_value(left, index, hash_enum)?
            },
            Expression::AnonymusFunction {params, body, is_proc} => {
                let outer_local = self.env.get_local_copy();
                Object::AnonFunc(Function {
                    name: None,
                    params,
                    body,
                    is_proc,
                    module: None,
                    outer: Some(Arc::new(Mutex::new(outer_local))),
                })
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
                self.eval_dotcall_expression(*l, *r, false, false)?
            },
            Expression::UObject(json) => {
                // 文字列展開する
                if let Object::String(ref s) = self.expand_string(json, true, false) {
                    match serde_json::from_str::<serde_json::Value>(s) {
                        Ok(v) => Object::UObject(UObject::new(v)),
                        Err(e) => return Err(UError::new(
                            UErrorKind::UObjectError,
                            UErrorMessage::JsonParseError(format!("Error message: {}", e)),
                        )),
                    }
                } else {
                    Object::Empty
                }
            },
            Expression::ComErrFlg => Object::Bool(self.com_err_flg),
            Expression::EmptyArgument => Object::EmptyParam,
            Expression::VarArgument(e) => Object::VarArgument(*e),
            Expression::Reference(e) => self.eval_reference(*e)?
        };
        if let Object::Reference(e) = obj {
            self.eval_expression(Expression::Reference(Box::new(e)))
        } else {
            Ok(obj)
        }
    }
    fn eval_reference(&mut self, expression: Expression) -> EvalResult<Object> {
        match expression {
            Expression::Identifier(Identifier(name)) => {
                self.env.get_from_outer(&name)
                    .ok_or(UError::new(UErrorKind::EvaluatorError, UErrorMessage::UnableToReference(name)))
                },
            Expression::Index(expr_array, expr_index, _) => {
                let array = match *expr_array {
                    Expression::DotCall(left, right) => {
                        let Expression::Identifier(Identifier(member)) = *right else {
                            return Err(UError::new(
                                UErrorKind::DotOperatorError,
                                UErrorMessage::InvalidRightExpression(*right)
                            ));
                        };
                        let instance = self.eval_reference(*left)?;
                        self.get_member(instance, member, false, true)?
                    },
                    Expression::Identifier(Identifier(name)) => self.env.get_from_outer(&name)
                        .ok_or(UError::new(UErrorKind::EvaluatorError, UErrorMessage::UnableToReference(name)))?,
                    _ => self.eval_reference(*expr_array)?,
                };
                let index = self.eval_reference(*expr_index)?;
                self.get_index_value(array, index, None)
            },
            Expression::DotCall(left, right) => {
                let Expression::Identifier(Identifier(member)) = *right else {
                    return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::InvalidRightExpression(*right)
                    ));
                };
                let instance = self.eval_reference(*left)?;
                self.get_member(instance, member, false, false)
            },
            Expression::Literal(literal) => {
                self.eval_literal(literal, true)
            },
            _ => Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::InvalidReference))
        }
    }

    fn eval_identifier(&self, identifier: Identifier) -> EvalResult<Object> {
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
                        None => match self.env.get_struct(&name) {
                            Some(o) => o,
                            None => return Err(UError::new(
                                UErrorKind::EvaluatorError,
                                UErrorMessage::NoIdentifierFound(name)
                            ))
                        }
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
                UErrorKind::PrefixError('-'),
                UErrorMessage::PrefixShouldBeNumber(right)
            ))
        }
    }

    fn eval_plus_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        if let Object::Num(n) = right {
            Ok(Object::Num(n))
        } else {
            Err(UError::new(
                UErrorKind::PrefixError('+'),
                UErrorMessage::PrefixShouldBeNumber(right)
            ))
        }
    }

    fn get_index_value(&mut self, left: Object, index: Object, hash_enum: Option<Object>) -> EvalResult<Object> {
        let obj = match &left {
            Object::Array(ref a) => if hash_enum.is_some() {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidKeyOrIndex(format!("[{}, {}]", index, hash_enum.unwrap()))
                ));
            } else if let Object::Num(i) = index {
                self.eval_array_index_expression(a.clone(), i as i64)?
            } else {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidIndex(index)
                ))
            },
            Object::HashTbl(h) => {
                let mut hash = h.lock().unwrap();
                let (key, i) = match index.clone(){
                    Object::Num(n) => (n.to_string(), Some(n as usize)),
                    Object::Bool(b) => (b.to_string(), None),
                    Object::String(s) => (s, None),
                    _ => return Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                };
                if let Some(hash_index_opt) = hash_enum {
                    if let Object::Num(n) = hash_index_opt {
                        if let Some(hahtblenum) = FromPrimitive::from_f64(n) {
                            match hahtblenum {
                                HashTblEnum::HASH_EXISTS => hash.check(key),
                                HashTblEnum::HASH_REMOVE => hash.remove(key),
                                HashTblEnum::HASH_KEY => if i.is_some() {
                                    hash.get_key(i.unwrap())
                                } else {
                                    return Err(UError::new(
                                    UErrorKind::EvaluatorError,
                                        UErrorMessage::MissingHashIndex("HASH_KEY".into())
                                    ));
                                },
                                HashTblEnum::HASH_VAL => if i.is_some() {
                                    hash.get_value(i.unwrap())
                                } else {
                                    return Err(UError::new(
                                        UErrorKind::EvaluatorError,
                                            UErrorMessage::MissingHashIndex("HASH_VAL".into())
                                    ));
                                },
                                _ => return Err(UError::new(
                                    UErrorKind::EvaluatorError,
                                    UErrorMessage::InvalidHashIndexOption(hash_index_opt)
                                ))
                            }
                        } else {
                            return Err(UError::new(
                                UErrorKind::EvaluatorError,
                                UErrorMessage::InvalidHashIndexOption(hash_index_opt)
                            ));
                        }
                    } else {
                        return Err(UError::new(
                            UErrorKind::EvaluatorError,
                            UErrorMessage::InvalidHashIndexOption(hash_index_opt)
                        ));
                    }
                } else {
                    hash.get(&key)
                }
            },
            Object::UObject(u) => if hash_enum.is_some() {
                return Err(UError::new(
                    UErrorKind::UObjectError,
                    UErrorMessage::InvalidKeyOrIndex(format!("[{}, {}]", index, hash_enum.unwrap())),
                ));
            } else {
                self.eval_uobject(u, index)?
            },
            Object::ComMember(ref disp, ref member) => {
                let key = index.to_variant()?;
                let keys = vec![key];
                let v = disp.get(member, Some(keys))?;
                Object::from_variant(&v)?
            },
            Object::ComObject(ref disp) => {
                // Item(key) の糖衣構文
                let key = index.to_variant()?;
                let keys = vec![key];
                let v = disp.get("Item", Some(keys))?;
                Object::from_variant(&v)?
            },
            Object::SafeArray(mut sa) => if hash_enum.is_some() {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidKeyOrIndex(format!("[{}, {}]", index, hash_enum.unwrap()))
                ));
            } else if let Object::Num(i) = index {
                let v = sa.get(i as i32)?;
                Object::from_variant(&v)?
            } else {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidIndex(index),
                ))
            },
            Object::ByteArray(arr) => if hash_enum.is_some() {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidKeyOrIndex(format!("[{}, {}]", index, hash_enum.unwrap()))
                ));
            } else if let Object::Num(i) = index {
                arr.get(i as usize)
                    .map(|n| Object::Num(*n as f64))
                    .ok_or(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::IndexOutOfBounds(index),
                    ))?
            } else {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::InvalidIndex(index)
                ))
            },
            o => return Err(UError::new(
                UErrorKind::Any("Evaluator::get_index_value".into()),
                UErrorMessage::NotAnArray(o.to_owned()),
            ))
        };
        Ok(obj)
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: i64) -> EvalResult<Object> {
        let max = (array.len() as i64) - 1;
        if index < 0 || index > max {
            return Err(UError::new(
                UErrorKind::EvaluatorError,
                UErrorMessage::IndexOutOfBounds(Object::Num(index as f64)),
            ));
        }
        let obj = array.get(index as usize).map_or(Object::Empty, |o| o.clone());
        Ok(obj)
    }

    fn eval_assign_expression(&mut self, left: Expression, value: Object) -> EvalResult<Object> {
        let left = self.expand_refence(left);

        self.class_instance_disposal(&left, &value);
        let assigned_value = value.clone();
        match left {
            Expression::Reference(e) => {
                match *e {
                    Expression::Identifier(Identifier(name)) => {
                        self.assign_identifier(&name, value, true)?;
                    }
                    Expression::Index(expr_array, expr_index, expr_hash_option) => {
                        if let Some(e) = *expr_hash_option {
                            let key = self.eval_expression(e)?;
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::InvalidKeyOrIndex(key.to_string()),
                            ));
                        }
                        self.assign_array(*expr_array, *expr_index, value, true, None)?;
                    }
                    Expression::DotCall(expr_object, expr_member) => {
                        self.assign_object_member(*expr_object, *expr_member, value, true)?;
                    },
                    _ => {
                        return Err(UError::new(UErrorKind::AssignError, UErrorMessage::InvalidReference));
                    }
                }
            },
            Expression::Identifier(Identifier(name)) => {
                self.assign_identifier(&name, value, false)?;
            },
            Expression::Index(expr_array, expr_index, expr_hash_option) => {
                if let Some(e) = *expr_hash_option {
                    let key = self.eval_expression(e)?;
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidKeyOrIndex(key.to_string()),
                    ));
                }
                self.assign_array(*expr_array, *expr_index, value, false, None)?;
            },
            Expression::DotCall(expr_object, expr_member) => {
                self.assign_object_member(*expr_object, *expr_member, value, false)?;
            },
            _ => return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::NotAVariable(left)
            ))
        }
        Ok(assigned_value)
    }
    fn assign_identifier(&mut self, name: &str, new: Object, is_reference: bool) -> EvalResult<()> {
        let maybe_this = if is_reference {
            self.env.get_from_outer("this")
        } else {
            self.env.get_variable("this", true)
        };
        if let Some(Object::This(mutex)) = maybe_this {
            if let Ok(mut module) = mutex.lock() {
                // module/classスコープ内であれば該当するメンバの値も更新してする
                module.assign(name, new.clone(), None)?;
            }
        }
        if is_reference {
            self.env.update_outer(name, new);
        } else {
            self.env.assign(name.into(), new)?;
        }
        Ok(())
    }
    fn assign_array(&mut self, expr_array: Expression, expr_index: Expression, new: Object, is_reference: bool, dimension: Option<Vec<Object>>) -> EvalResult<()> {
        match expr_array {
            Expression::Reference(expr_ref) => {
                self.assign_array(*expr_ref, expr_index, new, true, dimension)?;
            }
            Expression::Identifier(Identifier(name)) => {
                let index = self.eval_expression(expr_index)?;
                let maybe_object = if is_reference {
                    self.env.get_from_outer(&name)
                } else {
                    self.env.get_variable(&name, true)
                };
                if let Some(object) = maybe_object {
                    let dimension = match dimension {
                        Some(mut d) => {
                            d.push(index.clone());
                            d
                        },
                        None => vec![index.clone()],
                    };
                    let dimension2 = Some(dimension.clone());
                    let (maybe_new, update) = Self::update_array_object(object.clone(), dimension, &new)
                        .map_err(|mut e| {
                            if let UErrorMessage::NotAnArray(_) = e.message {
                                e.message = UErrorMessage::NotAnArray(name.clone().into());
                            }
                            e
                        })?;

                    if update {
                            if let Some(new_value) = maybe_new {
                                self.update_module_member_on_assignment(&name, new.clone(), is_reference, dimension2)?;
                                if is_reference {
                                    self.env.update_outer(&name, new_value)
                                } else {
                                    self.env.assign(name, new_value)?;
                                }
                            }
                        }
                    }
            },
            Expression::DotCall(expr_object, expr_member) => {
                let index = self.eval_expression(expr_index)?;
                let dimension = match dimension {
                    Some(mut d) => {
                        d.push(index.clone());
                        Some(d)
                    },
                    None => Some(vec![index.clone()]),
                };
                let instance = if is_reference {
                    self.eval_reference(*expr_object)
                } else {
                    self.eval_expression(*expr_object)
                }?;
                match instance {
                    Object::Module(mutex) |
                    Object::This(mutex) => {
                        match *expr_member {
                            Expression::Identifier(Identifier(name)) => {
                                mutex.lock().unwrap().assign(&name, new, dimension)?;
                            },
                            _ => return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::SyntaxError
                            ))
                        }
                    },
                    Object::Instance(mutex) => {
                        if let Expression::Identifier(Identifier(name)) = *expr_member {
                            let ins = mutex.lock().unwrap();
                            let mut module = ins.module.lock().unwrap();
                            module.assign(&name, new, dimension)?;
                        } else {
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::SyntaxError
                            ));
                        }
                    },
                    // Value::Array
                    Object::UObject(uo) => {
                        if let Expression::Identifier(Identifier(name)) = *expr_member {
                            let new_value = Self::object_to_serde_value(new)?;
                            uo.set(index, new_value, Some(name))?;
                        } else {
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::SyntaxError
                            ));
                        }
                    },
                    Object::ComObject(ref disp) => {
                        if let Expression::Identifier(Identifier(member)) = *expr_member {
                            let key = index.to_variant()?;
                            let keys = vec![key];
                            let var_value = new.to_variant()?;
                            disp.set(&member, var_value, Some(keys))?;
                        }
                    },
                    Object::Element(ref e) => {
                        if let Expression::Identifier(Identifier(name)) = *expr_member {
                            let value = Self::object_to_serde_value(new)?;
                            e.set_property(&name, value)?
                        } else {
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::SyntaxError
                            ));
                        }
                    },
                    Object::ElementProperty(ref ep) => {
                        if let Expression::Identifier(Identifier(name)) = *expr_member {
                            let value = Self::object_to_serde_value(new)?;
                            ep.set(&name, value)?;
                        } else {
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::SyntaxError
                            ));
                        }
                    },
                    o => return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::InvalidObject(o),
                    ))
                }
            },
            Expression::Index(expr_inner_array, expr_inner_index, _) => {
                let index = self.eval_expression(expr_index)?;
                let dimension = match dimension {
                    Some(mut d) => {
                        d.push(index);
                        Some(d)
                    },
                    None => Some(vec![index]),
                };
                self.assign_array(*expr_inner_array, *expr_inner_index, new, is_reference, dimension)?;
            }
            _ => return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::SyntaxError,
            ))
        };
        Ok(())
    }
    /// 戻り値
    /// (Some(更新された配列), 変数を更新すべきかどうか)
    pub fn update_array_object(array: Object, mut dimension: Vec<Object>, new: &Object) -> EvalResult<(Option<Object>, bool)> {
        let Some(index) = dimension.pop() else {
            return Ok((None, true));
        };

        match array {
            Object::Array(mut arr) => {
                let Object::Num(i) = index else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ));
                };
                let Some(obj) = arr.get_mut(i as usize) else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::IndexOutOfBounds(index)
                    ));
                };
                let (maybe_new, update) = Self::update_array_object(obj.clone(), dimension, new)?;
                if update {
                    match maybe_new {
                        Some(new) => *obj = new,
                        None => *obj = new.to_owned(),
                    }
                }
                Ok((Some(Object::Array(arr)), true))
            },
            Object::HashTbl(mutex) => {
                let name = match index {
                    Object::Num(n) => n.to_string(),
                    Object::Bool(b) => b.to_string(),
                    Object::String(s) => s,
                    _ => return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                };
                let mut hash = mutex.lock().unwrap();
                let obj = hash.get(&name);
                let (maybe_new, update) = Self::update_array_object(obj.clone(), dimension, new)?;
                if update {
                    if let Some(new_array) = maybe_new {
                        hash.insert(name, new_array);
                    } else {
                        hash.insert(name, new.to_owned());
                    }
                }
                Ok((None, false))
            },
            Object::ComObject(disp) => {
                // Item(key) の糖衣構文
                let key = index.to_variant()?;
                let keys = vec![key];
                let var_value = new.to_variant()?;
                disp.set("Item", var_value, Some(keys))?;
                Ok((Some(Object::ComObject(disp)), true))
            },
            Object::SafeArray(mut sa) => {
                if let Object::Num(i) = index {
                    let mut var_value = new.to_variant()?;
                    sa.set(i as i32, &mut var_value)?;
                    Ok((Some(Object::SafeArray(sa)), true))
                } else {
                    Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            Object::ByteArray(mut arr) => {
                let Object::Num(i) = index else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ));
                };
                let Some(val) = arr.get_mut(i as usize) else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ));
                };

                if let Object::Num(n) = new {
                    let new_val = u8::try_from(*n as i64)
                        .map_err(|_| UError::new(UErrorKind::AssignError, UErrorMessage::NotAnByte(new.to_owned())))?;
                    *val = new_val;
                } else {
                    return Err(UError::new(UErrorKind::AssignError, UErrorMessage::NotAnByte(new.to_owned())));
                }
                Ok((Some(Object::ByteArray(arr)), true))
            }
            _ => Err(UError::new(UErrorKind::AssignError, UErrorMessage::NotAnArray("".into())))
        }
    }
    fn assign_object_member(&mut self, expr_object: Expression, expr_member: Expression, new: Object, is_reference: bool) -> EvalResult<()> {
        let instance = if is_reference {
            self.eval_reference(expr_object)
        } else {
            self.eval_expression(expr_object)
        }?;
        match instance {
            Object::Module(m) => {
                match expr_member {
                    Expression::Identifier(Identifier(name)) => {
                        m.lock().unwrap().assign_public(&name, new, None)?;
                    },
                    _ => return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::SyntaxError
                    ))
                }
            },
            Object::Instance(m) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    let ins = m.lock().unwrap();
                    ins.module.lock().unwrap().assign_public(&name, new, None)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::SyntaxError
                    ));
                }
            },
            Object::This(m) => {
                if let Expression::Identifier(Identifier(member)) = expr_member {
                    let mut module = m.lock().unwrap();
                    module.assign(&member, new, None)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::SyntaxError
                    ));
                }
            },
            Object::Global => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    self.env.assign_public(name, new)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::GlobalVariableNotFound(None),
                    ))
                }
            },
            Object::UObject(uo) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    let index = Object::String(name);
                    let new_value = Self::object_to_serde_value(new)?;
                    uo.set(index, new_value, None)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::UStruct(_, _, m) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    let mut u = m.lock().unwrap();
                    u.set(name, new)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::UStructError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::ComObject(ref disp) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    let var_arg = new.to_variant()?;
                    disp.set(&name, var_arg, None)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::Element(ref e) => {
                if let Expression::Identifier(i) = expr_member {
                    let name = i.0;
                    let value = Self::object_to_serde_value(new)?;
                    e.set_property(&name, value)?
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::SyntaxError
                    ));
                }
            },
            Object::ElementProperty(ref ep) => {
                if let Expression::Identifier(i) = expr_member {
                    let name = i.0;
                    let value = Self::object_to_serde_value(new)?;
                    ep.set(&name, value)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::SyntaxError
                    ));
                }
            },
            o => return Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidObject(o)
            )),
        }
        Ok(())
    }
    /// 左辺がクラスインスタンスかつNOTHINGが代入された場合にdisposeする
    fn class_instance_disposal(&mut self, left: &Expression, new_value: &Object) {
        if let Object::Nothing = new_value {
            if let Ok(Object::Instance(ref mutex)) = self.eval_expression(left.clone()) {
                // 代入値がNOTHING、かつ元の値がクラスインスタンス
                let mut ins = mutex.lock().unwrap();
                if ! ins.is_dropped {
                    // 破棄済みでなければdispose
                    ins.dispose();
                }
            }
        }
    }
    /// 変数が参照渡しであれば参照元の式を展開する
    fn expand_refence(&self, expression: Expression) -> Expression {
        match expression {
            Expression::Identifier(ref identifier) => {
                if let Ok(Object::Reference(expr_ref)) = self.eval_identifier(identifier.clone()) {
                    let expr_ref = self.expand_refence_index(expr_ref);
                    Expression::Reference(Box::new(expr_ref))
                } else {
                    expression
                }
            }
            Expression::Index(e, index, h) => {
                let expr = self.expand_refence(*e);
                Expression::Index(Box::new(expr), index, h)
            }
            Expression::DotCall(e, member) => {
                let expr = self.expand_refence(*e);
                Expression::DotCall(Box::new(expr), member)
            },
            _ => expression
        }
    }
    /// 参照元の式が配列だったら添字も参照にする
    fn expand_refence_index(&self, expr_ref: Expression) -> Expression {
        match expr_ref {
            Expression::Index(expr_arr, expr_index, expr_hash) => {
                let expr_arr = self.expand_refence_index(*expr_arr);
                // 添字も参照にする
                let expr_index = Box::new(Expression::Reference(expr_index));
                Expression::Index(Box::new(expr_arr), expr_index, expr_hash)
            },
            e => e
        }
    }

    fn to_number(obj: &Object) -> Option<f64> {
        match obj {
            Object::Num(n) => Some(*n),
            Object::String(s) => match s.parse::<f64>() {
                Ok(n) => Some(n),
                Err(_) => None
            },
            Object::Empty => Some(0.0),
            Object::Bool(b) => Some(*b as i32 as f64),
            Object::Version(v) => Some(v.parse()),
            _ => None
        }
    }

    fn eval_infix_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        // VARIANT型だったらObjectに戻す
        if let Object::Variant(variant) = left {
            return self.eval_infix_expression(infix, Object::from_variant(&variant.0)?, right);
        }
        if let Object::Variant(variant) = right {
            return self.eval_infix_expression(infix, left, Object::from_variant(&variant.0)?);
        }
        // 論理演算子なら両辺の真性を評価してから演算する
        // ビット演算子なら両辺を数値とみなして演算する
        match infix {
            // 論理演算子
            Infix::AndL |
            Infix::OrL |
            Infix::XorL => return self.eval_infix_logical_operator_expression(
                infix, left.is_truthy(), right.is_truthy()
            ),
            // ビット演算子
            Infix::AndB |
            Infix::OrB |
            Infix::XorB => {
                let n_left = Self::to_number(&left);
                let n_right = Self::to_number(&right);
                if n_left.is_some() && n_right.is_some() {
                    return self.eval_infix_number_expression(infix, n_left.unwrap(), n_right.unwrap());
                } else {
                    return Err(UError::new(
                        UErrorKind::BitOperatorError,
                        UErrorMessage::LeftAndRightShouldBeNumber(left, infix, right),
                    ));
                }
            },
            _ => ()
        }
        match &left {
            Object::Num(n1) => {
                match right {
                    Object::Num(n) => {
                        self.eval_infix_number_expression(infix, *n1, n)
                    },
                    Object::String(s) => {
                        if infix == Infix::Plus {
                            self.eval_infix_string_expression(infix, &n1.to_string(), &s)
                        } else {
                            match s.parse::<f64>() {
                                Ok(n2) => self.eval_infix_number_expression(infix, *n1, n2),
                                Err(_) => self.eval_infix_string_expression(infix, &n1.to_string(), &s)
                            }
                        }
                    },
                    Object::Empty => self.eval_infix_number_expression(infix, *n1, 0.0),
                    Object::Bool(b) => self.eval_infix_number_expression(infix, *n1, b as i64 as f64),
                    Object::Version(v) => self.eval_infix_number_expression(infix, *n1, v.parse()),
                    _ => self.eval_infix_misc_expression(infix, left, right),
                }
            },
            Object::String(s1) => {
                match right {
                    Object::String(s2) => self.eval_infix_string_expression(infix, s1, &s2),
                    Object::Num(n) => {
                        if infix == Infix::Plus {
                            self.eval_infix_string_expression(infix, s1, &n.to_string())
                        } else {
                            match s1.parse::<f64>() {
                                Ok(n2) => self.eval_infix_number_expression(infix, n2, n),
                                Err(_) => if infix == Infix::Multiply {
                                    let s = s1.repeat(n as usize);
                                    Ok(Object::String(s))
                                } else {
                                    self.eval_infix_string_expression(infix, s1, &n.to_string())
                                }
                            }
                        }
                    },
                    Object::Bool(_) => self.eval_infix_string_expression(infix, s1, &right.to_string()),
                    Object::Empty => self.eval_infix_empty_expression(infix, left),
                    Object::Version(v) => self.eval_infix_string_expression(infix, s1, &v.to_string()),
                    Object::Null => self.eval_infix_null_expression(infix, Some(left), None),
                    _ => self.eval_infix_string_expression(infix, s1, &right.to_string())
                }
            },
            Object::Bool(l) => match right {
                Object::Bool(b) => self.eval_infix_logical_operator_expression(infix, *l, b),
                Object::String(s) => self.eval_infix_string_expression(infix, &left.to_string(), &s),
                Object::Empty => self.eval_infix_empty_expression(infix, left),
                Object::Num(n) => self.eval_infix_number_expression(infix, *l as i64 as f64, n),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Empty => match right {
                Object::Num(n) => self.eval_infix_number_expression(infix, 0.0, n),
                Object::String(_) => self.eval_infix_empty_expression(infix, right),
                Object::Empty => Ok(Object::Bool(true)),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Version(v1) => match right {
                Object::Version(v2) => self.eval_infix_number_expression(infix, v1.parse(), v2.parse()),
                Object::Num(n) => self.eval_infix_number_expression(infix, v1.parse(), n),
                Object::String(s) => self.eval_infix_string_expression(infix, &v1.to_string(), &s),
                _ => self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Array(a) => if infix == Infix::Plus {
                let mut new = a.to_owned();
                new.push(right);
                Ok(Object::Array(new))
            } else {
                self.eval_infix_misc_expression(infix, left, right)
            },
            Object::Null => self.eval_infix_null_expression(infix, None, Some(right)),
            _ => self.eval_infix_misc_expression(infix, left, right)
        }
    }

    fn eval_infix_misc_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => if let Object::String(s) = right {
                Object::String(format!("{}{}", left, s.clone()))
            } else {
                return Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::TypeMismatch(left, infix, right),
                ))
            },
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::TypeMismatch(left, infix, right),
            ))
        };
        Ok(obj)
    }

    fn eval_infix_null_expression(&mut self, infix: Infix, left: Option<Object>, right: Option<Object>) -> EvalResult<Object> {
        if let Some(obj) = left {
            match infix {
                Infix::Plus => if let Object::Num(n) = obj {
                    Ok(Object::Num(n))
                } else if let Object::String(s) = obj {
                    Ok(Object::String(format!("{s}\0")))
                } else {
                    self.eval_infix_misc_expression(infix, obj, Object::Null)
                },
                _ => self.eval_infix_misc_expression(infix, obj, Object::Null)
            }
        } else if let Some(obj) = right {
            match infix {
                Infix::Plus => if let Object::Num(n) = obj {
                    Ok(Object::Num(n))
                } else if let Object::String(s) = obj {
                    Ok(Object::String(format!("\0{s}")))
                } else {
                    self.eval_infix_misc_expression(infix, Object::Null, obj)
                },
                Infix::Multiply => if let Object::Num(n) = obj {
                    let repeated = "\0".repeat(n as usize);
                    Ok(Object::String(repeated))
                } else {
                    self.eval_infix_misc_expression(infix, Object::Null, obj)
                },
                _ => self.eval_infix_misc_expression(infix, Object::Null, obj)
            }
        } else {
            self.eval_infix_misc_expression(infix, Object::Null, Object::Null)
        }
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
            Infix::And | Infix::AndB => Object::Num((left as i64 & right as i64) as f64),
            Infix::Or | Infix::OrB => Object::Num((left as i64 | right as i64) as f64),
            Infix::Xor | Infix::XorB => Object::Num((left as i64 ^ right as i64) as f64),
            Infix::Assign => return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::SyntaxError
            )),
            _ => return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::TypeMismatch(left.into(), infix, right.into()),
            )),
        };
        match obj {
            Object::Num(n) => if ! n.is_finite() {
                // 無限またはNaNはエラーにする
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::NotFinite(n),
                ))
            } else {
                Ok(Object::Num(n))
            },
            o => Ok(o)
        }
    }

    fn eval_infix_string_expression(&mut self, infix: Infix, left: &str, right: &str) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            _ => return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::BadStringInfix(infix),
            ))
        };
        Ok(obj)
    }

    fn eval_infix_empty_expression(&mut self, infix: Infix, other: Object) -> EvalResult<Object> {
        let obj = match infix {
            Infix::Plus => Object::String(other.to_string()),
            Infix::Equal => Object::Bool(other.is_equal(&Object::Empty)),
            Infix::NotEqual => Object::Bool(! other.is_equal(&Object::Empty)),
            _ => return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::BadStringInfix(infix),
            ))
        };
        Ok(obj)
    }

    fn eval_infix_logical_operator_expression(&mut self, infix: Infix, left: bool, right: bool) -> EvalResult<Object> {
        let obj = match infix {
            Infix::And | Infix::AndL => Object::Bool(left && right),
            Infix::Or | Infix::OrL => Object::Bool(left || right),
            Infix::Xor | Infix::XorL => Object::Bool(left != right),
            _ => self.eval_infix_number_expression(infix, left as i64 as f64, right as i64 as f64)?
        };
        Ok(obj)
    }

    fn eval_literal(&mut self, literal: Literal, is_reference: bool) -> EvalResult<Object> {
        let obj = match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::ExpandableString(value) => self.expand_string(value, true, is_reference),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects, is_reference)?,
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
            Literal::NaN => Object::Num(f64::NAN),
            Literal::TextBlock(text, is_ex) => if is_ex {
                Object::ExpandableTB(text)
            } else {
                self.expand_string(text, false, is_reference)
            },
        };
        Ok(obj)
    }

    fn expand_string(&self, string: String, expand_var: bool, is_reference: bool) -> Object {
        let re = Regex::new("<#([^>]+)>").unwrap();
        let mut new_string = string.clone();
        for cap in re.captures_iter(string.as_str()) {
            let expandable = cap.get(1).unwrap().as_str();
            let rep_to: Option<Cow<str>> = match expandable.to_ascii_uppercase().as_str() {
                "CR" => Some("\r\n".into()),
                "TAB" => Some("\t".into()),
                "DBL" => Some("\"".into()),
                text => if expand_var {
                    if is_reference {
                        self.env.get_from_outer(text)
                    } else {
                        self.env.get_variable(text, false)
                    }.map(|o| o.to_string().into())
                } else {
                    continue;
                },
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    fn eval_array_literal(&mut self, expr_items: Vec<Expression>, is_reference: bool) -> EvalResult<Object> {
        let mut arr = vec![];
        for e in expr_items {
            let obj = if is_reference {
                self.eval_reference(e)
            } else {
                self.eval_expression(e)
            }?;
            arr.push(obj);
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
                        None => match self.env.get_struct(&name) {
                            Some(o) => Ok(o),
                            None => match self.env.get_variable(&name, true) {
                                Some(o) => Ok(o),
                                None => return Err(UError::new(
                                    UErrorKind::UndefinedError,
                                    UErrorMessage::FunctionNotFound(name),
                                )),
                            }
                        }
                    }
                }
            },
            Expression::DotCall(left, right) => Ok(
                self.eval_dotcall_expression(*left, *right, true, false)?
            ),
            _ => Ok(self.eval_expression(expression)?)
        }
    }

    fn new_task(&mut self, func: Function, arguments: Vec<(Option<Expression>, Object)>) -> UTask {
        // task用のselfを作る
        let mut task_self = Evaluator {
            env: Environment {
                current: Arc::new(Mutex::new(Layer {
                    local: Vec::new(),
                    outer: None,
                })),
                global: Arc::clone(&self.env.global)
            },
            ignore_com_err: false,
            com_err_flg: false,
            lines: self.lines.clone(),
            balloon: None,
        };
        // 関数を非同期実行し、UTaskを返す
        let handle = thread::spawn(move || {
            // このスレッドでのCOMを有効化
            com_object::com_initialize()?;

            let ret = func.invoke(&mut task_self, arguments, false);

            com_object::com_uninitialize();

            ret
        });
        let task = UTask {
            handle: Arc::new(Mutex::new(Some(handle))),
        };
        // Object::Task(task)
        task
    }

    fn await_task(&mut self, task: UTask) -> EvalResult<Object> {
        let mut handle = task.handle.lock().unwrap();
        match handle.take().unwrap().join() {
            Ok(res) => res,
            Err(e) => {
                Err(UError::new(
                    UErrorKind::TaskError,
                    UErrorMessage::TaskEndedIncorrectly(format!("{:?}", e))
                ))
            }
        }
    }

    fn builtin_func_result(&mut self, result: BuiltinFuncReturnValue, is_await: bool) -> EvalResult<Object> {
        let obj = match result {
            BuiltinFuncReturnValue::Eval(s) => {
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
                        UErrorKind::EvalParseErrors(errors.len()),
                        UErrorMessage::ParserErrors(parse_errors),
                    ));
                }
                self.eval(program, false)?.map_or(Object::Empty, |o| o)
            },
            BuiltinFuncReturnValue::GetEnv => {
                self.env.get_env()
            },
            BuiltinFuncReturnValue::ListModuleMember(name) => {
                self.env.get_module_member(&name)
            },
            BuiltinFuncReturnValue::BuiltinConstName(e) => {
                if let Some(Expression::Identifier(Identifier(name))) = e {
                    self.env.get_name_of_builtin_consts(&name)
                } else {
                    Object::Empty
                }
            },
            BuiltinFuncReturnValue::Task(func, arguments) => {
                let task = self.new_task(func, arguments);
                if is_await {
                    self.await_task(task)?
                } else {
                    Object::Task(task)
                }
            },
            BuiltinFuncReturnValue::GetLogPrintWinId => {
                let id = match LOGPRINTWIN.get() {
                    Some(m) => {
                        let lp = m.lock().unwrap();
                        builtins::window_control::get_id_from_hwnd(lp.hwnd())
                    },
                    None => -1.0,
                };
                Object::Num(id)
            },
            BuiltinFuncReturnValue::Balloon(balloon) => {
                match balloon {
                    Some(new) => match self.balloon {
                        Some(ref mut old) => old.redraw(new),
                        None => {
                            new.draw();
                            self.balloon = Some(new);
                        },
                    },
                    None => self.balloon = None,
                }
                Object::Empty
            },
            BuiltinFuncReturnValue::BalloonID => {
                match &self.balloon {
                    Some(b) => {
                        let hwnd = b.hwnd();
                        let id = builtins::window_control::get_id_from_hwnd(hwnd);
                        Object::Num(id)
                    },
                    None => Object::Num(-1.0),
                }
            },
            BuiltinFuncReturnValue::Token { token, remained, expression } => {
                if let Some(left) = expression {
                    let _ = self.eval_assign_expression(left, Object::String(remained));
                }
                Object::String(token)
            },
            BuiltinFuncReturnValue::Qsort(expr, array, exprs, arrays) => {
                if let Some(left) = expr {
                    let _ = self.eval_assign_expression(left, Object::Array(array));
                }
                for (expr, array) in exprs.into_iter().zip(arrays.into_iter()) {
                    if let Some(left) = expr {
                        if let Some(arr) = array {
                            let _ = self.eval_assign_expression(left, Object::Array(arr));
                        }
                    }
                }
                Object::Empty
            },
            BuiltinFuncReturnValue::Reference {refs, result} => {
                for (expr, value) in refs {
                    if let Some(left) = expr {
                        let _ = self.eval_assign_expression(left, value);
                    }
                }
                result
            },
            BuiltinFuncReturnValue::Result(obj) => obj,
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
            Object::Function(f) => f.invoke(self, arguments, false),
            Object::AsyncFunction(f) => {
                let task = self.new_task(f, arguments);
                if is_await {
                    self.await_task(task)
                } else {
                    Ok(Object::Task(task))
                }
            },
            Object::AnonFunc(f) => f.invoke(self, arguments, false),
            Object::BuiltinFunction(name, expected_len, f) => {
                if expected_len >= arguments.len() as i32 {
                    match f(BuiltinFuncArgs::new(arguments)) {
                        Ok(r) => self.builtin_func_result(r, is_await),
                        Err(e) => Err(e.to_uerror(name)),
                    }
                } else {
                    let l = arguments.len();
                    Err(UError::new(
                        UErrorKind::BuiltinFunctionError(name),
                        UErrorMessage::TooManyArguments(l, expected_len as usize)
                    ))
                }
            },
            // class constructor
            Object::Class(name, block) => {
                let m = self.eval_module_statement(&name, block)?;
                let constructor = {
                    let module = m.lock().unwrap();
                    match module.get_constructor() {
                        Some(constructor) => {
                            constructor
                        },
                        None => return Err(UError::new(
                            UErrorKind::ClassError,
                            UErrorMessage::ConstructorNotDefined(name.clone()),
                        )),
                    }
                };
                constructor.invoke(self, arguments, true)?;
                let ins = Arc::new(Mutex::new(ClassInstance::new(name, m, self.clone())));
                Ok(Object::Instance(ins))
            },
            Object::Struct(name, size, members) => {
                let ustruct =match arguments.len() {
                    0 => self.new_ustruct(&name, size, members, None)?,
                    1 => match arguments[0].1 {
                        Object::Num(n) => self.new_ustruct(&name, size, members, Some(n as usize))?,
                        _ => return Err(UError::new(
                            UErrorKind::UStructError,
                            UErrorMessage::InvalidStructArgument(name)
                        ))
                    },
                    n => return Err(UError::new(
                        UErrorKind::UStructError,
                        UErrorMessage::TooManyArguments(n, 1)
                    ))
                };
                Ok(ustruct)
            },
            Object::DefDllFunction(name, dll_path, params, ret_type) => {
                self.invoke_def_dll_function(name, dll_path, params, ret_type, arguments)
            },
            Object::ComMember(ref disp, name) => self.invoke_com_function(disp, &name, arguments),
            Object::ComObject(ref disp) => {
                // Item(key)の糖衣構文
                let mut keys = vec![];
                for (_, obj) in arguments {
                    let key = obj.to_variant()?;
                    keys.push(key)
                }
                let v = disp.get("Item", Some(keys))?;
                let obj = Object::from_variant(&v)?;
                Ok(obj)
            },
            Object::BrowserFunc(ref b, name) => {
                let args = arguments.into_iter().map(|(_, o)|o).collect();
                let res =  Self::invoke_browser_function(b.clone(), &name, args)?;
                Ok(res)
            },
            Object::ElementFunc(ref e, name) => {
                let args = arguments.into_iter().map(|(_, o)|o).collect();
                let res = Self::invoke_element_function(e.clone(), &name, args)?;
                Ok(res)
            }
            o => Err(UError::new(
                UErrorKind::EvaluatorError,
                UErrorMessage::NotAFunction(o),
            )),
        }
    }

    fn invoke_def_dll_function(&mut self, name: String, dll_path: String, params: Vec<DefDllParam>, ret_type: DllType, arguments: Vec<(Option<Expression>, Object)>) -> EvalResult<Object> {
        // dllを開く
        let lib = dlopen::raw::Library::open(&dll_path)?;
        unsafe {
            // 関数のシンボルを得る
            let f: *const c_void = lib.symbol(&name)?;
            // cifで使う
            let mut arg_types = vec![];
            let mut args = vec![];
            // 渡された引数のインデックス
            let mut i = 0;
            // 引数の実の値を保持するリスト
            let mut dll_args: Vec<DllArg> = vec![];
            // varされた場合に値を返す変数のリスト
            let mut var_list: Vec<(String, usize)> = vec![];

            for param in params {
                match param {
                    DefDllParam::Param {dll_type, is_var, is_array} => {
                        let (arg_exp, obj) = match arguments.get(i) {
                            Some((a, o)) => (a, o),
                            None => return Err(UError::new(
                                UErrorKind::DllFuncError,
                                UErrorMessage::DllMissingArgument(dll_type, i + 1),
                            ))
                        };
                        // 引数が変数なら変数名を得ておく
                        let arg_name = if let Some(Expression::Identifier(Identifier(ref name))) = arg_exp {
                            Some(name.to_string())
                        } else {
                            None
                        };

                        if is_array {
                            match obj {
                                Object::Array(_) => {
                                    let arr_arg = match DllArg::new_array(obj, &dll_type) {
                                        Ok(a) => a,
                                        Err(_) => return Err(UError::new(
                                            UErrorKind::DllFuncError,
                                            UErrorMessage::DllArrayHasInvalidType(dll_type, i + 1),
                                        ))
                                    };

                                    dll_args.push(arr_arg);
                                    arg_types.push(Type::pointer());
                                    // 配列はvarの有無に関係なく値を更新する
                                    if arg_name.is_some() {
                                        var_list.push((arg_name.unwrap(), i));
                                    }
                                },
                                _ => return Err(UError::new(
                                    UErrorKind::DllFuncError,
                                    UErrorMessage::DllArgumentIsNotArray(dll_type, i + 1)
                                ))
                            }
                        } else {
                            let t = Self::convert_to_libffi_type(&dll_type)?;
                            let dllarg = match DllArg::new(obj, &dll_type) {
                                Ok(a) => a,
                                Err(e) => return Err(UError::new(
                                    UErrorKind::DllFuncError,
                                    UErrorMessage::DllConversionError(dll_type, i + 1, e)
                                ))
                            };
                            match dllarg {
                                // null文字列の場合はvoid型にしておく
                                DllArg::Null => arg_types.push(Type::void()),
                                _ => arg_types.push(t)
                            }
                            dll_args.push(dllarg);
                            if is_var && arg_name.is_some() {
                                // var/ref が付いていれば後に値を更新
                                var_list.push((arg_name.unwrap(), dll_args.len() - 1));
                            }
                        }
                        i += 1;
                    },
                    DefDllParam::Struct(params) => {
                        let mut struct_size: usize = 0;
                        let mut members: Vec<(Option<String>, usize, DllArg)> = vec![];
                        // let mut struct_args: Vec<DllArg> = vec![];
                        for param in params {
                            match param {
                                DefDllParam::Param {dll_type, is_var: _, is_array} => {
                                    let (arg_exp, obj) = match arguments.get(i) {
                                        Some((a, o)) => (a, o),
                                        None => return Err(UError::new(
                                            UErrorKind::DllFuncError,
                                            UErrorMessage::DllMissingArgument(dll_type, i + 1)
                                        ))
                                    };
                                    // 引数が変数なら変数名を得ておく
                                    let arg_name = if let Some(Expression::Identifier(Identifier(ref name))) = arg_exp {
                                        Some(name.to_string())
                                    } else {
                                        None
                                    };

                                    let arg = if is_array {
                                        match DllArg::new_array(obj, &dll_type) {
                                            Ok(a) => a,
                                            Err(_) => return Err(UError::new(
                                                UErrorKind::DllFuncError,
                                                UErrorMessage::DllArrayHasInvalidType(dll_type, i + 1)
                                            ))
                                        }
                                    } else {
                                        match DllArg::new(obj, &dll_type) {
                                            Ok(a) => a,
                                            Err(e) => return Err(UError::new(
                                                UErrorKind::DllFuncError,
                                                UErrorMessage::DllArgumentTypeUnexpected(dll_type, i + 1, e)
                                            ))
                                        }
                                    };
                                    let size = arg.size();
                                    // struct_args.push(arg);
                                    members.push((arg_name, struct_size, arg));
                                    struct_size += size;
                                    i += 1;
                                },
                                DefDllParam::Struct(_) => return Err(UError::new(
                                    UErrorKind::DllFuncError,
                                    UErrorMessage::DllNestedStruct
                                )),
                            }
                        }
                        let structure = new_dll_structure(struct_size);
                        for (_, offset, arg) in &members {
                            match arg {
                                DllArg::Int(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Uint(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Hwnd(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Float(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Double(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Word(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::Byte(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::LongLong(v) => set_value_to_structure(structure, *offset, *v),
                                DllArg::IntArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::UintArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::HwndArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::FloatArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::DoubleArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::WordArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::ByteArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::LongLongArray(v) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::String(v, _) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::WString(v, _) => {
                                    let p = *v.as_ptr() as usize;
                                    set_value_to_structure(structure, *offset, p);
                                },
                                DllArg::Pointer(v) => set_value_to_structure(structure, *offset, v),
                                _ => return Err(UError::new(
                                    UErrorKind::DllFuncError,
                                    UErrorMessage::DllArgNotAllowedInStruct
                                )),
                            }
                        }
                        dll_args.push(DllArg::Struct(structure, members));
                        arg_types.push(Type::pointer());
                        var_list.push(("".into(), dll_args.len() -1));
                    },
                }
            }

            for dll_arg in &dll_args {
                args.push(dll_arg.to_arg());
            }

            let cif = Cif::new(arg_types.into_iter(), Self::convert_to_libffi_type(&ret_type)?);

            // 関数実行
            let result = match ret_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => {
                    let result = cif.call::<i32>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Uint |
                DllType::Dword => {
                    let result = cif.call::<u32>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Hwnd => {
                    let result = cif.call::<isize>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Float => {
                    let result = cif.call::<f32>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Double => {
                    let result = cif.call::<f64>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Word => {
                    let result = cif.call::<u16>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Byte |
                DllType::Char |
                DllType::Boolean => {
                    let result = cif.call::<u8>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Longlong => {
                    let result = cif.call::<i64>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Pointer => {
                    let result = cif.call::<usize>(CodePtr::from_ptr(f), &args);
                    Object::Num(result as f64)
                },
                DllType::Void => {
                    cif.call::<*mut c_void>(CodePtr::from_ptr(f), &args);
                    Object::Empty
                }
                _ =>  {
                    let result = cif.call::<*mut c_void>(CodePtr::from_ptr(f), &args);
                    println!("[warning] {} is not fully supported for return type.", ret_type);
                    Object::Num(result as isize as f64)
                }
            };

            // varの処理
            for (name, index) in var_list {
                let arg = &dll_args[index];
                match arg {
                    DllArg::Struct(p, m) => {
                        for (name, offset, arg) in m {
                            if name.is_some() {
                                let obj = get_value_from_structure(*p, *offset, arg);
                                self.env.assign(name.to_owned().unwrap(), obj)?;
                            }
                        }
                        free_dll_structure(*p);
                    },
                    DllArg::UStruct(p, m) => {
                        // 値をコピーしたらmallocした構造体は用済みなのでfreeする
                        let mut u = m.lock().unwrap();
                        u.from_pointer(*p as usize, true);
                        free_dll_structure(*p);
                    },
                    _ => {
                        let obj = arg.to_object();
                        self.env.assign(name, obj)?;
                    },
                }
                // if let DllArg::Struct(p, m) = arg {
                //     for (name, offset, arg) in m {
                //         if name.is_some() {
                //             let obj = get_value_from_structure(*p, *offset, arg);
                //             self.env.assign(name.to_owned().unwrap(), obj)?;
                //         }
                //     }
                //     free_dll_structure(*p);
                // } else {
                //     let obj = arg.to_object();
                //     self.env.assign(name, obj)?;
                // }
            }

            Ok(result)
        }
    }

    fn convert_to_libffi_type(dll_type: &DllType) -> EvalResult<Type> {
        let t = match dll_type {
            DllType::Int |
            DllType::Long |
            DllType::Bool => Type::i32(),
            DllType::Uint => Type::u32(),
            DllType::Hwnd => Type::isize(),
            DllType::String |
            DllType::Wstring => Type::pointer(),
            DllType::Float => Type::f32(),
            DllType::Double => Type::f64(),
            DllType::Word => Type::u16(),
            DllType::Dword => Type::u32(),
            DllType::Byte => Type::u8(),
            DllType::Char => Type::u8(),
            DllType::Pchar => Type::pointer(), // pointer to char
            DllType::Wchar => Type::u16(), // utf-16
            DllType::PWchar => Type::pointer(), // pointer to wchar
            DllType::Boolean => Type::u8(),
            DllType::Longlong => Type::i64(),
            DllType::SafeArray => Type::pointer(),
            DllType::Void => Type::void(),
            DllType::Pointer => Type::usize(),
            DllType::Struct => Type::pointer(),
            DllType::CallBack => Type::pointer(),
            DllType::Unknown(u) => return Err(UError::new(
                UErrorKind::DllFuncError,
                UErrorMessage::DllUnknownType(u.to_string())
            )),
        };
        Ok(t)
    }

    fn invoke_com_function(&mut self, disp: &IDispatch, name: &str, arguments: Vec<(Option<Expression>, Object)>) -> EvalResult<Object> {
        let mut var_index = vec![];
        let mut var_args = vec![];
        for (_, obj) in arguments {
            let v = if let Object::VarArgument(e) = obj {
                let o = self.eval_expression(e.clone())?;
                let i = var_args.len();
                var_index.push((i, e));
                o.to_variant()?
            } else {
                obj.to_variant()?
            };
            var_args.push(v);
        }

        let result = disp.run(name, &mut var_args)?;
        if var_index.len() > 0 {
            for (i, e) in var_index {
                let var = &var_args[i];
                let o = Object::from_variant(var)?;
                self.eval_assign_expression(e, o)?;
            }
        }
        Ok(Object::from_variant(&result)?)
    }

    fn get_browser_property(browser: &Browser, member: &str) -> EvalResult<Object> {
        match member.to_ascii_lowercase().as_str() {
            "document" => {
                let doc = browser.document()?;
                Ok(Object::Element(doc))
            },
            "pageid" => {
                let id = browser.id.to_string();
                Ok(Object::String(id))
            },
            "source" => match browser.execute_script("document.documentElement.outerHTML", None, None)? {
                Some(v) => Ok(v.into()),
                None => Ok(Object::Empty)
            },
            "url" => match browser.execute_script("document.URL", None, None)? {
                Some(v) => Ok(v.into()),
                None => Ok(Object::Empty)
            }
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(member.to_string())
            ))
        }
    }
    fn get_element_property(element: &Element, member: &str) -> EvalResult<Object> {
        // 特定のメンバ名の取得を試みるがなければプロパティ取得に移行
        match member.to_ascii_lowercase().as_str() {
            "url" => if let Some(url) = element.url()? {
                return Ok(url.into());
            },
            "parent" => if let Some(elem) = element.get_parent()? {
                return Ok(Object::Element(elem));
            },
            _ => {}
        }
        let obj = match element.get_property(&member)? {
            Value::Array(_) |
            Value::Object(_) => {
                let ep = ElementProperty::new(element.clone(), member.to_string());
                Object::ElementProperty(ep)
            }
            v => v.into()
        };
        Ok(obj)
    }
    fn invoke_browser_function(browser: Browser, name: &str, args: Vec<Object>) -> EvalResult<Object> {
        let get_arg = |i: usize| args.get(i).unwrap_or(&Object::Empty).to_owned();
        match name.to_ascii_lowercase().as_str() {
            "navigate" => {
                let uri = get_arg(0).to_string();
                let loaded = browser.navigate(&uri)?;
                Ok(Object::Bool(loaded))
            },
            "wait" => {
                let limit = match get_arg(0) {
                    Object::Num(n) => n,
                    _ => 10.0
                };
                let loaded = browser.wait_for_page_load(limit)?;
                Ok(Object::Bool(loaded))
            },
            "execute" => {
                let script = get_arg(0).to_string();
                let value = match get_arg(1) {
                    Object::UObject(uo) => {
                        Some(uo.value())
                    },
                    Object::Empty => None,
                    o => Some(Self::object_to_serde_value(o.to_owned())?)
                };
                let res = match get_arg(2) {
                    Object::Empty => browser.execute_script(&script, value, None)?,
                    o => {
                        let name = o.to_string();
                        browser.execute_script(&script, value, Some(name.as_str()))?
                    }
                };
                Ok(res.map_or_else(|| Object::Empty, |v| v.into()))
            }
            "reload" => {
                let ignore_cache = get_arg(0).is_truthy();
                browser.reload(ignore_cache)?;
                Ok(Object::Empty)
            }
            "close" => {
                browser.close()?;
                Ok(Object::Empty)
            },
            "gettabs" => {
                let filter = match get_arg(0) {
                    Object::Empty => None,
                    o => Some(o.to_string())
                };
                let tabs = browser.get_tabs(filter)?;
                let arr = tabs.into_iter().map(|t| {
                    Object::Array(vec![
                        t.title.into(),
                        t.url.into(),
                        t.id.into(),
                    ])
                }).collect();
                Ok(Object::Array(arr))
            },
            "newtab" => {
                let uri = get_arg(0).to_string();
                let new = browser.new_tab(&uri)?;
                Ok(Object::Browser(new))
            },
            "activate" => {
                browser.activate()?;
                Ok(Object::Empty)
            },
            "windowid" => browser.get_window_id().map_err(|e| e.into()),
            "dialog" => {
                let (accept, prompt) = match get_arg(0) {
                    Object::String(s) => (true, Some(s)),
                    o => (o.is_truthy(), None)
                };
                browser.dialog(accept, prompt)?;
                Ok(Object::Empty)
            },
            "setdownloadpath" => {
                let path = match get_arg(0) {
                    Object::Empty |
                    Object::EmptyParam => None,
                    o => Some(o.to_string())
                };
                browser.set_download_path(path)?;
                Ok(Object::Empty)
            },
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    fn invoke_element_function(element: Element, name: &str, args: Vec<Object>) -> EvalResult<Object> {
        let get_arg = |i: usize| args.get(i).unwrap_or(&Object::Empty).to_owned();
        match name.to_ascii_lowercase().as_str() {
            "execute" => {
                let script = get_arg(0).to_string();
                let value = match get_arg(1) {
                    Object::UObject(uo) => {
                        Some(uo.value())
                    },
                    Object::Empty => None,
                    o => Some(Self::object_to_serde_value(o.to_owned())?)
                };
                let res = match get_arg(2) {
                    Object::Empty => element.execute_script(&script, value, None)?,
                    o => {
                        let name = o.to_string();
                        element.execute_script(&script, value, Some(name.as_str()))?
                    }
                };
                Ok(res.into())
            },
            "queryselector" => {
                let selector = get_arg(0).to_string();
                let o = match element.query_selector(&selector)? {
                    Some(e) => Object::Element(e),
                    None => Object::Empty
                };
                Ok(o)
            },
            "queryselectorall" => {
                let selector = get_arg(0).to_string();
                let arr = element.query_selector_all(&selector)?
                    .into_iter()
                    .map(|e| Object::Element(e))
                    .collect();
                Ok(Object::Array(arr))
            },
            "focus" => {
                element.focus()?;
                Ok(Object::Empty)
            },
            "input" => {
                let input = get_arg(0).to_string();
                element.input(&input)?;
                Ok(Object::Empty)
            },
            "clear" => {
                element.clear()?;
                Ok(Object::Empty)
            },
            "setfile" => {
                let files = match get_arg(0) {
                    Object::Array(arr) => arr.into_iter().map(|o| o.to_string()).collect(),
                    o => vec![o.to_string()]
                };
                element.set_file_input(files)?;
                Ok(Object::Empty)
            },
            "click" => {
                element.click()?;
                Ok(Object::Empty)
            },
            "select" => {
                element.select()?;
                Ok(Object::Empty)
            },
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }

    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> EvalResult<Object> {
        let condition = self.eval_expression(condition)?;
        if condition.is_truthy() {
            self.eval_expression(consequence)
        } else {
            self.eval_expression(alternative)
        }
    }

    fn eval_dotcall_expression(&mut self, left: Expression, right: Expression, is_func: bool, is_com_property: bool) -> EvalResult<Object> {
        let instance = match left {
            Expression::Identifier(_) |
            Expression::Index(_, _, _) |
            Expression::FuncCall{func:_, args:_, is_await:_} |
            Expression::DotCall(_, _) |
            Expression::UObject(_) => {
                self.eval_expression(left)?
            },
            Expression::Reference(e) => {
                self.eval_reference(*e)?
            }
            e => return Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidLeftExpression(e),
            )),
        };
        let Expression::Identifier(Identifier(member)) = right else {
            return Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidRightExpression(right)
            ));
        };
        self.get_member(instance, member, is_func, is_com_property)
    }
    fn get_member(&self, instance: Object, member: String, is_func: bool, is_com_property: bool) -> EvalResult<Object> {
        match instance {
            Object::Module(m) => {
                self.get_module_member(&m, &member, is_func)
            },
            Object::Instance(m) => {
                let ins = m.lock().unwrap();
                self.get_module_member(&ins.module, &member, is_func)
            },
            Object::This(m) => {
                let module = m.lock().unwrap();
                if is_func {
                    module.get_function(&member)
                } else {
                    match module.get_member(&member) {
                        Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true, false)),
                        res => res
                    }
                }
            },
            Object::Global => {
                self.env.get_global(&member, is_func)
            },
            Object::Class(name, _) => Err(UError::new(
                UErrorKind::ClassError,
                UErrorMessage::ClassMemberCannotBeCalledDirectly(name)
            )),
            Object::UObject(u) => {
                self.eval_uobject(&u, member.into())
            },
            Object::Enum(e) => {
                if let Some(n) = e.get(&member) {
                    Ok(Object::Num(n))
                } else {
                    Err(UError::new(
                        UErrorKind::EnumError,
                        UErrorMessage::MemberNotFound(member)
                    ))
                }
            },
            Object::UStruct(_, _, m) => {
                let u = m.lock().unwrap();
                u.get(member)
            },
            Object::ComObject(ref disp) => {
                let obj = if is_func || is_com_property {
                    Object::ComMember(disp.clone(), member)
                } else {
                    let v = disp.get(&member, None)?;
                    Object::from_variant(&v)?
                };
                Ok(obj)
            },
            Object::Browser(ref b) => if is_func {
                Ok(Object::BrowserFunc(b.clone(), member))
            } else {
                Self::get_browser_property(b, &member)
            },
            Object::Element(ref e) => if is_func {
                Ok(Object::ElementFunc(e.clone(), member))
            } else {
                Self::get_element_property(e, &member)
            },
            Object::ElementProperty(ref e) => {
                let member = e.property(Some(&member));
                Self::get_element_property(&e.element, &member)
            },
            o => Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::DotOperatorNotSupported(o)
            )),
        }
    }

    fn get_module_member(&self, mutex: &Arc<Mutex<Module>>, member: &String, is_func: bool) -> EvalResult<Object> {
        let module = mutex.lock().unwrap(); // Mutex<Module>をロック
        if is_func {
            module.get_function(&member)
        } else if module.is_local_member(&member) {
            if let Some(Object::This(this)) = self.env.get_variable("this", true) {
                if this.try_lock().is_err() {
                    // ロックに失敗した場合、上でロックしているMutexと同じだと判断
                    // なので自分のモジュールメンバの値を返す
                    return module.get_member(&member);
                }
            }
            Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::IsPrivateMember(module.name(), member.to_string())
            ))
        } else {
            match module.get_public_member(&member) {
                Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true, false)),
                res => res
            }
        }
    }
    /// 代入処理時にmodule/classスコープ内であれば同名メンバも更新する
    fn update_module_member_on_assignment(&self, name: &str, new: Object, is_reference: bool, dimension: Option<Vec<Object>>) -> EvalResult<()> {
        let maybe_this = if is_reference {
            self.env.get_from_outer("this")
        } else {
            self.env.get_variable("this", true)
        };
        // thisがあればmodule/classスコープ
        if let Some(Object::This(mutex)) = maybe_this {
            if let Ok(mut module) = mutex.lock() {
                module.assign(name, new, dimension)?;
            }
        }
        Ok(())
    }

    fn eval_uobject(&self, uobject: &UObject, index: Object) -> EvalResult<Object> {
        let o = match uobject.get(&index)? {
            Some(value) => match value {
                Value::Null => Object::Null,
                Value::Bool(b) => Object::Bool(b),
                Value::Number(n) => match n.as_f64() {
                    Some(f) => Object::Num(f),
                    None => return Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::CanNotConvertToNumber(n.clone())
                    )),
                },
                Value::String(s) => {
                    self.expand_string(s.clone(), true, false)
                },
                Value::Array(_) |
                Value::Object(_) => {
                    let pointer = uobject.pointer(Some(index)).unwrap();
                    let new_obj = uobject.clone_with_pointer(pointer);
                    Object::UObject(new_obj)
                },
            },
            None => return Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::InvalidMemberOrIndex(index.to_string()),
            )),
        };
        Ok(o)
    }

    fn object_to_serde_value(o: Object) -> EvalResult<serde_json::Value> {
        let v = match o {
            Object::Null => serde_json::Value::Null,
            Object::Bool(b) => serde_json::Value::Bool(b),
            Object::Num(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
            Object::String(ref s) => serde_json::Value::String(s.clone()),
            Object::UObject(ref u) => u.value(),
            o => return Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::CanNotConvertToUObject(o)
            )),
        };
        Ok(v)
    }

}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::error::evaluator::{UErrorKind,UErrorMessage,DefinitionType,ParamTypeDetail};

    fn eval_test(input: &str, expected: Result<Option<Object>, UError>, ast: bool) {
        let mut e = Evaluator::new(Environment::new(vec![]));
        let program = Parser::new(Lexer::new(input)).parse();
        if ast {
            println!("{:?}", program);
        }
        let result = e.eval(program, true);
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
                Err(e) => panic!("this test should be ok: {}\n error: {}", input, e),
            },
            Err(expected_err) => match result {
                Ok(_) => panic!("this test should occure error:\n{}", input),
                Err(result_err) => assert_eq!(result_err, expected_err),
            },
        }
    }


    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        let program = Parser::new(Lexer::new(input)).parse();
        let mut e = Evaluator::new(Environment::new(vec![]));
        match e.eval(program, false) {
            Ok(_) => e,
            Err(err) => panic!("\nError:\n{:#?}\ninput:\n{}\n", err, input)
        }
    }

    //
    fn eval_test_with_env(e: &mut Evaluator, input: &str, expected: Result<Option<Object>, UError>) {
        let program = Parser::new(Lexer::new(input)).parse();
        let result = e.eval(program, false);
        match expected {
            Ok(expected_obj) => match result {
                Ok(result_obj) => if result_obj.is_some() && expected_obj.is_some() {
                    let left = result_obj.unwrap();
                    let right = expected_obj.unwrap();
                    if ! left.is_equal(&right) {
                        panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", left, right, input);
                    }
                } else if result_obj.is_some() || expected_obj.is_some() {
                    // どちらかがNone
                    panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", result_obj, expected_obj, input);
                },
                Err(e) => panic!("this test should be ok: {}\n error: {}\n", input, e),
            },
            Err(expected_err) => match result {
                Ok(_) => panic!("this test should occure error:\n{}", input),
                Err(result_err) => if result_err != expected_err {
                    panic!("\nresult: {}\nexpected: {}\n\ninput: {}\n", result_err, expected_err, input);
                },
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
                    UErrorKind::DefinitionError(DefinitionType::Variable),
                    UErrorMessage::AlreadyDefined("hoge".into())
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
            (
                r#"
// gh-27
hashtbl a
a['a'] = 'a'
hashtbl b
b['b'] = 'b'
h = a
h = b // 再代入に成功すればOK
h == b
        "#,
                Ok(Some(Object::Bool(true)))
            ),
            (
                r#"
hash hoge = hash_casecare or hash_sort
    foo = 1
    bar = 2
endhash
hoge['foo'] = 1 and hoge['bar'] = 2
        "#,
                {
                    Ok(Some(Object::Bool(true)))
                }
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
    fn test_assign_multi_dimensional_array() {
        let test_cases = vec![
            (
                r#"
hoge = [[1],[2]]
hoge[0][0] = 100
hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![Object::Num(100.0)]),
                    Object::Array(vec![Object::Num(2.0)]),
                ])))
            ),
            (
                r#"
hoge = [[[1]]]
hoge[0][0][0] = 100
hoge
                "#,
                Ok(Some(Object::Array(vec![
                    Object::Array(vec![
                        Object::Array(vec![Object::Num(100.0)]),
                    ]),
                ])))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test(input, expected, false);
        }
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
                    UErrorKind::ArrayError,
                    UErrorMessage::InvalidArraySize
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
                    UErrorKind::SyntaxError,
                    UErrorMessage::ForError("for i = 0 to 5s".into())
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
            (
                r#"
a = 0
for i = 0 to 0
    a = 1
else
    a = 2
endfor
a
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = 0
for i = 0 to -1
    // ここを通らない場合
    a = 1
else
    a = 2
endfor
a
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = 0
for i = 0 to 0
    a = 1
    break
else
    a = 2
endfor
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
            (
                r#"
a = 0
for n in [1,2,3]
    a = 1
else
    a = 2
endfor
a
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = 0
for n in []
    // ここを通らない場合
    a = 1
else
    a = 2
endfor
a
                "#,
                Ok(Some(Object::Num(2.0)))
            ),
            (
                r#"
a = 0
for n in [1,2,3]
    a = 1
    break
else
    a = 2
endfor
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
if true then print "test succeed!" else print "should not be printed"
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
    a = "test succeed!"
else
    a = "should not get this message"
endif
a
                "#,
                Ok(Some(Object::String("test succeed!".to_string())))
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
    a = "test1 succeed!"
endif
a
                "#,
                Ok(Some(Object::String("test1 succeed!".to_string())))
            ),
            (
                r#"
if false then
    a = "should not get this message"
elseif false then
    a = "should not get this message"
elseif true then
    a = "test2 succeed!"
endif
a
                "#,
                Ok(Some(Object::String("test2 succeed!".to_string())))
            ),
            (
                r#"
if false then
    a = "should not get this message"
elseif false then
    a = "should not get this message"
else
    a = "test3 succeed!"
endif
a
                "#,
                Ok(Some(Object::String("test3 succeed!".to_string())))
            ),
            (
                r#"
if true then
    a = "test4 succeed!"
elseif true then
    a = "should not get this message"
else
    a = "should not get this message"
endif
a
                "#,
                Ok(Some(Object::String("test4 succeed!".to_string())))
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
        a = "test1 succeed!"
    case 2
        a = "should not get this message"
    default
        a = "should not get this message"
selend
a
                "#,
                Ok(Some(Object::String("test1 succeed!".to_string())))
            ),
            (
                r#"
select 3
    case 1
        a = "should not get this message"
    case 2, 3
        a = "test2 succeed!"
    default
        a = "should not get this message"
selend
a
                "#,
                Ok(Some(Object::String("test2 succeed!".to_string())))
            ),
            (
                r#"
select 6
    case 1
        a = "should not get this message"
    case 2, 3
        a = "should not get this message"
    default
        a = "test3 succeed!"
selend
a
                "#,
                Ok(Some(Object::String("test3 succeed!".to_string())))
            ),
            (
                r#"
select 6
    default
        a = "test4 succeed!"
selend
a
                "#,
                Ok(Some(Object::String("test4 succeed!".to_string())))
            ),
            (
                r#"
select true
    case 1 = 2
        a = "should not get this message"
    case 2 = 2
        a = "test5 succeed!"
selend
a
                "#,
                Ok(Some(Object::String("test5 succeed!".to_string())))
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
                Ok(Some(Object::Num(1.0)))
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
                    UErrorKind::DefinitionError(DefinitionType::Variable),
                    UErrorMessage::AlreadyDefined("dim_and_dim".into())
                ))
            ),
            (
                r#"
public pub_and_const = 1
const pub_and_const = 2
                "#,
                Err(UError::new(
                    UErrorKind::DefinitionError(DefinitionType::Const),
                    UErrorMessage::AlreadyDefined("pub_and_const".into())
                ))
            ),
            (
                r#"
const const_and_const = 1
const const_and_const = 2
                "#,
                Err(UError::new(
                    UErrorKind::DefinitionError(DefinitionType::Const),
                    UErrorMessage::AlreadyDefined("const_and_const".into())
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
                    UErrorKind::DefinitionError(DefinitionType::Variable),
                    UErrorMessage::AlreadyDefined("hash_and_hash".into())
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
                    UErrorKind::DefinitionError(DefinitionType::Function),
                    UErrorMessage::AlreadyDefined("func_and_func".into())
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
                Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::NoIdentifierFound("v".into())
                ))
            ),
            (
                "M.v",
                Err(UError::new(
                    UErrorKind::DotOperatorError,
                    UErrorMessage::IsPrivateMember("M".into(), "v".into())
                ))
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
    fn test_uobject() {
        let input1 = r#"
dim obj = @{
    "foo": 1,
    "bar": {
        "baz": 2,
        "qux": [3, 4, 5]
    }
}@
        "#;
        let mut e = eval_env(input1);
        let test_cases = vec![
            (
                "obj.foo",
                Ok(Some(Object::Num(1.0)))
            ),
            (
                "obj.FOO",
                Ok(Some(Object::Num(1.0)))
            ),
            (
                "obj.bar.baz",
                Ok(Some(Object::Num(2.0)))
            ),
            (
                "obj.bar.qux[0]",
                Ok(Some(Object::Num(3.0)))
            ),
            (
                "obj.foo = 2; obj.foo",
                Ok(Some(Object::Num(2.0)))
            ),
            (
                "obj.bar.qux[1] = 9; obj.bar.qux[1]",
                Ok(Some(Object::Num(9.0)))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test_with_env(&mut e, input, expected);
        }
    }

    #[test]
    fn test_param_type() {
        let input1 = r#"
function hoge(s: string, c: myclass, n: number = 2, b: bool = false)
    result = EMPTY
fend
function fuga(a: array, h: hash, f: func, u: uobject)
    result = EMPTY
fend
public arr = [1,2,3]
public hashtbl h
public uo = @{"a": 1}@
class myclass
    procedure myclass()
    fend
endclass
class myclass2
    procedure myclass2()
    fend
endclass
        "#;
        let mut e = eval_env(input1);
        let test_cases = vec![
            (
                "hoge(3, myclass())",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("s".into(), ParamTypeDetail::String)
                ))
            ),
            (
                "hoge('hoge', myclass2())",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("c".into(), ParamTypeDetail::UserDefinition("myclass".into()))
                ))
            ),
            (
                "hoge('hoge', myclass(), 'aaa')",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("n".into(), ParamTypeDetail::Number)
                ))
            ),
            (
                "hoge('hoge', myclass(),2, 'aaa')",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("b".into(), ParamTypeDetail::Bool)
                ))
            ),
            (
                "hoge('hoge', myclass(), 5, true)",
                Ok(Some(Object::Empty))
            ),
            (
                "fuga('hoge', h, hoge, uo)",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("a".into(), ParamTypeDetail::Array)
                ))
            ),
            (
                "fuga(arr, arr, hoge, uo)",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("h".into(), ParamTypeDetail::HashTbl)
                ))
            ),
            (
                "fuga(arr, h, 'hoge', uo)",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("f".into(), ParamTypeDetail::Function)
                ))
            ),
            (
                "fuga(arr, h, hoge, 1)",
                Err(UError::new(
                    UErrorKind::FuncCallError,
                    UErrorMessage::InvalidParamType("u".into(), ParamTypeDetail::UObject)
                ))
            ),
            (
                "fuga(arr, h, hoge, uo)",
                Ok(Some(Object::Empty))
            ),
        ];
        for (input, expected) in test_cases {
            eval_test_with_env(&mut e, input, expected);
        }
    }

    #[test]
    fn test_reference() {
        let input1 = r#"
function test(ref p)
    p = "reference test"
fend
        "#;
        let mut e = eval_env(input1);
        let test_cases = vec![
            (
                r#"
v = "hoge"
test(v)
v
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
arr = ["hoge"]
test(arr[0])
arr[0]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
arr = ["hoge"]
i = 0
test(arr[i])
arr[i]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
function test2(ref p: array, i: number)
    p[i] = "test2"
    result = p[i]
fend
arr = ["hoge"]
test2(arr, 0)
arr[0]
                "#,
                Ok(Some("test2".into()))
            ),
            (
                r#"
arr = [["foo"], ["bar"]]
test(arr[0][0])
arr[0][0]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
function test3(ref p: array, i: number, j: number)
    p[i][j] = "test3"
fend
arr = [["foo"], ["bar"]]
test3(arr, 0, 0)
arr[0][0]
                "#,
                Ok(Some("test3".into()))
            ),
            (
                r#"
arr = [[["foo"]]]
test(arr[0][0][0])
arr[0][0][0]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
function test4(ref p: array, i: number, j: number, k: number)
    p[i][j][k] := "test4"
fend
arr = [[["foo"]]]
test4(arr, 0, 0, 0)
arr[0][0][0]
                "#,
                Ok(Some("test4".into()))
            ),
            (
                r#"
module M
    public p = "module"
    public q = [1]
    public r = [[1]]
endmodule
test(M.p)
M.p
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
test(M.q[0])
M.q[0]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
test2(M.q, 0)
M.q[0]
                "#,
                Ok(Some("test2".into()))
            ),
            (
                r#"
test(M.r[0][0])
M.r[0][0]
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
test3(M.r, 0, 0)
M.r[0][0]
                "#,
                Ok(Some("test3".into()))
            ),
            (
                r#"
class C
    procedure C
    fend
    public p = "class"
endclass
ins = C()
test(ins.p)
ins.p
                "#,
                Ok(Some("reference test".into()))
            ),
            (
                r#"
class Z
    procedure Z()
    fend
    public p = "Z"
endclass

class Y
    procedure Y()
        this.z = [Z()]
    fend
    public p = "Y"
    public z
endclass

class X
    procedure X()
        this.y = Y()
    fend
    public p = "X"
    public y
endclass

function test5(ref r)
    r = "test5"
fend

x = X()
test5(x.y.z[0].p)
x.y.z[0].p
                "#,
                Ok(Some("test5".into()))
            ),
            (
                r#"
function test6(ref r: X)
    r.y.z[0].p = "test6"
fend

x = X()
test6(x)
x.y.z[0].p
                "#,
                Ok(Some("test6".into()))
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