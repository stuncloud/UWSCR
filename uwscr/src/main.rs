#![cfg_attr(feature="gui", windows_subsystem = "windows")]
// #![cfg_attr(not(test), windows_subsystem = "windows")]
// #![windows_subsystem = "windows"]

use std::io::Write;
use std::path::PathBuf;
use std::env;
use std::str::FromStr;
use std::panic;
use std::sync::{Arc, Mutex};

use uwscr::script;
use uwscr::repl;
use uwscr::record::{record_desktop, RecordLevel};
use parser::serializer;
use evaluator::builtins::get_builtin_names;
use util::get_script;
use util::logging::{out_log, LogType};
use util::settings::{
    FileMode,
    out_default_setting_file, out_json_schema_file
};
use util::winapi::{show_message, shell_execute, FORCE_WINDOW_MODE};
use util::error::UWSCRErrorTitle;
// use uwscr::language_server::UwscrLanguageServer;

use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use clap::{Parser, ValueEnum};

fn main() {
    unsafe { let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }

    if cfg!(debug_assertions) {
        start_uwscr();
    } else {
        let buffer = Arc::new(Mutex::new(String::default()));
        let old_hook = panic::take_hook();
        panic::set_hook({
            let buffer = buffer.clone();
            Box::new(move |info| {
                let mut buffer = buffer.lock().unwrap();
                let s = info.to_string();
                buffer.push_str(&s);
            })
        });

        let result = panic::catch_unwind(|| {
            start_uwscr();
        });

        panic::set_hook(old_hook);

        if result.is_err() {
            let err = buffer.lock().unwrap();
            out_log(&err, LogType::Panic);
            // attach_console();
            show_message(&err, &UWSCRErrorTitle::Panic.to_string(), true);
            // free_console();
        }
    }
}

fn start_uwscr() {
    let mode = Mode::new();
    match mode {
        Mode::OnlineHelp => {
            shell_execute("https://stuncloud.github.io/UWSCR/".into(), None);
        },
        Mode::License => {
            shell_execute("https://stuncloud.github.io/UWSCR/_static/license.html".into(), None);
        },
        Mode::Script(p, params, ast) => {
            let exe_path = env::args().next().unwrap_or_default();
            match get_script(&p) {
                Ok(s) => match script::run(s, &exe_path, p, params, ast) {
                    Ok(_) => {},
                    Err(script::ScriptError(title, errors)) => {
                        let err = errors.join("\r\n");
                        out_log(&err, LogType::Error);
                        show_message(&err, &title.to_string(), true);
                    }
                },
                Err(e) => {
                    show_message(&e.to_string(), &UWSCRErrorTitle::InitializeError.to_string(), true);
                }
            }
        },
        Mode::Code(c) => {
            match script::run_code(c) {
                Ok(_) => {},
                Err(errors) => {
                    let err = errors.join("\r\n");
                    show_message(&err, "uwscr --code", true);
                }
            }
        }
        Mode::Repl(p, params, ast) => {
            let exe_path = env::args().next().unwrap_or_default();
            match p {
                Some(path) => match get_script(&path) {
                    Ok(script) => {
                        repl::run(Some(script), exe_path, Some(path), params, ast)
                    },
                    Err(e) => eprintln!("{e}"),
                },
                None => {
                    repl::run(None, exe_path, None, Vec::new(), ast);
                },
            }
        },
        Mode::Lib(p) => {
            let dlg_title = "uwscr --lib";
            // attach_console();
            let path = p.clone();
            let mut script_fullpath = if p.is_absolute() {
                p.clone()
            } else {
                match env::current_dir() {
                    Ok(cur) => cur.join(&p),
                    Err(e) => {
                        show_message(&e.to_string(), dlg_title, true);
                        return;
                    }
                }
            };

            let dir = match path.parent() {
                Some(p) => p,
                None => {
                    show_message("faild to get script directory.", dlg_title, true);
                    return;
                },
            };
            match std::env::set_current_dir(dir) {
                Err(e) => {
                    show_message(&e.to_string(), dlg_title, true);
                    return;
                },
                _ => {},
            };
            match get_script(&script_fullpath) {
                Ok(s) => {
                    let names = get_builtin_names();
                    match serializer::serialize(s, names) {
                        Some(bin) => {
                            // uwslファイルとして保存
                            script_fullpath.set_extension("uwsl");
                            serializer::save(script_fullpath, bin);
                        },
                        None => {},
                    }
                },
                Err(e) => {
                    show_message(&e.to_string(), dlg_title, true);
                }
            }
            // free_console();
        },
        Mode::Settings(fm) => {
            let dlg_title = "uwscr --settings";
            // attach_console();
            match out_default_setting_file(fm) {
                Ok(ref s) => show_message(s, dlg_title, false),
                Err(e) => show_message(&e.to_string(), dlg_title, true)
            }
            // free_console();
        },
        Mode::Schema(p) => {
            let dlg_title = "uwscr --schema";
            // attach_console();
            let dir = match p {
                Some(p) => p,
                None => match PathBuf::from_str(".") {
                    Ok(p) => p,
                    Err(e) => {
                        show_message(&e.to_string(), dlg_title, true);
                        return;
                    }
                }
            };
            match out_json_schema_file(dir) {
                Ok(ref s) => show_message(s, dlg_title, false),
                Err(e) => show_message(&e.to_string(), dlg_title, true)
            }
            // free_console();
        },
        Mode::Record(path) => {
            let dlg_title = "uwscr --record";
            match record_desktop(RecordLevel::Low) {
                Ok(Some(script)) => {
                    let script = script.join("\r\n");
                    match path {
                        Some(path) => {
                            let file = std::fs::OpenOptions::new()
                                .write(true)
                                .create(true)
                                .open(&path);
                            match file {
                                Ok(mut file) => {
                                    let buf = script.as_bytes();
                                    match file.write_all(buf) {
                                        Ok(_) => {
                                            let message = format!("Saved script to {path:?}");
                                            show_message(&message, dlg_title, false);
                                        },
                                        Err(e) => {
                                            show_message(&e.to_string(), dlg_title, true);
                                        },
                                    }
                                },
                                Err(e) => {
                                    show_message(&e.to_string(), dlg_title, true);
                                },
                            }
                        },
                        None => {
                            match util::clipboard::Clipboard::new() {
                                Ok(cb) => {
                                    cb.send_str(script);
                                    show_message("Copied script to clipboard", dlg_title, false)
                                },
                                Err(_) => {
                                    show_message("Failed to open clipboard", dlg_title, true);
                                },
                            }
                        }
                    }
                },
                Ok(None) => {},
                Err(e) => {
                    println!("\u{001b}[31m[debug] {e}\u{001b}[0m");
                },
            };
        },
        // Mode::LanguageServer => {
        //     match UwscrLanguageServer::run() {
        //         Ok(_) => {},
        //         Err(e) => {
        //             show_message(&e.to_string(), "UWSCR Language Server", true);
        //         },
        //     }
        // },
    }
}

