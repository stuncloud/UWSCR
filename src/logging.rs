use std::env;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::fmt;

use chrono::Local;


pub fn init(dir: &PathBuf) {
    let mut file_path = dir.clone();
    file_path.push("uwscr");
    file_path.set_extension("log");
    env::set_var("UWSCR_LOG_FILE", file_path.to_str().unwrap());
}

pub fn out_log(log: &String, log_type: LogType) {
    let path = match env::var("UWSCR_LOG_FILE") {
        Ok(s) => s,
        Err(_) => return
    };
    let mut file = OpenOptions::new().create(true).write(true).append(true).open(path).unwrap();
    if log_type == LogType::Print {
        let lines = log.lines().collect::<Vec<&str>>();
        write!(file, "{} {}  {}\r\n", Local::now().format("%Y-%m-%d %H:%M:%S"), log_type, lines[0]).expect("Unable to write log file");
        for i in 1..(lines.len()) {
            write!(file, "                    {}  {}\r\n", log_type, lines[i]).expect("Unable to write log file");
        }
    } else {
        write!(file, "{} {}  {}\r\n", Local::now().format("%Y-%m-%d %H:%M:%S"), log_type, log).expect("Unable to write log file");
    }
}

#[derive(Debug, PartialEq)]
pub enum LogType {
    Error,
    Print,
}

impl fmt::Display for LogType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogType::Error => write!(f,"[ERROR]"),
            LogType::Print => write!(f,"[PRINT]"),
        }
    }
}