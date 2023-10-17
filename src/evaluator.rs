pub mod object;
pub mod environment;
pub mod builtins;
pub mod def_dll;

use crate::com::Com;
use crate::ast::*;
use crate::evaluator::environment::*;
use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::def_dll::*;
use crate::evaluator::builtins::system_controls::{POFF, poff::{sign_out, power_off, shutdown, reboot}};
use crate::error::UWSCRErrorTitle;
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::gui::{LogPrintWin, UWindow, Balloon};
use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::logging::{out_log, LogType};
use crate::settings::*;
use crate::winapi::{show_message,FORCE_WINDOW_MODE};
use windows::Win32::Foundation::HWND;

use std::borrow::Cow;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::sync::{Arc, Mutex};
use std::ffi::c_void;
use std::panic;
use std::ops::{Add, Sub, Mul, Div, Rem, BitOr, BitAnd, BitXor};

use num_traits::FromPrimitive;
use regex::Regex;
use serde_json;
use once_cell::sync::OnceCell;
use serde_json::Value;

pub static LOGPRINTWIN: OnceCell<Mutex<LogPrintWin>> = OnceCell::new();

type EvalResult<T> = Result<T, UError>;

#[derive(Debug, Clone)]
pub struct Evaluator {
    pub env: Environment,
    pub ignore_com_err: bool,
    pub com_err_flg: bool,
    lines: Vec<String>,
    pub balloon: Option<Balloon>,
    pub mouseorg: Option<MouseOrg>,
    pub gui_print: Option<bool>,
}

impl Evaluator {
    pub fn clear(&mut self) {
        system_controls::sound::remove_recognizer();
        self.env.clear();
    }
    pub fn clear_local(&mut self) {
        system_controls::sound::remove_recognizer();
        self.env.clear_local();
    }

    pub fn new(env: Environment) -> Self {
        Evaluator {
            env,
            ignore_com_err: false,
            com_err_flg: false,
            lines: vec![],
            balloon: None,
            mouseorg: None,
            gui_print: None,
        }
    }
    fn new_thread(&mut self) -> Self {
        Evaluator {
            env: Environment {
                current: Arc::new(Mutex::new(Layer {
                    local: Vec::new(),
                    outer: None,
                })),
                global: self.env.global.clone()
            },
            ignore_com_err: false,
            com_err_flg: false,
            lines: self.lines.clone(),
            balloon: None,
            mouseorg: None,
            gui_print: self.gui_print,
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
        let Program { global, script, mut lines } = program;
        self.lines.append(&mut lines);

        // グローバル定義を評価
        for statement in global {
            self.eval_statement(statement)?;
        }

        if self.gui_print.is_none() {
            self.gui_print = if cfg!(feature="gui") {
                Some(true)
            } else {
                let mut settings = USETTINGS.lock().unwrap();
                // --windowが指定されていた場合はOPTION設定に関わらずtrue
                if let Some(true) = FORCE_WINDOW_MODE.get() {
                    settings.options.gui_print = true;
                    Some(true)
                } else {
                    Some(settings.options.gui_print)
                }
            };
            if let Some(true) = self.gui_print {
                if let Some(lp) = LOGPRINTWIN.get() {
                    let mut guard = lp.lock().unwrap();
                    guard.set_visibility(true, false);
                }
            }
        }

        // スクリプト実行部分の評価
        for statement in script {
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
                Err(e) => {
                    match e.kind {
                        UErrorKind::ExitExit(n) => {
                            self.clear();
                            std::process::exit(n);
                        },
                        UErrorKind::Poff(poff, flg) => {
                            self.invoke_poff(&poff, flg)?;
                        },
                        _ => {
                            if clear {
                                self.clear();
                            }
                            return Err(e);
                        }
                    }
                },
            }
        }
        if clear {
            self.clear();
        }

        Ok(result)
    }

