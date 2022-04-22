use crate::settings::usettings_singleton;

use std::env;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::fmt;

use chrono::Local;


pub fn init(dir: &PathBuf) {
    let singleton = usettings_singleton(None);
    let u = singleton.0.lock().unwrap();
    match u.options.log_path {
        Some(ref s) => {
            let mut path = PathBuf::from(&s);
            if path.is_dir() {
                path.push("uwscr.log");
            }
            env::set_var("UWSCR_LOG_FILE", path.as_os_str())
        },
        None => {
            let mut path = dir.clone();
            path.push("uwscr");
            path.set_extension("log");
            env::set_var("UWSCR_LOG_FILE", path.to_str().unwrap());
        },
    }
    let lines = u.options.log_lines;
    env::set_var("UWSCR_LOG_LINES", lines.to_string());
    let mut log = u.options.log_file;
    if log > 4 {log = 1};
    env::set_var("UWSCR_LOG_TYPE", log.to_string());
}

pub fn out_log(log: &String, log_type: LogType) {
    if env::var("UWSCR_LOG_TYPE").unwrap_or("1".into()).as_str() == "1" {
        return;
    }
    if log.len() == 0 {
        return;
    }
    let path = match env::var("UWSCR_LOG_FILE") {
        Ok(s) => s,
        Err(_) => return
    };
    let _max_lines = env::var("UWSCR_LOG_LINES").unwrap_or("400".into()).parse::<u32>().unwrap_or(400);
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
    Panic,
}

impl fmt::Display for LogType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogType::Error => write!(f,"[ERROR]"),
            LogType::Print => write!(f,"[PRINT]"),
            LogType::Panic => write!(f,"[PANIC]"),
        }
    }
}