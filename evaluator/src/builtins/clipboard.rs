use windows::{
    core::{HSTRING, PWSTR},
    Win32::{
        Foundation::{HANDLE, HGLOBAL},
        System::{
            DataExchange::{
                OpenClipboard, CloseClipboard, IsClipboardFormatAvailable,
                GetClipboardData, SetClipboardData, EmptyClipboard
            },
            Memory::{
                GlobalLock, GlobalUnlock,
                GlobalAlloc, GMEM_MOVEABLE,
            },
            Ole::{
                CF_UNICODETEXT, CF_BITMAP
            }
        },
        Graphics::Gdi::HBITMAP,
    }
};

use crate::error::{
    UError,
    UErrorKind::ClipboardError,
    UErrorMessage::FailedToOpenClipboard,
};

pub struct Clipboard;

impl Clipboard {
    pub fn new() -> Result<Self, UError> {
        unsafe {
            OpenClipboard(None)
                .map(|_| Self)
                .map_err(|_| UError::new(ClipboardError, FailedToOpenClipboard))
        }
    }
    pub fn get_str(&self) -> Option<String> {
        unsafe {
            IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).ok()?;
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
            let hmem = HGLOBAL(handle.0 as *mut std::ffi::c_void);
            let ptr = GlobalLock(hmem) as *mut u16;
            let pwstr = PWSTR::from_raw(ptr);
            let str = pwstr.to_hstring().ok()?.to_string_lossy();
            let _ = GlobalUnlock(hmem);
            Some(str)
        }
    }
    pub fn send_str(&self, str: String) -> bool {
        unsafe {
            if EmptyClipboard().is_ok() {
                let hstring = HSTRING::from(str);
                let src = hstring.as_ptr();
                let len = (hstring.len() + 1) * std::mem::size_of::<u16>();
                let Ok(hglobal) = GlobalAlloc(GMEM_MOVEABLE, len) else {
                    return  false;
                };
                let dst = GlobalLock(hglobal) as _;
                std::ptr::copy_nonoverlapping(src, dst, hstring.len());
                let hmem = HANDLE(hglobal.0 as isize);
                let _ = GlobalUnlock(hglobal);
                SetClipboardData(CF_UNICODETEXT.0 as u32, hmem).is_ok()
            } else {
                false
            }
        }
    }
    pub fn set_bmp(&self, hbmp: HBITMAP) {
        unsafe {
            if EmptyClipboard().is_ok() {
                let hmem = HANDLE(hbmp.0);
                let _ = SetClipboardData(CF_BITMAP.0 as u32, hmem);
            }
        }
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}