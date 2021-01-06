use std::env;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::prelude::*;
use chrono::Local;


pub fn init(dir: &PathBuf) {
    let mut file_path = dir.clone();
    file_path.push("uwscr");
    file_path.set_extension("log");
    env::set_var("UWSCR_LOG_FILE", file_path.to_str().unwrap());
}

pub fn out_log(log: &String) {
    let path = match env::var("UWSCR_LOG_FILE") {
        Ok(s) => s,
        Err(_) => return
    };
    let mut file = OpenOptions::new().create(true).write(true).append(true).open(path).unwrap();
    writeln!(file, "{}  {}", Local::now().format("%Y-%m-%d %H:%M:%S"), log).expect("Unable to write log file");
}