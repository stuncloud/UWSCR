use crate::gui::{UWindow, UWindowResult, WindowBuilder};
use crate::{
    Evaluator,
    object::function::Function
};
use util::error::UWSCRErrorTitle;
use util::winapi::show_message;
use util::logging::{out_log, LogType};
use parser::ast::{FuncParam, ParamKind, Expression};

use std::sync::{Arc, Mutex, OnceLock, LazyLock};
use std::collections::HashMap;

use windows::core::{w, PCWSTR};
use windows::Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT},
        UI::{
            WindowsAndMessaging::{
                DefWindowProcW, DestroyWindow,
                WM_HOTKEY, WM_CLOSE,
            },
            Input::KeyboardAndMouse::{
                    RegisterHotKey, UnregisterHotKey,
                    HOT_KEY_MODIFIERS,
                },
        },
    };

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();
static HOTKEY_WINDOW: LazyLock<Arc<Mutex<Option<SetHotKeyWindow>>>> = LazyLock::new(|| {Arc::new(Mutex::new(None))});

pub fn set_hot_key(vk: u32, mo: u32, func: Function, evaluator: &Evaluator) -> UWindowResult<()> {
    let mut mutex = HOTKEY_WINDOW.lock().unwrap();
    if let Some(shkw) = mutex.as_mut() {
        shkw.add(vk, mo, func);
    } else {
        let mut shkw = SetHotKeyWindow::new(evaluator)?;
        shkw.add(vk, mo, func);
        *mutex = Some(shkw);
    }
    Ok(())
}
pub fn remove_hot_key(vk: u32, mo: u32) {
    let mut mutex = HOTKEY_WINDOW.lock().unwrap();
    if let Some(shkw) = mutex.as_mut() {
        if shkw.remove(vk, mo) == 0 {
            shkw.close();
            *mutex = None;
        }
    }
}

struct SetHotKeyWindow {
    hwnd: HWND,
    evaluator: Evaluator,
    /// キーは(VK, MOD)
    /// 値は(ホットキーID, ユーザー定義関数)
    keymap: HashMap<(u32, u32), (i32, Function)>,
    id: i32,
}
impl SetHotKeyWindow {
    fn new(evaluator: &Evaluator) -> UWindowResult<Self> {
        let hwnd = Self::create_window("SetHotKeyDummyWin")?;
        Ok(Self {
            hwnd,
            evaluator: evaluator.clone(),
            keymap: HashMap::new(),
            id: 0,
        })
    }
    fn next_id(&mut self) -> i32 {
        self.id += 1;
        self.id
    }
    fn add(&mut self, vk: u32, mo: u32, mut func: Function) {
        unsafe {
            let k = (vk, mo);
            let id = if let Some((id, _)) = self.keymap.get(&k) {
                *id
            } else {
                self.next_id()
            };
            let fsmodifiers = HOT_KEY_MODIFIERS(mo);
            if RegisterHotKey(self.hwnd, id, fsmodifiers, vk).is_ok() {
                // 関数の引数を書き換える
                func.params = vec![
                    FuncParam::new(Some("HOTKEY_VK".into()), ParamKind::Identifier),
                    FuncParam::new(Some("HOTKEY_MOD".into()), ParamKind::Identifier),
                ];
                self.keymap.insert(k, (id, func));
            }
        }
    }
    fn remove(&mut self, vk: u32, mo: u32) -> usize {
        unsafe {
            let k = (vk, mo);
            if let Some((id, _)) = self.keymap.get(&k) {
                let _ = UnregisterHotKey(self.hwnd, *id);
                self.keymap.remove(&k);
            }
            self.keymap.len()
        }
    }
    fn close(&self) {
        self.destroy();
    }
}
impl UWindow<()> for SetHotKeyWindow {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_HOTKEY => {
                let vk = lparam.hi_word();
                let mo = lparam.lo_word();
                let maybe_func = {
                    let mutex = HOTKEY_WINDOW.lock().unwrap();
                    match mutex.as_ref() {
                        Some(shkw) => {
                            if let Some((_, f)) = shkw.keymap.get(&(vk, mo)) {
                                Some((f.clone(), shkw.evaluator.clone()))
                            } else {
                                None
                            }
                        },
                        None => None,
                    }
                };
                if let Some((function, mut evaluator)) = maybe_func {
                    // 引数としてキー情報を渡す
                    let arguments = vec![
                        (Some(Expression::EmptyArgument), vk.into()),
                        (Some(Expression::EmptyArgument), mo.into()),
                    ];
                    if let Err(err) = function.invoke(&mut evaluator, arguments, None) {
                        if let Ok(mutex) = HOTKEY_WINDOW.lock() {
                            if let Some(shkw) = mutex.as_ref() {
                                shkw.close();
                            }
                        }
                        evaluator.clear_local();
                        let msg = err.errror_text_with_line();
                        out_log(&msg, LogType::Error);
                        show_message(&msg, &UWSCRErrorTitle::SetHotKey.to_string(), true);
                        std::process::exit(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            },
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            },
            msg => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    const CLASS_NAME: PCWSTR = w!("UWSCR.SetHotKeyDummyWin");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_window_class(&REGISTER_CLASS)?;
        WindowBuilder::new(title, Self::CLASS_NAME)
            .size(Some(0), Some(0), Some(0), Some(0))
            .build()
    }

    fn draw(&self) -> UWindowResult<()> {
        unimplemented!()
    }

    fn font(&self) -> windows::Win32::Graphics::Gdi::HFONT {
        unimplemented!()
    }
}

trait LparamExt {
    fn hi_word(&self) -> u32;
    fn lo_word(&self) -> u32;
}
impl LparamExt for LPARAM {
    fn hi_word(&self) -> u32 {
        let hi = (self.0 >> 16) & 0xFFFF;
        hi as u32
    }

    fn lo_word(&self) -> u32 {
        let lo = self.0 & 0xFFFF;
        lo as u32
    }
}