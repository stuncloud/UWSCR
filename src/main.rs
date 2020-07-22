
use std::env;
use std::path::Path;

fn main() {
    let args:Vec<String> = env::args().collect();
    let opttype = Options::parse(args);

    match opttype {
        OptionType::Path(p) => {

        },
        OptionType::Help(c) => {
            Options::show_help("uwscr.exe", &c);
        },
        OptionType::Repl => {println!("repl mode")},
        OptionType::Setting => {println!("show settings")},
    }
}

pub enum OptionType {
    Path(String),
    Help(String),
    Repl,
    Setting,
}

pub struct Options {
    // optiontype: OptionType,
    // value: String
}

impl Options {
    fn parse(args: Vec<String>) -> OptionType {
        if args.len() < 2 {
            return OptionType::Help("!!! arguments required".to_string());
        };
        let opt1: &str = &args[1];
        match opt1 {
            "/r" | "--repl" => OptionType::Repl,
            "/h" | "--help" => OptionType::Help("".to_string()),
            "/s" | "--settings" => OptionType::Setting,
            p if Path::new(p).exists() => {
                OptionType::Path(p.to_string())
            },
            _ => OptionType::Help("!!! not valid command !".to_string())
        }
    }
    fn show_help(exe: &str, caution: &str) {
        println!(r#"{1}

USAGE:
    {0} [option]
    {0} path\to\script

OPTIONS:
    -h, --help    show help
    -r, --repl    start repl
        "#, exe, caution);
    }
}

