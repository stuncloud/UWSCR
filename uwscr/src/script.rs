use std::path::{Path, PathBuf};
use std::env;

use evaluator::environment::Environment;
use evaluator::Evaluator;
use evaluator::builtins::get_builtin_names;
use parser::*;
use parser::lexer::Lexer;
use util::com::Com;
// use util::winapi::{
//     WString, PcwstrExt,
// };
use util::error::UWSCRErrorTitle;

use windows::{
    core::{PWSTR, HSTRING},
    Win32::{
        Foundation::MAX_PATH,
        Storage::FileSystem::GetFullPathNameW
    }
};

pub struct ScriptError(pub UWSCRErrorTitle, pub Vec<String>);

pub fn run(script: String, exe_path: &str, script_path: &str, params: Vec<String>) -> Result<(), ScriptError> {
    let uwscr_dir = match get_parent_full_path(exe_path) {
        Ok(s) => s,
        Err(_) => return Err(ScriptError(
            UWSCRErrorTitle::InitializeError,
            vec!["unable to get uwscr path".into()]
        ))
    };
    let script_dir = match get_parent_full_path(script_path) {
        Ok(s) => s,
        Err(_) => return Err(ScriptError(
            UWSCRErrorTitle::InitializeError,
            vec!["unable to get script path".into()]
        ))
    };
    env::set_var("GET_UWSC_DIR", &uwscr_dir);
    env::set_var("GET_SCRIPT_DIR", &script_dir);
    match get_script_name(script_path) {
        Some(ref s) => {
            env::set_var("GET_UWSC_NAME", s);
            // デフォルトダイアログタイトルを設定
            env::set_var("UWSCR_DEFAULT_TITLE", &format!("UWSCR - {}", s))
        },
        None => {}
    }
    match env::set_current_dir(&script_dir) {
        Err(_)=> return Err(ScriptError(
            UWSCRErrorTitle::InitializeError,
            vec!["unable to set current directory".into()]
        )),
        _ => {}
    };
    // let visible = ! attach_console();

    let names = get_builtin_names();
    let parser = Parser::new(Lexer::new(&script), Some(script_dir), Some(names));
    let program = parser.parse()
        .map_err(|errors| {
            ScriptError(
                UWSCRErrorTitle::StatementError,
                errors.into_iter().map(|e| e.to_string() ).collect()
            )
        })?;

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

pub fn out_ast(script: String, path: &String) -> Result<(String, Option<String>), String> {
    let script_dir = match get_parent_full_path(path) {
        Ok(s) => s,
        Err(_) => {
            return Err("unable to get script path".to_string());
        },
    };
    env::set_var("GET_SCRIPT_DIR", &script_dir);
    match env::set_current_dir(&script_dir) {
        Err(_)=> {
            return Err("unable to set current directory".to_string());
        },
        _ => {}
    };
    if let Some(name) = get_script_name(path) {
        env::set_var("GET_UWSC_NAME", name);
    }

    let names = get_builtin_names();
    let parser = Parser::new(Lexer::new(&script), Some(script_dir), Some(names));
    let (program, errors) = parser.parse_to_program_and_errors();

    let err = if errors.len() > 0 {
        let emsg = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\r\n");
        Some(format!("got {} parse error{}\r\n{}", errors.len(), if errors.len()>1 {"s"} else {""}, emsg))
    } else {None};

    let ast = format!("Global: {:#?}\nScript: {:#?}", program.global, program.script);
    Ok((ast, err))
}

pub fn get_parent_full_path(path: &str) -> Result<PathBuf, String> {
    let mut buffer = [0; MAX_PATH as usize];
    let lpfilename = HSTRING::from(path);
    let mut filepart = PWSTR::null();
    unsafe {
        GetFullPathNameW(&lpfilename, Some(&mut buffer), Some(&mut filepart));
    }
    let full_path = String::from_utf16_lossy(&buffer);
    Ok(Path::new(full_path.as_str()).parent().unwrap().to_owned())
}

pub fn get_script_name(path: &str) -> Option<String> {
    Path::new(path).file_name().unwrap().to_os_string().into_string().ok()
}