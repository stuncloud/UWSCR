use std::path::{Path, PathBuf};
use std::env;

use crate::evaluator::environment::Environment;
use crate::evaluator::Evaluator;
use crate::parser::*;
use crate::lexer::Lexer;
use crate::winapi::{
    to_wide_string, attach_console, free_console,
    WString, PcwstrExt,
};
use windows::{
    core::{PWSTR, PCWSTR},
    Win32::{
        Foundation::{
            MAX_PATH,
        },
        Storage::{
            FileSystem::{
                GetFullPathNameW,
            }
        }
    }
};
use crate::logging;

pub fn run(script: String, exe_path: &str, script_path: &str, params: Vec<String>) -> Result<(), Vec<String>> {
    let uwscr_dir = match get_parent_full_path(exe_path) {
        Ok(s) => s,
        Err(_) => return Err(vec![
            "unable to get uwscr path".into()
        ])
    };
    let script_dir = match get_parent_full_path(script_path) {
        Ok(s) => s,
        Err(_) => return Err(vec![
            "unable to get script path".into()
        ])
    };
    logging::init(&script_dir);
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
        Err(_)=> return Err(vec![
            "unable to set current directory".into()
        ]),
        _ => {}
    };
    let visible = ! attach_console();

    let mut parser = Parser::new(Lexer::new(&script));
    parser.set_script_dir(script_dir);
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors.into_iter().map(|e| format!("{}", e)).collect());
    }

    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(env);
    Evaluator::start_logprint_win(visible);
    if let Err(e) = evaluator.eval(program, true) {
        #[cfg(debug_assertions)] println!("\u{001b}[90m[script::run] Evaluator Error: {:#?}\u{001b}[0m", &e);
        let line = &e.get_line();
        return Err(vec![line.to_string(), e.to_string()])
    }
    Evaluator::stop_logprint_win();
    free_console();
    Ok(())
}

pub fn run_code(code: String) -> Result<(), Vec<String>> {
    let mut parser = Parser::new(Lexer::new(&code));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors.into_iter().map(|e| format!("{}", e)).collect());
    }

    let env = Environment::new(vec![]);
    let mut evaluator = Evaluator::new(env);
    if let Err(e) = evaluator.eval(program, true) {
        let line = &e.get_line();
        return Err(vec![line.to_string(), e.to_string()])
    }
    Ok(())
}

pub fn out_ast(script: String, path: &String) -> Result<(String, Option<String>), String> {
    let script_dir = match get_parent_full_path(path) {
        Ok(s) => s,
        Err(_) => {
            return Err("unable to get script path".to_string());
        },
    };
    logging::init(&script_dir);
    match env::set_current_dir(&script_dir) {
        Err(_)=> {
            return Err("unable to set current directory".to_string());
        },
        _ => {}
    };

    let mut parser = Parser::new(Lexer::new(&script));
    parser.set_script_dir(script_dir);
    let program = parser.parse();
    let errors = parser.get_errors();
    let err = if errors.len() > 0 {
        let emsg = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\r\n");
        Some(format!("got {} parse error{}\r\n{}", errors.len(), if errors.len()>1 {"s"} else {""}, emsg))
    } else {None};
    let ast = program.0.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join("\r\n");
    Ok((ast, err))
}

pub fn get_parent_full_path(path: &str) -> Result<PathBuf, String> {
    let mut buffer = [0; MAX_PATH as usize];
    let lpfilename = path.to_wide_null_terminated().to_pcwstr();
    let mut filepart = PWSTR::null();
    unsafe {
        GetFullPathNameW(lpfilename, &mut buffer, &mut filepart);
    }
    let full_path = String::from_utf16_lossy(&buffer);
    Ok(Path::new(full_path.as_str()).parent().unwrap().to_owned())
}

pub fn get_script_name(path: &str) -> Option<String> {
    Path::new(path).file_name().unwrap().to_os_string().into_string().ok()
}