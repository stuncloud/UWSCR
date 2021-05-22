pub mod bindings {
    windows::include_bindings!();
}

use bindings::{
    Windows::{
        Win32::{
            System::{
                SystemServices::{
                    MAX_PATH, PWSTR
                },
                WindowsProgramming::{
                    GetSystemDirectoryW, GetWindowsDirectoryW
                },
            },
            UI::{
                WindowsAndMessaging::{
                    HWND, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                    GetSystemMetrics,
                },
                Shell::{
                    SHGetSpecialFolderPathW,
                },
            },
            Graphics::{
                Gdi::{
                    GET_DEVICE_CAPS_INDEX,
                    GetDC, GetDeviceCaps,
                },
            },
        }
    }
};

use crate::evaluator::UError;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub fn to_wide_string(string: &String) -> Vec<u16> {
    OsStr::new(string.as_str()).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}

pub fn get_system_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetSystemDirectoryW(PWSTR(buffer.as_mut_ptr()), buffer.len() as u32);
    }
    String::from_utf16_lossy(&buffer)
}

pub fn get_windows_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetWindowsDirectoryW(PWSTR(buffer.as_mut_ptr()), buffer.len() as u32);
    }
    String::from_utf16_lossy(&buffer)
}

pub fn get_special_directory(csidl: i32) -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        SHGetSpecialFolderPathW(HWND::NULL, PWSTR(buffer.as_mut_ptr()), csidl, false);
    }
    String::from_utf16_lossy(&buffer).trim_matches(char::from(0)).to_string()
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
        let dc = GetDC(HWND::NULL);
        let bitspixel = 12;
        GetDeviceCaps(dc, GET_DEVICE_CAPS_INDEX(bitspixel))
    }
}


// convert windows::Error to UError
impl From<windows::Error> for UError {
    fn from(e: windows::Error) -> Self {
        UError::new(
            "Windows Api Error".into(),
            e.message(),
            Some(format!("{}", e.code().0))
        )
    }
}

