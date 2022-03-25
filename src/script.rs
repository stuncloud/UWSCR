use std::path::{Path, PathBuf};
use std::env;

use crate::evaluator::environment::Environment;
use crate::evaluator::Evaluator;
use crate::parser::*;
use crate::lexer::Lexer;
use crate::settings::{load_settings};
use crate::winapi::{
    to_wide_string, attach_console, free_console,
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

pub fn run(script: String, mut args: Vec<String>) -> Result<(), Vec<String>> {
    // 設定ファイルを読み込む
    // 失敗したらデフォルト設定が適用される
    match load_settings() {
        Ok(_) => {},
        Err(e) => eprintln!("failed to load settings: {}", e),
    };

    let params = args.drain(2..).collect();
    let uwscr_dir = match get_parent_full_path(&args[0]) {
        Ok(s) => s,
        Err(_) => return Err(vec![
            "unable to get uwscr path".into()
        ])
    };
    let script_dir = match get_parent_full_path(&args[1]) {
        Ok(s) => s,
        Err(_) => return Err(vec![
            "unable to get script path".into()
        ])
    };
    logging::init(&script_dir);
    env::set_var("GET_UWSC_DIR", &uwscr_dir);
    env::set_var("GET_SCRIPT_DIR", &script_dir);
    match get_script_name(&args[1]) {
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
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors.into_iter().map(|e| format!("{}", e)).collect());
    }

    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(env);
    Evaluator::start_logprint_win(visible);
    if let Err(e) = evaluator.eval(program) {
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
    if let Err(e) = evaluator.eval(program) {
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
    let program = parser.parse();
    let errors = parser.get_errors();
    let err = if errors.len() > 0 {
        let emsg = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\r\n");
        Some(format!("got {} parse error{}\r\n{}", errors.len(), if errors.len()>1 {"s"} else {""}, emsg))
    } else {None};
    let ast = program.0.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join("\r\n");
    Ok((ast, err))
}

pub fn get_parent_full_path(path: &String) -> Result<PathBuf, String> {
    let mut buffer = [0; MAX_PATH as usize];
    let file = to_wide_string(path);
    let mut filepart = PWSTR::default();
    unsafe {
        GetFullPathNameW(PCWSTR(file.as_ptr()), &mut buffer, &mut filepart);
    }
    let full_path = String::from_utf16_lossy(&buffer);
    Ok(Path::new(full_path.as_str()).parent().unwrap().to_owned())
}

pub fn get_script_name(path: &String) -> Option<String> {
    Path::new(path.as_str()).file_name().unwrap().to_os_string().into_string().ok()
}