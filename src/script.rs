use std::rc::Rc;
use std::cell::RefCell;

use crate::evaluator::builtins::init_builtins;
use crate::evaluator::env::Env;
use crate::evaluator::object::Object;
use crate::evaluator::Evaluator;
use crate::parser::Parser;
use crate::parser::ParseError;
use crate::lexer::Lexer;

pub fn run(script: String) -> Result<(), Vec<ParseError>> {

    let env = Env::from(init_builtins());
    let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));
    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors);
    }
    match evaluator.eval(program) {
        Some(Object::Error(msg)) => eprintln!("evaluator error: {}", msg),
        _ => ()
    }

    Ok(())
}

