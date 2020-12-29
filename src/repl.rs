use std::rc::Rc;
use std::cell::RefCell;
use std::io::{stdin, stdout, Write};

use crate::evaluator::environment::Environment;
use crate::evaluator::object::Object;
use crate::evaluator::Evaluator;
use crate::parser::Parser;
use crate::parser::ParseErrorKind;
use crate::lexer::Lexer;

pub fn run(script: Option<String>) {
    let env = Environment::new(vec![]);
    let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));
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
            match evaluator.eval(program) {
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
                    ParseErrorKind::BlockNotClosedCorrectly => {
                        multiline = input.clone();
                        require_newline = true;
                        continue;
                    },
                    _ => eprintln!("{}", error)
                }
            }
        } else {
            match evaluator.eval(program) {
                Ok(Some(Object::Exit)) => {
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

