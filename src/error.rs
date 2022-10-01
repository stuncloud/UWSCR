pub mod evaluator;
pub mod parser;

use windows::Win32::Globalization::GetUserDefaultUILanguage;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub enum Locale {
    Jp,
    En,
}

pub static CURRENT_LOCALE: Lazy<Locale> = Lazy::new(||{
    match unsafe{GetUserDefaultUILanguage()} {
        0x0411 => Locale::Jp,
        _ => Locale::En
    }
});

#[macro_export]
macro_rules! write_locale {
    ($f:expr, $jp:literal, $en:literal $(,$args:expr)*) => {
        {
            match *CURRENT_LOCALE {
                Locale::Jp => write!($f, $jp $(,$args)*),
                Locale::En => write!($f, $en $(,$args)*)
            }
        }
    };
    // 4つ目以降の引数がない場合も , を許容する
    ($f:expr, $jp:literal, $en:literal,) => {
        {
            match *CURRENT_LOCALE {
                Locale::Jp => write!($f, $jp),
                Locale::En => write!($f, $en)
            }
        }
    };
}

pub enum UWSCRErrorTitle {
    StatementError,
    RuntimeError,
    InitializeError,
    Panic
}

impl std::fmt::Display for UWSCRErrorTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UWSCRErrorTitle::StatementError => write_locale!(f,
                "UWSCR構文エラー",
                "UWSCR Statement Error",
            ),
            UWSCRErrorTitle::RuntimeError => write_locale!(f,
                "UWSCR実行時エラー",
                "UWSCR Runtime Error",
            ),
            UWSCRErrorTitle::InitializeError => write_locale!(f,
                "初期化エラー",
                "UWSCR Initializing Error",
            ),
            UWSCRErrorTitle::Panic => write!(f,"UWSCR Panic"),
        }
    }
}

impl Into<Vec<String>> for evaluator::UError {
    fn into(self) -> Vec<String> {
        vec![
            self.get_line().to_string(),
            self.to_string(),
        ]
    }
}