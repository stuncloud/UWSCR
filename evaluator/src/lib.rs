pub mod object;
pub mod environment;
pub mod builtins;
pub mod def_dll;
pub mod error;
pub mod gui;

use environment::*;
use object::*;
use builtins::*;
use def_dll::*;
use error::{UError, UErrorKind, UErrorMessage};
use builtins::system_controls::{POFF, poff::{sign_out, power_off, shutdown, reboot}};
use gui::{UWindow, LogPrintWin, FontFamily};

use util::com::Com;
use util::winapi::{show_message,FORCE_WINDOW_MODE};
use util::logging::{self, out_log, LogType};
use util::settings::*;
use util::error::UWSCRErrorTitle;
use parser::ast::*;
use parser::Parser;
use parser::lexer::Lexer;

use std::borrow::Cow;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::sync::{Arc, Mutex, OnceLock, Once};
use std::ffi::c_void;
use std::panic;
use std::ops::{Add, Sub, Mul, Div, Rem, BitOr, BitAnd, BitXor};

use windows::Win32::Foundation::HWND;

use num_traits::FromPrimitive;
use regex::Regex;

pub static LOGPRINTWIN: OnceLock<Mutex<Result<LogPrintWin, UError>>> = OnceLock::new();
// static FORCE_BOOL: OnceLock<bool> = OnceLock::new();
static CONDITION_TYPE: OnceLock<ConditionType> = OnceLock::new();
static INIT_LOG_FILE: Once = Once::new();

type EvalResult<T> = Result<T, UError>;

enum ShortCircuitCondition {
    And(bool),
    Or(bool),
    Other(bool),
}
impl ShortCircuitCondition {
    fn into_bool(self) -> bool {
        self.into()
    }
}
impl From<ShortCircuitCondition> for bool {
    fn from(cond: ShortCircuitCondition) -> Self {
        match cond {
            ShortCircuitCondition::And(b) => b,
            ShortCircuitCondition::Or(b) => b,
            ShortCircuitCondition::Other(b) => b,
        }
    }
}

#[derive(Debug, Default)]
#[allow(clippy::upper_case_acronyms)]
enum ConditionType {
    ForceBool,
    UWSC,
    #[default]
    Default,
}

