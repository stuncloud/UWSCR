use std::path::PathBuf;
use std::error::Error;
use std::fs;

use structopt::{clap::ArgGroup, StructOpt};
use encoding_rs::{UTF_8, SHIFT_JIS};

use uwscr::script;
use uwscr::repl;

#[derive(StructOpt, Debug)]
#[structopt(group = ArgGroup::with_name("command").required(false))]
struct Opt {
    #[structopt(long, short, group = "command", help = "対話モードで起動します")]
    repl: bool,

    #[structopt(long, short, help = "構文木を出力します")]
    ast: bool,

    #[structopt(long = "server", short= "s", group = "command", help = "Language serverを起動します")]
    language_server: bool,

    #[structopt(name = "FILE", parse(from_os_str), group = "command", help = "スクリプトファイルのパス")]
    file: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    if opt.repl {
        repl::run();
        return Ok(());
    }
    if opt.language_server {
        println!("Language serverは未実装です");
        return Ok(());
    }
    match opt.file {
        Some(path) => {
            let bytes = fs::read(path)?;
            let script = match get_utf8(&bytes) {
                Ok(utf8) => utf8,
                Err(()) => {
                    eprintln!("failed to decode file");
                    return Ok(())
                }
            };
            if opt.ast {
                script::out_ast(script);
                return Ok(());
            }
            match script::run(script) {
                Ok(_) => Ok(()),
                Err(errors) => {
                    eprintln!("parser had {} error[s]", errors.len());
                    for err in errors {
                        eprintln!("{}", err);
                    }
                    Ok(())
                }
            }
        },
        None => {
            repl::run();
            Ok(())
        }
    }
}

fn get_utf8(bytes: &Vec<u8>) -> Result<String, ()> {
    let (cow, _, err) = UTF_8.decode(bytes);
    if ! err {
        return Ok(cow.to_string());
    } else {
        let (cow, _, err) = SHIFT_JIS.decode(bytes);
        if ! err {
            return Ok(cow.to_string());
        }
    }
    Err(())
}