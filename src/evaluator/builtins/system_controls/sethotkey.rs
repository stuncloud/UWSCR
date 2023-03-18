use crate::gui::{Window, UWindow, UWindowError, UWindowResult};
use crate::ast::{FuncParam, ParamKind, Expression};
use crate::winapi::show_message;
use crate::error::UWSCRErrorTitle;
use crate::logging::{out_log, LogType};
use crate::evaluator::{
    Evaluator,
    object::function::Function
};

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use once_cell::sync::{Lazy, OnceCell};

use windows::{
    Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT},
        UI::{
            WindowsAndMessaging::{
                WINDOW_STYLE, WINDOW_EX_STYLE,
                DefWindowProcW, PostQuitMessage, SendMessageW,
                WM_HOTKEY, WM_DESTROY, WM_CLOSE,
            },
            Input::{
                KeyboardAndMouse::{
                    RegisterHotKey, UnregisterHotKey,
                    HOT_KEY_MODIFIERS,
                },
            },
        },
    },
};

static HOTKEY_WINDOW: Lazy<Arc<Mutex<Option<SetHotKeyWindow>>>> = Lazy::new(|| {Arc::new(Mutex::new(None))});
static CLASS_NAME: OnceCell<Result<String, UWindowError>> = OnceCell::new();

pub fn set_hot_key(vk: u32, mo: u32, func: Function, evaluator: &Evaluator) -> UWindowResult<()> {
    let mut mutex = HOTKEY_WINDOW.lock().unwrap();
    if let Some(shkw) = mutex.as_mut() {
        shkw.add(vk, mo, func);
    } else {
        let mut shkw = SetHotKeyWindow::new(&evaluator)?;
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
        let class_name = Window::get_class_name("UWSCR.SetHotKey", &CLASS_NAME, Some(Self::wndproc))?;
        let hwnd = Window::create_window(None, &class_name, "SetHotKey", WINDOW_EX_STYLE(0), WINDOW_STYLE(0), 0, 0, 0, 0, None)?;
        let evaluator = evaluator.clone();
        let keymap = HashMap::new();
        let id = 0;
        Ok(Self { hwnd, evaluator, keymap, id })
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
            if RegisterHotKey(self.hwnd, id, fsmodifiers, vk).as_bool() {
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
                UnregisterHotKey(self.hwnd, *id);
                self.keymap.remove(&k);
            }
            self.keymap.len()
        }
    }
    fn close(&self) {
        unsafe {
            SendMessageW(self.hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}
impl UWindow<()> for SetHotKeyWindow {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
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
                    if let Err(err) = function.invoke(&mut evaluator, arguments, false) {
                        if let Ok(mutex) = HOTKEY_WINDOW.lock() {
                            if let Some(shkw) = mutex.as_ref() {
                                shkw.close();
                            }
                        }
                        evaluator.clear();
                        let msg = err.to_string();
                        out_log(&msg, LogType::Error);
                        show_message(&msg, &UWSCRErrorTitle::RuntimeError.to_string(), true);
                        std::process::exit(0);
                    }
                }
                DefWindowProcW(hwnd, umsg, wparam, lparam)
            },
            WM_CLOSE |
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            },
            msg => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

trait LparamExt {
    fn hi_word(&self) -> u32;
    fn lo_word(&self) -> u32;
}
impl LparamExt for LPARAM {
    fn hi_word(&self) -> u32 {
        let hi = (self.0 & 0xFFFF0000) >> 16;
        hi as u32
    }

    fn lo_word(&self) -> u32 {
        let lo = self.0 & 0xFFFF;
        lo as u32
    }
}