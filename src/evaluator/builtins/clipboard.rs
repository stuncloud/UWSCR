use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{HANDLE, HGLOBAL},
        System::{
            DataExchange::{
                OpenClipboard, CloseClipboard, IsClipboardFormatAvailable,
                GetClipboardData, SetClipboardData, EmptyClipboard
            },
            Memory::{
                GlobalLock, GlobalUnlock, GlobalSize, GlobalFree,
                GlobalAlloc, GMEM_MOVEABLE,
            },
            Ole::{
                CF_UNICODETEXT, CF_BITMAP
            }
        },
        Graphics::Gdi::HBITMAP,
    }
};

use crate::error::evaluator::{
    UError,
    UErrorKind::ClipboardError,
    UErrorMessage::FailedToOpenClipboard,
};
use crate::winapi::from_wide_string;

pub struct Clipboard;

impl Clipboard {
    pub fn new() -> Result<Self, UError> {
        unsafe {
            if OpenClipboard(None).as_bool() {
                Ok(Self)
            } else {
                Err(UError::new(ClipboardError, FailedToOpenClipboard))
            }
        }
    }
    pub fn get_str(&self) -> Option<String> {
        unsafe {
            if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).as_bool() {
                let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
                let hmem = HGLOBAL(handle.0);
                let len = GlobalSize(hmem);
                let ptr = GlobalLock(hmem) as *mut u16;
                let wide = std::slice::from_raw_parts(ptr, len / 2 - 1);
                let str = from_wide_string(&wide);
                GlobalUnlock(hmem);
                GlobalFree(hmem).ok()?;
                Some(str)
            } else {
                None
            }
        }
    }
    pub fn send_str(&self, str: String) -> bool {
        unsafe {
            if EmptyClipboard().as_bool() {
                let hstring = HSTRING::from(str);
                let src = hstring.as_ptr();
                let len = (hstring.len() + 1) * std::mem::size_of::<u16>();
                let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, len) else {
                    return  false;
                };
                let dst = GlobalLock(hmem) as _;
                std::ptr::copy_nonoverlapping(src, dst, hstring.len());
                GlobalUnlock(hmem);
                let result = SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(hmem.0)).is_ok();
                let _ = GlobalFree(hmem);
                result
            } else {
                false
            }
        }
    }
    pub fn set_bmp(&self, hbmp: HBITMAP) {
        unsafe {
            if EmptyClipboard().as_bool() {
                let hmem = HANDLE(hbmp.0);
                let _ = SetClipboardData(CF_BITMAP.0 as u32, hmem);
            }
        }
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}