pub mod bindings {
    windows::include_bindings!();
}

use bindings::{
    Windows::{
        Win32::{
            Foundation:: {
                MAX_PATH, PWSTR, PSTR, HWND
            },
            System::{
                SystemInformation::{
                    GetSystemDirectoryW, GetWindowsDirectoryW
                },
            },
            UI::{
                WindowsAndMessaging::{
                    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
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
            Globalization::{
                CP_ACP, WC_COMPOSITECHECK, MB_PRECOMPOSED,
                WideCharToMultiByte, MultiByteToWideChar,
            },
        }
    }
};

use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use std::{ffi::OsStr};
use std::os::windows::ffi::OsStrExt;

pub fn to_ansi_bytes(string: &str) -> Vec<u8> {
    unsafe {
        let mut wide = to_wide_string(string);
        let len = WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            PWSTR(wide.as_mut_ptr()),
            wide.len() as i32,
            PSTR::NULL,
            0,
            PSTR::NULL,
            &mut 0
        );
        if len > 0 {
            let mut result: Vec<u8> = Vec::with_capacity(len as usize);
            result.set_len(len as usize);
            WideCharToMultiByte(
                CP_ACP,
                WC_COMPOSITECHECK,
                PWSTR(wide.as_mut_ptr()),
                wide.len() as i32,
                PSTR(result.as_mut_ptr()),
                result.len() as i32,
                PSTR::NULL,
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
        let mut wide = to_wide_string(string);
        let len = WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            PWSTR(wide.as_mut_ptr()),
            wide.len() as i32,
            PSTR::NULL,
            0,
            PSTR::NULL,
            &mut 0
        );
        len as usize - 1
    }
}

pub fn from_ansi_bytes(ansi: &Vec<u8>) -> String {
    unsafe {
        let mut ansi = ansi.clone();
        let ansi_pointer = ansi.as_mut_ptr();
        let len = MultiByteToWideChar(
            CP_ACP,
            MB_PRECOMPOSED,
            PSTR(ansi_pointer),
            ansi.len() as i32,
            PWSTR::NULL,
            0
        );
        if len > 0 {
            let mut wide: Vec<u16> = Vec::with_capacity(len as usize);
            wide.set_len(len as usize);
            MultiByteToWideChar(
                CP_ACP,
                MB_PRECOMPOSED,
                PSTR(ansi_pointer),
                ansi.len() as i32,
                PWSTR(wide.as_mut_ptr()),
                wide.len() as i32
            );
            String::from_utf16_lossy(&wide)
        } else {
            String::new()
        }
    }
}

pub fn to_wide_string(string: &str) -> Vec<u16> {
    let result = OsStr::new(string).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();
    result
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
            UErrorKind::Win32Error(e.code().0),
            UErrorMessage::Win32Error(e.message()),
        )
    }
}
