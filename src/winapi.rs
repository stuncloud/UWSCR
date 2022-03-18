use windows::{
    core::{PSTR, PCSTR},
    Win32::{
        Foundation:: {
            MAX_PATH, HWND
        },
        System::{
            SystemInformation::{
                GetSystemDirectoryW, GetWindowsDirectoryW
            },
            Console::{
                ATTACH_PARENT_PROCESS,
                AttachConsole, FreeConsole, AllocConsole,
                GetConsoleCP,
            }
        },
        UI::{
            WindowsAndMessaging::{
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                MESSAGEBOX_STYLE, MB_OK, MB_ICONEXCLAMATION,
                GetSystemMetrics, MessageBoxW
            },
            Shell::{
                SHGetSpecialFolderPathW,
            },
        },
        Graphics::{
            Gdi::{
                BITSPIXEL,
                GetDC, GetDeviceCaps,
            },
        },
        Globalization::{
            CP_ACP, WC_COMPOSITECHECK, MB_PRECOMPOSED,
            WideCharToMultiByte, MultiByteToWideChar,
        },
    }
};


use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use std::{ffi::OsStr};
use std::os::windows::ffi::OsStrExt;

pub fn to_ansi_bytes(string: &str) -> Vec<u8> {
    unsafe {
        let wide = to_wide_string(string);
        let len = WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            &wide,
            PSTR::default(),
            0,
            PCSTR::default(),
            &mut 0
        );
        if len > 0 {
            let mut result: Vec<u8> = Vec::with_capacity(len as usize);
            result.set_len(len as usize);
            WideCharToMultiByte(
                CP_ACP,
                WC_COMPOSITECHECK,
                &wide,
                PSTR(result.as_mut_ptr()),
                result.len() as i32,
                PCSTR::default(),
                &mut 0
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
            PSTR::default(),
            0,
            PCSTR::default(),
            &mut 0
        );
        len as usize - 1
    }
}

pub fn from_ansi_bytes(ansi: &Vec<u8>) -> String {
    unsafe {
        let ansi = ansi.clone();
        let len = MultiByteToWideChar(
            CP_ACP,
            MB_PRECOMPOSED,
            &ansi,
            &mut vec![]
        );
        if len > 0 {
            let mut wide: Vec<u16> = Vec::with_capacity(len as usize);
            wide.set_len(len as usize);
            MultiByteToWideChar(
                CP_ACP,
                MB_PRECOMPOSED,
                &ansi,
                &mut wide
            );
            String::from_utf16_lossy(&wide)
        } else {
            String::new()
        }
    }
}

pub fn to_wide_string(string: &str) -> Vec<u16> {
    let result = OsStr::new(string).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    result
}

pub fn get_system_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetSystemDirectoryW(&mut buffer);
    }
    String::from_utf16_lossy(&buffer)
}

pub fn get_windows_directory() -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        GetWindowsDirectoryW(&mut buffer);
    }
    String::from_utf16_lossy(&buffer)
}

pub fn get_special_directory(csidl: i32) -> String {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        SHGetSpecialFolderPathW(HWND::default(), &mut buffer, csidl, false);
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
        let dc = GetDC(HWND::default());
        GetDeviceCaps(dc, BITSPIXEL)
    }
}

pub fn attach_console() -> bool {
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS).as_bool()
    }
}
pub fn free_console() -> bool {
    unsafe {
        FreeConsole().as_bool()
    }
}
pub fn alloc_console() -> bool {
    unsafe {
        AllocConsole().as_bool()
    }
}

pub fn is_console() -> bool {
    unsafe {
        GetConsoleCP() != 0
    }
}

pub fn message_box(message: &str, title: &str, utype: MESSAGEBOX_STYLE) {
    unsafe {
        MessageBoxW(HWND(0), message, title, utype);
    }
}

pub fn show_message(message: &str, title: &str, is_error: bool) {
    match (is_console(), is_error) {
        (true, true) => eprintln!("{}", message),
        (true, false) => println!("{}", message),
        (false, true) => message_box(message, title, MB_ICONEXCLAMATION),
        (false, false) => message_box(message, title, MB_OK),
    }
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
