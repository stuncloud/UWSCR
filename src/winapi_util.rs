use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use winapi::{
    um::{
        sysinfoapi,
        shlobj,
        winuser,
        wingdi,
    },
    shared::{
        minwindef::MAX_PATH
    }
};

pub fn buffer_to_string( buffer: &[u16] ) -> Result<String, String> {
    buffer.iter()
        .position(|wch| wch == &0)
        .ok_or("String : Can't find zero terminator !".to_owned())
        .and_then(
            |ix| String::from_utf16( &buffer[..ix] )
            .map_err(|e| e.to_string())
        )
}

pub fn to_wide_string(string: &String) -> Vec<u16> {
    OsStr::new(string.as_str()).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}

pub fn get_system_directory() -> String {
    let mut buffer = [0; MAX_PATH];
    unsafe {
        sysinfoapi::GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32);
    }
    buffer_to_string(&buffer).unwrap_or("".into())
}

pub fn get_windows_directory() -> String {
    let mut buffer = [0; MAX_PATH];
    unsafe {
        sysinfoapi::GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32);
    }
    buffer_to_string(&buffer).unwrap_or("".into())
}

pub fn get_special_directory(csidl: i32) -> String {
    let mut buffer = [0; MAX_PATH];
    unsafe {
        shlobj::SHGetSpecialFolderPathW(null_mut(), buffer.as_mut_ptr(), csidl, 0);
    }
    buffer_to_string(&buffer).unwrap_or("".into())
}

pub fn get_screen_width() -> i32 {
    unsafe {
        winuser::GetSystemMetrics(winuser::SM_CXVIRTUALSCREEN)
    }
}

pub fn get_screen_height() -> i32 {
    unsafe {
        winuser::GetSystemMetrics(winuser::SM_CYVIRTUALSCREEN)
    }
}

pub fn get_color_depth() -> i32 {
    unsafe {
        let dc = winuser::GetDC(null_mut());
        wingdi::GetDeviceCaps(dc, wingdi::BITSPIXEL)
    }
}