#[derive(Debug)]
pub struct Evaluator {
    pub env: Environment,
    pub ignore_com_err: bool,
    pub com_err_flg: bool,
    lines: Vec<String>,
    pub mouseorg: Option<MouseOrg>,
    pub gui_print: Option<bool>,
    special_char: bool,
    short_circuit: bool,
}
impl Clone for Evaluator {
    fn clone(&self) -> Self {
        Self {
            env: self.env.clone(),
            ignore_com_err: false,
            com_err_flg: false,
            lines: self.lines.clone(),
            mouseorg: None,
            gui_print: self.gui_print,
            special_char: self.special_char,
            short_circuit: self.short_circuit,
        }
    }
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
            mouseorg: None,
            gui_print: None,
            special_char: false,
            short_circuit: true,
        }
    }
    fn new_thread(&mut self) -> Self {
        let current = {
            let current = self.env.current.lock().unwrap();
            current.clone()
        };
        Evaluator {
            env: Environment {
                current: Arc::new(Mutex::new(current)),
                global: self.env.global.clone(),
            },
            ignore_com_err: false,
            com_err_flg: false,
            lines: self.lines.clone(),
            mouseorg: None,
            gui_print: self.gui_print,
            special_char: self.special_char,
            short_circuit: self.short_circuit,
        }
    }

    fn start_logprint_win(visible: bool) {
        if LOGPRINTWIN.get().is_none() {
            let title = match std::env::var("GET_UWSC_NAME") {
                Ok(name) => format!("UWSCR - {}", name),
                Err(_) => "UWSCR".to_string(),
            };
            let font = {
                let usettings = USETTINGS.lock().unwrap();
                FontFamily::new(&usettings.logfont.name, usettings.logfont.size)
            };
            thread::spawn(move || {
                let logprint = LogPrintWin::new(&title, visible, Some(font))
                    .map_err(|_| UError::new(UErrorKind::InitializeError, UErrorMessage::FailedToInitializeLogPrintWindow));
                let cloned = logprint.clone();
                LOGPRINTWIN.get_or_init(move || Mutex::new(logprint));
                if let Ok(lp) = cloned {
                    lp.message_loop().ok();
                }
            });
            while LOGPRINTWIN.get().is_none() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    fn stop_logprint_win() -> EvalResult<()> {
        if let Some(m) = LOGPRINTWIN.get() {
            let guard = m.lock().unwrap();
            let lp = guard.as_ref()
                .map_err(|e| e.clone())?;
            lp.close();
        }
        Ok(())
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

    pub fn get_variable(&self, name: &str) -> Option<Object> {
        let obj = match self.env.get_variable(name)? {
            Object::ExpandableTB(string) => {
                self.expand_string(string, true, None)
            },
            o => o,
        };
        Some(obj)
    }

    pub fn eval(&mut self, program: Program, clear: bool) -> EvalResult<Option<Object>> {
        let mut result = None;
        let Program { global, script, mut lines } = program;
        self.lines.append(&mut lines);

        // グローバル定義を評価
        for statement in global {
            self.eval_statement(statement)?;
        }

        INIT_LOG_FILE.call_once(|| {
            if let Ok(dir) = env::var("GET_SCRIPT_DIR") {
                let dir = PathBuf::from(dir);
                logging::init(&dir);
            }
        });


        if cfg!(feature="gui") {
            Self::start_logprint_win(true);
            self.gui_print = Some(true);
        } else if self.gui_print.is_none() {
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
                Self::start_logprint_win(true);
            }
        }

        // スクリプト実行部分の評価
        for statement in script {
            let res = stacker::maybe_grow(2 * 1024 * 1024, 20*1024*1024, || {
                self.eval_statement(statement)
            });
            match res {
                Ok(opt) => if let Some(o) = opt {
                    match o {
                        Object::Exit => {
                            result = Some(Object::Exit);
                            break;
                        },
                        _ => result = Some(o),
                    }
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
        Self::stop_logprint_win()?;
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
                if cmd.spawn().is_ok() {
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
                Ok(result) => if let Some(o) = result {
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

    fn eval_hash_sugar_statement(&mut self, hash: HashSugar, module: Option<&mut Module>) -> EvalResult<()> {
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
        match module {
            Some(module) => {
                if hash.is_public {
                    self.env.define_module_public(&name, object.clone())?;
                    module.add(name, object, ContainerType::Public);
                } else {
                    self.env.define_module_variable(&name, object.clone())?;
                    module.add(name, object, ContainerType::Variable);
                }
            },
            None => {
                if hash.is_public {
                    self.env.define_public(&name, object)?;
                } else {
                    self.env.define_local(&name, object)?;
                }
            },
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
                    let guard = lp.lock().unwrap();
                    let lp = guard.as_ref()
                        .map_err(|e| e.clone())?;
                    lp.print(&msg);
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

    fn set_option_settings(&mut self, opt: OptionSetting) {
        let mut usettings = USETTINGS.lock().unwrap();
        match opt {
            OptionSetting::Explicit(b) => usettings.options.explicit = b,
            OptionSetting::SameStr(b) => usettings.options.same_str = b,
            OptionSetting::OptPublic(b) => usettings.options.opt_public = b,
            OptionSetting::OptFinally(b) => usettings.options.opt_finally = b,
            OptionSetting::SpecialChar(b) => {
                usettings.options.special_char = b;
                self.special_char = b;
            },
            OptionSetting::ShortCircuit(b) => {
                usettings.options.short_circuit = b;
                self.short_circuit = b;
            },
            OptionSetting::NoStopHotkey(b) => usettings.options.no_stop_hot_key = b,
            OptionSetting::TopStopform(_) => {},
            OptionSetting::FixBalloon(b) => usettings.options.fix_balloon = b,
            OptionSetting::Defaultfont(s) => {
                let mut name_size = s.split(",");
                let name = name_size.next().unwrap();
                let size = name_size.next().unwrap_or("15").parse::<i32>().unwrap_or(15);
                usettings.options.default_font = DefaultFont::new(name, size);
            },
            OptionSetting::Position(x, y) => {
                usettings.options.position.left = x;
                usettings.options.position.top = y;
            },
            OptionSetting::Logpath(s) => {
                let mut path = PathBuf::from(&s);
                if path.is_dir() {
                    path.push("uwscr.log");
                }
                usettings.options.log_path = Some(s);
            },
            OptionSetting::Loglines(n) => {
                usettings.options.log_lines = n as u32;
            },
            OptionSetting::Logfile(n) => {
                let n = if !(0..=4).contains(&n) {1} else {n};
                usettings.options.log_file = n as u8;
            },
            OptionSetting::Dlgtitle(s) => {
                unsafe { env::set_var("UWSCR_DEFAULT_TITLE", &s); }
                usettings.options.dlg_title = Some(s);
            },
            OptionSetting::GuiPrint(b) => {
                usettings.options.gui_print = b;
            },
            OptionSetting::ForceBool(b) => {
                usettings.options.force_bool = b;
            },
            OptionSetting::CondUwsc(b) => {
                usettings.options.cond_uwsc = b;
            }
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
            Statement::Dim(vec, in_loop) => {
                for (i, e) in vec {
                    let (name, value) = self.eval_definition_statement(i, e)?;
                    if in_loop {
                        self.env.in_loop_dim_definition(&name, value);
                    } else {
                        self.env.define_local(&name, value)?;
                    }
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
            Statement::HashTbl(v, is_public) => {
                for (i, hashopt) in v {
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
                self.eval_hash_sugar_statement(hash, None)?;
                Ok(None)
            },
            Statement::Print(e) => self.eval_print_statement(e),
            Statement::Call(block, args) => {
                let Program { global:_, script, lines:_ } = block;
                let params = vec![
                    FuncParam::new(Some("PARAM_STR".into()), ParamKind::Identifier)
                ];
                let params_str_expr = Expression::Literal(Literal::Array(args.clone()));
                let param_str = args
                    .into_iter()
                    .map(|expr| self.eval_expression(expr).map(|obj| obj.to_string()))
                    .collect::<EvalResult<Vec<String>>>()?;
                let arguments = vec![
                    (Some(params_str_expr), Object::ParamStr(param_str))
                ];
                let func = Function::new_call(params, script);
                func.invoke(self, arguments, None)?;
                Ok(None)
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
            Statement::Repeat(stmt, b) => self.eval_repeat_statement(*stmt, b),
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
                self.env.define_module(&name, Object::Module(module.clone()))?;
                // コンストラクタがあれば実行する
                // let module = self.env.get_module(&name);
                let constructor = {
                    module.lock().unwrap().get_constructor()
                };
                if let Some(f) = constructor {
                    let this = Some(function::This::Module(module));
                    f.invoke(self, vec![], this)?;
                }
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
                    if name.contains("@with_tmp_") {
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

    fn get_condition_type() -> &'static ConditionType {
        CONDITION_TYPE.get_or_init(|| {
            let usetttings = USETTINGS.lock().unwrap();
            match (usetttings.options.force_bool, usetttings.options.cond_uwsc) {
                (true, _) => ConditionType::ForceBool,
                (false, true) => ConditionType::UWSC,
                (false, false) => ConditionType::Default,
            }
        })
    }

    /// 条件式の真偽をboolで返す
    fn eval_conditional_expression(&mut self, expression: Expression) -> EvalResult<bool> {
        let cond_type = Self::get_condition_type();
        if self.short_circuit {
            self.eval_conditional_expression_short_circuit(expression, cond_type).map(|c| c.into())
        } else {
            self.eval_conditional_expression_inner(expression, cond_type)
        }
    }
    fn eval_conditional_expression_inner(&mut self, expression: Expression, cond_type: &ConditionType) -> EvalResult<bool> {
        match cond_type {
            ConditionType::ForceBool => match self.eval_expression(expression)? {
                Object::Bool(b) => Ok(b),
                _ => Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::ForceBoolError))
            },
            ConditionType::UWSC => {
                let obj = self.eval_expression(expression)?;
                obj.as_uwsc_cond()
            },
            ConditionType::Default => Ok(self.eval_expression(expression)?.is_truthy()),
        }
    }
    fn eval_conditional_expression_short_circuit(&mut self, expression: Expression, cond_type: &ConditionType) -> EvalResult<ShortCircuitCondition> {
        match expression {
            Expression::Infix(Infix::And, l, r) |
            Expression::Infix(Infix::AndL, l, r) => {
                match self.eval_conditional_expression_short_circuit(*l, cond_type)? {
                    ShortCircuitCondition::Other(true) => {
                        match self.eval_conditional_expression_short_circuit(*r, cond_type)? {
                            ShortCircuitCondition::Other(true) => {
                                Ok(ShortCircuitCondition::Other(true))
                            },
                            ShortCircuitCondition::Other(false) => {
                                Ok(ShortCircuitCondition::And(false))
                            },
                            short => Ok(short)
                        }
                    },
                    ShortCircuitCondition::Other(false) => {
                        Ok(ShortCircuitCondition::And(false))
                    },
                    short => Ok(short)
                }
            },
            Expression::Infix(Infix::Or, l, r) |
            Expression::Infix(Infix::OrL, l, r) => {
                match self.eval_conditional_expression_short_circuit(*l, cond_type)? {
                    ShortCircuitCondition::And(true) |
                    ShortCircuitCondition::Other(true) => {
                        Ok(ShortCircuitCondition::Or(true))
                    },
                    ShortCircuitCondition::And(false) |
                    ShortCircuitCondition::Other(false) => {
                        match self.eval_conditional_expression_short_circuit(*r, cond_type)? {
                            ShortCircuitCondition::Other(true) => {
                                Ok(ShortCircuitCondition::Or(true))
                            },
                            ShortCircuitCondition::Other(false) => {
                                Ok(ShortCircuitCondition::Other(false))
                            },
                            ShortCircuitCondition::And(b) |
                            ShortCircuitCondition::Or(b) => {
                                Ok(ShortCircuitCondition::Or(b))
                            },
                        }
                    },
                    short => Ok(short)
                }
            },
            expression => {
                self.eval_conditional_expression_inner(expression, cond_type)
                    .map(ShortCircuitCondition::Other)
            },
        }
    }
    fn eval_expression_short_circuit(&mut self, expression: Expression) -> EvalResult<ShortCircuitCondition> {
        match expression {
            Expression::Infix(Infix::AndL, l, r) => {
                match self.eval_expression_short_circuit(*l)? {
                    ShortCircuitCondition::Other(true) => {
                        match self.eval_expression_short_circuit(*r)? {
                            ShortCircuitCondition::Other(true) => {
                                Ok(ShortCircuitCondition::Other(true))
                            },
                            ShortCircuitCondition::Other(false) => {
                                Ok(ShortCircuitCondition::And(false))
                            },
                            short => Ok(short)
                        }
                    },
                    ShortCircuitCondition::Other(false) => {
                        Ok(ShortCircuitCondition::And(false))
                    },
                    short => Ok(short)
                }
            },
            Expression::Infix(Infix::OrL, l, r) => {
                match self.eval_expression_short_circuit(*l)? {
                    ShortCircuitCondition::And(true) |
                    ShortCircuitCondition::Other(true) => {
                        Ok(ShortCircuitCondition::Or(true))
                    },
                    ShortCircuitCondition::And(false) |
                    ShortCircuitCondition::Other(false) => {
                        match self.eval_expression_short_circuit(*r)? {
                            ShortCircuitCondition::Other(true) => {
                                Ok(ShortCircuitCondition::Or(true))
                            },
                            ShortCircuitCondition::Other(false) => {
                                Ok(ShortCircuitCondition::Other(false))
                            },
                            ShortCircuitCondition::And(b) |
                            ShortCircuitCondition::Or(b) => {
                                Ok(ShortCircuitCondition::Or(b))
                            },
                        }
                    },
                    short => Ok(short)
                }
            },
            expression => {
                let b = self.eval_expression(expression)?.is_truthy();
                Ok(ShortCircuitCondition::Other(b))
            },
        }
    }

    fn eval_if_line_statement(&mut self, condition: Expression, consequence: StatementWithRow, alternative: Option<StatementWithRow>) -> EvalResult<Option<Object>> {
        if self.eval_conditional_expression(condition)? {
            self.eval_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_if_statement(&mut self, condition: Expression, consequence: BlockStatement, alternative: Option<BlockStatement>) -> EvalResult<Option<Object>> {
        if self.eval_conditional_expression(condition)? {
            self.eval_block_statement(consequence)
        } else {
            match alternative {
                Some(s) => self.eval_block_statement(s),
                None => Ok(None)
            }
        }
    }

    fn eval_elseif_statement(&mut self, condition: Expression, consequence: BlockStatement, alternatives: Vec<(Option<StatementWithRow>, BlockStatement)>) -> EvalResult<Option<Object>> {
        if self.eval_conditional_expression(condition)? {
            return self.eval_block_statement(consequence);
        } else {
            for (altcond, block) in alternatives {
                match altcond {
                    Some(StatementWithRow { statement: Statement::Expression(cond), row, line, script_name }) => {
                        // elseif
                        match self.eval_conditional_expression(cond) {
                            Ok(b) => if b {
                                return self.eval_block_statement(block);
                            },
                            Err(mut e) => {
                                e.set_line(row, line, script_name);
                                return Err(e);
                            },
                        }
                    },
                    None => {
                        // else
                        return self.eval_block_statement(block);
                    },
                    _ => unreachable!(),
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
            if let Some(o) = self.eval_loopblock_statement(block.clone())? { match o {
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
            } };
            counter += step;
            self.env.assign(&var, Object::Num(counter as f64))?;
        };
        if ! broke && alt.is_some() {
            let block = alt.unwrap();
            self.eval_block_statement(block)?;
        }
        Ok(None)
    }

    fn eval_for_in_statement(
        &mut self,
        loopvar: Identifier,
        index_var: Option<Identifier>,
        islast_var: Option<Identifier>,
        collection: Expression,
        block: BlockStatement,
        alt: Option<BlockStatement>
    ) -> EvalResult<Option<Object>> {
        let Identifier(var) = &loopvar;
        match self.eval_expression(collection)? {
            Object::Array(arr) => {
                self.eval_for_in_statement_inner(arr, var, index_var, islast_var, block, alt)
            },
            Object::String(s) => {
                let chars = s.chars().collect();
                self.eval_for_in_statement_inner(chars, var, index_var, islast_var, block, alt)
            },
            Object::HashTbl(h) => {
                let keys = h.lock().unwrap().keys();
                self.eval_for_in_statement_inner(keys, var, index_var, islast_var, block, alt)
            },
            Object::ByteArray(arr) => {
                self.eval_for_in_statement_inner(arr, var, index_var, islast_var, block, alt)
            },
            Object::Browser(b) => {
                let tabs = b.get_tabs()?;
                self.eval_for_in_statement_inner(tabs, var, index_var, islast_var, block, alt)
            },
            Object::RemoteObject(remote) => {
                let vec = remote.to_object_vec()?;
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            },
            Object::WebViewRemoteObject(remote) => {
                let vec = remote.to_object_vec()?;
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            },
            Object::ComObject(com) => {
                let vec = com.to_object_vec()?;
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            },
            Object::UObject(uo) => {
                let vec = uo.to_object_vec()?;
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            },
            Object::HtmlNode(node) => {
                let vec = node.into_vec().ok_or(UError::new(
                    UErrorKind::SyntaxError, UErrorMessage::ForInError
                ))?;
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            }
            #[cfg(feature="chkimg")]
            Object::ChkClrResult(vec) => {
                self.eval_for_in_statement_inner(vec, var, index_var, islast_var, block, alt)
            },
            Object::ParamStr(v) => {
                self.eval_for_in_statement_inner(v, var, index_var, islast_var, block, alt)
            }
            _ => Err(UError::new(
                UErrorKind::SyntaxError,
                UErrorMessage::ForInError
            ))
        }
    }
    fn eval_for_in_statement_inner<O: Into<Object>>(
        &mut self,
        col_obj: Vec<O>,
        var: &str,
        index_var: Option<Identifier>,
        islast_var: Option<Identifier>,
        block: BlockStatement,
        alt: Option<BlockStatement>,
    ) -> EvalResult<Option<Object>> {
        let mut broke = false;
        let len = col_obj.len();
        for (i, o) in col_obj.into_iter().enumerate() {
            self.env.assign(var, o.into())?;
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
        self.eval_conditional_expression(expression)
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

    fn eval_repeat_statement(&mut self, stmt: StatementWithRow, block: BlockStatement) -> EvalResult<Option<Object>> {
        if let StatementWithRow { statement: Statement::Expression(expr), row, line, script_name } = stmt {
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
                match self.eval_conditional_expression(expr.clone()) {
                    Ok(b) => if b {
                        break;
                    },
                    Err(mut e) => {
                        e.set_line(row, line, script_name);
                        return Err(e);
                    },
                }
            }
            Ok(None)
        } else {
            unreachable!();
        }
    }

    fn eval_funtcion_definition_statement(&mut self, name: &String, params: Vec<FuncParam>, body: BlockStatement, is_proc: bool, is_async: bool) -> EvalResult<Object> {
        for statement in &body {
            if let Statement::Function{name: _, params: _, body: _, is_proc: _, is_async: _} = statement.statement {
                return Err(UError::new(
                    UErrorKind::FuncDefError,
                    UErrorMessage::NestedDefinition
                ))
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
                Statement::Dim(vec, _) => {
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
                Statement::HashTbl(v, is_pub) => {
                    for (i, opt) in v {
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
                Statement::Hash(hash) => {
                    self.eval_hash_sugar_statement(hash, Some(&mut module))?;
                }
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
                            // public hashtbl
                            Statement::HashTbl(v, true) => {
                                for (i, opt) in v {
                                    let (name, hashtbl) = self.eval_hashtbl_definition_statement(i, opt)?;
                                    self.env.define_module_public(&name, hashtbl.clone())?;
                                    module.add(name, hashtbl, ContainerType::Public);
                                }
                            },
                            Statement::Hash(ref hash) => {
                                if hash.is_public {
                                    self.eval_hash_sugar_statement(hash.clone(), Some(&mut module))?;
                                } else {
                                    new_body.push(statement);
                                }
                            }
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
        module.remove_outer_from_private_func();
        let m = Arc::new(Mutex::new(module));
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
            if let Some(Object::Exit) = obj {
                return Ok(obj)
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
                                    let err = e.errror_text_with_line();
                                    out_log(&err, LogType::Error);
                                    let title = UWSCRErrorTitle::ThreadError.to_string();
                                    show_message(&err, &title, true);
                                }
                            }
                            _ => {
                                let err = e.errror_text_with_line();
                                out_log(&err, LogType::Error);
                                let title = UWSCRErrorTitle::ThreadError.to_string();
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
                let result = evaluator.eval_function_call_expression(*func, args, false);
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
                                b = (l / a) + (if l % a == 0 {0} else {1});
                            }
                            a.checked_mul(b).unwrap_or_default()
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
                                    if let Some(o) = o {
                                        dimension.push(o);
                                    } else {
                                        break;
                                    }
                                }
                                array.push(Object::Array(dimension));
                                if tmp.is_empty() {
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
                if self.short_circuit && (i == Infix::AndL || i == Infix::OrL) {
                    self.eval_expression_short_circuit(Expression::Infix(i, l, r))
                        .map(|c| c.into_bool().into())?
                } else {
                    let left = self.eval_expression(*l)?;
                    let right = self.eval_expression(*r)?;
                    self.eval_infix_expression(i, left, right)?
                }
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
                self.eval_function_call_expression(*func, args, is_await)?
            },
            Expression::Assign(l, r) => {
                let value = self.eval_expression(*r)?;
                self.eval_assign_expression(*l, value)?
            },
            Expression::CompoundAssign(l, r, i) => {
                let left = self.eval_expression(*l.clone())?;
                let right = self.eval_expression(*r)?;
                if i == Infix::Plus {
                    if let Object::UObject(u) = left {
                        // UObjectの配列はpushできる
                        // let new_value = Self::object_to_serde_value(right)?;
                        return if u.push(right) {
                            Ok(Object::UObject(u))
                        } else {
                            Err(UError::new(
                                UErrorKind::UObjectError,
                                UErrorMessage::PlusAssignToObjectTypeValueNotAllowed,
                            ))
                        };
                    }
                }
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
                if let Object::String(json) = self.expand_string(json, true, None) {
                    Object::UObject(UObject::from_json_str(&json)?)
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
        let Identifier(name) = &identifier;
        self.get_variable(name)
            .or(self.env.get_function(name))
            .or(self.env.get_module(name))
            .or(self.env.get_class(name))
            .or(self.env.get_struct(name))
            .ok_or(UError::new(
                UErrorKind::EvaluatorError,
                UErrorMessage::NoIdentifierFound(identifier.0)
            ))
    }
    fn eval_dot_op_identifier(&mut self, identifier: Identifier) -> EvalResult<Object> {
        let Identifier(name) = &identifier;
        let obj = self.env.get_module(name)
            .or(self.get_variable(name))
            .ok_or(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::InvalidDotLeftIdentifier(identifier)
            ))?;
        if let Object::Reference(expression, outer) = obj {
            self.eval_reference(expression, &outer)
        } else {
            Ok(obj)
        }
    }

    fn eval_prefix_expression(&mut self, prefix: Prefix, right: Object) -> EvalResult<Object> {
        match prefix {
            Prefix::Not => self.eval_not_operator_expression(right),
            Prefix::Minus => self.eval_minus_operator_expression(right),
            Prefix::Plus => self.eval_plus_operator_expression(right),
        }
    }

    fn eval_not_operator_expression(&mut self, right: Object) -> EvalResult<Object> {
        let b = match Self::get_condition_type() {
            ConditionType::ForceBool => match right {
                Object::Bool(b) => b,
                _ => Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::ForceBoolError))?,
            },
            ConditionType::UWSC => right.as_uwsc_cond()?,
            ConditionType::Default => right.is_truthy(),
        };
        Ok((! b).into())
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
            } else if let Some(i) = index.as_f64(false) {
                self.eval_array_index_expression(a.clone(), i)?
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
                                HashTblEnum::HASH_KEY => if let Some(index) = i {
                                    hash.get_key(index)
                                } else {
                                    return Err(UError::new(
                                    UErrorKind::EvaluatorError,
                                        UErrorMessage::MissingHashIndex("HASH_KEY".into())
                                    ));
                                },
                                HashTblEnum::HASH_VAL => if let Some(index) = i {
                                    hash.get_value(index)
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
                u.get(&index)?
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
                        remote.get(Some(name), Some(&index))?
                    },
                    MemberCaller::ComObject(com) => {
                        com.get_property_by_index(name, vec![index.clone()])?
                    },
                    MemberCaller::WebViewRemoteObject(remote) => {
                        let index = index.to_string();
                        remote.get_property(name, Some(&index))?
                    }
                    MemberCaller::Module(_) |
                    MemberCaller::ClassInstance(_) |
                    MemberCaller::WebViewForm(_) |
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
                    MemberCaller::UObject(_) => {
                        unreachable!();
                    }
                }
            },
            Object::ParamStr(param_str) => {
                if let Object::Num(i) = index {
                    match param_str.get(i as usize) {
                        Some(str) => str.to_string().into(),
                        None => Object::Empty,
                    }
                } else {
                    return Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            Object::HtmlNode(mut node) => {
                if let Object::Num(i) = index {
                    node.set_index(i as usize);
                    Object::HtmlNode(node)
                } else {
                    return Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            }
            #[cfg(feature="chkimg")]
            Object::ChkClrResult(vec) => {
                if let Object::Num(i) = index {
                    vec.get(i as usize)
                        .map(|found| found.into())
                        .ok_or(UError::new(
                            UErrorKind::EvaluatorError,
                            UErrorMessage::IndexOutOfBounds(index),
                        ))?
                } else {
                    return Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            #[cfg(feature="chkimg")]
            Object::ColorFound(found) => {
                let found = Object::from(found);
                self.get_index_value(found, index, None)?
            }
            o => return Err(UError::new(
                UErrorKind::Any("Evaluator::get_index_value".into()),
                UErrorMessage::NotAnArray(o.to_owned()),
            ))
        };
        Ok(obj)
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: f64) -> EvalResult<Object> {
        if array.is_empty() {
            Err(UError::new(
                UErrorKind::EvaluatorError,
                UErrorMessage::IndexOutOfBounds(Object::Num(index)),
            ))
        } else {
            let max = array.len() - 1;
            if index < 0.0 || index as usize > max {
                return Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::IndexOutOfBounds(Object::Num(index)),
                ));
            }
            let obj = array.get(index as usize).map_or(Object::Empty, |o| o.clone());
            Ok(obj)
        }
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
                let index = self.eval_expression(*expr_index)?;
                self.assign_index(*expr_array, index, value, None)?;
            },
            Expression::DotCall(expr_object, expr_member) => {
                self.update_object_member(*expr_object, *expr_member, value)?;
            },
            Expression::FuncCall { func, args, is_await: false } => {
                let index = match args.len() {
                    0 => Object::Empty,
                    1 => self.eval_expression(args[0].to_owned())?,
                    _ => {
                        return Err(UError::new(
                            UErrorKind::AssignError,
                            UErrorMessage::Any("Too many parameters".into())
                        ));
                    },
                };
                self.assign_index(*func, index, value, None)?;
            }
            e => return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::NotAVariable(e)
            ))
        }
        Ok(assigned_value)
    }
    fn assign_identifier(&mut self, name: &str, new: Object) -> EvalResult<()> {
        match self.get_variable("this").unwrap_or_default() {
            Object::Module(mutex) => {
                let mut this = mutex.lock().unwrap();
                if this.has_member(name) {
                    this.assign(name, new, None)?;
                } else {
                    self.env.assign(name, new)?;
                }
            },
            Object::Instance(mutex) => {
                let ins = mutex.lock().unwrap();
                let mut this = ins.module.lock().unwrap();
                if this.has_member(name) {
                    this.assign(name, new, None)?;
                } else {
                    self.env.assign(name, new)?;
                }
            }
            _ => {
                self.env.assign(name, new)?;
            }
        }
        Ok(())
    }

    /// 配列要素の更新
    fn update_array(&mut self, name: &str, index: Object, dimensions: Option<Vec<Object>>, new: Object) -> EvalResult<()> {
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
                match self.get_variable("this").unwrap_or_default() {
                    Object::Module(mutex) => {
                        let mut this = mutex.lock().unwrap();
                        if this.has_member(name) {
                            this.assign(name, new_value, None)?;
                        } else {
                            self.env.assign(name, new_value)?;
                        }
                    },
                    Object::Instance(mutex) => {
                        let ins = mutex.lock().unwrap();
                        let mut this = ins.module.lock().unwrap();
                        if this.has_member(name) {
                            this.assign(name, new_value, None)?;
                        } else {
                            self.env.assign(name, new_value)?;
                        }
                    }
                    _ => {
                        self.env.assign(name, new_value)?;
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
        // 変数がthisかどうかチェックする
        let is_this = Module::is_it_this(&expr_object);
        let instance = self.eval_expr(expr_object)?;
        match instance {
            Object::Module(mutex) => {
                let mut guard = mutex.lock().unwrap();
                if is_this {
                    guard.assign(&member, new, dimension)?;
                } else {
                    guard.assign_public(&member, new, dimension)?;
                }
            },
            Object::Instance(mutex) => {
                let ins = mutex.lock().unwrap();
                let mut guard = ins.module.lock().unwrap();
                if is_this {
                    guard.assign(&member, new, dimension)?;
                } else {
                    guard.assign_public(&member, new, dimension)?;
                }
            },
            // Value::Array
            Object::UObject(uo) => {
                uo.set(index, new, Some(member))?;
            },
            Object::ComObject(com) => {
                com.set_property_by_index(&member, index, new)?;
            },
            Object::RemoteObject(ref remote) => {
                let value = new.try_into()?;
                remote.set(Some(&member), Some(&index.to_string()), value)?;
            },
            Object::WebViewRemoteObject(ref remote) => {
                let index = index.to_string();
                remote.set_property(&member, new, Some(&index))?;
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
    fn assign_index(&mut self, expr_array: Expression, index: Object, new: Object, dimensions: Option<Vec<Object>>) -> EvalResult<()> {
        match expr_array {
            // 配列要素の更新
            Expression::Identifier(Identifier(ref name)) => {
                if let Object::Reference(e, outer) = self.eval_expr(expr_array.clone())? {
                    let mut outer_env = self.clone();
                    outer_env.env.current = outer;
                    outer_env.assign_index(e, index, new, dimensions)?;
                } else {
                    self.update_array(name, index, dimensions, new)?;
                }
            },
            // オブジェクトメンバの配列要素の更新
            Expression::DotCall(expr_object, expr_member) => {
                let Expression::Identifier(Identifier(member)) = *expr_member else {
                    return Err(UError::new(UErrorKind::AssignError, UErrorMessage::MemberShouldBeIdentifier));
                };
                self.update_member_array(*expr_object, member, index, dimensions, new)?;
            },
            // 多次元配列の場合添字の式をexpr_dimensionsに積む
            Expression::Index(expr_inner_array, expr_inner_index, _) => {
                let dimensions = match dimensions {
                    Some(mut d) => {
                        d.push(index);
                        Some(d)
                    },
                    None => Some(vec![index]),
                };
                let inner_index = self.eval_expression(*expr_inner_index)?;
                self.assign_index(*expr_inner_array, inner_index, new, dimensions)?;
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
                let value = new.clone().try_into()?;
                remote.set(None, Some(&index), value)?;
                Ok((None, false))
            },
            Object::UObject(uobj) => {
                uobj.set(index, new.clone(), None)?;
                Ok((None, false))
            }
            _ => Err(UError::new(UErrorKind::AssignError, UErrorMessage::NotAnArray("".into())))
        }
    }
    fn update_object_member(&mut self, expr_object: Expression, expr_member: Expression, new: Object) -> EvalResult<()>{
        let instance = self.eval_expr(expr_object)?;
        match instance {
            Object::Module(m) => {
                match expr_member {
                    Expression::Identifier(Identifier(name)) => {
                        let mut module = m.lock().unwrap();
                        if module.is_local_member(&name, false) {
                            // ローカルメンバだった場合thisと比較し、同一モジュールであればローカルメンバへ代入
                            if let Some(Object::Module(this)) = self.get_variable("this") {
                                if this.try_lock().is_err() {
                                    module.assign(&name, new, None)?;
                                } else {
                                    return Err(UError::new(UErrorKind::AssignError, UErrorMessage::PrivateAssignNotAllowed))
                                }
                            } else {
                                return Err(UError::new(UErrorKind::AssignError, UErrorMessage::PrivateAssignNotAllowed))
                            }
                        } else {
                            module.assign_public(&name, new, None)?;
                        }
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
                    let mut module = ins.module.lock().unwrap();
                    if module.is_local_member(&name, false) {
                        // ローカルメンバだった場合、thisと比較
                        if let Some(Object::Instance(this)) = self.get_variable("this") {
                            if this.try_lock().is_err() {
                                // thisがロックできない場合に限りローカルメンバの代入を行う
                                module.assign(&name, new, None)?;
                            } else {
                                return Err(UError::new(UErrorKind::AssignError, UErrorMessage::PrivateAssignNotAllowed))
                            }
                        } else {
                            return Err(UError::new(UErrorKind::AssignError, UErrorMessage::PrivateAssignNotAllowed))
                        }
                    } else {
                        module.assign_public(&name, new, None)?;
                    }
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
                    uo.set(index, new, None)?;
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
            Object::WebViewRemoteObject(ref remote) => {
                if let Expression::Identifier(Identifier(name)) = expr_member {
                    remote.set_property(&name, new, None)?;
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
        if self.special_char {
            Object::String(string)
        } else {
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
                            self.get_variable(name)
                        }.map(|o| o.to_string().into())
                    } else {
                        continue;
                    },
                };
                new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
            }
            Object::String(new_string)
        }
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
                            None => match self.get_variable(&name) {
                                Some(o) => Ok(o),
                                None => Err(UError::new(
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
            let ret = func.invoke(&mut evaluator, arguments, None);
            com.uninit();
            ret
        });

        UTask {
            handle: Arc::new(Mutex::new(Some(handle))),
        }
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
        // let parser = Parser::new(Lexer::new(script), None, false);
        let parser = Parser::new_eval_parser(Lexer::new(script));
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

    fn eval_function_call_expression(&mut self, func: Expression, args: Vec<Expression>, is_await: bool) -> EvalResult<Object> {
        let func_object = self.eval_expression_for_func_call(func)?;
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
                Object::Function(f) => f.invoke(self, arguments, None),
                Object::AsyncFunction(f) => {
                    let task = self.new_task(f, arguments);
                    if is_await {
                        self.await_task(task)
                    } else {
                        Ok(Object::Task(task))
                    }
                },
                Object::AnonFunc(f) => f.invoke(self, arguments, None),
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
                    let module = self.eval_module_statement(&name, block)?;
                    let constructor = {
                        let guard = module.lock().unwrap();
                        match guard.get_constructor() {
                            Some(constructor) => {
                                constructor
                            },
                            None => return Err(UError::new(
                                UErrorKind::ClassError,
                                UErrorMessage::ConstructorNotDefined(name.clone()),
                            )),
                        }
                    };
                    let ins = Arc::new(Mutex::new(ClassInstance::new(name, module, self.clone())));
                    let this = Some(function::This::Class(ins.clone()));
                    constructor.invoke(self, arguments, this)?;
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
                    let r = remote.invoke_as_function(args, is_await)?;
                    Ok(r)
                },
                Object::MemberCaller(method, member) => {
                    match method {
                        MemberCaller::Module(m) => {
                            let obj = self.get_module_member(&m, &member, true)?;
                            match obj {
                                Object::Function(f) |
                                Object::AnonFunc(f) => {
                                    let this = Some(function::This::Module(m));
                                    f.invoke(self, arguments, this)
                                },
                                Object::DefDllFunction(f) => {
                                    f.invoke(arguments, self)
                                },
                                _ => unreachable!(),
                            }
                        },
                        MemberCaller::ClassInstance(ins) => {
                            let obj = {
                                let guard = ins.lock().unwrap();
                                self.get_module_member(&guard.module, &member, true)
                            }?;
                            match obj {
                                Object::Function(f) |
                                Object::AnonFunc(f) => {
                                    let this = Some(function::This::Class(ins));
                                    f.invoke(self, arguments, this)
                                },
                                Object::DefDllFunction(f) => {
                                    f.invoke(arguments, self)
                                },
                                _ => unreachable!(),
                            }
                        },
                        MemberCaller::BrowserBuilder(mutex) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
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
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            browser.invoke_method(&member, args)
                        },
                        MemberCaller::TabWindow(tab) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            tab.invoke_method(&member, args)
                        },
                        MemberCaller::RemoteObject(remote) => {
                            let args = arguments.into_iter()
                                .map(|(_, o)| browser::RemoteFuncArg::from_object(o))
                                .collect::<EvalResult<Vec<browser::RemoteFuncArg>>>()?;
                            remote.invoke_method(&member, args, is_await)
                        },
                        MemberCaller::WebRequest(mutex) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
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
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            res.invoke_method(&member, args)
                        },
                        MemberCaller::HtmlNode(node) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            node.invoke_method(&member, args)
                        },
                        MemberCaller::ComObject(_) => {
                            unreachable!()
                        },
                        MemberCaller::UStruct(ust) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            ust.invoke_method(&member, args)
                        },
                        MemberCaller::WebViewForm(form) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            let obj = form.invoke_method(&member, args, self)?;
                            Ok(obj)
                        },
                        MemberCaller::WebViewRemoteObject(remote) => {
                            let args = arguments.into_iter()
                                .map(|(_, arg)| arg)
                                .collect();
                            let obj = remote.invoke_method(&member, args, is_await)?;
                            Ok(obj)
                        },
                        MemberCaller::UObject(uobj) => uobj.invoke_method(&member)
                    }
                },
                o => Err(UError::new(
                    UErrorKind::EvaluatorError,
                    UErrorMessage::NotAFunction(o),
                )),
            }
        }
    }

    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> EvalResult<Object> {
        if self.eval_conditional_expression(condition)? {
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
            Expression::Identifier(identifier) => {
                self.eval_dot_op_identifier(identifier)?
            },
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
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::Module(m), member))
                } else {
                    self.get_module_member(&m, &member, is_func)
                }
            },
            Object::Instance(ins) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::ClassInstance(ins), member))
                } else {
                    let guard = ins.lock().unwrap();
                    self.get_module_member(&guard.module, &member, is_func)
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
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::UObject(u), member))
                } else {
                    let index = member.into();
                    u.get(&index)
                }
            },
            Object::Enum(e) => {
                if is_func {
                    Err(UError::new(
                        UErrorKind::EnumError,
                        UErrorMessage::CanNotCallMethod(member)
                    ))
                } else if let Some(n) = e.get(&member) {
                    Ok(Object::Num(n))
                } else {
                    Err(UError::new(
                        UErrorKind::EnumError,
                        UErrorMessage::MemberNotFound(member)
                    ))
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
            },
            Object::WebViewForm(form) => {
                if is_func {
                    Ok(Object::MemberCaller(MemberCaller::WebViewForm(form), member))
                } else {
                    form.get_property(&member).map_err(|e| e.into())
                }
            },
            Object::WebViewRemoteObject(remote) => {
                if is_func || is_indexed_property {
                    Ok(Object::MemberCaller(MemberCaller::WebViewRemoteObject(remote), member))
                } else {
                    remote.get_property(&member, None).map_err(|e| e.into())
                }
            },
            o => Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::DotOperatorNotSupported(o)
            )),
        }
    }

    fn get_module_member(&self, mutex: &Arc<Mutex<Module>>, member: &String, is_func: bool) -> EvalResult<Object> {
        let module = mutex.try_lock().expect("Dead lock: Evaluator::get_module_member");
        if module.is_local_member(member, is_func) {
            match self.get_variable("this").unwrap_or_default() {
                Object::Module(this) => {
                    if this.try_lock().is_err() {
                        // ロックに失敗した場合thisと呼び出し元が同一と判断し、プライベートメンバを返す
                        if is_func {
                            return module.get_function(member);
                        } else {
                            return module.get_member(member);
                        }
                    }
                }
                Object::Instance(ins) => {
                    if ins.try_lock().is_err() {
                        // ロックに失敗した場合moduleとインスタンス内のモジュールは同一と判断し、プライベートメンバを返す
                        if is_func {
                            return module.get_function(member);
                        } else {
                            return module.get_member(member);
                        }
                    }
                }
                _ => {}
            }
            let member_name = if is_func {
                member.to_string() + "()"
            } else {
                member.to_string()
            };
            Err(UError::new(
                UErrorKind::DotOperatorError,
                UErrorMessage::IsPrivateMember(module.name(), member_name)
            ))
        } else if is_func {
            module.get_function(member)
        } else {
            match module.get_public_member(member) {
                Ok(Object::ExpandableTB(text)) => Ok(self.expand_string(text, true, None)),
                Ok(Object::Function(_)) => {
                    Ok(Object::MemberCaller(MemberCaller::Module(mutex.clone()), member.clone()))
                },
                res => res
            }
        }
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

    use crate::*;
    use parser::lexer::Lexer;
    use parser::Parser;
    use crate::error::{UErrorKind,UErrorMessage,DefinitionType,ParamTypeDetail};

    use rstest::{rstest, fixture};

    // 変数とか関数とか予め定義しておく
    fn eval_env(input: &str) -> Evaluator {
        match Parser::new(Lexer::new(input), None, None).parse() {
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

    /// 評価後のObjectをexpectedと比較
    fn expect_object_test(evaluator: Option<&mut Evaluator>, input: &str, expected: Object) {
        match Parser::new(Lexer::new(input), None, None).parse() {
            Ok(program) => {
                if let Some(e) = evaluator {
                    match e.eval(program, false) {
                        Ok(result) => {
                            assert_eq!(result, Some(expected));
                        },
                        Err(err) => {
                            panic!("Evaluator Error: {err:?}");
                        },
                    }
                } else {
                    let mut e = Evaluator::new(Environment::new(vec![]));
                    match e.eval(program, true) {
                        Ok(result) => {
                            assert_eq!(result, Some(expected));
                        },
                        Err(err) => {
                            panic!("Evaluator Error: {err:?}");
                        },
                    }
                }
            },
            Err(err) => {
                panic!("Parse Error: {err:#?}");
            }
        }
    }
    /// 評価後のエラーをexpectedと比較
    fn expect_error_test(evaluator: Option<&mut Evaluator>, input: &str, expected_kind: UErrorKind, expected_message: UErrorMessage) {
        match Parser::new(Lexer::new(input), None, None).parse() {
            Ok(program) => {
                let expected = UError::new(expected_kind, expected_message);
                if let Some(e) = evaluator {
                    match e.eval(program, false) {
                        Ok(_) => {
                            panic!("Error expected: {expected:?}");
                        },
                        Err(err) => {
                            assert_eq!(err, expected);
                        },
                    }
                } else {
                    let mut e = Evaluator::new(Environment::new(vec![]));
                    match e.eval(program, true) {
                        Ok(_) => {
                            panic!("Error expected: {expected:?}");
                        },
                        Err(err) => {
                            assert_eq!(err, expected);
                        },
                    }
                }
            },
            Err(err) => {
                panic!("Parse Error: {err:#?}");
            }
        }
    }

    #[rstest]
    #[case("5", Object::Num(5.0))]
    #[case("10", Object::Num(10.0))]
    #[case("-5", Object::Num(-5.0))]
    #[case("-10", Object::Num(-10.0))]
    #[case("1.23", Object::Num(1.23))]
    #[case("-1.23", Object::Num(-1.23))]
    #[case("+(-5)", Object::Num(-5.0))]
    #[case("1 + 2 + 3 - 4", Object::Num(2.0))]
    #[case("2 * 3 * 4", Object::Num(24.0))]
    #[case("-3 + 3 * 2 + -3", Object::Num(0.0))]
    #[case("5 + 3 * -2", Object::Num(-1.0))]
    #[case("6 / 3 * 2 + 1", Object::Num(5.0))]
    #[case("1.2 + 2.4", Object::Num(3.5999999999999996))]
    #[case("1.2 * 3", Object::Num(3.5999999999999996))]
    #[case("2 * (5 + 10)", Object::Num(30.0))]
    #[case("3 * 3 * 3 + 10", Object::Num(37.0))]
    #[case("3 * (3 * 3) + 10", Object::Num(37.0))]
    #[case("(5 + 10 * 2 + 15 / 3) * 2 + -10", Object::Num(50.0))]
    #[case("1 + TRUE", Object::Num(2.0))]
    #[case("1 + false", Object::Num(1.0))]
    #[case("TRUE + 1", Object::Num(2.0))]
    #[case("5 mod 3", Object::Num(2.0))]
    fn test_num_expression(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[fixture]
    #[once]
    fn no_opt_same_str_fixture() -> Evaluator {
        eval_env("OPTION SAMESTR=FALSE")
    }
    #[rstest]
    #[case(r#""hoge" + "fuga""#, Object::String("hogefuga".to_string()))]
    #[case(r#""hoge" + 100"#, Object::String("hoge100".to_string()))]
    #[case(r#"400 + "fuga""#, Object::String("400fuga".to_string()))]
    #[case(r#""hoge" + TRUE"#, Object::String("hogeTrue".to_string()))]
    #[case(r#""hoge" + FALSE"#, Object::String("hogeFalse".to_string()))]
    #[case(r#"TRUE + "hoge""#, Object::String("Truehoge".to_string()))]
    #[case(r#""hoge" = "hoge""#, Object::Bool(true))]
    #[case(r#""hoge" == "hoge""#, Object::Bool(true))]
    #[case(r#""hoge" == "fuga""#, Object::Bool(false))]
    #[case(r#""hoge" == "HOGE""#, Object::Bool(true))]
    #[case(r#""hoge" == 1"#, Object::Bool(false))]
    #[case(r#""hoge" != 1"#, Object::Bool(true))]
    #[case(r#""hoge" <> 1"#, Object::Bool(true))]
    #[case(r#""hoge" <> "hoge""#, Object::Bool(false))]
    fn test_string_infix(#[case] input: &str, #[case] expected: Object) {
        let mut e = no_opt_same_str_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }

    // #[fixture]
    // fn opt_same_str_fixture() -> Evaluator {
    //     eval_env("OPTION SAMESTR")
    // }
    // #[rstest]
    // #[case(r#""hoge" == "HOGE""#, false.into())]
    // #[case(r#""HOGE" == "HOGE""#, true.into())]
    // fn test_same_str(#[case] input: &str, #[case] expected: Object) {
    //     let mut e = opt_same_str_fixture();
    //     expect_object_test(Some(&mut e), input, expected)
    // }

    #[rstest]
    #[case(
        r#"
dim hoge = 1
hoge = 2
hoge
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
dim HOGE = 2
hoge
        "#,
        Object::Num(2.0)
    )]
    fn test_assign_variable(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected)
    }
    #[rstest]
    #[case(r#"
dim hoge = 2
dim hoge = 3
        "#,
        UErrorKind::DefinitionError(DefinitionType::Variable),
        UErrorMessage::AlreadyDefined("hoge".into())
    )]
    fn test_assign_variable_error(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        expect_error_test(None, input, expected_kind, expected_message)
    }

    #[rstest]
    #[case(
        r#"
hashtbl hoge
hoge["test"] = 2
hoge["test"]
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
hashtbl hoge
hoge["test"] = 2
hoge["TEST"]
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
hashtbl hoge
hoge[1.23] = 2
hoge[1.23]
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
hashtbl hoge
hoge[FALSE] = 2
hoge[FALSE]
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
hashtbl hoge = HASH_CASECARE
hoge["abc"] = 1
hoge["ABC"] = 2
hoge["abc"] + hoge["ABC"]
        "#,
        Object::Num(3.0)
    )]
    #[case(
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
        Object::String("cdba".to_string())
    )]
    #[case(
        r#"
public hashtbl hoge
hoge["a"] = "hoge"

function f(key)
result = hoge[key]
fend

f("a")
        "#,
        Object::String("hoge".to_string())
    )]
    #[case(
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
        Object::Bool(true)
    )]
    #[case(
        r#"
