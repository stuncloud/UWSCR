use super::LockHardExConst;

use windows::{
    Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT,},
        UI::{
            Input::{
                KeyboardAndMouse::{
                    BlockInput,
                },
            },
            WindowsAndMessaging::{
                SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx,
                // GetForegroundWindow, WindowFromPoint,
                // GetWindowThreadProcessId,
                HHOOK,
                HC_ACTION,
                WH_KEYBOARD_LL, WH_MOUSE_LL,
                // MSLLHOOKSTRUCT, // KBDLLHOOKSTRUCT,
            }
        },
    }
};

use once_cell::sync::{Lazy};
use std::sync::{Arc, Mutex};

pub fn lock(flg: bool) -> bool {
    unsafe { BlockInput(flg).as_bool() }
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
}

impl LockHard {
    fn new() -> Self {
        // Self { hwnd: None, hhook: None, message: 0 }
        Self { mouse: None, keyboard: None, hwnd: None }
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
                    Err(e) => {
                        println!("\u{001b}[31m[WH_KEYBOARD_LL] error: {:?}\u{001b}[0m", e);
                        false
                    },
                }
            } else {true};
            let mo = if mouse {
                match SetWindowsHookExW(WH_MOUSE_LL, Some(Self::ll_mouse_hook), None, 0) {
                    Ok(hhk) => {
                        self.keyboard = Some(hhk);
                        true
                    },
                    Err(e) => {
                        println!("\u{001b}[31m[WH_MOUSE_LL] error: {:?}\u{001b}[0m", e);
                        false
                    },
                }
            } else {true};

            if kb && mo {
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
            let result = match (self.keyboard, self.mouse) {
                (None, None) => true,
                (None, Some(hmo)) => UnhookWindowsHookEx(hmo).as_bool(),
                (Some(hkb), None) => UnhookWindowsHookEx(hkb).as_bool(),
                (Some(hkb), Some(hmo)) => {
                    UnhookWindowsHookEx(hkb).as_bool() &&
                    UnhookWindowsHookEx(hmo).as_bool()
                },
            };
            if result {
                self.keyboard = None;
                self.mouse = None;
                self.hwnd = None;
            }
            result
        }
    }
    unsafe extern "system"
    fn ll_mouse_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if ncode == HC_ACTION as i32 {
            // if let Ok(lh) = LOCKHARD.lock() {
            //     match lh.hwnd {
            //         Some(hwnd) => {
            //             let msllhookstruct = *(lparam.0 as *mut MSLLHOOKSTRUCT);
            //             println!("\u{001b}[33m[debug] pt: {:?}\u{001b}[0m", msllhookstruct.pt);
            //             if hwnd == WindowFromPoint(msllhookstruct.pt) {
            //                 return LRESULT(1);
            //             }
            //         },
            //         None => return LRESULT(1),
            //     }
            // }
            return LRESULT(1);
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
    unsafe extern "system"
    fn ll_keyboard_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if ncode == HC_ACTION as i32 {
            // if let Ok(lh) = LOCKHARD.lock() {
            //     match lh.hwnd {
            //         Some(hwnd) => {
            //             if hwnd == GetForegroundWindow() {
            //                 return LRESULT(1);
            //             }
            //         },
            //         None => return LRESULT(1),
            //     }
            // }
            return LRESULT(1);
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
    // fn get_hook_proc(&self) -> dyn Fn(i32, WPARAM, LPARAM) -> LRESULT {
    //     let hwnd = self.hwnd;
    //     move |ncode, wparam, lparam| unsafe {
    //         if ncode == HC_ACTION as i32 {
    //             match hwnd {
    //                 Some(hwnd) => if hwnd == GetForegroundWindow() {
    //                     return LRESULT(1);
    //                 },
    //                 None => return LRESULT(1),
    //             }
    //         }
    //         CallNextHookEx(None, ncode, wparam, lparam)
    //     }
    // }
    // fn contains(&self, message: u32) -> bool {
    //     (self.message & message) == message
    // }
}
