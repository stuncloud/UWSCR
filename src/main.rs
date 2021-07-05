use std::path::PathBuf;
use std::env;

use uwscr::script;
use uwscr::repl;
use uwscr::evaluator::builtins::system_controls::shell_execute;
use uwscr::logging::{out_log, LogType};
use uwscr::get_script;
use uwscr::serializer;
use uwscr::settings::out_default_setting_file;


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
                match get_script(&p) {
                    Ok(s) => match script::run(s, args.get_args()) {
                        Ok(_) => {},
                        Err(errors) => {
                            for err in errors {
                                out_log(&err, LogType::Error);
                                eprintln!("{}", err);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            },
            Mode::Code(c) => {
                match script::run_code(c) {
                    Ok(_) => {},
                    Err(errors) => {
                        for err in errors {
                            // out_log(&err, LogType::Error);
                            eprintln!("{}", err);
                        }
                    }
                }
            }
            Mode::Repl(p) => {
                let exe_path = args.args[0].clone();
                if p.is_some() {
                    match get_script(&p.unwrap()) {
                        Ok(s) => repl::run(Some(s), exe_path, Some(args.args[2].clone())),
                        Err(e) => {
                            eprintln!("{}", e)
                        }
                    }
                } else {
                    repl::run(None, exe_path, None)
                }
            },
            Mode::Ast(p, b) => {
                let path = p.clone().into_os_string().into_string().unwrap();
                match get_script(&p) {
                    Ok(s) => script::out_ast(s, &path, b),
                    Err(e) => {
                        eprintln!("{}", e)
                    }
                }
            },
            Mode::Lib(p) => {
                let path = p.clone();
                let mut script_fullpath = if p.is_absolute() {
                    p.clone()
                } else {
                    match env::current_dir() {
                        Ok(cur) => cur.join(&p),
                        Err(e) => {
                            eprintln!("--lib: {}", e);
                            return;
                        }
                    }
                };

                let dir = match path.parent() {
                    Some(p) => p,
                    None => {
                        eprintln!("faild to get script directory.");
                        return;
                    },
                };
                match std::env::set_current_dir(dir) {
                    Err(e) => {
                        eprintln!("{}", e);
                        return;
                    },
                    _ => {},
                };
                match get_script(&script_fullpath) {
                    Ok(s) => match serializer::serialize(s) {
                        Some(bin) => {
                            // uwslファイルとして保存
                            script_fullpath.set_extension("uwsl");
                            serializer::save(script_fullpath, bin);
                        },
                        None => {},
                    },
                    Err(e) => {
                        eprintln!("--lib: {}", e)
                    }
                }
            },
            Mode::Settings => {
                match out_default_setting_file() {
                    Ok(s) => println!("{}", s),
                    Err(e) => eprintln!("{}", e)
                }
            }
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
            "-c" | "--code" => if self.args.len() > 2 {
                let code = self.args.clone().drain(2..).collect::<Vec<_>>();
                Ok(Mode::Code(code.join(" ")))
            } else {
                Err("code is required.".into())
            },
            "-a" | "--ast" => match self.get_path() {
                Ok(Some(p)) => Ok(Mode::Ast(p, false)),
                Ok(None) => Err("FILE is required".to_string()),
                Err(e) => Err(e)
            },
            "--ast-force" => match self.get_path() {
                Ok(Some(p)) => Ok(Mode::Ast(p, true)),
                Ok(None) => Err("FILE is required".to_string()),
                Err(e) => Err(e)
            },
            "-l" | "--lib" => match self.get_path() {
                Ok(Some(p)) => Ok(Mode::Lib(p)),
                Ok(None) => Err("FILE is required".to_string()),
                Err(e) => Err(e)
            },
            "-s" | "--settings" => Ok(Mode::Settings),
            "--language-server" => self.get_port().map(|p| Mode::Server(p)),
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

    pub fn get_args(&self) -> Vec<String> {
        self.args.clone()
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
        println!("  uwscr FILE [params]          : パラメータ付きスクリプトの実行");
        println!("                                 半角スペース区切りで複数指定可能");
        println!("                                 スクリプトからはPARAM_STRで値を取得");
        println!("  uwscr [(-r|--repl) [FILE]]   : Replを起動 (スクリプトを指定するとそれを実行してから起動)");
        println!("  uwscr (-a|--ast) FILE        : スクリプトの構文木を出力");
        println!("  uwscr --ast-force FILE       : 構文エラーでも構文木を出力");
        println!("  uwscr (-l|--lib) FILE        : スクリプトからuwslファイルを生成する");
        println!("  uwscr (-c|--code) CODE       : 渡された文字列を評価して実行する");
        println!("  uwscr (-s|--settings)        : 設定ファイル(settings.json)を開く");
        // println!("  uwscr --language-server [PORT]   : Language Serverとして起動、デフォルトポートはxxx");
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
    Code(String),
    Ast(PathBuf, bool),
    Lib(PathBuf),
    Server(Option<u16>),
    Help,
    Version,
    Settings,
    OnlineHelp,
}