hash hoge = hash_casecare or hash_sort
foo = 1
bar = 2
endhash
hoge['foo'] = 1 and hoge['bar'] = 2
        "#,
        Object::Bool(true)
    )]
    fn test_assign_hashtbl(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(r#"
dim hoge[] = 1,3,5
hoge[0] = "hoge"
hoge[0]
        "#,
        "hoge".into()
    )]
    fn test_assign_array(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(r#"
hoge = [1,3,5]
hoge[0] = 2
hoge[0]
        "#,
        2.into()
    )]
    fn test_assign_array_literal(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
hoge = [[1],[2]]
hoge[0][0] = 100
hoge
        "#,
        Object::Array(vec![
            Object::Array(vec![Object::Num(100.0)]),
            Object::Array(vec![Object::Num(2.0)]),
        ])
    )]
    #[case(
        r#"
hoge = [[[1]]]
hoge[0][0][0] = 100
hoge
        "#,
        Object::Array(vec![
            Object::Array(vec![
                Object::Array(vec![Object::Num(100.0)]),
            ]),
        ])
    )]
    fn test_assign_multi_dimensional_array(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
public hoge = 1
hoge
        "#,
        1.into()
    )]
    fn test_public(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
        dim hoge[3] = 1,2
        hoge
        "#,
        Object::Array(vec![
            Object::Num(1.0),
            Object::Num(2.0),
            Object::Empty,
            Object::Empty,
        ])
    )]
    #[case(
        r#"
        dim hoge[2][2] = 1,2,3, 4,5,6, 7
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        dim hoge[2, 2] = 1,2,3, 4,5,6, 7
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        // 省略
        dim hoge[, 2] = 1,2,3, 4,5,6, 7
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        // 多次元
        dim hoge[1][1][1] = 0,1, 2,3, 4,5, 6,7
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        // 省略
        dim hoge[][1][1] = 0,1, 2,3, 4,5, 6,7
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        // EMPTY埋め
        dim hoge[1][1][1] = 0,1, 2,3, 4
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    #[case(
        r#"
        // 省略+EMPTY埋め
        dim hoge[][1][1] = 0,1, 2,3, 4,5, 6
        hoge
        "#,
        Object::Array(vec![
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
        ])
    )]
    fn test_array_definition(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }
    #[rstest]
    #[case(
        format!(r#"
// usize超え
dim hoge[{}][1]
hoge
        "#, usize::MAX),
        UErrorKind::ArrayError,
        UErrorMessage::InvalidArraySize
    )]
    fn test_array_definition_err(#[case] input: String, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        expect_error_test(None, &input, expected_kind, expected_message);
    }
    #[test]
    fn test_print() {
        let input = r#"
hoge = "print test"
print hoge
        "#;
        expect_object_test(None, input, "print test".into());
    }

    #[rstest]
    #[case(
        r#"
