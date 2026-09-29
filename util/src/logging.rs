use crate::settings::USETTINGS;

use std::env;
use std::path::{PathBuf, Path};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{BufReader, SeekFrom};
use std::fmt;

use chrono::Local;


pub fn init(dir: &Path) {
    let u = USETTINGS.lock().unwrap();
    let path = match u.options.log_path {
        Some(ref s) => {
            let mut path = PathBuf::from(&s);
            if path.is_dir() {
                path.push("uwscr.log");
            }
            unsafe { env::set_var("UWSCR_LOG_FILE", path.as_os_str()); }
            path
        },
        None => {
            let mut path = dir.to_path_buf();
            path.push("uwscr");
            path.set_extension("log");
            unsafe { env::set_var("UWSCR_LOG_FILE", path.to_str().unwrap()); }
            path
        },
    };
    let lines = u.options.log_lines;
    unsafe { env::set_var("UWSCR_LOG_LINES", lines.to_string()); }
    let mut log = u.options.log_file;
    if log > 4 {log = 1};
    if log == 4 && path.exists() {
        let _dummy = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path);
    }
    if log != 1 {
        unsafe { env::set_var("UWSCR_LOG_TYPE", log.to_string()); }
    }
}

pub fn out_log(log: &String, log_type: LogType) {
    if let Err(e) = out_log_inner(log, log_type) {
        eprintln!("Error occured on logging: {e}");
    }
}
pub fn out_log_inner(log: &String, log_type: LogType) -> Result<(), std::io::Error>{
    if log.is_empty() {
        return Ok(());
    }
    let log_option = env::var("UWSCR_LOG_TYPE").ok().and_then(|t| t.parse::<u8>().ok());
    if log_option.is_none() && log_type != LogType::Panic {
        return Ok(());
    }
    let Ok(path) = env::var("UWSCR_LOG_FILE") else {
        return Ok(());
    };
    let no_date_time = log_option.is_some_and(|n| n == 2);

    let max_lines = env::var("UWSCR_LOG_LINES").ok().and_then(|l| l.parse::<usize>().ok()).unwrap_or(400);

    {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        match log_type {
            LogType::Error |
            LogType::Print |
            LogType::Info => {
                if no_date_time {
                    let padding = format!("{log_type}").len();
                    for (i, line) in log.lines().enumerate() {
                        if i == 0 {
                            write!(file, "{log_type}  {line}\r\n")?;
                        } else {
                            write!(file, "{:>1$}  {line}\r\n", " ", padding)?;
                        }
                    }
                } else {
                    let date_time = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let padding = format!("{date_time} {log_type}").len();
                    for (i, line) in log.lines().enumerate() {
                        if i == 0 {
                            write!(file, "{date_time} {log_type}  {line}\r\n")?;
                        } else {
                            write!(file, "{:>1$}  {line}\r\n", " ", padding)?;
                        }
                    }
                }
            },
            LogType::Panic => {
                if no_date_time {
                    write!(file, "{log_type}  {log}\r\n")?;
                } else {
                    let date_time = Local::now().format("%Y-%m-%d %H:%M:%S");
                    write!(file, "{date_time} {log_type}  {log}\r\n")?;
                }
            },
        }
    }

    // let path = env::var("UWSCR_LOG_FILE").unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)?;

    let rows = BufReader::new(&file).lines().count();

    if rows > max_lines {
        file.seek(SeekFrom::Start(0))?;
        let lines = BufReader::new(file).lines();
        let n = rows - max_lines;
        let new = lines.enumerate()
            .filter_map(|(i, line), | (i >= n).then_some(line.unwrap_or_default()))
            .fold(String::new(), |a, b| a + "\r\n" + &b);

        let mut file = OpenOptions::new().write(true).truncate(true).open(path)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(new.as_bytes())?;
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum LogType {
    Error,
    Print,
    Panic,
    Info
}

impl fmt::Display for LogType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogType::Error => write!(f,"[ERROR]"),
            LogType::Print => write!(f,"[PRINT]"),
            LogType::Panic => write!(f,"[PANIC]"),
            LogType::Info  => write!(f,"[INFO ]"),
        }
    }
}