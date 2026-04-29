use crate::gui::{UWindow, UWindowResult, WindowBuilder, GdiObject};
use crate::{
    Evaluator,
    object::function::Function
};
use util::error::UWSCRErrorTitle;
use util::winapi::show_message;
use util::logging::{out_log, LogType};
use parser::ast::{FuncParam, Expression};

use std::sync::{Arc, RwLock, OnceLock, LazyLock, mpsc};
use std::collections::HashMap;
use std::thread::JoinHandle;

use windows::core::{w, PCWSTR};
use windows::Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT},
        UI::{
            WindowsAndMessaging::{
                DefWindowProcW,
                WM_HOTKEY,
                MSG, GetMessageW, TranslateMessage, DispatchMessageW,
                PostMessageW,
                WM_USER,
            },
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey,
                HOT_KEY_MODIFIERS,
            },
        },
        Graphics::Gdi::HFONT,
    };

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();
pub(super) static HOTKEY_WINDOW_HANDLER: LazyLock<SetHotKeyHandler> = LazyLock::new(SetHotKeyHandler::new);

/// RegisterHotkeyを実行させるためのメッセージ
/// - WParam: 上位word: vk, 下位word: modキー
/// - LParam: ホットキーID
const WM_REGISTER_HOTKEY: u32 = WM_USER;
/// UnregisterHotkeyを実行させるためのメッセージ
/// - WParam: なし
/// - LParam: ホットキーID
const WM_UNREGISTER_HOTKEY: u32 = WM_USER + 1;

#[derive(Default)]
pub(super)  struct SetHotKeyHandler {
    window: Arc<RwLock<Option<SetHotKeyThread>>>,
}
impl SetHotKeyHandler  {
    fn new() -> Self {
        Self {
            window: Arc::new(RwLock::new(None)),
        }
    }
    pub(super)  fn set(&self, vk: u32, mo: u32, mut func: Function, evaluator: &Evaluator) {
        func.params = FuncParam::hotkey_func_params();
        let mut lock = self.window.write().unwrap();
        if let Some(shkt) = lock.as_mut() {
            shkt.set(vk, mo, func);
        } else {
            let mut shkt = SetHotKeyThread::new(evaluator.clone());
            shkt.set(vk, mo, func);
            lock.replace(shkt);
        }
    }
    pub(super)  fn remove(&self, vk: u32, mo: u32) {
        if let Some(shkt) = self.window.write().unwrap().as_mut() {
            shkt.remove(vk, mo);
        }
    }
    fn get_func(&self, vk: u32, mo: u32) -> Option<Function> {
        let lock = self.window.read().unwrap();
        let shkt = lock.as_ref()?;
        shkt.get(vk, mo)
    }
}
struct SetHotKeyThread {
    _handle: JoinHandle<()>,
    /// (vk, mod), (ホットキーID, ユーザー定義関数)
    keymap: HashMap<(u32, u32), (i32, Function)>,
    id: i32,
    hwnd: HWND,
}
impl SetHotKeyThread {
    fn new(evaluator: Evaluator) -> Self {
        let (sender, recver) = mpsc::channel();
        let _handle = std::thread::spawn(move || {
            let shkw = SetHotKeyWindow::new(evaluator).unwrap();
            let _ = sender.send(shkw.hwnd);
            let _ = shkw.message_loop();
        });
        let hwnd = recver.recv().unwrap();
        Self {
            _handle,
            keymap: HashMap::new(),
            id: 0,
            hwnd,
        }
    }
    fn next_id(&mut self) -> i32 {
        self.id += 1;
        self.id
    }
    fn set(&mut self, vk: u32, mo: u32, func: Function) {
        let k = (vk, mo);
        let id = if let Some((id, _)) = self.keymap.get(&k) {
            *id
        } else {
            self.next_id()
        };
        SetHotKeyWindow::register_hotkey(self.hwnd, id, vk, mo);
        self.keymap.insert(k, (id, func));
    }
    fn remove(&mut self, vk: u32, mo: u32) -> usize {
        let k = (vk, mo);
        if let Some((id, _)) = self.keymap.get(&k) {
            SetHotKeyWindow::unregister_hotkey(self.hwnd, *id);
            self.keymap.remove(&k);
        }
        self.keymap.len()
    }
    fn get(&self, vk: u32, mo: u32) -> Option<Function> {
        self.keymap.get(&(vk, mo))
            .map(|(_, func)| func.clone())
    }
}


