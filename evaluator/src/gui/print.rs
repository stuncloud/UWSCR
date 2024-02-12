use super::*;

use windows::core::HSTRING;
use windows::Win32::UI::{
    Controls::{WC_EDITW, EM_SETSEL, EM_REPLACESEL},
    WindowsAndMessaging::{
        WS_CHILD, WS_VSCROLL, WS_VISIBLE,
        ES_LEFT,ES_WANTRETURN,ES_AUTOHSCROLL,ES_AUTOVSCROLL,ES_MULTILINE
    },
};

use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct LogPrintWin {
    hwnd: HWND,
    hfont: Gdi::HFONT,
    edit: HWND,
    /// logprint関数の表示フラグ
    visible: bool,
}

impl LogPrintWin {
    const ID_EDIT: i32 = 100;

    pub fn new(title: &str, visible: bool, font: Option<FontFamily>) -> UWindowResult<Self> {
        let hwnd = Self::create_window(title)?;
        let hfont = font.unwrap_or_default().create()?;
        let edit = Self::set_edit(hwnd)?;
        let mut logprint = Self { hwnd, hfont, edit, visible };

        logprint.set_font(edit, "");
        logprint.set_visibility(visible, false);

        Ok(logprint)
    }
    pub fn set_visibility(&mut self, visible: bool, show_now: bool) {
        self.visible = visible;
        if self.visible {
            if show_now {
                self.show();
            }
        } else {
            self.hide();
        }
    }
    pub fn print(&self, message: &str) {
        if self.visible {
            self.show();
        }
        unsafe {
            let message = format!("{message}\r\n");
            let hstring = HSTRING::from(message);
            let ptr = hstring.as_ptr() as isize;
            let text_len = wm::GetWindowTextLengthW(self.edit);
            wm::SendMessageW(self.edit, EM_SETSEL, WPARAM(text_len as usize), LPARAM(text_len as isize));
            wm::SendMessageW(self.edit, EM_REPLACESEL, None, LPARAM(ptr));
        }
    }
    pub fn close(&self) {
        self.destroy();
    }
    pub fn set_new_pos(&self, x: Option<i32>, y: Option<i32>, width: Option<i32>, height: Option<i32>) {
        let rect = self.get_rect().unwrap_or_default();
        let x = x.unwrap_or(rect.left);
        let y = y.unwrap_or(rect.top);
        let width = width.unwrap_or(rect.right - rect.left);
        let height = height.unwrap_or(rect.bottom - rect.top);
        self.move_to(x, y, width, height);
    }
    fn set_edit(parent: HWND) -> UWindowResult<HWND> {
        let style = WS_CHILD|WS_VSCROLL|WS_VISIBLE|WINDOW_STYLE(
            (ES_LEFT|ES_WANTRETURN|ES_AUTOHSCROLL|ES_AUTOVSCROLL|ES_MULTILINE) as u32
        );
        let edit = WindowBuilder::new("", WC_EDITW)
            .parent(parent)
            .style(style)
            .ex_style(wm::WS_EX_TOOLWINDOW)
            .menu(Self::ID_EDIT as isize)
            .build()?;
        unsafe {
            Self::resize_edit(parent, Some(edit));
        }
        Ok(edit)
    }
    unsafe fn resize_edit(parent: HWND, edit: Option<HWND>) {
        let rect = Self::get_client_rect(parent);
        let edit = edit.unwrap_or(wm::GetDlgItem(parent, Self::ID_EDIT));
        let _ = wm::MoveWindow(edit, rect.left, rect.top, rect.right-rect.left, rect.bottom-rect.top, true);
    }
    unsafe fn get_client_rect(hwnd: HWND) -> RECT {
        let mut rect = RECT::default();
        let _ = wm::GetClientRect(hwnd, &mut rect);
        rect
    }
}

impl UWindow<()> for LogPrintWin {
    const CLASS_NAME: PCWSTR = w!("UWSCR.LogPrintWin");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_window_class(&REGISTER_CLASS)?;
        let hwnd = WindowBuilder::new(title, Self::CLASS_NAME)
            .style(wm::WS_OVERLAPPEDWINDOW)
            .ex_style(wm::WS_EX_NOACTIVATE)
            .size(Some(100), Some(100), Some(800), Some(600))
            .build()?;
        Ok(hwnd)
    }

    fn draw(&self) -> UWindowResult<()> {
        unimplemented!()
    }

    fn message_loop(&self) -> UWindowResult<()> {
        unsafe {
            let mut msg = wm::MSG::default();
            while wm::GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                wm::TranslateMessage(&msg);
                wm::DispatchMessageW(&msg);
            }
            Ok(())
        }
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            wm::WM_CLOSE => {
                let _ = wm::ShowWindow(hwnd, wm::SW_HIDE);
                LRESULT(0)
            },
            wm::WM_SIZE => {
                Self::resize_edit(hwnd, None);
                LRESULT(0)
            }
            msg => wm::DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn font(&self) -> Gdi::HFONT {
        self.hfont
    }
}