for i = 0 to 3
next
i
        "#,
        Object::Num(4.0)
    )]
    #[case(
        r#"
for i = 0 to 2
i = 10
next
i
        "#,
        Object::Num(3.0)
    )]
    #[case(
        r#"
for i = 0 to 5 step 2
next
i
        "#,
        Object::Num(6.0)
    )]
    #[case(
        r#"
for i = 5 to 0 step -2
next
i
        "#,
        Object::Num(-1.0)
    )]
    #[case(
        r#"
for i = "0" to "5" step "2"
next
i
        "#,
        Object::Num(6.0)
    )]
    #[case(
        r#"
a = 1
for i = 0 to 3
continue
a = a  + 1
next
a
        "#,
        Object::Num(1.0)
    )]
    #[case(
        r#"
a = 1
for i = 0 to 20
break
a = 5
next
a
        "#,
        Object::Num(1.0)
    )]
    #[case(
        r#"
a = 0
for i = 0 to 0
a = 1
else
a = 2
endfor
a
        "#,
        Object::Num(2.0)
    )]
    #[case(
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
        Object::Num(2.0)
    )]
    #[case(
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
        Object::Num(1.0)
    )]
    fn test_for(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
for i = 0 to "5s"
next
        "#,
        UErrorKind::SyntaxError,
        UErrorMessage::ForError("for i = 0 to 5s".into())
    )]
    fn test_for_err(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        expect_error_test(None, input, expected_kind, expected_message);
    }

    #[rstest]
    #[case(
        r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