enum Mode {
    /// ファイルパス, PARAM_STR, ast
    Script(PathBuf, Vec<String>, Option<(bool, bool)>),
    /// [モジュール], PARAM_STR, ast
    Repl(Option<PathBuf>, Vec<String>, Option<(bool, bool)>),
    /// ファイルパス
    Lib(PathBuf),
    Code(String),
    Settings(FileMode),
    OnlineHelp,
    License,
    Schema(Option<PathBuf>),
    // LanguageServer,
    Record(Option<PathBuf>),
}
impl Mode {
    fn new() -> Self {
        let args = CommandArgs::parse();
        let ast = args.ast.then_some((args._continue, args.prettify));

        if let Some(code) = args.code {
            Self::Code(code)
        } else if let Some(opt) = args.settings {
            let mode = match opt {
                Some(cmd) => match cmd {
                    SettingCommand::Initialize => FileMode::Init,
                    SettingCommand::Merge => FileMode::Merge,
                },
                None => FileMode::Open,
            };
            Self::Settings(mode)
        } else if let Some(path) = args.schema {
            Self::Schema(path)
        } else if args.online_help {
            Self::OnlineHelp
        } else if args.license {
            Self::License
        } else if let Some(path) = args.record {
            Self::Record(path)
        } else if let Some(script) = args.script {
            let param_str = args.script_args.unwrap_or_default();
            if args.repl {
                Self::Repl(Some(script), param_str, ast)
            } else if args.lib {
                Self::Lib(script)
            } else {
                if args.window {
                    FORCE_WINDOW_MODE.get_or_init(|| true);
                }
                Self::Script(script, param_str, ast)
            }
        } else {
            Self::Repl(None, Vec::new(), ast)
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CommandArgs {
    /// Replモードで起動
    #[arg(short, long)]
    repl: bool,
    /// windowモードを強制する
    #[arg(short, long, requires="script")]
    window: bool,
    /// スクリプトからuwslファイルを生成する
    #[arg(short, long, requires="script")]
    lib: bool,
    /// 渡された文字列を評価して実行する
    #[arg(short, long)]
    code: Option<String>,
    /// 設定ファイルを開く
    #[arg(short, long, name="OPTION")]
    settings: Option<Option<SettingCommand>>,
    /// 指定ディレクトリに設定ファイルのjson schemaファイル(uwscr-settings-schema.json)を出力する
    #[arg(long, name="OUTPUT_DIR")]
    schema: Option<Option<PathBuf>>,
    /// オンラインヘルプを表示する
    #[arg(short, long="online-help")]
    online_help: bool,
    /// サードパーティライセンスを表示
    #[arg(long)]
    license: bool,

    /// 読み込んだスクリプトの構文木を表示する
    #[arg(short, long, requires="script")]
    ast: bool,

    /// AST出力後に実行を継続する
    #[arg(long="continue", requires="ast")]
    _continue: bool,
    /// ASTの出力を見やすくする
    #[arg(short, long, requires="ast")]
    prettify: bool,

    /// 実行するスクリプトのパス
    script: Option<PathBuf>,
    /// PARAM_STRに渡される引数
    #[arg(name="PARAM_STR", requires="script")]
    script_args: Option<Vec<String>>,

    /// 低レベル記録を行う、ファイルパス未指定時はクリップボードに保存
    #[arg(long, name="FILE")]
    record: Option<Option<PathBuf>>
}


#[derive(Debug, Clone, ValueEnum)]
enum SettingCommand {
    /// 設定ファイルを初期化する
    #[value(name="init")]
    Initialize,
    /// 可能な限り既存の設定ファイルの内容を維持する
    Merge,
}
