use std::rc::Rc;
use std::cell::RefCell;

use crate::evaluator::environment::Environment;
use crate::evaluator::object::Object;
use crate::evaluator::Evaluator;
use crate::parser::Parser;
use crate::parser::ParseError;
use crate::lexer::Lexer;

pub fn run(script: String) -> Result<(), Vec<ParseError>> {

    let env = Environment::new();
    let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));
    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        return Err(errors);
    }
    match evaluator.eval(program) {
        Some(Object::Error(msg)) => eprintln!("evaluator error: {}", msg),
        Some(Object::UError(err)) => eprintln!("{}", err),
        _ => ()
    }

    Ok(())
}

pub fn out_ast(script: String) {

    let mut parser = Parser::new(Lexer::new(&script));
    let program = parser.parse();
    let errors = parser.get_errors();
    if errors.len() > 0 {
        eprintln!("got {} parse error[s]", errors.len());
        for err in errors {
            eprintln!("{}", err);
        }
        eprintln!("");
    }
    for statement in program {
        println!("{:?}", statement);
    }
}