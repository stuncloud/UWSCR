pub mod evaluator;
pub mod parser;

use std::sync::Once;
use windows::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Debug, Clone)]
pub enum Locale {
    Jp,
    En,
}

pub fn locale_singleton() -> Box<Locale> {
    static mut SINGLETON: Option<Box<Locale>> = None;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once( || {
            let singleton = match GetUserDefaultUILanguage() {
                0x0411 => Locale::Jp,
                _ => Locale::En
            };
            SINGLETON = Some(Box::new(singleton));
        });
        SINGLETON.clone().unwrap()
    }
}

#[macro_export]
macro_rules! write_locale {
    ($f:expr, $jp:literal, $en:literal $(,$args:expr)*) => {
        {
            let locale = locale_singleton();
            match *locale {
                Locale::Jp => write!($f, $jp $(,$args)*),
                Locale::En => write!($f, $en $(,$args)*)
            }
        }
    };
    // 4つ目以降の引数がない場合も , を許容する
    ($f:expr, $jp:literal, $en:literal,) => {
        {
            let locale = locale_singleton();
            match *locale {
                Locale::Jp => write!($f, $jp),
                Locale::En => write!($f, $en)
            }
        }
    };
}