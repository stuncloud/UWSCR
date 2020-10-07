use std::path::PathBuf;
use std::error::Error;
use std::fs;
use std::rc::Rc;
use std::cell::RefCell;


use crate::evaluator::builtins::init_builtins;
use crate::evaluator::env::Env;
use crate::evaluator::object::Object;
use crate::evaluator::Evaluator;
use crate::parser::Parser;
use crate::lexer::Lexer;

pub fn run(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let script = fs::read_to_string(path)?;

    let env = Env::from(init_builtins());
    let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));
    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        for err in errors {
            eprintln!("parse error: {}", err);
        }
    }
    match evaluator.eval(program) {
        Some(Object::Error(msg)) => eprintln!("evaluator error: {}", msg),
        _ => ()
    }

    Ok(())
}

