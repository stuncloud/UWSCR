use std::mem::ManuallyDrop;
use super::interface::*;

use windows::{
    core::{w, AsImpl, PCWSTR},
    Win32::{
        Foundation::{
            HGLOBAL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM
        },
        Graphics::Gdi::{ClientToScreen, HFONT},
        System::{
            Com::{
                IDataObject, IDataObject_Impl, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL
            },
            Memory::{
                GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE
            },
            Ole::{
                DoDragDrop, IDropSource, OleInitialize, OleUninitialize, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE
            }
        },
        UI::{
            Shell::DROPFILES,
            WindowsAndMessaging::{
                DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, PostMessageW, PostQuitMessage, SetForegroundWindow, TranslateMessage, MSG, WM_APP, WM_CREATE, WM_DESTROY, WM_DROPFILES, WS_EX_TOOLWINDOW
            },
        }
    }
};

// use crate::builtins::file_control::interface::*;

use super::super::window_low::{get_current_pos, move_mouse_to};

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
pub fn dropfile(hwnd: HWND, files: Vec<String>, x: i32, y: i32) -> bool {
    /*
        ## todo

        1. まずOLEドロップを試す (DoDragDrop)
        2. 失敗だったらメッセージ送信する
     */
    unsafe {
        // 各パスの末尾に \0 を付与して連結し、さらに末尾に \0 をつける
        // {path1}\0{path2}\0\0
        let joined = files.join("\0") + "\0\0";
        let files = joined.encode_utf16()
            .flat_map(|n| n.to_le_bytes())
            .collect::<Vec<_>>();
        let dropfiles_size = std::mem::size_of::<DROPFILES>();
        let buffer_size = dropfiles_size + files.len();
        let mut buffer = vec![0u8; buffer_size];
        let p_dropfiles = buffer.as_mut_ptr() as *mut DROPFILES;
        let mut pt = POINT { x, y };
        if let Some(dropfiles) = p_dropfiles.as_mut() {
            dropfiles.pFiles = dropfiles_size as u32;
            dropfiles.pt = pt;
            dropfiles.fNC = false.into();
            dropfiles.fWide = true.into();
        }
        std::ptr::copy_nonoverlapping(files.as_ptr(), buffer[dropfiles_size..].as_mut_ptr(), files.len());

        let Ok(hglobal) = GlobalAlloc(GMEM_MOVEABLE, buffer.len()) else {
            return false;
        };
        let pglobal = GlobalLock(hglobal);
        std::ptr::copy_nonoverlapping(buffer.as_ptr(), pglobal as _, buffer.len());
        let _ = GlobalUnlock(hglobal);

        let cur_pos = get_current_pos().ok();
        SetForegroundWindow(hwnd);
        ClientToScreen(hwnd, &mut pt);
        move_mouse_to(pt.x, pt.y);

        let result = if ole_drop(hglobal) {
            // OLEドロップ成功
            true
        } else {
            // OLEドロップ失敗時はメッセージを送る
            let wparam = WPARAM(hglobal.0 as usize);
            PostMessageW(hwnd, WM_DROPFILES, wparam, None).is_ok()
        };

        if let Some(p) = cur_pos {
            std::thread::sleep(std::time::Duration::from_millis(30));
            move_mouse_to(p.x, p.y);
        }
        result
    }
}

unsafe fn ole_drop(hglobal: HGLOBAL) -> bool {
    let source = DropSource::new();
    let data = DropFiles::new();
    let fmt = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let med = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 {
            hGlobal: hglobal
        },
        pUnkForRelease: ManuallyDrop::new(None),
    };

    let data = IDataObject::from(data);
    let _ = data.as_impl().SetData(&fmt, &med, false.into());
    let source = IDropSource::from(source);
    dbg!(&data, &source);
    if let Ok(drop_win) = OleDrop::new(data, source) {
        drop_win.message_loop().unwrap_or_default()
    } else {
        false
    }
}

use crate::{UWindow, gui::{WindowBuilder, UWindowResult, FontFamily}};
use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

struct OleDrop {
    data: IDataObject,
    source: IDropSource,
    hwnd: HWND,
    font: HFONT,
}
impl OleDrop {
    fn new(data: IDataObject, source: IDropSource) -> UWindowResult<Self> {
        let font = FontFamily::default().create()?;
        let hwnd = Self::create_window("drop dummy win")?;
        Ok(Self { data, source, hwnd, font })
    }
    fn do_drop(&self) -> bool {
        unsafe {
            let mut pdweffect = DROPEFFECT::default();
            let dwokeffects = DROPEFFECT_MOVE|DROPEFFECT_COPY|DROPEFFECT_LINK;
            DoDragDrop(&self.data, &self.source, dwokeffects, &mut pdweffect)
                .ok()
                .inspect(|_| {dbg!(pdweffect);})
                .inspect_err(|e| {dbg!(e);})
                .is_ok()
        }
    }
    fn destroy(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd)
                .inspect_err(|e|{dbg!(e);});
        }
    }
}
const DO_DRAG_AND_DROP: u32 = WM_APP + 100;
impl UWindow<bool> for OleDrop {
    const CLASS_NAME: PCWSTR = w!("UWSCR.Drop");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_window_class(&REGISTER_CLASS)?;
        WindowBuilder::new(title, Self::CLASS_NAME)
            .ex_style(WS_EX_TOOLWINDOW)
            .build()
    }

    fn draw(&self) -> UWindowResult<()> {
        Ok(())
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn font(&self) -> HFONT {
        self.font
    }

    fn message_loop(&self) -> UWindowResult<bool> {
        unsafe {
            let _ = OleInitialize(None);

            let mut msg = MSG::default();
            let hwnd = HWND::default();
            let mut result = false;
            while GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
                if msg.message == DO_DRAG_AND_DROP {
                    result = self.do_drop();
                    dbg!(&result);
                    self.destroy();
                } else {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            OleUninitialize();
            Ok(result)
        }
    }

    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                let _ = PostMessageW(hwnd, DO_DRAG_AND_DROP, None, None);
                LRESULT(0)
            },
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            msg => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}