struct SetHotKeyWindow {
    hwnd: HWND,
    evaluator: Evaluator,
}
impl Drop for SetHotKeyWindow {
    fn drop(&mut self) {
        self.close();
    }
}
impl SetHotKeyWindow {
    fn new(evaluator: Evaluator) -> UWindowResult<Self> {
        let hwnd = Self::create_window("SetHotKeyDummyWin")?;
        Ok(Self {
            hwnd,
            evaluator,
        })
    }
    fn register_hotkey(hwnd: HWND, id: i32, vk: u32, mo: u32) {
        unsafe {
            let wparam = WPARAM::from_hi_lo_word(vk, mo);
            let lparam = LPARAM(id as isize);
            let _ = PostMessageW(hwnd, WM_REGISTER_HOTKEY, wparam, lparam);
        }
    }
    fn unregister_hotkey(hwnd: HWND, id: i32) {
        unsafe {
            let lparam = LPARAM(id as isize);
            let _ = PostMessageW(hwnd, WM_UNREGISTER_HOTKEY, None, lparam);
        }
    }
    fn close(&mut self) {
        self.destroy();
    }
}
impl UWindow<()> for SetHotKeyWindow {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn message_loop(&self) -> UWindowResult<()> {
        unsafe {
            let mut msg = MSG::default();
            let hwnd = HWND::default();
            while GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
                match msg.message {
                    WM_HOTKEY => {
                        let (vk, mo) = msg.lParam.to_hi_lo_word();
                        if let Some(func) = HOTKEY_WINDOW_HANDLER.get_func(vk, mo) {
                            // 引数としてキー情報を渡す
                            let arguments = vec![
                                (Some(Expression::EmptyArgument), vk.into()),
                                (Some(Expression::EmptyArgument), mo.into()),
                            ];
                            let mut evaluator = self.evaluator.clone();
                            if let Err(err) = func.invoke(&mut evaluator, arguments, None) {
                                let msg = err.errror_text_with_line();
                                out_log(&msg, LogType::Error);
                                show_message(&msg, &UWSCRErrorTitle::SetHotKey.to_string(), true);
                                std::process::exit(0);
                            }
                        }
                    },
                    WM_REGISTER_HOTKEY => {
                        let id = msg.lParam.0 as i32;
                        let (vk, mo) = msg.wParam.to_hi_lo_word();
                        let _ = RegisterHotKey(self.hwnd, id, HOT_KEY_MODIFIERS(mo), vk);
                    },
                    WM_UNREGISTER_HOTKEY => {
                        let id = msg.lParam.0 as i32;
                        let _ = UnregisterHotKey(self.hwnd, id);
                    },
                    _ => {},
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            Ok(())
        }

    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            DefWindowProcW(hwnd, msg, wparam, lparam)
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

    fn font(&self) -> &GdiObject<HFONT> {
        unimplemented!()
    }
}

trait ParamExt {
    fn from_hi_lo_word(hi: u32, lo: u32) -> Self;
    fn to_hi_lo_word(self) -> (u32, u32);
}
impl ParamExt for LPARAM {
    fn from_hi_lo_word(hi: u32, lo: u32) -> Self {
        let param = (hi & 0xFFFF) << 16 | (lo & 0xFFFF);
        Self(param as isize)
    }
    fn to_hi_lo_word(self) -> (u32, u32) {
        let hi = (self.0 >> 16) & 0xFFFF;
        let lo = self.0 & 0xFFFF;
        (hi as u32, lo as u32)
    }
}
impl ParamExt for WPARAM {
    fn from_hi_lo_word(hi: u32, lo: u32) -> Self {
        let param = (hi & 0xFFFF) << 16 | (lo & 0xFFFF);
        Self(param as usize)
    }

    fn to_hi_lo_word(self) -> (u32, u32) {
        let hi = (self.0 >> 16) & 0xFFFF;
        let lo = self.0 & 0xFFFF;
        (hi as u32, lo as u32)
    }
}