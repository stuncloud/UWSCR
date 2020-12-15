use std::path::PathBuf;
use std::fs;
use std::env;

use encoding_rs::{UTF_8, SHIFT_JIS};
use regex::Regex;

use uwscr::script;
use uwscr::repl;
use uwscr::evaluator::builtins::system_controls::shell_execute;


fn main() {
    let args = Args::new();
    match args.get() {
        Ok(m) => match m {
            Mode::Help => args.help(None),
            Mode::Version => args.version(),
            Mode::OnlineHelp => {
                shell_execute("https://github.com/stuncloud/UWSCR/wiki".into(), None);
            },
            Mode::Script(p) => {
                match get_script(p) {
                    Ok(s) => match script::run(s) {
                        Ok(_) => {},
                        Err(errors) => {
                            eprintln!("parser had {} error[s]", errors.len());
                            for err in errors {
                                eprintln!("{}", err);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e)
                    }
                }
            },
            Mode::Repl(p) => {
                if p.is_some() {
                    match get_script(p.unwrap()) {
                        Ok(s) => repl::run(Some(s)),
                        Err(e) => {
                            eprintln!("{}", e)
                        }
                    }
                } else {
                    repl::run(None)
                }
            },
            Mode::Ast(p) => {
                match get_script(p) {
                    Ok(s) => script::out_ast(s),
                    Err(e) => {
                        eprintln!("{}", e)
                    }
                }
            },
            Mode::Server(_p) => {
                println!("Language serverは未実装です");
            },
        },
        Err(err) => args.help(Some(err.as_str()))
    }
}


#[derive(Debug)]
struct Args {
    args: Vec<String>,
    version: String
}

impl Args {
    fn new() -> Self {
        Args {
            args: env::args().collect(),
            version: env!("CARGO_PKG_VERSION").to_owned()
        }
    }

    fn get(&self) -> Result<Mode, String> {
        if self.args.len() < 2 {
            return Ok(Mode::Repl(None))
        }
        match self.args[1].to_ascii_lowercase().as_str() {
            "-h"| "--help" => Ok(Mode::Help),
            "-v"| "--version" => Ok(Mode::Version),
            "-o"| "--online-help" => Ok(Mode::OnlineHelp),
            "-r" | "--repl" => self.get_path().map(|p| Mode::Repl(p)),
            "-a" | "--ast" => match self.get_path() {
                Ok(Some(p)) => Ok(Mode::Ast(p)),
                Ok(None) => Err("FILE is required".to_string()),
                Err(e) => Err(e)
            },
            "-s" | "--server" => self.get_port().map(|p| Mode::Server(p)),
            _ => {
                Ok(Mode::Script(PathBuf::from(self.args[1].clone())))
            },
        }
    }

    fn get_path(&self) -> Result<Option<PathBuf>, String> {
        if self.args.len() > 2 {
            let path = PathBuf::from(self.args[2].clone());
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    fn get_port(&self) -> Result<Option<u16>, String> {
        if self.args.len() > 2 {
            let port = match self.args[2].parse::<u16>() {
                Ok(p) => p,
                Err(e) => return Err(format!("{}", e))
            };
            Ok(Some(port))
        } else {
            Ok(None)
        }
    }

    pub fn help(&self, err: Option<&str>) {
        if err.is_some() {
            println!("error: {}", err.unwrap());
            println!("");
        } else {
            println!("uwscr {}", self.version);
            println!("");
        }
        println!("Usage:");
        println!("  uwscr FILE                   : スクリプトの実行");
        println!("  uwscr [(-r|--repl) [FILE]]   : Replを起動 (スクリプトを指定するとそれを実行してから起動)");
        println!("  uwscr (-a|--ast) FILE        : スクリプトの構文木を出力");
        // println!("  uwscr (-s|--server) [PORT]   : Language Serverとして起動、デフォルトポートはxxx");
        println!("  uwscr (-h|--help)            : このヘルプを表示");
        println!("  uwscr (-v|--version)         : UWSCRのバージョンを表示");
        println!("  uwscr (-o|--online-help)     : オンラインヘルプを表示");
    }

    pub fn version(&self) {
        println!("uwscr {}", self.version);
    }
}

enum Mode {
    Script(PathBuf),
    Repl(Option<PathBuf>),
    Ast(PathBuf),
    Server(Option<u16>),
    Help,
    Version,
    OnlineHelp,
}

fn get_script(path: PathBuf) -> Result<String, String> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return Err(format!("{}", e))
    };
    let re = Regex::new("(\r\n|\r|\n)").unwrap();
    get_utf8(&bytes).map(|s| re.replace_all(s.as_str(), "\r\n").to_string())
}

fn get_utf8(bytes: &Vec<u8>) -> Result<String, String> {
    let (cow, _, err) = UTF_8.decode(bytes);
    if ! err {
        return Ok(cow.to_string());
    } else {
        let (cow, _, err) = SHIFT_JIS.decode(bytes);
        if ! err {
            return Ok(cow.to_string());
        }
    }
    Err("unsupported encoding".into())
}