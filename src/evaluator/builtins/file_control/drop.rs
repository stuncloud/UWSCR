use std::ffi::OsStr;
use std::path::PathBuf;

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{
            HWND, POINT, WPARAM, RECT,
        },
        Graphics::Gdi::ClientToScreen,
        UI::{
            Shell::DROPFILES,
            WindowsAndMessaging::{
                PostMessageW, WM_DROPFILES,
                GetClientRect, SetForegroundWindow,
            },
        },
        System::Memory::{
                GlobalLock, GlobalUnlock,
                GlobalAlloc, GMEM_MOVEABLE,
            }
    }
};

use super::super::window_low::{get_current_pos, move_mouse_to};

pub fn get_list_hstring(dir: String, files: Vec<String>) -> HSTRING {
    let dir = PathBuf::from(dir);
    let pathes = files.into_iter()
        .map(|file| {
            let mut dir = dir.clone();
            dir.push(file);
            let mut path = dir.into_os_string();
            path.push(&OsStr::new("\0"));
            path
        })
        .collect::<Vec<_>>();
    let list = pathes.join(&OsStr::new(""));
    HSTRING::from(&list)
}
pub fn get_point(hwnd: HWND, x: Option<i32>, y: Option<i32>) -> (i32, i32) {
    match (x, y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            unsafe {
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let x = (rect.right - rect.left) / 2;
                let y = (rect.bottom - rect.top) / 2;
                (x, y)
            }
        },
    }
}

/// pathは複数パスをnull連結する
pub fn dropfile(hwnd: HWND, files: &HSTRING, x: i32, y: i32) -> bool {
    unsafe {
        let dropfiles_size = std::mem::size_of::<DROPFILES>();
        let buffer_size = dropfiles_size + (files.len() + 1) * 2;
        let mut buffer = vec![0u8; buffer_size];
        let p_dropfiles = buffer.as_mut_ptr() as *mut DROPFILES;
        let mut pt = POINT { x, y };
        (*p_dropfiles).pFiles = dropfiles_size as u32;
        (*p_dropfiles).pt = pt;
        (*p_dropfiles).fNC = false.into();
        (*p_dropfiles).fWide = true.into();
        std::ptr::copy_nonoverlapping(files.as_ptr(), buffer[dropfiles_size..].as_mut_ptr() as *mut u16, files.len() + 1);

        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, buffer_size) else {
            return false;
        };
        let pglobal = GlobalLock(hmem);
        std::ptr::copy_nonoverlapping(buffer.as_ptr(), pglobal as *mut u8, buffer_size);
        let _ = GlobalUnlock(hmem);
        let wparam = WPARAM(pglobal as usize);

        let cur_pos = get_current_pos().ok();
        SetForegroundWindow(hwnd);
        ClientToScreen(hwnd, &mut pt);
        move_mouse_to(pt.x, pt.y);
        let result = PostMessageW(hwnd, WM_DROPFILES, wparam, None).is_ok();
        if let Some(p) = cur_pos {
            std::thread::sleep(std::time::Duration::from_millis(30));
            move_mouse_to(p.x, p.y);
        }
        result
    }
}
