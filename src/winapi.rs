use windows::{
    core::{PCSTR, PCWSTR, HSTRING},
    Win32::{
        Foundation:: {
            MAX_PATH, HWND, WPARAM, LPARAM,
            GetLastError
        },
        System::{
            SystemInformation::{
                GetSystemDirectoryW, GetWindowsDirectoryW
            },
            Console::{
                // ATTACH_PARENT_PROCESS,
                // AttachConsole, FreeConsole, AllocConsole,
                GetConsoleCP, GetConsoleWindow,
                // GetStdHandle, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE,
            },
        },
        UI::{
            WindowsAndMessaging::{
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                MESSAGEBOX_STYLE, MB_OK, MB_ICONEXCLAMATION,
                GetSystemMetrics, MessageBoxW,
                GetClassNameW, GetWindowTextW,
                GetWindowLongW, GWL_STYLE,
            },
            Shell::SHGetSpecialFolderPathW,
        },
        Graphics::Gdi::{
            BITSPIXEL,
            GetDC, GetDeviceCaps,
        },
        Globalization::{
            CP_ACP, WC_COMPOSITECHECK, MB_PRECOMPOSED,
            WideCharToMultiByte, MultiByteToWideChar,
        },
    }
};


use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use once_cell::sync::OnceCell;

pub static FORCE_WINDOW_MODE: OnceCell<bool> = OnceCell::new();

pub fn to_ansi_bytes(string: &str) -> Vec<u8> {
    unsafe {
        let wide = to_wide_string(string);
        let len = WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            &wide,
            None,
            PCSTR::null(),
            None
        );
        if len > 0 {
            let mut result: Vec<u8> = Vec::with_capacity(len as usize);
            result.set_len(len as usize);
            WideCharToMultiByte(
                CP_ACP,
                WC_COMPOSITECHECK,
                &wide,
                Some(&mut result),
                PCSTR::null(),
                None
            );
            result
        } else {
            vec![]
        }
    }
}

pub fn get_ansi_length(string: &str) -> usize {
    unsafe {
        let wide = to_wide_string(string);
        let len = WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            &wide,
            None,
            PCSTR::null(),
            None
        );
        len as usize - 1
    }
}

pub fn from_ansi_bytes(ansi: &[u8]) -> String {
    unsafe {
        let len = MultiByteToWideChar(
            CP_ACP,
            MB_PRECOMPOSED,
            &ansi,
            Some(&mut vec![])
        );
        if len > 0 {
            let mut wide: Vec<u16> = Vec::with_capacity(len as usize);
            wide.set_len(len as usize);
            MultiByteToWideChar(
                CP_ACP,
                MB_PRECOMPOSED,
                &ansi,
                Some(&mut wide)
            );
            String::from_utf16_lossy(&wide)
                .trim_end_matches(char::is_control)
                .to_string()
        } else {
            String::new()
        }
    }
}

pub fn to_wide_string(string: &str) -> Vec<u16> {
    let result = OsStr::new(string).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    result
}
pub fn from_wide_string(wide: &[u16]) -> String {
    String::from_utf16_lossy(&wide)
        .trim_end_matches(char::is_control)
        .to_string()
}

pub fn contains_unicode_char(string: &str) -> bool {
    unsafe {
        let wide = to_wide_string(string);
        #[allow(non_snake_case)]
        let mut lpUsedDefaultChar = 0;
        WideCharToMultiByte(
            CP_ACP,
            0,
            &wide,
            None,
            PCSTR::null(),
            Some(&mut lpUsedDefaultChar)
        );
        lpUsedDefaultChar != 0
    }
}

pub fn get_system_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetSystemDirectoryW(Some(&mut buffer));
    }
    from_wide_string(&buffer)
}

pub fn get_windows_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetWindowsDirectoryW(Some(&mut buffer));
    }
    from_wide_string(&buffer)
}

pub fn get_special_directory(csidl: i32) -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        SHGetSpecialFolderPathW(HWND::default(), &mut buffer, csidl, false);
    }
    from_wide_string(&buffer)
}

pub fn get_screen_width() -> i32 {
    unsafe {
        GetSystemMetrics(SM_CXVIRTUALSCREEN)
    }
}

pub fn get_screen_height() -> i32 {
    unsafe {
        GetSystemMetrics(SM_CYVIRTUALSCREEN)
    }
}

pub fn get_color_depth() -> i32 {
    unsafe {
        let dc = GetDC(HWND::default());
        GetDeviceCaps(dc, BITSPIXEL)
    }
}

#[cfg(feature="gui")]
use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, AllocConsole,
    ATTACH_PARENT_PROCESS,
    GetStdHandle,
    STD_OUTPUT_HANDLE, STD_ERROR_HANDLE
};

