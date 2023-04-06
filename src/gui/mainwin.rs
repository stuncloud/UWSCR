
use super::{Window, UWindow, UWindowResult};

use windows::{
    Win32::{
        Foundation::{HWND,},
        UI::{
            WindowsAndMessaging::{
                WINDOW_STYLE , WINDOW_EX_STYLE,
            },
        },
    }
};
use once_cell::sync::OnceCell;

static MAINWIN_CLASS: OnceCell<UWindowResult<String>> = OnceCell::new();
pub static MAINWIN_HWND: OnceCell<MainWin> = OnceCell::new();

#[derive(Debug)]
pub struct MainWin {
    hwnd: HWND,
}

#[allow(dead_code)]
impl MainWin {
    pub fn new(version: &String) -> UWindowResult<()> {
        let title = format!("UWSCR {}", version);
        let class_name = Window::get_class_name("UWSCR.Main", &MAINWIN_CLASS, Some(Self::wndproc))?;
        let hwnd = Window::create_window(
            None,
            &class_name,
            &title,
            WINDOW_EX_STYLE(0),
            WINDOW_STYLE(0),
            100,
            100,
            100,
            100,
            None
        )?;
        MAINWIN_HWND.get_or_init(move || {
            MainWin {hwnd}
        });
        Ok(())
    }
}

impl UWindow<()> for MainWin {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
}