a = a + n
next
a
        "#,
        Object::Num(15.0)
    )]
    #[case(
        r#"
a = ""
for c in "hoge"
a = c + a
next
a
        "#,
        Object::String("egoh".to_string())
    )]
    #[case(
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
        Object::Num(6.0)
    )]
    #[case(
        r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
a = a + n
if n = 3 then break
next
a
        "#,
        Object::Num(6.0)
    )]
    #[case(
        r#"
dim hoge[] = 1,2,3,4,5
a = 0
for n in hoge
continue
a = a + n
next
a
        "#,
        Object::Num(0.0)
    )]
    #[case(
        r#"
a = 0
for n in [1,2,3]
a = 1
else
a = 2
endfor
a
        "#,
        Object::Num(2.0)
    )]
    #[case(
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
        Object::Num(2.0)
    )]
    #[case(
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
        Object::Num(1.0)
    )]
    fn test_forin(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
a = 5
while a > 0
a = a -1
wend
a
        "#,
        Object::Num(0.0)
    )]
    #[case(
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
        Object::Num(4.0)
    )]
    fn test_while(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
a = 5
repeat
a = a - 1
until a < 1
a
        "#,
        Object::Num(0.0)
    )]
    #[case(
        r#"
a = 2
repeat
a = a - 1
if a < 0 then break else continue
until false
a
        "#,
        Object::Num(-1.0)
    )]
    fn test_repeat(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
if true then a = "a is true" else a = "a is false"
a
        "#,
        Object::String("a is true".to_string())
    )]
    #[case(
        r#"
