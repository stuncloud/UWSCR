use windows::{
    core::{PCSTR, PCWSTR, HSTRING, w},
    Win32::{
        Foundation:: {
            MAX_PATH, HWND, WPARAM, LPARAM, BOOL,
            GetLastError,
            E_INVALIDARG,
        },
        System::{
            SystemInformation::{
                GetSystemDirectoryW, GetWindowsDirectoryW
            },
            Console::GetConsoleWindow,
        },
        UI::{
            WindowsAndMessaging::{
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                MESSAGEBOX_STYLE, MB_OK, MB_ICONEXCLAMATION,
                GetSystemMetrics, MessageBoxW,
                GetClassNameW, GetWindowTextW,
                GetWindowLongW, GWL_STYLE,
                SW_SHOWNORMAL
            },
            Shell::{ SHGetSpecialFolderPathW, ShellExecuteW },
        },
        Graphics::Gdi::{
            BITSPIXEL,
            GetDC, GetDeviceCaps,
        },
        Globalization::{
            CP_ACP, WC_COMPOSITECHECK, MB_PRECOMPOSED,
            WideCharToMultiByte, MultiByteToWideChar,
        },
        Storage::FileSystem::GetFullPathNameW,
    }
};

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{PathBuf, Path};
use std::sync::OnceLock;

pub static FORCE_WINDOW_MODE: OnceLock<bool> = OnceLock::new();

pub fn shell_execute(cmd: String, params: Option<String>) -> bool {
    unsafe {
        let cmd = HSTRING::from(cmd);
        let params = params.map(HSTRING::from).unwrap_or_default();
        let hinstance = ShellExecuteW(
            HWND::default(),
            w!("open"),
            &cmd,
            &params,
            PCWSTR::null(),
            SW_SHOWNORMAL
        );
        hinstance.0 > 32
    }
}

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
        ) as usize;
        if len > 0 {
            let mut result: Vec<u8> = vec![0; len];
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
            ansi,
            None
        ) as usize;
        if len > 0 {
            let mut wide: Vec<u16> = vec![0; len];
            MultiByteToWideChar(
                CP_ACP,
                MB_PRECOMPOSED,
                ansi,
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
    String::from_utf16_lossy(wide)
        .trim_end_matches(char::is_control)
        .to_string()
}

pub fn contains_unicode_char(string: &str) -> bool {
    unsafe {
        let wide = to_wide_string(string);
        let mut lp_use_default_char = BOOL::from(false);
        WideCharToMultiByte(
            CP_ACP,
            0,
            &wide,
            None,
            PCSTR::null(),
            Some(&mut lp_use_default_char)
        );
        lp_use_default_char.as_bool()
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


pub fn get_console_hwnd() -> HWND {
    unsafe {
        GetConsoleWindow()
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
    if cfg!(feature="gui") {
        match is_error {
            true => message_box(message, title, MB_ICONEXCLAMATION),
            false => message_box(message, title, MB_OK),
        }
    } else {
        match is_error {
            true => eprintln!("{title}\n{message}"),
            false => println!("{}", message),
        }
    }
}

pub fn get_absolute_path<P: AsRef<Path>>(path: P) -> PathBuf {
    unsafe {
        let path = path.as_ref();
        let spath = path.to_string_lossy();
        let lpfilename = HSTRING::from(spath.as_ref());
        let mut buffer = [0; MAX_PATH as usize];
        let len = GetFullPathNameW(&lpfilename, Some(&mut buffer), None) as usize;
        let absolute = String::from_utf16_lossy(&buffer[..len]);
        PathBuf::from(absolute)
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

// pub trait PcwstrExt {
//     fn to_pcwstr(&self) -> PCWSTR;
// }

// impl<'a> PcwstrExt for Vec<u16> {
//     fn to_pcwstr(&self) -> PCWSTR {
//         PCWSTR::from_raw(self.as_ptr())
//     }
// }

pub fn get_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class_buffer = [0; 512];
        let len = GetClassNameW(hwnd, &mut class_buffer);
        String::from_utf16_lossy(&class_buffer[..len as usize])
    }
}

pub fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let mut title_buffer = [0; 512];
        let len = GetWindowTextW(hwnd, &mut title_buffer);
        String::from_utf16_lossy(&title_buffer[..len as usize])
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SystemError {
    code: u32,
    msg: String,
}
impl SystemError {
    pub fn new() -> Self {
        unsafe {
            match GetLastError() {
                Ok(_) => Self::default(),
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

#[derive(Debug, Clone, PartialEq)]
pub struct Win32Error {
    error: windows::core::Error,
    hint: Option<String>,
}
impl Win32Error {
    fn new(error: windows::core::Error, hint: &str) -> Self {
        Self {
            error,
            hint: Some(hint.to_string())
        }
    }
    pub fn is_invalid_arg_error(&self) -> bool {
        self.error.code() == E_INVALIDARG
    }
}
impl std::fmt::Display for Win32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.error.code().0;
        let msg = self.error.message().to_string();
        write!(f, "0x{code:08X} {msg}")?;
        if let Some(hint) = &self.hint {
            write!(f, " ({hint})")?;
        }
        Ok(())
    }
}
impl From<windows::core::Error> for Win32Error {
    fn from(error: windows::core::Error) -> Self {
        Self { error, hint: None }
    }
}
pub trait WindowsResultExt<T> {
    fn err_hint(self, hint: &str) -> std::result::Result<T, Win32Error>;
}
impl<T> WindowsResultExt<T> for windows::core::Result<T> {
    fn err_hint(self, hint: &str) -> std::result::Result<T, Win32Error> {
        self.map_err(|error| Win32Error::new(error, hint))
    }
}