    fn invoke_poff(&mut self, poff: &POFF, flg: bool) -> EvalResult<()> {
        match poff {
            POFF::P_POWEROFF => {
                self.clear();
                power_off(flg);
            },
            POFF::P_SHUTDOWN => {
                self.clear();
                shutdown(flg);
            },
            POFF::P_LOGOFF => {
                self.clear();
                sign_out(flg);
            },
            POFF::P_REBOOT => {
                self.clear();
                reboot(flg);
            },
            POFF::P_UWSC_REEXEC => {
                use std::process::{self, Command};
                // 自身を再実行
                let path = env::current_exe()?;
                let args = env::args().collect::<Vec<_>>();
                let mut cmd = Command::new(path);
                if flg {
                    cmd.args(&args[1..]);
                }
                // let vars = env::vars();
                // let dir = env::current_dir()?;
                // cmd.envs(vars)
                //     .stdin(Stdio::inherit())
                //     .stdout(Stdio::inherit())
                //     .stderr(Stdio::inherit())
                //     .current_dir(dir);
                if let Ok(_) = cmd.spawn() {
                    self.clear();
                    process::exit(0);
                }
            },
            _ => {},
        }
        Ok(())
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
                self.expand_string(s, true, None).to_string()
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
        let msg = match self.eval_expression(expression)? {
            Object::Null => "NULL".to_string(),
            Object::Empty => "EMPTY".to_string(),
            obj => obj.to_string()
        };

        out_log(&msg, LogType::Print);

        if self.gui_print.unwrap_or(false) {
            match LOGPRINTWIN.get() {
                Some(lp) => {
                    lp.lock().unwrap().print(msg.to_string());
                },
                None => {
                    println!("{msg}");
                },
            }
        } else {
            println!("{msg}");
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
                if let Object::String(s) = self.expand_string(s.clone(), true, None) {
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
                if let Object::String(s) = self.expand_string(s.clone(), true, None) {
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
                if let Object::String(ref s) = self.expand_string(s.clone(), true, None) {
                    env::set_var("UWSCR_DEFAULT_TITLE", s.as_str());
                    usettings.options.dlg_title = Some(s.to_string());
                }
            },
            OptionSetting::GuiPrint(b) => {
                usettings.options.gui_print = b;
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
                let value = self.eval_literal(s, None)?;
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
                // let Program(body, _) = block;
                let Program { global:_, script, lines:_ } = block;
                let params = vec![
                    FuncParam::new(Some("PARAM_STR".into()), ParamKind::Identifier)
                ];
                let params_str = Expression::Literal(Literal::Array(args));
                let arguments = vec![
                    (Some(params_str.clone()), self.eval_expression(params_str)?)
                ];
                let func = Function::new_call(params, script);
                func.invoke(self, arguments).map(|_| None)
            },
            Statement::DefDll{name, alias, params, ret_type, path} => {
                let params = DefDll::convert_params(params, self)?;
                let defdll = DefDll::new(name, alias, path, params, ret_type)?;
                self.env.define_dll_function(defdll)?;
                Ok(None)
            },
            Statement::Struct(identifier, members) => {
                let name = identifier.0;
                let memberdef = ustruct::MemberDefVec::new(members, self)?;
                let sdef = StructDef::new(name, memberdef);
                self.env.define_struct(sdef)?;
                Ok(None)
            }
            Statement::Expression(e) => Ok(Some(self.eval_expression(e)?)),
            Statement::For {loopvar, from, to, step, block, alt} => {
                self.eval_for_statement(loopvar, from, to, step, block, alt)
            },
            Statement::ForIn {loopvar, index_var, islast_var, collection, block, alt} => {
                self.eval_for_in_statement(loopvar, index_var, islast_var, collection, block, alt)
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
                        f.invoke(self, vec![])?;
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
        self.env.assign(&var, Object::Num(counter as f64))?;
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
                            self.env.assign(&var, Object::Num(counter as f64))?;
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
            self.env.assign(&var, Object::Num(counter as f64))?;
        };
        if ! broke && alt.is_some() {
            let block = alt.unwrap();
            self.eval_block_statement(block)?;
        }
        Ok(None)
    }

    fn eval_for_in_statement(&mut self, loopvar: Identifier, index_var: Option<Identifier>, islast_var: Option<Identifier>, collection: Expression, block: BlockStatement, alt: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        let Identifier(var) = loopvar;
        let col_obj = match self.eval_expression(collection)? {
            Object::Array(a) => a,
            Object::String(s) => s.chars().map(|c| Object::String(c.to_string())).collect::<Vec<Object>>(),
            Object::HashTbl(h) => h.lock().unwrap().keys(),
            Object::ByteArray(arr) => arr.iter().map(|n| Object::Num(*n as f64)).collect(),
            Object::Browser(b) => b.get_tabs()?.into_iter().map(|t| Object::TabWindow(t)).collect(),
            Object::RemoteObject(remote) => remote.to_object_vec()?,
            Object::ComObject(com) => com.to_object_vec()?,
            Object::UObject(uo) => uo.to_object_vec()?,
            _ => return Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForInError
            ))
        };

        let mut broke = false;
        let len = col_obj.len();
        for (i, o) in col_obj.into_iter().enumerate() {
            self.env.assign(&var, o)?;
            if let Some(Identifier(name)) = &index_var {
                self.env.assign(name, i.into())?;
            }
            if let Some(Identifier(name)) = &islast_var {
                let is_last = i + 1 == len;
                self.env.assign(name, is_last.into())?;
            }
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
        let func = Function::new_named(name.into(), params, body, is_proc);
        if is_async {
            Ok(Object::AsyncFunction(func))
        } else {
            Ok(Object::Function(func))
        }
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
                    let value = self.eval_literal(s, None)?;
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
                                let value = self.eval_literal(s, None)?;
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
                    let func = Function::new_named(func_name.clone(), params, new_body, is_proc);
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
                Statement::DefDll { name, alias, params, ret_type, path } => {
                    let params = DefDll::convert_params(params, self)?;
                    let defdll = DefDll::new(name, alias, path, params, ret_type)?;
                    match &defdll.alias {
                        Some(alias) => module.add(alias.clone(), Object::DefDllFunction(defdll), ContainerType::Function),
                        None => module.add(defdll.name.clone(), Object::DefDllFunction(defdll), ContainerType::Function),
                    }
                },
                _ => {
                    let mut err = UError::new(
                        UErrorKind::ModuleError,
                        UErrorMessage::InvalidModuleMember,
                    );
                    err.set_line(statement.row, statement.line.clone(), statement.script_name.clone());
                    return Err(err);
                }
            }
        }
        self.env.restore_scope(&None);
        let m = Arc::new(Mutex::new(module));
        {
            let mut module = m.lock().unwrap();
            module.set_module_reference(m.clone());
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
            Err(e) => match e.kind {
                UErrorKind::ExitExit(_) |
                UErrorKind::Poff(_,_) => {
                    if opt_finally && finally.is_some() {
                        self.eval_block_statement(finally.unwrap())?;
                    }
                    return Err(e)
                },
                _ => {
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
            let mut evaluator = self.new_thread();
            thread::spawn(move || {
                // このスレッドでのCOMを有効化
                let com = match Com::init() {
                    Ok(com) => com,
                    Err(_) => {
                        panic!("Failed to initialize COM on new thread");
                    }
                };
                let old_hook = panic::take_hook();
                let uerror = Arc::new(Mutex::new(None::<UError>));
                let uerror2 = uerror.clone();
                let evaluator2 = evaluator.clone();
                panic::set_hook(Box::new(move |panic_info|{
                    let maybe_uerror = uerror2.lock().unwrap();
                    // attach_console();
                    match maybe_uerror.as_ref() {
                        Some(e) => match &e.kind {
                            UErrorKind::ExitExit(n) => {
                                std::process::exit(*n);
                            }
                            UErrorKind::Poff(poff, flg) => {
                                let mut evaluator = evaluator2.to_owned();
                                if let Err(e) = evaluator.invoke_poff(poff, *flg) {
                                    let err = e.to_string();
                                    out_log(&err, LogType::Error);
                                    let title = UWSCRErrorTitle::RuntimeError.to_string();
                                    show_message(&err, &title, true);
                                }
                            }
                            _ => {
                                let err = e.to_string();
                                out_log(&err, LogType::Error);
                                let title = UWSCRErrorTitle::RuntimeError.to_string();
                                show_message(&err, &title, true);
                            }
                        },
                        None => {
                            let err = panic_info.to_string();
                            out_log(&err, LogType::Panic);
                            show_message(&err, "Panic on thread", true);
                        },
                    }
                    // free_console();
                    std::process::exit(0);
                }));
                let result = evaluator.eval_function_call_expression(func, args, false);
                evaluator.clear_local();
                com.uninit();
                if let Err(e) = result {
                    {
                        let mut m = uerror.lock().unwrap();
                        *m = Some(e);
                    }
                    panic!("");
                } else {
                    panic::set_hook(old_hook);
                }
            });
        }
        Ok(None)
    }

    /// 式を評価する
    /// 参照をそのまま返す
    fn eval_expr(&mut self, expression: Expression) -> EvalResult<Object> {
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
            Expression::Literal(l) => self.eval_literal(l, None)?,
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
                    Expression::DotCall(left, right) => {
                        self.eval_object_member(*left, *right, true)?
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
                let func = Function::new_anon(params, body, is_proc, Arc::new(Mutex::new(outer_local)));
                Object::AnonFunc(func)
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
            Expression::DotCall(left, right) => {
                self.eval_object_member(*left, *right, false)?
            },
            Expression::UObject(json) => {
                // 文字列展開する
                if let Object::String(ref s) = self.expand_string(json, true, None) {
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
            Expression::Callback => Object::Empty,
            Expression::RefArg(e) => self.eval_expr(*e)?,
        };
        Ok(obj)
    }
    /// 式を評価する
    /// 参照の場合は参照先から値を得る
    fn eval_expression(&mut self, expression: Expression) -> EvalResult<Object> {
        let obj = self.eval_expr(expression)?;
        if let Object::Reference(expression, outer) = obj {
            self.eval_reference(expression, &outer)
        } else {
            Ok(obj)
        }
    }
    fn eval_reference(&mut self, expression: Expression, outer: &Arc<Mutex<Layer>>) -> EvalResult<Object> {
        let obj = match expression {
            Expression::Identifier(Identifier(name)) => {
                self.env.get_from_reference(&name, outer)
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
                        let instance = self.eval_reference(*left, outer)?;
                        self.get_member(instance, member, false, true)?
                    },
                    Expression::Identifier(Identifier(name)) => self.env.get_from_reference(&name, outer)
                        .ok_or(UError::new(UErrorKind::EvaluatorError, UErrorMessage::UnableToReference(name)))?,
                    _ => self.eval_reference(*expr_array, outer)?,
                };
                let index = self.eval_reference(*expr_index, outer)?;
                self.get_index_value(array, index, None)
            },
            Expression::DotCall(left, right) => {
                let Expression::Identifier(Identifier(member)) = *right else {
                    return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::InvalidRightExpression(*right)
                    ));
                };
                let instance = self.eval_reference(*left, outer)?;
                self.get_member(instance, member, false, false)
            },
            Expression::Literal(literal) => {
                self.eval_literal(literal, Some(outer))
            },
            _ => Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::InvalidReference))
        }?;
        if let Object::Reference(expression, outer) = obj {
            self.eval_reference(expression, &outer)
        } else {
            Ok(obj)
        }
    }

    fn eval_identifier(&self, identifier: Identifier) -> EvalResult<Object> {
        let Identifier(name) = identifier;
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
        let obj = match left {
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
                self.eval_uobject(&u, index)?
            },
            Object::ComObject(com) => {
                com.get_by_index(vec![index])?
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
            Object::RemoteObject(remote) => {
                let index = index.to_string();
                remote.get(None, Some(&index))?
            },
            Object::Browser(brwoser) => {
                if let Object::Num(i) = index {
                    let tab = brwoser.get_tab(i as usize)?;
                    Object::TabWindow(tab)
                } else {
                    return Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            Object::MemberCaller(ref method, ref name) => {
                match method {
                    MemberCaller::RemoteObject(remote) => {
                        let index = index.to_string();
                        remote.get(Some(&name), Some(&index))?
                    },
                    MemberCaller::ComObject(com) => {
                        com.get_property_by_index(name, vec![index.clone()])?
                    },
                    MemberCaller::UStruct(_) |
                    MemberCaller::BrowserBuilder(_) |
                    MemberCaller::Browser(_) |
                    MemberCaller::TabWindow(_) |
                    MemberCaller::WebRequest(_) |
                    MemberCaller::WebResponse(_) |
                    MemberCaller::HtmlNode(_) => {
                        return Err(UError::new(
                            UErrorKind::DotOperatorError,
                            UErrorMessage::NotAnArray(left)
                        ))
                    },
                }
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
        if let Object::Global = value {
            return Err(UError::new(UErrorKind::AssignError, UErrorMessage::GlobalCanNotBeAssigned))
        }
        let assigned_value = value.clone();
        match left {
            Expression::Identifier(Identifier(ref name)) => {
                if let Ok(Object::Reference(e, outer)) = self.eval_expr(left.clone()) {
                    // self.assign_reference(e, value, outer)?;
                    let mut outer_env = self.clone();
                    outer_env.env.current = outer;
                    outer_env.eval_assign_expression(e, value)?;
                } else {
                    self.assign_identifier(name, value)?;
                }
            },
            Expression::Index(expr_array, expr_index, expr_hash_option) => {
                if let Some(e) = *expr_hash_option {
                    let key = self.eval_expression(e)?;
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidKeyOrIndex(key.to_string()),
                    ));
                }
                self.assign_index(*expr_array, *expr_index, value, None)?;
            },
            Expression::DotCall(expr_object, expr_member) => {
                self.update_object_member(*expr_object, *expr_member, value)?;
            },
            e => return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::NotAVariable(e)
            ))
        }
        Ok(assigned_value)
    }
    fn assign_identifier(&mut self, name: &str, new: Object) -> EvalResult<()> {
        match self.env.get_variable("this", true).unwrap_or_default() {
            Object::Module(mutex) => {
                let mut this = mutex.lock().unwrap();
                if this.has_member(name) {
                    this.assign(name, new, None)?;
                } else {
                    self.env.assign(name.into(), new)?;
                }
            },
            Object::Instance(mutex) => {
                let ins = mutex.lock().unwrap();
                let mut this = ins.module.lock().unwrap();
                if this.has_member(name) {
                    this.assign(name, new, None)?;
                } else {
                    self.env.assign(name.into(), new)?;
                }
            }
            _ => {
                self.env.assign(name.into(), new)?;
            }
        }
        Ok(())
    }

    /// 配列要素の更新
    fn update_array(&mut self, name: &str, expr_index: Expression, dimensions: Option<Vec<Object>>, new: Object) -> EvalResult<()> {
        let index = self.eval_expression(expr_index)?;
        let object = self.eval_identifier(Identifier(name.into()))?;
        let dimension = match dimensions {
            Some(mut d) => {
                d.push(index.clone());
                d
            },
            None => vec![index.clone()],
        };
        let (maybe_new, update) = Self::update_array_object(object.clone(), dimension, &new)
            .map_err(|mut e| {
                if let UErrorMessage::NotAnArray(_) = e.message {
                    e.message = UErrorMessage::NotAnArray(name.into());
                }
                e
            })?;
        if update {
            if let Some(new_value) = maybe_new {
                match self.env.get_variable("this", true).unwrap_or_default() {
                    Object::Module(mutex) => {
                        let mut this = mutex.lock().unwrap();
                        if this.has_member(name) {
                            this.assign(name, new_value, None)?;
                        } else {
                            self.env.assign(name.into(), new_value)?;
                        }
                    },
                    Object::Instance(mutex) => {
                        let ins = mutex.lock().unwrap();
                        let mut this = ins.module.lock().unwrap();
                        if this.has_member(name) {
                            this.assign(name, new_value, None)?;
                        } else {
                            self.env.assign(name.into(), new_value)?;
                        }
                    }
                    _ => {
                        self.env.assign(name.into(), new_value)?;
                    }
                }
            }
        }
        Ok(())
    }
    /// メンバ配列要素の更新
    fn update_member_array(&mut self, expr_object: Expression, member: String, index: Object, dimensions: Option<Vec<Object>>, new: Object) -> EvalResult<()> {
        let dimension = match dimensions {
            Some(mut d) => {
                d.push(index.clone());
                Some(d)
            },
            None => Some(vec![index.clone()]),
        };
        let instance = self.eval_expr(expr_object)?;
        match instance {
            Object::Module(mutex) => {
                mutex.lock().unwrap().assign_public(&member, new, dimension)?;
            },
            Object::Instance(mutex) => {
                let ins = mutex.lock().unwrap();
                let mut module = ins.module.lock().unwrap();
                module.assign_public(&member, new, dimension)?;
            },
            // Value::Array
            Object::UObject(uo) => {
                let new_value = Self::object_to_serde_value(new)?;
                uo.set(index, new_value, Some(member))?;
            },
            Object::ComObject(com) => {
                com.set_property_by_index(&member, index, new)?;
            },
            Object::RemoteObject(ref remote) => {
                let value = Self::object_to_serde_value(new)?;
                remote.set(Some(&member), Some(&index.to_string()), value.into())?;
            },
            Object::Reference(e, outer) => {
                let mut outer_env = self.clone();
                outer_env.env.current = outer;
                outer_env.update_member_array(e, member, index, dimension, new)?;
            },
            Object::UStruct(ust) => {
                ust.set_array_member_by_name(&member, index, new)?;
            }
            o => return Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidObject(o),
            ))
        }
        Ok(())
    }
    /// 配列要素への代入
    /// arr[i] = hoge
    /// dim2[j][i] = fuga
    /// foo.bar[i] = baz
    fn assign_index(&mut self, expr_array: Expression, expr_index: Expression, new: Object, dimensions: Option<Vec<Object>>) -> EvalResult<()> {
        match expr_array {
            // 配列要素の更新
            Expression::Identifier(Identifier(ref name)) => {
                if let Object::Reference(e, outer) = self.eval_expr(expr_array.clone())? {
                    let mut outer_env = self.clone();
                    outer_env.env.current = outer;
                    outer_env.assign_index(e, expr_index, new, dimensions)?;
                } else {
                    self.update_array(name, expr_index, dimensions, new)?;
                }
            },
            // オブジェクトメンバの配列要素の更新
            Expression::DotCall(expr_object, expr_member) => {
                let index = self.eval_expression(expr_index)?;
                let Expression::Identifier(Identifier(member)) = *expr_member else {
                    return Err(UError::new(UErrorKind::AssignError, UErrorMessage::MemberShouldBeIdentifier));
                };
                self.update_member_array(*expr_object, member, index, dimensions, new)?;
            },
            // 多次元配列の場合添字の式をexpr_dimensionsに積む
            Expression::Index(expr_inner_array, expr_inner_index, _) => {
                let index = self.eval_expression(expr_index)?;
                let dimensions = match dimensions {
                    Some(mut d) => {
                        d.push(index);
                        Some(d)
                    },
                    None => Some(vec![index]),
                };
                self.assign_index(*expr_inner_array, *expr_inner_index, new, dimensions)?;
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
            Object::ComObject(com) => {
                com.set_by_index(index, new.to_owned())?;
                Ok((None, false))
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
            },
            Object::RemoteObject(remote) => {
                let index = index.to_string();
                let value = Self::object_to_serde_value(new.clone())?;
                remote.set(None, Some(&index), value.into())?;
                Ok((None, false))
            },
            _ => Err(UError::new(UErrorKind::AssignError, UErrorMessage::NotAnArray("".into())))
        }
    }
    fn update_object_member(&mut self, expr_object: Expression, expr_member: Expression, new: Object) -> EvalResult<()>{
        let instance = self.eval_expr(expr_object)?;
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
            Object::Global => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    self.env.assign_public(&name, new)?;
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
            Object::UStruct(ust) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    ust.set_by_name(&name, new)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::UStructError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::ComObject(com) => {
                if let Expression::Identifier(Identifier(prop)) = expr_member {
                    com.set_property(&prop, new)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::RemoteObject(ref remote) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    let value = browser::RemoteFuncArg::from_object(new)?;
                    remote.set(Some(&name), None, value)?;
                } else {
                    return Err(UError::new(
                        UErrorKind::DotOperatorError,
                        UErrorMessage::SyntaxError,
                    ));
                }
            },
            Object::Reference(e, outer) => {
                let mut outer_env = self.clone();
                outer_env.env.current = outer;
                outer_env.update_object_member(e, expr_member, new)?;
            },
            o => return Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidObject(o)
            )),
        }
        Ok(())
    }

    fn eval_infix_expression(&mut self, infix: Infix, left: Object, right: Object) -> EvalResult<Object> {
        match infix {
            Infix::Plus => left.add(right),
            Infix::Minus => left.sub(right),
            Infix::Multiply => left.mul(right),
            Infix::Divide => left.div(right),
            Infix::Equal => left.equal(&right),
            Infix::NotEqual => left.not_equal(&right),
            Infix::GreaterThanEqual => left.greater_than_equal(&right),
            Infix::GreaterThan => left.greater_than(&right),
            Infix::LessThanEqual => left.less_than_equal(&right),
            Infix::LessThan => left.less_than(&right),
            // 数値同士ならビット演算、そうでなければ論理演算
            Infix::And => {
                if left.as_f64(true).is_some() && right.as_f64(true).is_some() {
                    left.bitand(right)
                } else {
                    left.logical_and(&right)
                }
            },
            Infix::Or => {
                if left.as_f64(true).is_some() && right.as_f64(true).is_some() {
                    left.bitor(right)
                } else {
                    left.logical_or(&right)
                }
            },
            Infix::Xor => {
                if left.as_f64(true).is_some() && right.as_f64(true).is_some() {
                    left.bitxor(right)
                } else {
                    left.logical_xor(&right)
                }
            },
            Infix::AndL => left.logical_and(&right),
            Infix::OrL => left.logical_or(&right),
            Infix::XorL => left.logical_xor(&right),
            Infix::AndB => left.bitand(right),
            Infix::OrB => left.bitor(right),
            Infix::XorB => left.bitxor(right),
            Infix::Mod => left.rem(right),
            Infix::Assign => Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::SyntaxError
            )),
        }
    }

    fn eval_literal(&mut self, literal: Literal, maybe_outer: Option<&Arc<Mutex<Layer>>>) -> EvalResult<Object> {
        let obj = match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::ExpandableString(value) => self.expand_string(value, true, maybe_outer),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects)?,
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
            Literal::NaN => Object::Num(f64::NAN),
            Literal::TextBlock(text, is_ex) => if is_ex {
                Object::ExpandableTB(text)
            } else {
                self.expand_string(text, false, maybe_outer)
            },
        };
        Ok(obj)
    }

    fn expand_string(&self, string: String, expand_var: bool, maybe_outer: Option<&Arc<Mutex<Layer>>>) -> Object {
        let re = Regex::new("<#([^>]+)>").unwrap();
        let mut new_string = string.clone();
        for cap in re.captures_iter(string.as_str()) {
            let expandable = cap.get(1).unwrap().as_str();
            let rep_to: Option<Cow<str>> = match expandable.to_ascii_uppercase().as_str() {
                "CR" => Some("\r\n".into()),
                "TAB" => Some("\t".into()),
                "DBL" => Some("\"".into()),
                "NULL" => Some("\0".into()),
                name => if expand_var {
                    if let Some(outer) = maybe_outer {
                        self.env.get_from_reference(name, outer)
                    } else {
                        self.env.get_variable(name, false)
                    }.map(|o| o.to_string().into())
                } else {
                    continue;
                },
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    fn eval_array_literal(&mut self, expr_items: Vec<Expression>) -> EvalResult<Object> {
        let mut arr = vec![];
        for e in expr_items {
            let obj = self.eval_expression(e)?;
            arr.push(obj);
        }
        Ok(Object::Array(arr))
    }

    fn eval_expression_for_func_call(&mut self, expression: Expression) -> EvalResult<Object> {
        // 関数定義から探してなかったら変数を見る
        match expression {
            Expression::Identifier(Identifier(name)) => {
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
            Expression::DotCall(left, right) => {
                self.eval_object_method(*left, *right)
            },
            e => self.eval_expression(e)
        }
    }

    fn new_task(&mut self, func: Function, arguments: Vec<(Option<Expression>, Object)>) -> UTask {
        // task用のselfを作る
        let mut evaluator = self.new_thread();
        // 関数を非同期実行し、UTaskを返す
        let handle = thread::spawn(move || {
            // このスレッドでのCOMを有効化
            let com = Com::init()?;
            let ret = func.invoke(&mut evaluator, arguments);
            com.uninit();
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
    pub fn invoke_eval_script(&mut self, script: &str) -> EvalResult<Object> {
        let parser = Parser::new(Lexer::new(script));
        match parser.parse() {
            Ok(program) => {
                self.eval(program, false).map(|o| o.unwrap_or_default())
            },
            Err(errors) => {
                let count = errors.len();
                let errors = errors.into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                Err(UError::new(
                    UErrorKind::EvalParseErrors(count),
                    UErrorMessage::ParserErrors(errors),
                ))
            },
        }
    }
    pub fn invoke_qsort_update(&mut self, expr: Option<Expression>, array: Vec<Object>, exprs: [Option<Expression>; 8], arrays: [Option<Vec<Object>>; 8]) -> EvalResult<()> {
        if let Some(left) = expr {
            self.eval_assign_expression(left, Object::Array(array))?;
        }
        for (expr, array) in exprs.into_iter().zip(arrays.into_iter()) {
            if let Some(left) = expr {
                if let Some(arr) = array {
                    self.eval_assign_expression(left, Object::Array(arr))?;
                }
            }
        }
        Ok(())
    }
    pub fn update_tokened_variable(&mut self, expression: Option<Expression>, remained: String) -> EvalResult<()> {
        self.update_reference(vec![(expression, remained.into())])
    }
    pub fn update_reference(&mut self, refs: Vec<(Option<Expression>, Object)>) -> EvalResult<()> {
        for (expr, value) in refs {
            if let Some(left) = expr {
                self.eval_assign_expression(left, value)?;
            }
        }
        Ok(())
    }

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>, is_await: bool) -> EvalResult<Object> {
        let func_object = self.eval_expression_for_func_call(*func)?;
        if let Object::MemberCaller(MemberCaller::ComObject(com), member) = func_object {
            // COMのメソッド呼び出し
            let mut comargs = ComObject::to_comarg(self, args)?;
            let obj =  com.invoke_method(&member, &mut comargs)?;
            for arg in comargs {
                if let ComArg::ByRef(left, value) = arg {
                    self.eval_assign_expression(left, value)?;
                }
            }
            Ok(obj)
        } else {
            // COMメソッド以外の関数呼び出し
            let arguments = args.into_iter()
                .map(|arg| Ok((Some(arg.clone()), self.eval_expression(arg)?)))
                .collect::<EvalResult<Vec<(Option<Expression>, Object)>>>()?;
            match func_object {
                Object::Function(f) => f.invoke(self, arguments),
                Object::AsyncFunction(f) => {
                    let task = self.new_task(f, arguments);
                    if is_await {
                        self.await_task(task)
                    } else {
                        Ok(Object::Task(task))
                    }
                },
                Object::AnonFunc(f) => f.invoke(self, arguments),
                Object::BuiltinFunction(name, expected_len, builtin) => {
                    if expected_len >= arguments.len() as i32 {
                        builtin(self, BuiltinFuncArgs::new(arguments, is_await))
                            .map_err(|err| err.to_uerror(name))
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
                    constructor.invoke(self, arguments)?;
                    let ins = Arc::new(Mutex::new(ClassInstance::new(name, m, self.clone())));
                    {
                        let mut guard = ins.lock().unwrap();
                        guard.set_instance_reference(ins.clone());
                    }
                    Ok(Object::Instance(ins))
                },
                Object::StructDef(sdef) => {
                    match arguments.len() {
                        0 => {
                            let ust = UStruct::try_from(&sdef)?;
                            Ok(Object::UStruct(ust))
                        },
                        1 => {
                            let o = &arguments[0].1;
                            if let Some(n) = o.as_f64(false) {
                                let ptr = n as isize as *const c_void;
                                let ust = UStruct::new_from_pointer(ptr, &sdef);
                                Ok(Object::UStruct(ust))
                            } else {
                                Err(UError::new(
                                    UErrorKind::UStructError,
                                    UErrorMessage::StructConstructorArgumentError
                                ))
                            }
                        },
                        n => Err(UError::new(
                            UErrorKind::UStructError,
                            UErrorMessage::TooManyArguments(n, 1)
                        ))
                    }
                },
                Object::DefDllFunction(defdll) => {
                    defdll.invoke(arguments, self)
                },
                Object::ComObject(com) => {
                    let index = arguments.into_iter().map(|(_, o)| o).collect();
                    let obj = com.get_by_index(index)?;
                    Ok(obj)
                },
                Object::RemoteObject(ref remote) => {
                    let args = arguments.into_iter()
                        .map(|(_, o)| browser::RemoteFuncArg::from_object(o))
                        .collect::<EvalResult<Vec<browser::RemoteFuncArg>>>()?;
                    remote.invoke_as_function(args, is_await)
                        .map(|r| r.into())
                },
                Object::MemberCaller(method, member) => {
                    let args = arguments.into_iter()
                        .map(|(_, arg)| arg)
                        .collect();
                    match method {
                        MemberCaller::BrowserBuilder(mutex) => {
                            let maybe_browser = {
                                let mut builder = mutex.lock().unwrap();
                                builder.invoke_method(&member, args)?
                            };
                            let obj = match maybe_browser {
                                Some(browser) => Object::Browser(browser),
                                None => Object::BrowserBuilder(mutex),
                            };
                            Ok(obj)
                        },
                        MemberCaller::Browser(browser) => {
                            browser.invoke_method(&member, args)
                        },
                        MemberCaller::TabWindow(tab) => {
                            tab.invoke_method(&member, args)
                        },
                        MemberCaller::RemoteObject(remote) => {
                            let args = args.into_iter()
                                .map(|o| browser::RemoteFuncArg::from_object(o))
                                .collect::<EvalResult<Vec<browser::RemoteFuncArg>>>()?;
                            remote.invoke_method(&member, args, is_await)
                        },
                        MemberCaller::WebRequest(mutex) => {
                            let maybe_obj = {
                                let mut req = mutex.lock().unwrap();
                                req.invoke_method(&member, args)?
                            };
                            let obj = match maybe_obj {
                                Some(obj) => obj,
                                None => Object::WebRequest(mutex),
                            };
                            Ok(obj)
                        },
                        MemberCaller::WebResponse(res) => {
                            res.invoke_method(&member, args)
                        },
                        MemberCaller::HtmlNode(node) => {
                            node.invoke_method(&member, args)
                        },
                        MemberCaller::ComObject(_) => {
                            // ここには来ない
                            Err(UError::new(UErrorKind::DotOperatorError, UErrorMessage::None))
                        },
                        MemberCaller::UStruct(ust) => {
                            ust.invoke_method(&member, args)
                        }
                    }
                },
                o => Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::NotAFunction(o),
                )),
            }
        }

    }

    // fn invoke_def_dll_function(&mut self, name: String, dll_path: String, params: Vec<DefDllParam>, ret_type: DllType, arguments: Vec<(Option<Expression>, Object)>) -> EvalResult<Object> {
    //     // dllを開く
    //     let lib = dlopen::raw::Library::open(&dll_path)?;
    //     unsafe {
    //         // 関数のシンボルを得る
    //         let f: *const c_void = lib.symbol(&name)?;
    //         // cifで使う
    //         let mut arg_types = vec![];
    //         let mut args = vec![];
    //         // 渡された引数のインデックス
    //         let mut i = 0;
    //         // 引数の実の値を保持するリスト
    //         let mut dll_args: Vec<DllArg> = vec![];
    //         // varされた場合に値を返す変数のリスト
    //         let mut var_list: Vec<(String, usize)> = vec![];

    //         for param in params {
    //             match param {
    //                 DefDllParam::Param {dll_type, is_var, is_array} => {
    //                     let (arg_exp, obj) = match arguments.get(i) {
    //                         Some((a, o)) => (a, o),
    //                         None => return Err(UError::new(
    //                             UErrorKind::DllFuncError,
    //                             UErrorMessage::DllMissingArgument(dll_type, i + 1),
    //                         ))
    //                     };
    //                     // 引数が変数なら変数名を得ておく
    //                     let arg_name = if let Some(Expression::Identifier(Identifier(ref name))) = arg_exp {
    //                         Some(name.to_string())
    //                     } else {
    //                         None
    //                     };

    //                     if is_array {
    //                         match obj {
    //                             Object::Array(_) => {
    //                                 let arr_arg = match DllArg::new_array(obj, &dll_type) {
    //                                     Ok(a) => a,
    //                                     Err(_) => return Err(UError::new(
    //                                         UErrorKind::DllFuncError,
    //                                         UErrorMessage::DllArrayHasInvalidType(dll_type, i + 1),
    //                                     ))
    //                                 };

    //                                 dll_args.push(arr_arg);
    //                                 arg_types.push(Type::pointer());
    //                                 // 配列はvarの有無に関係なく値を更新する
    //                                 if arg_name.is_some() {
    //                                     var_list.push((arg_name.unwrap(), i));
    //                                 }
    //                             },
    //                             _ => return Err(UError::new(
    //                                 UErrorKind::DllFuncError,
    //                                 UErrorMessage::DllArgumentIsNotArray(dll_type, i + 1)
    //                             ))
    //                         }
    //                     } else {
    //                         let t = Self::convert_to_libffi_type(&dll_type)?;
    //                         let dllarg = match DllArg::new(obj, &dll_type) {
    //                             Ok(a) => a,
    //                             Err(e) => return Err(UError::new(
    //                                 UErrorKind::DllFuncError,
    //                                 UErrorMessage::DllConversionError(dll_type, i + 1, e)
    //                             ))
    //                         };
    //                         match dllarg {
    //                             // null文字列の場合はvoid型にしておく
    //                             DllArg::Null => arg_types.push(Type::void()),
    //                             _ => arg_types.push(t)
    //                         }
    //                         dll_args.push(dllarg);
    //                         if is_var && arg_name.is_some() {
    //                             // var/ref が付いていれば後に値を更新
    //                             var_list.push((arg_name.unwrap(), dll_args.len() - 1));
    //                         }
    //                     }
    //                     i += 1;
    //                 },
    //                 DefDllParam::Struct(params) => {
    //                     let mut struct_size: usize = 0;
    //                     let mut members: Vec<(Option<String>, usize, DllArg)> = vec![];
    //                     // let mut struct_args: Vec<DllArg> = vec![];
    //                     for param in params {
    //                         match param {
    //                             DefDllParam::Param {dll_type, is_var: _, is_array} => {
    //                                 let (arg_exp, obj) = match arguments.get(i) {
    //                                     Some((a, o)) => (a, o),
    //                                     None => return Err(UError::new(
    //                                         UErrorKind::DllFuncError,
    //                                         UErrorMessage::DllMissingArgument(dll_type, i + 1)
    //                                     ))
    //                                 };
    //                                 // 引数が変数なら変数名を得ておく
    //                                 let arg_name = if let Some(Expression::Identifier(Identifier(ref name))) = arg_exp {
    //                                     Some(name.to_string())
    //                                 } else {
    //                                     None
    //                                 };

    //                                 let arg = if is_array {
    //                                     match DllArg::new_array(obj, &dll_type) {
    //                                         Ok(a) => a,
    //                                         Err(_) => return Err(UError::new(
    //                                             UErrorKind::DllFuncError,
    //                                             UErrorMessage::DllArrayHasInvalidType(dll_type, i + 1)
    //                                         ))
    //                                     }
    //                                 } else {
    //                                     match DllArg::new(obj, &dll_type) {
    //                                         Ok(a) => a,
    //                                         Err(e) => return Err(UError::new(
    //                                             UErrorKind::DllFuncError,
    //                                             UErrorMessage::DllArgumentTypeUnexpected(dll_type, i + 1, e)
    //                                         ))
    //                                     }
    //                                 };
    //                                 let size = arg.size();
    //                                 // struct_args.push(arg);
    //                                 members.push((arg_name, struct_size, arg));
    //                                 struct_size += size;
    //                                 i += 1;
    //                             },
    //                             DefDllParam::Struct(_) => return Err(UError::new(
    //                                 UErrorKind::DllFuncError,
    //                                 UErrorMessage::DllNestedStruct
    //                             )),
    //                         }
    //                     }
    //                     let structure = new_dll_structure(struct_size);
    //                     for (_, offset, arg) in &members {
    //                         match arg {
    //                             DllArg::Int(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Uint(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Hwnd(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Float(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Double(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Word(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::Byte(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::LongLong(v) => set_value_to_structure(structure, *offset, *v),
    //                             DllArg::IntArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::UintArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::HwndArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::FloatArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::DoubleArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::WordArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::ByteArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::LongLongArray(v) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::String(v, _) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::WString(v, _) => {
    //                                 let p = *v.as_ptr() as usize;
    //                                 set_value_to_structure(structure, *offset, p);
    //                             },
    //                             DllArg::Pointer(v) => set_value_to_structure(structure, *offset, v),
    //                             _ => return Err(UError::new(
    //                                 UErrorKind::DllFuncError,
    //                                 UErrorMessage::DllArgNotAllowedInStruct
    //                             )),
    //                         }
    //                     }
    //                     dll_args.push(DllArg::Struct(structure, members));
    //                     arg_types.push(Type::pointer());
    //                     var_list.push(("".into(), dll_args.len() -1));
    //                 },
    //             }
    //         }

    //         for dll_arg in &dll_args {
    //             args.push(dll_arg.to_arg());
    //         }

    //         let cif = Cif::new(arg_types.into_iter(), Self::convert_to_libffi_type(&ret_type)?);

    //         // 関数実行
    //         let result = match ret_type {
    //             DllType::Int |
    //             DllType::Long |
    //             DllType::Bool => {
    //                 let result = cif.call::<i32>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Uint |
    //             DllType::Dword => {
    //                 let result = cif.call::<u32>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Hwnd => {
    //                 let result = cif.call::<isize>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Float => {
    //                 let result = cif.call::<f32>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Double => {
    //                 let result = cif.call::<f64>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Word => {
    //                 let result = cif.call::<u16>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Byte |
    //             DllType::Char |
    //             DllType::Boolean => {
    //                 let result = cif.call::<u8>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Longlong => {
    //                 let result = cif.call::<i64>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Pointer => {
    //                 let result = cif.call::<usize>(CodePtr::from_ptr(f), &args);
    //                 Object::Num(result as f64)
    //             },
    //             DllType::Void => {
    //                 cif.call::<*mut c_void>(CodePtr::from_ptr(f), &args);
    //                 Object::Empty
    //             }
    //             _ =>  {
    //                 let result = cif.call::<*mut c_void>(CodePtr::from_ptr(f), &args);
    //                 println!("[warning] {} is not fully supported for return type.", ret_type);
    //                 Object::Num(result as isize as f64)
    //             }
    //         };

    //         // varの処理
    //         for (name, index) in var_list {
    //             let arg = &dll_args[index];
    //             match arg {
    //                 DllArg::Struct(p, m) => {
    //                     for (name, offset, arg) in m {
    //                         if let Some(name) = name {
    //                             let obj = get_value_from_structure(*p, *offset, arg);
    //                             self.env.assign(name, obj)?;
    //                         }
    //                     }
    //                     free_dll_structure(*p);
    //                 },
    //                 DllArg::UStruct(_) => {
    //                 },
    //                 _ => {
    //                     let obj = arg.to_object();
    //                     self.env.assign(&name, obj)?;
    //                 },
    //             }
    //         }

    //         Ok(result)
    //     }
    // }


    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> EvalResult<Object> {
        let condition = self.eval_expression(condition)?;
        if condition.is_truthy() {
            self.eval_expression(consequence)
        } else {
            self.eval_expression(alternative)
        }
    }

    fn eval_object_member(&mut self, left: Expression, right: Expression, is_indexed_property: bool) -> EvalResult<Object> {
        let (instance, member) = self.eval_dot_operator(left, right)?;
        self.get_member(instance, member, false, is_indexed_property)
    }
    fn eval_object_method(&mut self, left: Expression, right: Expression) -> EvalResult<Object> {
        let (instance, member) = self.eval_dot_operator(left, right)?;
        self.get_member(instance, member, true, true)
    }
    fn eval_dot_operator(&mut self, left: Expression, right: Expression) -> EvalResult<(Object, String)> {
        let instance = match left {
            Expression::Identifier(_) |
            Expression::Index(_, _, _) |
            Expression::FuncCall{func:_, args:_, is_await:_} |
            Expression::DotCall(_, _) |
            Expression::UObject(_) => {
                self.eval_expression(left)?
            },
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
        Ok((instance, member))
    }
    fn get_member(&self, instance: Object, member: String, is_func: bool, is_indexed_property: bool) -> EvalResult<Object> {
        match instance {
            Object::Module(m) => {
                self.get_module_member(&m, &member, is_func)
            },
            Object::Instance(m) => {
                let ins = m.lock().unwrap();
                self.get_module_member(&ins.module, &member, is_func)
            },
            Object::Global => {
                self.env.get_global(&member, is_func)
            },
            Object::Class(name, _) => Err(UError::new(
                UErrorKind::ClassError,
                UErrorMessage::ClassMemberCannotBeCalledDirectly(name)
            )),
            Object::UObject(u) => {
                if is_func {
                    Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::CanNotCallMethod(member)
                    ))
                } else {
                    self.eval_uobject(&u, member.into())
                }
            },
            Object::Enum(e) => {
                if is_func {
                    Err(UError::new(
                        UErrorKind::EnumError,
                        UErrorMessage::CanNotCallMethod(member)
                    ))
                } else {
                    if let Some(n) = e.get(&member) {
                        Ok(Object::Num(n))
                    } else {
                        Err(UError::new(
                            UErrorKind::EnumError,
                            UErrorMessage::MemberNotFound(member)
                        ))
                    }
                }
            },
            Object::UStruct(ust) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::UStruct(ust), member))
                } else {
                    ust.get_by_name(&member)
                }
            },
            Object::ComObject(com) => {
                if is_func || is_indexed_property {
                    Ok(Object::MemberCaller(MemberCaller::ComObject(com), member))
                } else {
                    let obj = com.get_property(&member)?;
                    Ok(obj)
                }
            },
            Object::BrowserBuilder(builder) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::BrowserBuilder(builder), member))
                } else {
                    Err(UError::new(
                        UErrorKind::BrowserControlError,
                        UErrorMessage::MemberNotFound(member)
                    ))
                }
            },
            Object::Browser(browser) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::Browser(browser), member))
                } else {
                    browser.get_property(&member)
                }
            },
            Object::TabWindow(tab) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::TabWindow(tab), member))
                } else {
                    tab.get_property(&member)
                }
            },
            Object::RemoteObject(remote) => {
                if is_func || is_indexed_property {
                    Ok(Object::MemberCaller(MemberCaller::RemoteObject(remote), member))
                } else {
                    remote.get(Some(&member), None)
                }
            },
            Object::WebRequest(req) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::WebRequest(req), member))
                } else {
                    let req = req.lock().unwrap();
                    req.get_property(&member)
                }
            },
            Object::WebResponse(res) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::WebResponse(res), member))
                } else {
                    res.get_property(&member)
                }
            },
            Object::HtmlNode(node) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::HtmlNode(node), member))
                } else {
                    node.get_property(&member)
                }
            }
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
            match self.env.get_variable("this", true).unwrap_or_default() {
                Object::Module(this) => {
                    if this.try_lock().is_err() {
                        // ロックに失敗した場合thisと呼び出し元が同一と判断し、自身のメンバの値を返す
                        return module.get_member(&member);
                    }
                }
                Object::Instance(this) => {
                    if this.try_lock().is_err() {
                        // ロックに失敗した場合thisと呼び出し元が同一と判断し、自身のメンバの値を返す
                        return module.get_member(&member);
                    }
                }
                _ => {}
            }
            Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::IsPrivateMember(module.name(), member.to_string())
            ))
        } else {
            match module.get_public_member(&member) {
                Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true, None)),
                res => res
            }
        }
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
                    self.expand_string(s.clone(), true, None)
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

    pub fn object_to_serde_value(o: Object) -> EvalResult<serde_json::Value> {
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

    fn set_mouseorg<T: Into<MorgTarget>, C: Into<MorgContext>>(&mut self, hwnd: HWND, target: T, context: C) {
        let morg = MouseOrg {
            hwnd,
            target: target.into(),
            context: context.into(),
        };
        self.mouseorg = Some(morg);
    }
    fn clear_mouseorg(&mut self) {
        self.mouseorg = None;
    }
}

