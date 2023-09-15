use super::LockHardExConst;

use windows::Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT, POINT},
        UI::{
            Input::KeyboardAndMouse::BlockInput,
            WindowsAndMessaging::{
                SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx,
                GetForegroundWindow,
                WindowFromPoint, GetParent, IsWindowVisible,
                // GetWindowThreadProcessId,
                HHOOK,
                HC_ACTION,
                WH_KEYBOARD_LL, WH_MOUSE_LL,
                MSLLHOOKSTRUCT, //KBDLLHOOKSTRUCT,
                WM_MOUSEMOVE,
                EVENT_SYSTEM_DESKTOPSWITCH,
                WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNTHREAD,
            },
            Accessibility::{
                SetWinEventHook, UnhookWinEvent,
                HWINEVENTHOOK,
            }
        },
    };

use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

pub fn lock(flg: bool) -> bool {
    unsafe { BlockInput(flg).is_ok() }
}

pub fn lock_ex(hwnd: Option<HWND>, mode: LockHardExConst) -> bool {
    let (keyboard, mouse) = match mode {
        LockHardExConst::LOCK_ALL => (true, true),
        LockHardExConst::LOCK_KEYBOARD => (true, false),
        LockHardExConst::LOCK_MOUSE => (false, true),
    };
    if let Ok(mut lh) = LOCKHARD.lock() {
        lh.lock(hwnd, keyboard, mouse)
    } else {
        false
    }
}

pub fn free_ex() -> bool {
    if let Ok(mut lh) = LOCKHARD.lock() {
        lh.free()
    } else {
        false
    }
}

static LOCKHARD: Lazy<Arc<Mutex<LockHard>>> = Lazy::new(|| Arc::new(Mutex::new(LockHard::new())));

struct LockHard {
    mouse: Option<HHOOK>,
    keyboard: Option<HHOOK>,
    hwnd: Option<HWND>,
    cad: Option<HWINEVENTHOOK>,
}

impl LockHard {
    fn new() -> Self {
        // Self { hwnd: None, hhook: None, message: 0 }
        Self { mouse: None, keyboard: None, hwnd: None, cad: None }
    }
    fn lock(&mut self, hwnd: Option<HWND>, keyboard: bool, mouse: bool) -> bool {
        unsafe {
            // すでにロックしていた場合は解除する
            self.free();
            let kb = if keyboard {
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::ll_keyboard_hook), None, 0) {
                    Ok(hhk) => {
                        self.keyboard = Some(hhk);
                        true
                    },
                    Err(_) => false,
                }
            } else {true};
            let mo = if mouse {
                match SetWindowsHookExW(WH_MOUSE_LL, Some(Self::ll_mouse_hook), None, 0) {
                    Ok(hhk) => {
                        self.mouse = Some(hhk);
                        true
                    },
                    Err(_) => false,
                }
            } else {true};

            if kb && mo {
                self.set_cad_event_hook();
                self.hwnd = hwnd;
                true
            } else {
                self.free();
                false
            }
        }
    }
    fn free(&mut self) -> bool {
        unsafe {
            let kb = if let Some(hhk) = self.keyboard {
                if UnhookWindowsHookEx(hhk).is_ok() {
                    self.keyboard = None;
                    true
                } else {false}
            } else {true};
            let mo = if let Some(hhk) = self.mouse {
                if UnhookWindowsHookEx(hhk).is_ok() {
                    self.mouse = None;
                    true
                } else {false}
            } else {true};
            let cad = if let Some(hwineventhook) = self.cad {
                if UnhookWinEvent(hwineventhook).as_bool() {
                    self.cad = None;
                    true
                } else {false}
            } else {true};
            kb && mo && cad
        }
    }
    fn window_from_point(point: POINT) -> HWND {
        unsafe {
            let mut hwnd = WindowFromPoint(point);
            loop {
                let parent = GetParent(hwnd);
                if parent.0 == 0 || ! IsWindowVisible(parent).as_bool() {
                    break hwnd;
                } else {
                    hwnd = parent;
                }
            }
        }
    }
    unsafe extern "system"
    fn ll_mouse_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if ncode == HC_ACTION as i32 && wparam.0 != WM_MOUSEMOVE as usize {
            let s = *(lparam.0 as *mut MSLLHOOKSTRUCT);
            if let Ok(lh) = LOCKHARD.lock() {
                match lh.hwnd {
                    Some(hwnd) => {
                        if hwnd == Self::window_from_point(s.pt) {
                            return LRESULT(1);
                        }
                    },
                    None => return LRESULT(1),
                }
            }
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
    unsafe extern "system"
    fn ll_keyboard_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if ncode == HC_ACTION as i32 {
            if let Ok(lh) = LOCKHARD.lock() {
                match lh.hwnd {
                    Some(hwnd) => {
                        if hwnd == GetForegroundWindow() {
                            return LRESULT(1);
                        }
                    },
                    None => return LRESULT(1),
                }
            }
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
    fn set_cad_event_hook(&mut self) {
        unsafe {
            let h = SetWinEventHook(
                EVENT_SYSTEM_DESKTOPSWITCH, EVENT_SYSTEM_DESKTOPSWITCH,
                None, Some(Self::win_event_proc),
                0, 0, WINEVENT_OUTOFCONTEXT|WINEVENT_SKIPOWNTHREAD
            );
            self.cad = Some(h);
        }
    }
    unsafe extern "system"
    fn win_event_proc(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
        let mut lh = LOCKHARD.lock().unwrap();
        lh.free();
    }
}
