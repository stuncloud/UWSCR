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