if 1 < 0 then a = "a is true" else a = "a is false"
a
        "#,
        Object::String("a is false".to_string())
    )]
    #[case(
        r#"
a = 1
if false then a = 5
a
        "#,
        Object::Num(1.0)
    )]
    fn test_if_1line(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
if true then
a = "a is true"
else
a = "a is false"
endif
a
        "#,
        Object::String("a is true".to_string())
    )]
    #[case(
        r#"
if 0 then
a = "a is true"
else
a = "a is false"
endif
a
        "#,
        Object::String("a is false".to_string())
    )]
    #[case(
        r#"
if true then
a = "test succeed!"
else
a = "should not get this message"
endif
a
        "#,
        Object::String("test succeed!".to_string())
    )]
    #[case(
        r#"
a = 1
if false then
a = 5
endif
a
        "#,
        Object::Num(1.0)
    )]
    fn test_if(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
if false then
a = "should not get this message"
elseif true then
a = "test1 succeed!"
endif
a
        "#,
        Object::String("test1 succeed!".to_string())
    )]
    #[case(
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
        Object::String("test2 succeed!".to_string())
    )]
    #[case(
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
        Object::String("test3 succeed!".to_string())
    )]
    #[case(
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
        Object::String("test4 succeed!".to_string())
    )]
    fn test_elseif(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
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
        Object::String("test1 succeed!".to_string())
    )]
    #[case(
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
        Object::String("test2 succeed!".to_string())
    )]
    #[case(
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
        Object::String("test3 succeed!".to_string())
    )]
    #[case(
        r#"
