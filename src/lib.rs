pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod evaluator;
pub mod script;
pub mod repl;
pub mod logging;
pub mod serializer;
// pub mod settings
pub mod winapi;


use encoding_rs::{UTF_8, SHIFT_JIS};
use regex::Regex;
use std::path::PathBuf;
use std::fs;


pub fn get_script(path: &PathBuf) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let re = Regex::new("(\r\n|\r|\n)").unwrap();
    get_utf8(&bytes).map(|s| re.replace_all(s.as_str(), "\r\n").to_string())
}

fn get_utf8(bytes: &Vec<u8>) -> std::io::Result<String> {
    let (cow, _, err) = UTF_8.decode(bytes);
    if ! err {
        return Ok(cow.to_string());
    } else {
        let (cow, _, err) = SHIFT_JIS.decode(bytes);
        if ! err {
            return Ok(cow.to_string());
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown encoding"))
}
