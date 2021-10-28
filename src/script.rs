use std::path::{Path, PathBuf};
use std::env;

use crate::evaluator::environment::Environment;
use crate::evaluator::Evaluator;
use crate::parser::*;
use crate::lexer::Lexer;
use crate::settings::load_settings;
use crate::winapi::{
    to_wide_string,
};
use windows::{
    Win32::{
        Foundation::{
            MAX_PATH, PWSTR,
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
        Ok(()) => {},
        Err(e) => eprintln!("failed to load settings: {}", e),
    }

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
    env::set_var("GET_UWSC_DIR", uwscr_dir.to_str().unwrap());
    env::set_var("GET_SCRIPT_DIR", script_dir.to_str().unwrap());
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
    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors.into_iter().map(|e| format!("{}", e)).collect());
    }

    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(env);
    if let Err(e) = evaluator.eval(program) {
        let line = &e.get_line();
        return Err(vec![line.to_string(), e.to_string()])
    }
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

pub fn out_ast(script: String, path: &String, force: bool) {

    let script_dir = match get_parent_full_path(path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("unable to get script path");
            return;
        },
    };
    logging::init(&script_dir);
    match env::set_current_dir(&script_dir) {
        Err(_)=> {
            eprintln!("unable to set current directory");
            return;
        },
        _ => {}
    };

    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        eprintln!("got {} parse error{}", errors.len(), if errors.len()>1 {"s"} else {""});
        for err in errors {
            eprintln!("{}", err);
        }
        eprintln!("");
        if ! force {
            return;
        }
    }
    for statement in program.0 {
        println!("{:?}", statement);
    }
}

pub fn get_parent_full_path(path: &String) -> Result<PathBuf, String> {
    let mut buffer = [0; MAX_PATH as usize];
    let mut file = to_wide_string(path);
    let mut filepart = PWSTR::default();
    unsafe {
        GetFullPathNameW(PWSTR(file.as_mut_ptr()), buffer.len() as u32, PWSTR(buffer.as_mut_ptr()), &mut filepart);
    }
    let full_path = String::from_utf16_lossy(&buffer);
    Ok(Path::new(full_path.as_str()).parent().unwrap().to_owned())
}

pub fn get_script_name(path: &String) -> Option<String> {
    Path::new(path.as_str()).file_name().unwrap().to_os_string().into_string().ok()
}