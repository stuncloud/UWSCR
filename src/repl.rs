use std::io::{stdin, stdout, Write};
use std::env;

use crate::evaluator::environment::Environment;
use crate::evaluator::object::Object;
use crate::evaluator::Evaluator;
use crate::parser::Parser;
use crate::error::parser::ParseErrorKind;
use crate::lexer::Lexer;
use crate::script::{get_parent_full_path, get_script_name};
use crate::com::Com;

pub fn run(script: Option<String>, exe_path: String, script_path: Option<String>) {
    match get_parent_full_path(&exe_path) {
        Ok(s) => {
            env::set_var("GET_UWSC_DIR", s.to_str().unwrap());
        },
        Err(e) => {
            eprintln!("failed to get uwscr.exe path ({})", e);
            return;
        }
    };
    if script_path.is_some() {
        match get_script_name(&script_path.clone().unwrap()) {
            Some(s) =>{
                env::set_var("GET_UWSC_NAME", &s);
                env::set_var("UWSCR_DEFAULT_TITLE", &format!("UWSCR - {}", &s))
            },
            None => {
                env::set_var("GET_UWSC_NAME", "");
                env::set_var("UWSCR_DEFAULT_TITLE", &format!("UWSCR - REPL"))
            }
        }
        match get_parent_full_path(&script_path.unwrap()) {
            Ok(s) => {
                env::set_var("GET_SCRIPT_DIR", s.to_str().unwrap());
            },
            Err(e) => {
                eprintln!("failed to get script path ({})", e);
                return;
            },
        };
    }

    // このスレッドでのCOMを有効化
    let _com = match Com::init() {
        Ok(com) => com,
        Err(e) => {
            eprintln!("failed to initialize COM: {e}");
            return;
        },
    };

    let env = Environment::new(vec![]);
    let mut evaluator = Evaluator::new(env);
    if script.is_some() {
        println!("loading script...");
        let mut parser = Parser::new(Lexer::new(&script.unwrap()));
        let program = parser.parse();
        let errors = parser.get_errors();
        if errors.len() > 0 {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        } else {
            match evaluator.eval(program, false) {
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                },
                _ => ()
            }
            println!("script loaded.");
        }
    }
    let mut require_newline = false;
    let mut multiline = String::new();
    loop {
        let mut buf = String::new();
        if require_newline {
            print!("       ");
            require_newline = false;
        } else {
            multiline = "".to_string();
            print!("uwscr> ");
        }
        stdout().flush().unwrap();
        stdin().read_line(&mut buf).ok();

        let input = if multiline.len() > 0 {
            format!("{}{}", multiline, buf)
        } else {
            buf
        };
        let mut parser = Parser::new(Lexer::new(&input));
        let program = parser.parse();
        let errors = parser.get_errors();
        if errors.len() > 0 {
            for error in errors {
                match error.clone().get_kind() {
                    ParseErrorKind::InvalidBlockEnd(_,_) => {
                        multiline = input.clone();
                        require_newline = true;
                        continue;
                    },
                    _ => {
                        eprintln!("{}", error);
                        require_newline = false;
                        break;
                    }
                }
            }
        } else {
            match evaluator.eval(program, false) {
                Ok(Some(Object::Exit)) => {
                    evaluator.clear();
                    println!("bye!");
                    break;
                },
                Ok(Some(o)) => println!("{}", o),
                Ok(None) => (),
                Err(e) => println!("{}", e),
            }
        }
    }
}