#[cfg(feature="gui")]
pub fn attach_console() -> bool {
    use libc::{setvbuf, open_osfhandle, fdopen, _IONBF, O_TEXT};
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() {
            let redirect = |nstdhandle| {
                let mode = std::ffi::CString::new("w").unwrap();
                let buf = std::ptr::null_mut();
                if let Ok(h)  = GetStdHandle(nstdhandle) {
                    let fd = open_osfhandle(h.0, O_TEXT);
                    let stream = fdopen(fd, mode.as_ptr());
                    setvbuf(stream, buf, _IONBF, 0);
                }
            };
            redirect(STD_OUTPUT_HANDLE);
            redirect(STD_ERROR_HANDLE);
            true
        } else {
            false
        }
    }
}
#[cfg(feature="gui")]
pub fn free_console() -> bool {
    unsafe {
        FreeConsole().is_ok()
    }
}
#[cfg(feature="gui")]
pub fn alloc_console() -> bool {
    unsafe {
        AllocConsole().is_ok()
    }
}

pub fn get_console_hwnd() -> HWND {
    unsafe {
        GetConsoleWindow()
    }
}

pub fn is_console() -> bool {
    let force_window_mode = FORCE_WINDOW_MODE.get().unwrap_or(&false);
    if *force_window_mode {
        false
    } else {
        unsafe {
            GetConsoleCP() != 0
        }
    }
}

pub fn message_box(message: &str, title: &str, utype: MESSAGEBOX_STYLE) {
    unsafe {
        let lptext = HSTRING::from(message);
        let lpcaption = HSTRING::from(title);
        MessageBoxW(HWND(0), &lptext, &lpcaption, utype);
    }
}

pub fn show_message(message: &str, title: &str, is_error: bool) {
    #[cfg(feature="gui")]
    attach_console();
    match (is_console(), is_error) {
        (true, true) => eprintln!("{title}\n{message}"),
        (true, false) => println!("{}", message),
        (false, true) => message_box(message, title, MB_ICONEXCLAMATION),
        (false, false) => message_box(message, title, MB_OK),
    }
    #[cfg(feature="gui")]
    free_console();
}

// convert windows::runtime::Error to UError
impl From<windows::core::Error> for UError {
    fn from(e: windows::core::Error) -> Self {
        UError::new(
            UErrorKind::Win32Error(e.code().0),
            UErrorMessage::Win32Error(e.message().to_string()),
        )
    }
}

pub trait WString {
    fn to_wide(&self) -> Vec<u16>;
    fn to_wide_null_terminated(&self) -> Vec<u16>;
}

impl WString for &str {
    fn to_wide(&self) -> Vec<u16> {
        self.encode_utf16().collect()
    }

    fn to_wide_null_terminated(&self) -> Vec<u16> {
        self.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

impl WString for String {
    fn to_wide(&self) -> Vec<u16> {
        self.encode_utf16().collect()
    }

    fn to_wide_null_terminated(&self) -> Vec<u16> {
        self.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

pub trait PcwstrExt {
    fn to_pcwstr(&self) -> PCWSTR;
}

impl<'a> PcwstrExt for Vec<u16> {
    fn to_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.as_ptr())
    }
}

pub fn get_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class_buffer = [0; 512];
        let len = GetClassNameW(hwnd, &mut class_buffer);
        let class = String::from_utf16_lossy(&class_buffer[..len as usize]);
        class
    }
}

pub fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let mut title_buffer = [0; 512];
        let len = GetWindowTextW(hwnd, &mut title_buffer);
        let title = String::from_utf16_lossy(&title_buffer[..len as usize]);
        title
    }
}

pub fn get_window_style(hwnd: HWND) -> i32 {
    unsafe {
        GetWindowLongW(hwnd, GWL_STYLE)
    }
}

pub fn make_wparam(lo: u16, hi: u16) -> WPARAM {
    let wparam = make_dword(lo, hi) as usize;
    WPARAM(wparam)
}
fn make_dword(lo: u16, hi: u16) -> u32 {
    (lo as u32 & 0xFFFF) | (hi as u32 & 0xFFFF) << 16
}

pub fn make_lparam(lo: i32, hi: i32) -> LPARAM {
    let lparam = make_word(lo, hi);
    LPARAM(lparam)
}
pub fn make_word(lo: i32, hi: i32) -> isize {
    let word = (lo & 0xFFFF) | (hi & 0xFFFF) << 16;
    word as isize
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemError {
    code: u32,
    msg: String,
}
impl SystemError {
    pub fn new() -> Self {
        unsafe {
            match GetLastError() {
                Ok(_) => Self { code: 0, msg: "".into() },
                Err(err) => {
                    let code = err.code().0 as u32;
                    let msg = err.message().to_string_lossy();
                    Self { code, msg }
                },
            }
        }
    }
}
impl std::fmt::Display for SystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.msg)
    }
}