#[derive(Debug, Clone)]
pub struct MouseOrg {
    hwnd: HWND,
    target: MorgTarget,
    context: MorgContext,
}
#[derive(Debug, Clone, PartialEq)]
pub enum MorgTarget {
    Window,
    Client,
    Direct
}
#[derive(Debug, Clone, PartialEq)]
pub enum MorgContext {
    Fore,
    Back
}
impl MouseOrg {
    pub fn is_back(&self) -> bool {
        self.context == MorgContext::Back
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
        match Parser::new(Lexer::new(input)).parse() {
            Ok(program) => {
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
            },
            Err(errors) => {
                eprintln!("{} parser errors on eval_test", errors.len());
                for error in errors {
                    eprintln!("{error}");
                }
                panic!("\nParse Error:\n{input}");
            },
        }
    }


    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        match Parser::new(Lexer::new(input)).parse() {
            Ok(program) => {
                let mut e = Evaluator::new(Environment::new(vec![]));
                match e.eval(program, false) {
                    Ok(_) => e,
                    Err(err) => panic!("\nError:\n{:#?}\ninput:\n{}\n", err, input)
                }
            },
            Err(errors) => {
                eprintln!("{} parser errors on eval_env", errors.len());
                for error in errors {
                    eprintln!("{error}");
                }
                panic!("\nParse Error:\n{input}");
            },
        }
    }

    //
    fn eval_test_with_env(e: &mut Evaluator, input: &str, expected: Result<Option<Object>, UError>) {
        match Parser::new(Lexer::new(input)).parse() {
            Ok(program) => {
                let result = e.eval(program, false);
                match expected {
                    Ok(expected_obj) => match result {
                        Ok(result_obj) => if result_obj.is_some() && expected_obj.is_some() {
                            let left = result_obj.unwrap();
                            let right = expected_obj.unwrap();
                            if ! left.is_equal(&right) {
                                panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", left, right, input);
                            }
                        } else {
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
            },
            Err(errors) => {
                eprintln!("{} parser errors on eval_test_with_env", errors.len());
                for error in errors {
                    eprintln!("{error}");
                }
                panic!("\nParse Error:\n{input}");
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
                    UErrorKind::DefinitionError(DefinitionType::Public),
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
            (
                r#"
procedure inner(ref r)
    r = "gh-68"
fend

procedure outer(ref r)
    inner(r)
fend

dim hoge
outer(hoge)
hoge
                "#,
                Ok(Some("gh-68".into()))
            ),
            (
                r#"
procedure test8(ref r)
    r = "test8"
fend
f = [[0]]
i = 0
j = 0
test8(f[i][j])

f[i][j]
                "#,
                Ok(Some("test8".into()))
            ),
            (
                r#"
procedure test9(ref r[])
    r[0] = "gh-67"
fend
a = [0]
test9(a)

a[0]
                "#,
                Ok(Some("gh-67".into()))
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