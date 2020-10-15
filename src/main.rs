use std::path::PathBuf;
use std::error::Error;
use structopt::{clap::ArgGroup, StructOpt};
use std::fs;

use uwscr::script;
use uwscr::repl;

#[derive(StructOpt, Debug)]
#[structopt(group = ArgGroup::with_name("command").required(false))]
struct Opt {
    #[structopt(long, short, group = "command", help = "対話モードで起動します")]
    repl: bool,

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
            let script = fs::read_to_string(path)?;
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

