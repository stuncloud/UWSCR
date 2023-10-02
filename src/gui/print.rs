use super::{Window, UWindow, UWindowResult, UWindowError, FontFamily, USETTINGS};

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{
            HWND,WPARAM,LPARAM,LRESULT,
        },
        UI::{
            WindowsAndMessaging::{
                MSG,
                WM_DESTROY, WM_CLOSE, WM_SIZE, WM_SYSCOMMAND, WM_QUIT,
                SW_HIDE,
                WINDOW_STYLE ,WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_CHILD, WS_VSCROLL,
                WINDOW_EX_STYLE,
                ES_MULTILINE,ES_WANTRETURN, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_LEFT,

                SC_CLOSE,
                ShowWindow,
                DefWindowProcW,
                SendMessageW, GetMessageW, TranslateMessage, DispatchMessageW, GetWindowTextLengthW
            },
            Input::KeyboardAndMouse::SetFocus,
            Controls::{EM_SETSEL, EM_REPLACESEL}
        },
    }
};
use once_cell::sync::OnceCell;
use once_cell::sync::Lazy;

const ID_EDIT: i32 = 101;

static LOGPRINTWIN_CLASS: OnceCell<Result<String, UWindowError>> = OnceCell::new();
static LOGPRINTWIN_TITLE: Lazy<String> = Lazy::new(|| {
    match std::env::var("GET_UWSC_NAME") {
        Ok(name) => format!("UWSCR - {}", name),
        Err(_) => format!("UWSCR"),
    }
});
pub static LOG_FONT: Lazy<FontFamily> = Lazy::new(|| {
    let usettings = USETTINGS.lock().unwrap();
    FontFamily::new(&usettings.logfont.name, usettings.logfont.size)
});

#[derive(Debug, Clone)]
pub struct LogPrintWin {
    hwnd: HWND,
    edit: HWND,
    visible: bool,
}

impl LogPrintWin {
    pub fn new(visible: bool) -> UWindowResult<Self> {
        let hwnd = Self::create()?;
        let rect = Window::get_client_rect(hwnd);
        let x = rect.left;
        let y = rect.top;
        let dwstyle = WS_CHILD|WS_VSCROLL|WS_VISIBLE|
            WINDOW_STYLE((ES_LEFT|ES_WANTRETURN|ES_AUTOHSCROLL|ES_AUTOVSCROLL|ES_MULTILINE) as u32);
        let edit = Window::create_window(
            Some(hwnd),
            "edit",
            "",
            WINDOW_EX_STYLE(0),
            dwstyle,
            x, y, 0, 0,
            Some(ID_EDIT)
        )?;
        let hfont = LOG_FONT.as_handle()?;
        Window::set_font(edit, hfont);

        Window::set_lr_margin(edit, 5);
        Self::reset_edit_size(hwnd, Some(edit));
        Window::update_window(hwnd);
        Ok(Self {hwnd, edit, visible})
    }

    fn create() -> UWindowResult<HWND> {
        let class_name = Window::get_class_name("UWSCR.LogPrintWin", &LOGPRINTWIN_CLASS, Some(Self::wndproc))?;
        let title = LOGPRINTWIN_TITLE.as_str();
        let hwnd = Window::create_window(
            None,
            &class_name,
            title,
            WINDOW_EX_STYLE(0),
            WS_OVERLAPPEDWINDOW,
            100,
            100,
            600,
            480,
            None
        );
        hwnd
    }
    fn reset_edit_size(hwnd: HWND, h_edit: Option<HWND>) {
        let rect = Window::get_client_rect(hwnd);
        let edit = h_edit.unwrap_or(Window::get_dlg_item(hwnd, ID_EDIT));
        Window::move_window(edit, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top);
    }
    pub fn print(&self, mut message: String) {
        if self.visible {
            self.show();
        }
        unsafe {
            Self::reset_edit_size(self.hwnd, Some(self.edit));
            let l = GetWindowTextLengthW(self.edit);
            SetFocus(self.edit);
            SendMessageW(self.edit, EM_SETSEL, WPARAM(l as usize), LPARAM(l as isize));
            message.push('\r');
            message.push('\n');
            let hmsg = HSTRING::from(message);
            let lparam = LPARAM(hmsg.as_ptr() as isize);
            SendMessageW(self.edit, EM_REPLACESEL, WPARAM(0), lparam);
        }
    }
    pub fn close(&self) {
        unsafe {
            // DestroyWindow(self.hwnd);
            SendMessageW(self.hwnd, WM_QUIT, WPARAM(0), LPARAM(self.hwnd.0));
        }
    }
    pub fn set_visibility(&mut self, visible: bool, show_now: bool) {
        self.visible = visible;
        if visible {
            if show_now {
                self.show();
            }
        } else {
            Window::hide(self.hwnd);
        }
    }
    pub fn move_to(&self, left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>) {
        let rect = Window::get_window_rect(self.hwnd);
        let x = left.unwrap_or(rect.left);
        let y = top.unwrap_or(rect.top);
        let w = width.unwrap_or(rect.right - rect.left);
        let h = height.unwrap_or(rect.bottom - rect.top);
        Window::move_window(self.hwnd, x, y, w, h);
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl UWindow<()> for LogPrintWin {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }


    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                // PostQuitMessage(0);
                return LRESULT(0);
            },
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            },
            WM_SYSCOMMAND => match (wparam.0 & 0xFFF0) as u32 {
                SC_CLOSE => {
                    ShowWindow(hwnd, SW_HIDE);
                    return LRESULT(0);
                },
                _ => {}
            },
            WM_SIZE => {
                Self::reset_edit_size(hwnd, None);
            },
            _ => {}
        }
        DefWindowProcW(hwnd, umsg, wparam, lparam)
    }

    fn message_loop(&self) -> UWindowResult<()> {
        unsafe {
            let mut msg = MSG::default();
            let result = loop {
                if GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    match msg.message {
                        _ => {}
                    }
                } else {
                    if self.hwnd.0 == msg.lParam.0 {
                        break ();
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };
            Ok(result)
        }
    }
}