select 6
default
a = "test4 succeed!"
selend
a
        "#,
        Object::String("test4 succeed!".to_string())
    )]
    #[case(
        r#"
select true
case 1 = 2
a = "should not get this message"
case 2 = 2
a = "test5 succeed!"
selend
a
        "#,
        Object::String("test5 succeed!".to_string())
    )]
    fn test_select(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
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
        Object::Num(5.0)
    )]
    #[case(
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
        Object::Num(5.0)
    )]
    #[case(
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
        Object::Num(5.0)
    )]
    #[case(
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
        Object::Num(7.0)
    )]
    #[case(
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
        Object::Num(2.0)
    )]
    fn test_block_in_loopblock(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
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
        Object::Num(5.0)
    )]
    #[case(
        r#"
hoge(5)

function hoge(n)
// no result
fend
        "#,
        Object::Empty
    )]
    #[case(
        r#"
a = hoge(5)
a == 5

procedure hoge(n)
result = n
fend
        "#,
        Object::Bool(false)
    )]
    #[case(
        r#"
a = 'should not be over written'
hoge(5)
a

procedure hoge(n)
a = n
fend
        "#,
        Object::String("should not be over written".to_string())
    )]
    #[case(
        r#"
f  = function(x, y)
result = x + y
fend

f(5, 10)
        "#,
        Object::Num(15.0)
    )]
    #[case(
        r#"
a = 1
p = procedure(x, y)
a = x + y
fend

p(5, 10)
a
        "#,
        Object::Num(1.0)
    )]
    #[case(
        r#"
closure = test_closure("testing ")
closure("closure")

function test_closure(s)
result = function(s2)
result = s + s2
fend
fend
        "#,
        Object::String("testing closure".to_string())
    )]
    #[case(
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
        Object::String("done".to_string())
    )]
    #[case(
        r#"
hoge(2, fuga)

function hoge(x, func)
result = func(x)
fend
function fuga(n)
result = n * 2
fend
        "#,
        Object::Num(4.0)
    )]
    fn test_function(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }

    #[test]
    fn test_comment() {
        let input = r#"
a = 1
// a = a + 2
a
        "#;
        let expected = 1.into();
        expect_object_test(None, input, expected);
    }

    #[rstest]
    #[case(
        r#"
public public_and_public = 1
public public_and_public = 2
public_and_public
        "#,
        2.into()
    )]
    fn test_duplicate_declaration(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
    }
    #[rstest]
    #[case(
        r#"
dim dim_and_dim = 1
dim dim_and_dim = 2
        "#,
        UErrorKind::DefinitionError(DefinitionType::Variable),
        UErrorMessage::AlreadyDefined("dim_and_dim".into())
    )]
    #[case(
        r#"
public pub_and_const = 1
const pub_and_const = 2
        "#,
        UErrorKind::DefinitionError(DefinitionType::Public),
        UErrorMessage::AlreadyDefined("pub_and_const".into())
    )]
    #[case(
        r#"
const const_and_const = 1
const const_and_const = 2
        "#,
        UErrorKind::DefinitionError(DefinitionType::Const),
        UErrorMessage::AlreadyDefined("const_and_const".into())
    )]
    #[case(
        r#"
hashtbl hash_and_hash
hashtbl hash_and_hash
        "#,
        UErrorKind::DefinitionError(DefinitionType::Variable),
        UErrorMessage::AlreadyDefined("hash_and_hash".into())
    )]
    #[case(
        r#"
function func_and_func()
fend
function func_and_func()
fend
        "#,
        UErrorKind::DefinitionError(DefinitionType::Function),
        UErrorMessage::AlreadyDefined("func_and_func".into())
    )]
    fn test_duplicate_declaration_err(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        expect_error_test(None, input, expected_kind, expected_message);
    }

    #[rstest]
    #[case(
        r#"
a = 1
a += 1
a
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
a = "hoge"
a += "fuga"
a
        "#,
        Object::String("hogefuga".to_string())
    )]
    #[case(
        r#"
a = 5
a -= 3
a
        "#,
        Object::Num(2.0)
    )]
    #[case(
        r#"
a = 2
a *= 5
a
        "#,
        Object::Num(10.0)
    )]
    #[case(
        r#"
a = 10
a /= 5
a
        "#,
        Object::Num(2.0)
    )]
    fn test_compound_assign(#[case] input: &str, #[case] expected: Object) {
        expect_object_test(None, input, expected);
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
        expect_object_test(None, input, 11.into())
    }

    #[fixture]
    #[once]
    fn scope_fixture() -> Evaluator {
        let definition = r#"
dim v = "script local"
public p = "public"
public p1 = "public"
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
        eval_env(definition)
    }
    #[rstest]
    #[case::get_v(
        "v",
        Object::String("script local".to_string())
    )]
    #[case::assign_v(
        r#"
        v += " 1"
        v
        "#,
        Object::String("script local 1".to_string())
    )]
    #[case::get_p(
        "p",
        Object::String("public".to_string())
    )]
    #[case::assign_p(
        r#"
        p1 += " 1"
        p1
        "#,
        Object::String("public 1".to_string())
    )]
    #[case::get_c(
        "c",
        Object::String("const".to_string())
    )]
    #[case::invoke_func(
        "func()",
        Object::String("function".to_string())
    )]
    #[case::get_f(
        "f",
        Object::String("variable".to_string())
    )]
    #[case::invoke_f(
        "f()",
        Object::String("function".to_string())
    )]
    #[case::invoke_get_p(
        "get_p()",
        Object::String("public".to_string())
    )]
    #[case::invoke_get_c(
        "get_c()",
        Object::String("const".to_string())
    )]
    #[case::get_module_p(
        "M.p",
        Object::String("module public".to_string())
    )]
    #[case::get_module_c(
        "M.c",
        Object::String("module const".to_string())
    )]
    #[case::invoke_module_func(
        "M.func()",
        Object::String("module function".to_string())
    )]
    #[case::invoke_module_get_v(
        "M.get_v()",
        Object::String("module local".to_string())
    )]
    #[case::invoke_module_get_this_v(
        "M.get_this_v()",
        Object::String("module local".to_string())
    )]
    #[case::invoke_module_get_m_v(
        "M.get_m_v()",
        Object::String("module local".to_string())
    )]
    #[case::invoke_module_get_p(
        "M.get_p()",
        Object::String("module public".to_string())
    )]
    #[case::invoke_module_get_outer_p2(
        "M.get_outer_p2()",
        Object::String("public 2".to_string())
    )]
    #[case::invoke_module_inner_func(
        "M.inner_func()",
        Object::String("module function".to_string())
    )]
    #[case::invoke_module_outer_func(
        "M.outer_func()",
        Object::String("function".to_string())
    )]
    #[case::invoke_module_set_a(
        "M.set_a(5)",
        Object::Num(5.0)
    )]
    fn test_scope(#[case] input: &str, #[case] expected: Object) {
        let mut e = scope_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }

    #[rstest]
    #[case(
        "get_v()",
        UErrorKind::EvaluatorError,
        UErrorMessage::NoIdentifierFound("v".into())
    )]
    #[case(
        "M.v",
        UErrorKind::DotOperatorError,
        UErrorMessage::IsPrivateMember("M".into(), "v".into())
    )]
    fn test_scope_err(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        let mut e = scope_fixture();
        expect_error_test(Some(&mut e), input, expected_kind, expected_message);
    }

    #[fixture]
    #[once]
    fn uobject_fixture() -> Evaluator {
        let input1 = r#"
dim obj = @{
    "foo": 1,
    "bar": {
        "baz": 2,
        "qux": [3, 4, 5]
    },
    "baz": [
        1,
        {
            "qux": 2
        }
    ],
    "quux": 0
}@
        "#;
        eval_env(input1)
    }
    #[rstest]
    #[case(
        "obj.foo",
        Object::Num(1.0)
    )]
    #[case(
        "obj.FOO",
        Object::Num(1.0)
    )]
    #[case(
        "obj.bar.baz",
        Object::Num(2.0)
    )]
    #[case(
        "obj.bar.qux[0]",
        Object::Num(3.0)
    )]
    #[case(
        "obj.quux = 2; obj.quux",
        Object::Num(2.0)
    )]
    #[case(
        "obj['quux'] = 5; obj['quux']",
        Object::Num(5.0)
    )]
    #[case(
        "obj.bar.qux[1] = 9; obj.bar.qux[1]",
        Object::Num(9.0)
    )]
    #[case(
        "obj.baz[0]",
        Object::Num(1.0)
    )]
    #[case(
        "obj.baz[1].qux",
        Object::Num(2.0)
    )]
    fn test_uobject(#[case] input: &str, #[case] expected: Object) {
        let mut e = uobject_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }

    #[fixture]
    #[once]
    fn param_type_fixture() -> Evaluator {
        let input = r#"
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
        eval_env(input)
    }

    #[rstest]
    #[case(
        "hoge('hoge', myclass(), 5, true)",
        Object::Empty
    )]
    #[case(
        "fuga(arr, h, hoge, uo)",
        Object::Empty
    )]
    fn test_param_type(#[case] input: &str, #[case] expected: Object) {

        let mut e = param_type_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }
    #[rstest]
    #[case(
        "hoge(3, myclass())",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("s".into(), ParamTypeDetail::String)
    )]
    #[case(
        "hoge('hoge', myclass2())",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("c".into(), ParamTypeDetail::UserDefinition("myclass".into()))
    )]
    #[case(
        "hoge('hoge', myclass(), 'aaa')",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("n".into(), ParamTypeDetail::Number)
    )]
    #[case(
        "hoge('hoge', myclass(),2, 'aaa')",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("b".into(), ParamTypeDetail::Bool)
    )]
    #[case(
        "fuga('hoge', h, hoge, uo)",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("a".into(), ParamTypeDetail::Array)
    )]
    #[case(
        "fuga(arr, arr, hoge, uo)",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("h".into(), ParamTypeDetail::HashTbl)
    )]
    #[case(
        "fuga(arr, h, 'hoge', uo)",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("f".into(), ParamTypeDetail::Function)
    )]
    #[case(
        "fuga(arr, h, hoge, 1)",
        UErrorKind::FuncCallError,
        UErrorMessage::InvalidParamType("u".into(), ParamTypeDetail::UObject)
    )]
    fn test_param_type_err(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        let mut e = param_type_fixture();
        expect_error_test(Some(&mut e), input, expected_kind, expected_message);
    }

    #[fixture]
    #[once]
    fn reference_fixture() -> Evaluator {
        let input = r#"
function test(ref p)
    p = "reference test"
fend
function test2(ref p: array, i: number)
    p[i] = "test2"
    result = p[i]
fend
function test3(ref p: array, i: number, j: number)
    p[i][j] = "test3"
fend
module M
    public p = "module"
    public q = [1]
    public r = [[1]]
endmodule
// test5, test6
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
        "#;
        eval_env(input)
    }
    #[rstest]
    #[case::variable(
        r#"
