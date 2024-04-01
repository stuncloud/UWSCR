use std::path::PathBuf;
use std::env;

use evaluator::environment::Environment;
use evaluator::Evaluator;
use evaluator::builtins::get_builtin_string_names;
use parser::*;
use parser::lexer::Lexer;
use util::com::Com;
// use util::winapi::{
//     WString, PcwstrExt,
// };
use util::error::UWSCRErrorTitle;
use util::winapi::{show_message, get_absolute_path};

pub struct ScriptError(pub UWSCRErrorTitle, pub Vec<String>);

pub fn run(script: String, script_path: PathBuf, params: Vec<String>, ast: Option<(bool, bool)>) -> Result<(), ScriptError> {
    let exe_full_path = env::current_exe()
        .map_err(|e| ScriptError(UWSCRErrorTitle::InitializeError, vec![e.to_string()]))?;
    let uwscr_dir = exe_full_path.parent()
        .ok_or(ScriptError(UWSCRErrorTitle::InitializeError, vec!["unable to get uwscr directory".into()]))?;
    env::set_var("GET_UWSC_DIR", &uwscr_dir.as_os_str());

    let script_full_path = get_absolute_path(&script_path);
    let script_dir = script_full_path.parent()
        .ok_or(ScriptError(UWSCRErrorTitle::InitializeError, vec!["unable to get script directory".into()]))?;
    env::set_var("GET_SCRIPT_DIR", &script_dir.as_os_str());

    if let Some(name) = script_path.file_name() {
        env::set_var("GET_UWSC_NAME", name);
        // デフォルトダイアログタイトルを設定
        env::set_var("UWSCR_DEFAULT_TITLE", &format!("UWSCR - {}", name.to_string_lossy()))
    }
    match env::set_current_dir(&script_dir) {
        Err(_)=> return Err(ScriptError(
            UWSCRErrorTitle::InitializeError,
            vec!["unable to set current directory".into()]
        )),
        _ => {}
    };

    let names = get_builtin_string_names();
    let parser = Parser::new(Lexer::new(&script), Some(script_dir.to_path_buf()), Some(names));

    let (program, errors) = parser.parse_to_program_and_errors();
    if let Some((_continue, pretty)) = ast {
        let message = if pretty {
            format!("{program:#?}")
        } else {
            format!("{program:?}")
        };
        show_message(&message, "uwscr --ast", false);
        if ! _continue {
            return Ok(());
        }
    }

    (errors.len() == 0).then(|| ())
        .ok_or(ScriptError(UWSCRErrorTitle::StatementError, errors.into_iter().map(|e| e.to_string()).collect()))?;

    // このスレッドでのCOMを有効化
    let com = match Com::init() {
        Ok(com) => com,
        Err(e) => {
            return Err(ScriptError(
                UWSCRErrorTitle::InitializeError,
                vec![e.to_string()]
            ));
        },
    };

    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(env);
    if let Err(e) = evaluator.eval(program, true) {
        #[cfg(debug_assertions)] println!("\u{001b}[90m[script::run] Evaluator Error: {:#?}\u{001b}[0m", &e);
        let line = &e.get_line();
        return Err(ScriptError(
            UWSCRErrorTitle::RuntimeError,
            vec![line.to_string(), e.to_string()]
        ))
    }
    com.uninit();

    Ok(())
}

pub fn run_code(code: String) -> Result<(), Vec<String>> {
    let parser = Parser::new(Lexer::new(&code), None, None);
    let program = parser.parse()
        .map_err(|errors| errors.into_iter().map(|err| err.to_string()).collect::<Vec<_>>() )?;

    // このスレッドでのCOMを有効化
    let com = match Com::init() {
        Ok(com) => com,
        Err(e) => {
            return Err(vec![e.to_string()]);
        },
    };

    let env = Environment::new(vec![]);
    let mut evaluator = Evaluator::new(env);
    if let Err(e) = evaluator.eval(program, true) {
        let line = &e.get_line();
        return Err(vec![line.to_string(), e.to_string()])
    }
    com.uninit();
    Ok(())
}
