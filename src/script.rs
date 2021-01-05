use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::rc::Rc;
use std::cell::RefCell;
use std::env;

use crate::evaluator::environment::Environment;
use crate::evaluator::Evaluator;
use crate::parser::*;
use crate::lexer::{Lexer, Position};
use crate::winapi_util::{buffer_to_string, to_wide_string};

use winapi::{
    um::fileapi::{GetFullPathNameW, },
    shared::minwindef::MAX_PATH,
};

pub fn run(script: String, mut args: Vec<String>) -> Result<(), Vec<ParseError>> {
    let params = args.drain(2..).collect();
    let uwscr_dir = match get_parent_full_path(&args[0]) {
        Ok(s) => s,
        Err(_) => {
            return Err(vec![
                ParseError::new(
                    ParseErrorKind::InvalidFilePath,
                    "unable to get uwscr path",
                    Position {row: 0, column: 0}
                )
            ]);
        }
    };
    let script_dir = match get_parent_full_path(&args[1]) {
        Ok(s) => s,
        Err(_) => {
            return Err(vec![
                ParseError::new(
                    ParseErrorKind::InvalidFilePath,
                    "unable to get script path",
                    Position {row: 0, column: 0}
                )
            ]);
        }
    };
    env::set_var("GET_UWSC_DIR", uwscr_dir.to_str().unwrap());
    env::set_var("GET_SCRIPT_DIR", script_dir.to_str().unwrap());
    env::set_var("GET_UWSC_NAME", get_script_name(&args[1]));
    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));
    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors);
    }
    match evaluator.eval(program) {
        Err(e) => eprintln!("{}", e),
        _ => ()
    }

    Ok(())
}

pub fn out_ast(script: String) {

    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        eprintln!("got {} parse error{}", errors.len(), if errors.len()>1 {"s"} else {""});
        for err in errors {
            eprintln!("{}", err);
        }
        eprintln!("");
    } else {
        for statement in program {
            println!("{:?}", statement);
        }
    }
}

fn get_parent_full_path(path: &String) -> Result<PathBuf, String> {
    let mut buffer = [0; MAX_PATH];
    let file = to_wide_string(path);
    unsafe {
        GetFullPathNameW(file.as_ptr(), buffer.len() as u32, buffer.as_mut_ptr(), null_mut());
    }
    let full_path = buffer_to_string(&buffer)?;
    Ok(Path::new(full_path.as_str()).parent().unwrap().to_owned())
}

fn get_script_name(path: &String) -> String {
    Path::new(path.as_str()).file_name().unwrap().to_os_string().into_string().unwrap_or("".into())
}