v = "hoge"
test(v)
v
        "#,
        "reference test".into()
    )]
    #[case::index(
        r#"
arr = ["hoge"]
test(arr[0])
arr[0]
        "#,
        "reference test".into()
    )]
    #[case::variable_index(
        r#"
arr = ["hoge"]
i = 0
test(arr[i])
arr[i]
        "#,
        "reference test".into()
    )]
    #[case::array(
        r#"
arr = ["hoge"]
test2(arr, 0)
arr[0]
        "#,
        "test2".into()
    )]
    #[case::index_2d(
        r#"
arr = [["foo"], ["bar"]]
test(arr[0][0])
arr[0][0]
        "#,
        "reference test".into()
    )]
    #[case::index_2d_variable(
        r#"
arr = [["foo"], ["bar"]]
test3(arr, 0, 0)
arr[0][0]
        "#,
        "test3".into()
    )]
    #[case::index_3d(
        r#"
arr = [[["foo"]]]
test(arr[0][0][0])
arr[0][0][0]
        "#,
        "reference test".into()
    )]
    #[case::index_3d_variable(
        r#"
function test4(ref p: array, i: number, j: number, k: number)
    p[i][j][k] := "test4"
fend
arr = [[["foo"]]]
test4(arr, 0, 0, 0)
arr[0][0][0]
        "#,
        "test4".into()
    )]
    #[case::module_public(
        r#"
test(M.p)
M.p
        "#,
        "reference test".into()
    )]
    #[case::module_public_index(
        r#"
test(M.q[0])
M.q[0]
        "#,
        "reference test".into()
    )]
    #[case::module_public_index_variable(
        r#"
test2(M.q, 0)
M.q[0]
        "#,
        "test2".into()
    )]
    #[case::module_public_2d_index(
        r#"
test(M.r[0][0])
M.r[0][0]
        "#,
        "reference test".into()
    )]
    #[case::module_public_2d_index_variable(
        r#"
test3(M.r, 0, 0)
M.r[0][0]
        "#,
        "test3".into()
    )]
    #[case::class_public(
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
        "reference test".into()
    )]
    #[case::test5(
        r#"
function test5(ref r)
    r = "test5"
fend

x = X()
test5(x.y.z[0].p)
x.y.z[0].p
        "#,
        "test5".into()
    )]
    #[case::test6(
        r#"
function test6(ref r: X)
    r.y.z[0].p = "test6"
fend

x = X()
test6(x)
x.y.z[0].p
        "#,
        "test6".into()
    )]
    #[case::gh68(
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
        "gh-68".into()
    )]
    #[case::test8(
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
        "test8".into()
    )]
    #[case::test9_gh67(
        r#"
procedure test9(ref r[])
    r[0] = "gh-67"
fend
a = [0]
test9(a)

a[0]
        "#,
        "gh-67".into()
    )]
    fn test_reference(#[case] input: &str, #[case] expected: Object) {
        let mut e = reference_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }

    fn class_fixture() -> Evaluator {
        let input = r#"
public x = 100
class Test
    dim name
    procedure Test(name: string)
        this.name = name
    fend
    function name()
        result = this.name
    fend
    dim private = function()
        result = "private"
    fend
    function call_private()
        result = private()
    fend
    procedure _Test_
        global.x = 2
    fend
endclass
ins = Test("test1")
        "#;
        eval_env(input)
    }
    #[rstest]
    #[case(
        r#"
        "<#ins>"
        "#,
        "instance of Test".into()
    )]
    #[case(
        r#"
        ins.name()
        "#,
        "test1".into()
    )]
    #[case(
        r#"
        ins.call_private()
        "#,
        "private".into()
    )]
    #[case(
        r#"
        ins = ""
        x
        "#,
        2.into()
    )]
    #[case(
        r#"
        ins1 = Test("hoge")
        ins2 = ins1
        ins2 = NOTHING
        ins1
        "#,
        Object::Nothing
    )]
    fn test_class(#[case] input: &str, #[case] expected: Object) {
        let mut e = class_fixture();
        expect_object_test(Some(&mut e), input, expected);
    }

    #[rstest]
    #[case(
        r#"
        ins.name
        "#,
        UErrorKind::DotOperatorError,
        UErrorMessage::IsPrivateMember("Test".into(), "name".into())
    )]
    #[case(
        r#"
        ins.private()
        "#,
        UErrorKind::DotOperatorError,
        UErrorMessage::IsPrivateMember("Test".into(), "private()".into())
    )]
    fn test_class_err(#[case] input: &str, #[case] expected_kind: UErrorKind, #[case] expected_message: UErrorMessage) {
        let mut e = class_fixture();
        expect_error_test(Some(&mut e), input, expected_kind, expected_message);
    }

    #[test]
    fn test_short_circuit() {
        let definition = r#"
OPTION SHORTCIRCUIT
public called = ""
function t(n)
    called += n
    result = true
fend
function f(n)
    called += n
    result = false
fend
        "#;
        let mut e = eval_env(definition);
        let mut evaluate = move |input: &str| {
            match Parser::new(Lexer::new(input), None, None).parse() {
                Ok(program) => {
                    match e.eval(program, false) {
                        Ok(obj) => match obj {
                            Some(obj) => {
                                match &obj {
                                    Object::Array(arr) => {
                                        let Object::Bool(b) = arr.first().expect("expect array object") else {
                                            panic!("bad result: {arr:?}");
                                        };
                                        let Object::String(s) = arr.get(1).expect("expect string object") else {
                                            panic!("bad result: {arr:?}");
                                        };
                                        (*b, s.to_string())
                                    },
                                    obj => panic!("bad result: {obj}"),
                                }
                            },
                            None => panic!("no object"),
                        },
                        Err(e) => panic!("eval error: {e}"),
                    }
                },
                Err(e) => panic!("parse error: {e:?}"),
            }
        };
        fn t(n: u8, called: &mut String) -> bool {
            *called = format!("{called}{n}");
            true
        }
        fn f(n: u8, called: &mut String) -> bool {
            *called = format!("{called}{n}");
            false
        }
        enum Infix {
            And,
            Or
        }
        impl std::fmt::Display for Infix {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Infix::And => write!(f, "and"),
                    Infix::Or => write!(f, "or"),
                }
            }
        }


        for b1 in [true, false] {
            for b2 in [true, false] {
                for b3 in [true, false] {
                    for i1 in &[Infix::And, Infix::Or] {
                        for i2 in &[Infix::And, Infix::Or] {
                            let uf1 = if b1 {"t(1)"} else {"f(1)"};
                            let uf2 = if b2 {"t(2)"} else {"f(2)"};
                            let uf3 = if b3 {"t(3)"} else {"f(3)"};
                            let input = format!(r#"
called = ""
a = {uf1} {i1} {uf2} {i2} {uf3} ? true : false
[a, called]
                            "#);
                            let mut called = String::new();
                            let f1 = if b1 {t} else {f};
                            let f2 = if b2 {t} else {f};
                            let f3 = if b3 {t} else {f};
                            let a = match (i1, i2) {
                                (Infix::And, Infix::And) => {
                                    f1(1, &mut called) && f2(2, &mut called) && f3(3, &mut called)
                                },
                                (Infix::And, Infix::Or) => {
                                    f1(1, &mut called) && f2(2, &mut called) || f3(3, &mut called)
                                },
                                (Infix::Or, Infix::And) => {
                                    f1(1, &mut called) || f2(2, &mut called) && f3(3, &mut called)
                                },
                                (Infix::Or, Infix::Or) => {
                                    f1(1, &mut called) || f2(2, &mut called) || f3(3, &mut called)
                                },
                            };
                            let (b, s) = evaluate(&input);
                            if a == b && called == s {
                                // ok
                                // println!("[debug] {a}:{s} {b}:{called}");
                            } else {
                                panic!("got {b}:{s} on {input}, but should be {a}:{called}");
                            }
                            let input = format!(r#"
called = ""
a = {uf1} {i1}L {uf2} {i2}L {uf3}
[a, called]
                            "#);
                            let (b, s) = evaluate(&input);
                            if a == b && called == s {
                                // ok
                                // println!("[debug] {a}:{s} {b}:{called}");
                            } else {
                                panic!("got {b}:{s} on {input}, but should be {a}:{called}");
                            }
                        }
                    }
                }
            }
        }

    }

}