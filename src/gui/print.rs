use super::*;

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
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let dwstyle = WS_CHILD|WS_VSCROLL|
            WINDOW_STYLE((ES_LEFT|ES_WANTRETURN|ES_AUTOHSCROLL|ES_AUTOVSCROLL|ES_MULTILINE) as u32);
        let edit = Window::create_window(
            Some(hwnd),
            "edit",
            "",
            WINDOW_EX_STYLE(0),
            dwstyle,
            x, y, width, height,
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
    pub fn print(&self, message: &str) {
        if self.visible {
            self.show();
        }
        unsafe {
            let l = GetWindowTextLengthW(self.edit);
            SetFocus(self.edit);
            SendMessageW(self.edit, EM_SETSEL, WPARAM(l as usize), LPARAM(l as isize));
            let mut wide: Vec<u16> = format!("{}\r\n\0", message).encode_utf16().collect();
            let lparam = LPARAM(wide.as_mut_ptr() as isize);
            SendMessageW(self.edit, EM_REPLACESEL, WPARAM(0), lparam);
        }
    }
    pub fn close(&self) {
        unsafe {
            // DestroyWindow(self.hwnd);
            SendMessageW(self.hwnd, WM_QUIT, WPARAM(0), LPARAM(self.hwnd.0));
        }
    }
    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
        if visible {
            self.show();
        }
    }
    pub fn move_to(&self, left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>) {
        let rect = Window::get_window_rect(self.hwnd);
        let (x, y) = if let Some(m) = Monitor::from_hwnd(self.hwnd) {
            let x = if let Some(x) = left {m.to_scaled(x)} else {rect.left};
            let y = if let Some(y) = top {m.to_scaled(y)} else {rect.top};
            (x, y)
        } else {
            let x = left.unwrap_or(rect.left);
            let y = top.unwrap_or(rect.top);
            (x, y)
        };
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