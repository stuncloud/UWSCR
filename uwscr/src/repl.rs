use std::env;
use std::path::PathBuf;

use evaluator::environment::Environment;
use evaluator::object::Object;
use evaluator::Evaluator;
use parser::Parser;
use parser::lexer::Lexer;
use util::com::Com;
use util::error::{
    CURRENT_LOCALE, Locale,
};
use util::write_locale;
use util::winapi::get_absolute_path;

use reedline::{
    Reedline, Signal,
    DefaultCompleter, ColumnarMenu,
    ReedlineMenu, MenuBuilder,
    KeyModifiers, KeyCode, ReedlineEvent, EditCommand,
    Emacs, default_emacs_keybindings,
    Prompt, PromptHistorySearch,
};
use std::borrow::Cow;

pub fn run(script: Option<String>, script_path: Option<PathBuf>, params: Vec<String>, ast: Option<(bool, bool)>) {
    match env::current_exe() {
        Ok(full) => {
            match full.parent() {
                Some(dir) => unsafe {
                    env::set_var("GET_UWSC_DIR", &dir.as_os_str());
                },
                None => {
                    eprintln!("failed to get uwscr directory");
                    return;
                },
            }
        },
        Err(e) => {
            eprintln!("failed to get uwscr path: {e}");
            return;
        },
    }
    if let Some(path) = script_path {
        let full = get_absolute_path(&path);
        if let Some(name) = full.file_name() {
            unsafe {
                env::set_var("GET_UWSC_NAME", name);
                env::set_var("UWSCR_DEFAULT_TITLE", &format!("UWSCR REPL - {}", name.to_string_lossy()))
            }
        }
        if let Some(dir) = full.parent() {
            unsafe {
                env::set_var("GET_SCRIPT_DIR", dir.as_os_str());
            }
        }
    }

    // このスレッドでのCOMを有効化
    let _com = match Com::init() {
        Ok(com) => com,
        Err(e) => {
            eprintln!("failed to initialize COM: {e}");
            return;
        },
    };

    let env = Environment::new(params);
    let mut evaluator = Evaluator::new(env);
    if let Some(script) = script {
        println!("loading module...");
        let parser = Parser::new(Lexer::new(&script), None, None);
        let (program, errors) = parser.parse_to_program_and_errors();

        if let Some((_continue, pretty)) = ast {
            let message = if pretty {
                format!("{program:#?}")
            } else {
                format!("{program:?}")
            };
            println!("\u{001b}[90m{message}\u{001b}[0m");
        }

        if let Err(e) = evaluator.eval(program, false) {
            eprint!("Evaluator Error:\n{e}");
        }

        if errors.is_empty() {
            println!("module loaded.");
        } else {
            for error in errors {
                eprintln!("Parse Error:\n{error}")
            }
            return;
        }
    }

    let funcs = evaluator.env.get_builtin_func_names();
    let consts = evaluator.env.get_builtin_const_names();
    let keywords = [
        "print", "call", "async", "await",
        "null", "empty", "nothing", "true", "false", "nan",
        "mod", "and", "andL", "andB", "or", "orL", "orB", "xor", "xorL", "xorB",
        "dim", "public", "const", "hashtbl",
        "select", "case", "selend",
        "if", "else", "elseif", "endif",
        "for", "next", "endfor", "while", "wend", "repeat", "until", "break", "continue",
        "procedure", "function", "fend",
        "module", "endmodule",
        "class", "endclass",
        "hash", "endhash",
        "enum", "endenum",
        "with", "endwith",
        "textblock", "endtextblock",
    ].map(|w| w.to_string()).to_vec();

    let mut editor = UReadLine::new(funcs, consts, keywords);
    println!("\u{001b}[36m{}\u{001b}[0m", UReadLineHint::Completion);
    println!("\u{001b}[36m{}\u{001b}[0m", UReadLineHint::NewLine);
    loop {
        match editor.readline() {
            Ok(sig) => match sig {
                Signal::Success(input) => {
                    let parser = Parser::new(Lexer::new(&input), None, None);
                    match parser.parse() {
                        Ok(program) => {
                            if let Some((_, p)) = ast {
                                if p {
                                    println!("\u{001b}[90m{program:#?}\u{001b}[0m");
                                } else {
                                    println!("\u{001b}[90m{program:?}\u{001b}[0m");
                                }
                            }
                            match evaluator.eval(program, false) {
                                Ok(o) => if let Some(o) = o {
                                    match o {
                                        Object::Exit => {
                                            println!("\u{001b}[33mbye!\u{001b}[0m");
                                            break;
                                        },
                                        o => {
                                            println!("\u{001b}[36m{o}\u{001b}[0m");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("\u{001b}[31m{e}\u{001b}[0m");
                                },
                            }
                        },
                        Err(errors) => {
                            print!("\u{001b}[33m");
                            for error in errors {
                                println!("{error}");
                            }
                            print!("\u{001b}[0m");
                        },
                    }
                },
                Signal::CtrlC |
                Signal::CtrlD => break,
            },
            Err(err) => {
                eprintln!("\u{001b}[31m{err}\u{001b}[0m");
                break;
            },
        }
    }
    evaluator.clear();

}

struct UReadLine(Reedline, UPrompt);

impl UReadLine {
    fn new(funcs: Vec<String>, consts: Vec<String>, keywords: Vec<String>) -> Self {

        // 補完
        let incl = &['_', '\n'];
        let mut completer = DefaultCompleter::with_inclusions(incl)
            .set_min_word_len(2);
        completer.insert(funcs);
        completer.insert(consts);
        completer.insert(keywords);


        let name = "menu1";
        let compmenu = ColumnarMenu::default().with_name(&name);
        let bindings = get_key_bindings(name);
        let menu = ReedlineMenu::EngineCompleter(Box::new(compmenu));

        let editor = Reedline::create()
            .with_completer(Box::new(completer))
            .with_menu(menu)
            .with_edit_mode(Box::new(bindings))
            .with_quick_completions(true);

        let prompt = UPrompt::new("uwscr");
        Self(editor, prompt)
    }
    fn readline(&mut self) -> std::io::Result<Signal> {
        self.0.read_line(&self.1)
    }
}


struct UPrompt {
    prompt: &'static str,
    multi_indicator: String,
    prompt_indicator: String,
}
impl Prompt for UPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Owned(self.prompt.to_string())
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::default()
    }

    fn render_prompt_indicator(&self, _: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed(&self.prompt_indicator)
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed(&self.multi_indicator)
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<str> {
        Cow::Owned(format!("({}) ",history_search.term))
    }
}

impl UPrompt {
    fn new(prompt: &'static str) -> Self {
        let len = prompt.len();
        let multi_indicator = format!("{:>1$}- ", "", len);
        let prompt_indicator = "> ".to_string();
        Self { prompt, multi_indicator, prompt_indicator }
    }
}

fn get_key_bindings(menu: &str) -> Emacs {
    let mut bindings = default_emacs_keybindings();
    bindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu(menu.to_string()),
            ReedlineEvent::MenuNext,
        ])
    );
    bindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline])
    );
    bindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Esc,
        ReedlineEvent::Edit(vec![EditCommand::Clear])
    );
    Emacs::new(bindings)
}

enum UReadLineHint {
    Completion,
    NewLine,
}

impl std::fmt::Display for UReadLineHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UReadLineHint::Completion => write_locale!(f,
                "補完: TAB",
                "Completion: TAB"
            ),
            UReadLineHint::NewLine => write_locale!(f,
                "改行: Alt+Enter",
                "New line: Alt+Enter"
            ),
        }
    }
}