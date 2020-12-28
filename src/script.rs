use std::rc::Rc;
use std::cell::RefCell;

use crate::evaluator::environment::Environment;
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