use crate::ast::Program;
use crate::parser::Parser;
use crate::lexer::Lexer;

use std::io::Read;
use std::fs;
use std::path::PathBuf;

pub fn serialize(script: String) -> Option<Vec<u8>> {
    let parser = Parser::new(Lexer::new(&script), None, true);
    match parser.parse() {
        Ok(program) => {
            bincode::serialize(&program).ok()
        },
        Err(errors) => {
            eprintln!("got {} parse error{}", errors.len(), if errors.len()>1 {"s"} else {""});
            for err in errors {
                eprintln!("{}", err);
            }
            eprintln!("");
            None
        },
    }
}

pub fn save(path: PathBuf, bin: Vec<u8>) {
    match fs::write(path, bin) {
        Ok(_) => {},
        Err(e) => eprintln!("{}", e)
    }
}

pub fn load(path: &PathBuf) -> Result<Vec<u8>, std::io::Error> {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return Err(e);
        }
    };
    let mut vec = Vec::new();
    match file.read_to_end(&mut vec) {
        Ok(_) => {},
        Err(e) => {
            return Err(e);
        }
    }
    Ok(vec)
}

pub fn deserialize(bin: Vec<u8>) -> Result<Program, bincode::Error> {
    bincode::deserialize(&bin)
}