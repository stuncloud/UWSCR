pub mod mainwin;
pub use mainwin::*;
pub mod msgbox;
pub use msgbox::*;
pub mod input;
pub use input::*;
pub mod print;
pub use print::*;
pub mod slctbox;
pub use slctbox::*;
pub mod popupmenu;
pub use popupmenu::*;
pub mod balloon;
pub use balloon::*;

use crate::winapi::{to_wide_string, WString, PcwstrExt, from_wide_string};
use crate::write_locale;
use crate::error::{CURRENT_LOCALE, Locale};
use crate::settings::USETTINGS;

pub use windows::{
    core::{PWSTR, PCWSTR},
    Win32::{
        Foundation::{
            HWND,WPARAM,LPARAM,LRESULT,
            HINSTANCE, SIZE, BOOL, RECT, POINT,
            GetLastError,
        },
        UI::{
            WindowsAndMessaging::{
                WNDCLASSEXW, WNDPROC, MSG, HMENU,
                HICON, HCURSOR, SYS_COLOR_INDEX,
                IDI_APPLICATION, IDI_ASTERISK, IDC_ARROW,
                WM_DESTROY, WM_COMMAND, WM_CLOSE, WM_KEYDOWN, WM_KEYUP, WM_SIZE, WM_SETFONT, WM_GETDLGCODE, WM_SYSCOMMAND, WM_QUIT, WM_CTLCOLORSTATIC, WM_LBUTTONDOWN,
                BM_CLICK,
                CS_HREDRAW, CS_VREDRAW,
                SW_SHOW, SW_HIDE,
                SET_WINDOW_POS_FLAGS, SWP_NOMOVE, SWP_NOSIZE, SWP_DRAWFRAME,
                WINDOW_STYLE ,WS_OVERLAPPED, WS_OVERLAPPEDWINDOW, WS_CAPTION, WS_VISIBLE, WS_TABSTOP,WS_SYSMENU, WS_CHILD, WS_GROUP, WS_BORDER, WS_VSCROLL,
                WINDOW_EX_STYLE, WS_EX_TOPMOST,
                BN_CLICKED,
                KF_REPEAT,
                ES_MULTILINE,ES_WANTRETURN, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_LOWERCASE, ES_UPPERCASE, ES_LEFT, ES_PASSWORD,
                EC_LEFTMARGIN, EC_RIGHTMARGIN,
                COLOR_BACKGROUND, COLOR_WINDOW,
                GWLP_WNDPROC,
                SC_CLOSE,
                RegisterClassExW, CreateWindowExW,
                ShowWindow, CloseWindow, DestroyWindow,
                PostQuitMessage,
                UnregisterClassW, IsDialogMessageW,
                LoadIconW, LoadCursorW,
                DefWindowProcW, DefDlgProcW,
                SendMessageW, GetMessageW, TranslateMessage, DispatchMessageW, PostMessageW,
                CallWindowProcW,
                GetClassInfoExW, SetWindowPos, MoveWindow,
                GetSystemMetrics, SM_CXSIZEFRAME, SM_CYSIZEFRAME, SM_CXSCREEN, SM_CYSCREEN,
                GetWindowRect, GetClientRect, FindWindowExW, GetDlgItem, GetDlgCtrlID,
                GetWindowTextW, GetWindowTextLengthW, SetWindowTextW,
                IsWindow,
                // slctbox
                LBS_NOTIFY, LBS_MULTIPLESEL,
                LB_ADDSTRING, LB_GETSELCOUNT, LB_GETSELITEMS,
                CBS_DROPDOWNLIST, CBS_AUTOHSCROLL,
                CB_ADDSTRING, CB_SETCURSEL, CB_GETCURSEL,
                BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON,
                BM_SETCHECK, BM_GETCHECK,
                PeekMessageW, PM_REMOVE,
                // poppumenu
                GetCursorPos, CreatePopupMenu, TrackPopupMenu, AppendMenuW, SetForegroundWindow,
                TPM_TOPALIGN,TPM_RETURNCMD,TPM_NONOTIFY,
                MF_POPUP, MF_ENABLED, MF_STRING,
                // balloon
                WS_EX_TOOLWINDOW,WS_POPUP,
            },
            Input::KeyboardAndMouse::{
                VIRTUAL_KEY, VK_TAB, VK_ESCAPE, VK_RETURN, VK_SHIFT, VK_RIGHT, VK_LEFT, VK_DOWN, VK_UP,
                SetFocus, GetFocus,
            },
            Controls::{
                EM_SETMARGINS, EM_GETRECT, EM_SETRECT, EM_SETSEL, EM_REPLACESEL,
                BST_CHECKED,
                // progress bar
                PBM_SETRANGE32, PBM_SETSTEP, PBM_STEPIT, PBM_SETPOS, PBM_SETMARQUEE,
                PBS_SMOOTH,
            }
        },
        Graphics::Gdi::{
            HBRUSH, HDC, HFONT,
            FW_DONTCARE,CHARSET_UNICODE, OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY,
            DEFAULT_PITCH, FF_DONTCARE,
            GetDC, ReleaseDC, SelectObject,
            GetTextExtentPoint32W, CreateFontW,
            UpdateWindow, SetBkColor,
            // balloon
            PAINTSTRUCT, TEXTMETRICW,
            TRANSPARENT,
            CreateSolidBrush, BeginPaint, FillRect, SetBkMode, SetTextColor, GetTextMetricsW, TextOutW, EndPaint,
        },
        System::{
            Diagnostics::Debug::{
                FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
            },
            SystemServices::{ LANG_NEUTRAL, SUBLANG_DEFAULT }
        }
    }
};

pub use once_cell::sync::{OnceCell, Lazy};

pub static FONT_FAMILY: Lazy<FontFamily> = Lazy::new(|| {
    let usettings = USETTINGS.lock().unwrap();
    FontFamily::new(&usettings.options.default_font.name, usettings.options.default_font.size)
});

type WindowProc = unsafe extern "system" fn(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;

#[derive(Debug)]
pub struct Window {}

impl Window {
    pub fn show(hwnd: HWND) {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
        }
    }
    pub fn hide(hwnd: HWND) {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }

    pub fn create_font(font_family: &str, font_size: i32) -> UWindowResult<HFONT> {
        unsafe {
            let hfont = CreateFontW(
                font_size,
                0,
                0,
                0,
                FW_DONTCARE as i32,
                false.into(),
                false.into(),
                false.into(),
                CHARSET_UNICODE.0,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                DEFAULT_QUALITY,
                FF_DONTCARE,
                font_family.to_wide_null_terminated().to_pcwstr()
            );
            if hfont.is_invalid() {
                Err(UWindowError::FailedToCreateFont(font_family.into()))
            } else {
                Ok(hfont)
            }
        }
    }

    #[allow(non_snake_case)]
    fn register_class(class_name: &str, wndproc: WNDPROC, color: Option<SYS_COLOR_INDEX>) -> UWindowResult<u16> {
        unsafe {
            let wide = to_wide_string(class_name);
            let hInstance = HINSTANCE::default();
            let hbrBackground = match color {
                Some(index) => HBRUSH(index.0 as isize),
                None => HBRUSH(COLOR_WINDOW.0 as isize)
            };
            let hIcon = LoadIconW(hInstance, IDI_APPLICATION)
                .map_err(|e| UWindowError::FailedToRegisterClass(class_name.into(), e.to_string()))?;
            let hCursor = LoadCursorW(hInstance, IDC_ARROW)
                .map_err(|e| UWindowError::FailedToRegisterClass(class_name.into(), e.to_string()))?;
            let hIconSm = LoadIconW(hInstance, IDI_APPLICATION)
                .map_err(|e| UWindowError::FailedToRegisterClass(class_name.into(), e.to_string()))?;
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: wndproc,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance,
                hIcon,
                hCursor,
                hbrBackground,
                lpszMenuName: PCWSTR::null(),
                lpszClassName: PCWSTR(wide.as_ptr()),
                hIconSm,
            };
            let n = RegisterClassExW(&wc);
            Ok(n)
        }
    }
    // 初回のみクラス登録を行い成功すればクラス名を返す
    fn get_class_name(class_name: &str, once_cell: &OnceCell<Result<String, UWindowError>>, wndproc: WNDPROC) -> UWindowResult<String> {
        once_cell.get_or_init(|| {
            if Window::register_class(class_name, wndproc, None)? == 0 {
                Err(UWindowError::FailedToRegisterClass(class_name.into(), "RegisterClassExW has failed".into()))
            } else {
                Ok(class_name.into())
            }
        }).clone()
    }
    fn create_window(
        parent: Option<HWND>,
        class_name: &str,
        title: Option<&str>,
        dwexstyle: WINDOW_EX_STYLE,
        dwstyle: WINDOW_STYLE,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: Option<i32>
    ) -> UWindowResult<HWND> {
        unsafe {
            let hmenu = id.map(|id| HMENU(id as isize));
            let lpwindowname = if let Some(s) = title {
                s.to_wide_null_terminated().to_pcwstr()
            } else {
                PCWSTR::null()
            };
            let lpclassname = class_name.to_wide_null_terminated().to_pcwstr();
            let hwnd = CreateWindowExW(
                dwexstyle,
                lpclassname,
                lpwindowname,
                dwstyle,
                x,
                y,
                width,
                height,
                parent,
                hmenu,
                None,
                std::ptr::null()
            );
            if hwnd.0 == 0 {
                let err = SystemError::new();
                Err(UWindowError::FailedToCreateWindow(class_name.into(), err))
            } else {
                Ok(hwnd)
            }
        }
    }
    fn create_panel(parent: HWND, rect: Option<RECT>, proc: Option<WindowProc>, id: Option<i32>) -> UWindowResult<HWND> {
        let (x, y, width, height) = match rect {
            Some(r) => (r.left, r.top, r.right - r.left, r.bottom - r.top),
            None => (0,0,100,100)
        };
        let hwnd = Window::create_window(
            Some(parent),
            "static",
            None,
            WINDOW_EX_STYLE(0),
            WS_CHILD|WS_VISIBLE,
            x,
            y,
            width,
            height,
            id
        )?;
        if let Some(p) = proc {
            Self::set_subclass(hwnd, p);
        }
        Ok(hwnd)
    }
    fn set_subclass(hwnd: HWND, proc: WindowProc) {
        unsafe {
            #[cfg(target_arch="x86_64")]
            {
                use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
                let dwnewlong = proc as *const WindowProc as isize;
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, dwnewlong);
            }
            #[cfg(target_arch="x86")]
            {
                use windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;
                let dwnewlong = proc as *const WindowProc as i32;
                SetWindowLongW(hwnd, GWLP_WNDPROC, dwnewlong);
            }
        }
    }
    fn send_message(hwnd: HWND, msg: u32, wparam: Option<usize>, lparam: Option<isize>) -> LRESULT {
        unsafe {
            let wparam = WPARAM(wparam.unwrap_or_default());
            let lparam = LPARAM(lparam.unwrap_or_default());
            SendMessageW(hwnd, msg, wparam, lparam)
        }
    }
    fn set_font(hwnd: HWND, hfont: HFONT) {
        unsafe {
            let wparam = WPARAM(hfont.0 as usize);
            SendMessageW(hwnd, WM_SETFONT, wparam, LPARAM(1));
        }
    }
    fn set_window_pos(hwnd: HWND, x: i32, y: i32, size: SIZE, flags: Option<SET_WINDOW_POS_FLAGS>) {
        unsafe {
            let uflags = flags.unwrap_or_default();
            SetWindowPos(hwnd, HWND::default(), x, y, size.cx, size.cy, uflags);
        }
    }
    fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            MoveWindow(hwnd, x, y, width, height, true);
        }
    }
    fn set_child(parent: HWND, class_name: &str, title: &str, x: i32, y: i32, size_opt: Option<SizeOption>, font: Option<HFONT>, styles: Option<WINDOW_STYLE>, id: Option<i32>) -> UWindowResult<Child> {
        let dwstyle = WS_CHILD|WS_VISIBLE|styles.unwrap_or_default();
        let hwnd = Self::create_window(
            Some(parent), class_name, Some(title), WINDOW_EX_STYLE(0), dwstyle, x, y, 0, 0, id
        )?;
        match font {
            Some(hfont) => {
                Self::set_font(hwnd, hfont);
            },
            None => {}
        }
        let mut size = SIZE::default();
        for line in title.lines() {
            let l_size = Self::get_string_size(line, hwnd, font);
            size.cx = size.cx.max(l_size.cx);
            size.cy += l_size.cy;
        }
        if let Some(opt) = size_opt {
            size.cx += opt.margin_x * 2;
            size.cy += opt.margin_y * 2;
            size.cx = size.cx.max(opt.min_width);
            size.cy = size.cy.max(opt.min_height);
        }

        let mut child = Child::from(hwnd);
        child.move_to(Some(x), Some(y), Some(size.cx), Some(size.cy));
        Ok(child)
    }
    pub fn set_label(parent: HWND, title: &str, x: i32, y: i32, font: Option<HFONT>, styles: Option<WINDOW_STYLE>) -> UWindowResult<Child> {
        Self::set_child(parent, "static", title, x, y, None, font, styles, None)
    }
    pub fn set_button(parent: HWND, title: &str, x: i32, y: i32, btn_type: i32, font: Option<HFONT>, styles: Option<WINDOW_STYLE>) -> UWindowResult<Child> {
        let opt = SizeOption {
            margin_x: 0,
            margin_y: 0,
            min_width: 100,
            min_height: 30,
        };
        let styles = styles.unwrap_or_default() | WS_TABSTOP;
        let mut btn = Self::set_child(parent, "button", title, x, y, Some(opt), font, Some(styles), Some(btn_type))?;
        btn.ctype = Some(ChildType::Button(btn_type));
        Ok(btn)
    }
    fn get_string_size(str: &str, hwnd: HWND, font: Option<HFONT>) -> SIZE {
        unsafe {
            let mut size = SIZE::default();
            let wide = to_wide_string(str);
            let hdc = GetDC(hwnd);
            let oldobj = if let Some(hfont) = font {
                let obj = SelectObject(hdc, hfont);
                Some(obj)
            } else {
                None
            };
            GetTextExtentPoint32W(hdc, &wide, &mut size);
            if let Some(obj) = oldobj {
                SelectObject(hdc, obj);
            }
            ReleaseDC(hwnd, hdc);
            size
        }
    }
    fn _set_margin(hwnd: HWND, left: i32, top: i32, right: i32, bottom: i32) {
        unsafe {
            let mut rect = RECT::default();
            let prect = &mut rect as *mut RECT as isize;
            SendMessageW(hwnd, EM_GETRECT, WPARAM::default(), LPARAM(prect));

            rect.left += left;
            rect.right += right;
            rect.top += top;
            rect.bottom += bottom;
            SendMessageW(hwnd, EM_SETRECT, WPARAM::default(), LPARAM(prect));
        }
    }
    pub fn set_lr_margin(hwnd: HWND, margin: i32) {
        unsafe {
            let wparam = (EC_LEFTMARGIN|EC_RIGHTMARGIN) as usize;
            let lparam = (margin * 0x1000 + margin) as isize;
            SendMessageW(hwnd, EM_SETMARGINS, WPARAM(wparam), LPARAM(lparam));
        }
    }
    fn get_window_margin(hwnd: HWND) -> (i32, i32) {
        let mut wrect = RECT::default();
        let mut crect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut wrect);
            GetClientRect(hwnd, &mut crect);
        }
        let w = (wrect.right - wrect.left) - crect.right;
        let h = (wrect.bottom - wrect.top) - crect.bottom;
        (w, h)
    }
    fn calculate_width(width_list: Vec<i32>, max_width: i32) -> i32 {
        let new_width = width_list.into_iter()
                                .reduce(|a,b| a.max(b))
                                .unwrap();
        new_width.min(max_width)
    }
    fn calculate_center(window_width: i32, btn_width: i32) -> i32 {
        window_width / 2 - btn_width / 2
    }
    fn calculate_center_pos(width: i32, height: i32) -> (i32, i32) {
        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let x = screen_w / 2 - width / 2;
            let y = screen_h / 2 - height / 2;
            (x, y)
        }
    }
    fn get_window_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect);
            rect
        }
    }
    fn get_client_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect);
            rect
        }
    }
    // fn is_point_in_rect(point: POINT, rect: RECT) -> bool {
    //     rect.left <= point.x &&
    //     point.x <= rect.right &&
    //     rect.top <= point.y &&
    //     point.y <= rect.bottom
    // }
    fn focus(hwnd: HWND) {
        unsafe {
            SetFocus(hwnd);
        }
    }
    fn get_edit_text(hwnd: HWND) -> String {
        unsafe {
            let nmaxcount = GetWindowTextLengthW(hwnd) + 1;
            let mut buf: Vec<u16> = Vec::with_capacity(nmaxcount as usize);
            buf.set_len(nmaxcount as usize);
            // buf.resize(nmaxcount as usize, 0);
            GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf)
                .trim_end_matches('\0').to_string()
        }
    }
    fn _get_dlg_ctrl_id(hwnd: HWND) -> i32 {
        unsafe {
            GetDlgCtrlID(hwnd)
        }
    }
    fn get_dlg_item(hwnd: HWND, id: i32) -> HWND {
        unsafe {
            GetDlgItem(hwnd, id)
        }
    }
    fn update_window(hwnd: HWND) {
        unsafe {
            UpdateWindow(hwnd);
        }
    }
    fn create_solid_brush(color: u32) -> HBRUSH {
        unsafe {
            CreateSolidBrush(color)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Child {
    hwnd: HWND,
    pub size: SIZE,
    pub x: i32,
    pub y: i32,
    pub ctype: Option<ChildType>
}
impl Child {
    pub fn _new(hwnd: HWND, size: SIZE, x: i32, y: i32, ctype: Option<ChildType>) -> Self {
        Self { hwnd, size, x, y, ctype }
    }
    pub fn move_to(&mut self, x: Option<i32>, y: Option<i32>, width: Option<i32>, height: Option<i32>) {
        let x = x.unwrap_or(self.x);
        let y = y.unwrap_or(self.y);
        let width = width.unwrap_or(self.size.cx);
        let height = height.unwrap_or(self.size.cy);
        Window::move_window( self.hwnd, x, y, width, height);
        let rect = Window::get_window_rect(self.hwnd);
        self.size = SIZE {
            cx: rect.right - rect.left,
            cy: rect.bottom - rect.top
        };
        self.x = x;
        self.y = y;
    }
}
impl Default for Child {
    fn default() -> Self {
        Self {
            hwnd: HWND::default(),
            size: SIZE::default(),
            x: 0,
            y: 0,
            ctype: None,
        }
    }
}
impl From<HWND> for Child {
    fn from(hwnd: HWND) -> Self {
        let rect = Window::get_window_rect(hwnd);
        let size = SIZE {
            cx: rect.right - rect.left,
            cy: rect.bottom - rect.top
        };
        Self {
            hwnd,
            size,
            x: rect.left,
            y: rect.top,
            ctype: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChildType {
    Button(i32)
}

struct SizeOption {
    pub margin_x: i32,
    pub margin_y: i32,
    pub min_width: i32,
    pub min_height: i32,
}

pub trait UWindow<T: Default> {
    fn hwnd(&self) -> HWND;
    fn show(&self) {
        Window::show(self.hwnd());
    }
    fn message_loop(&self) -> UWindowResult<T> {
        unsafe {
            let mut msg = MSG::default();
            let result = loop {
                if GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    match msg.message {
                        _ => {}
                    }
                } else {
                    break T::default()
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };
            Ok(result)
        }
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(hwnd.0));
                LRESULT(0)
            },
            msg => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
    unsafe extern "system"
    fn subclass(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        CallWindowProcW(Some(Self::wndproc), hwnd, umsg, wparam, lparam)
    }
}

pub type UWindowResult<T> = std::result::Result<T, UWindowError>;

#[derive(Debug, Clone, PartialEq)]
pub enum UWindowError {
    /// class名, エラーメッセージ
    FailedToCreateWindow(String, SystemError),
    FailedToRegisterClass(String, String),
    FailedToCreateFont(String),
    SlctBoxIndexOverFlowed(i32),
    SlctBoxInvalidIndex(i32),
    FailedToCreatePopupMenu(String),
}

impl std::fmt::Display for UWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UWindowError::FailedToCreateWindow(cls, e) => write_locale!(f,
                "ウィンドウ({cls})の作成に失敗: {e}",
                "Failed to create window: {cls}",
            ),
            UWindowError::FailedToRegisterClass(cls, reason) => write_locale!(f,
                "クラス登録に失敗: {}, {}",
                "Failed to register class: {}, {}",
                cls, reason
            ),
            UWindowError::FailedToCreateFont(font) => write_locale!(f,
                "フォント名が不正: {}",
                "Invalid font family: {}",
                font
            ),
            UWindowError::SlctBoxIndexOverFlowed(size) => write_locale!(f,
                "要素数過多({}): slctboxの要素数は31までです",
                "Too many items: {}, should be less than 32",
                size
            ),
            UWindowError::SlctBoxInvalidIndex(index) => write_locale!(f,
                "不正なインデックス({}): 該当するアイテムが見つかりません",
                "Invalid index: {}, no item found",
                index
            ),
            UWindowError::FailedToCreatePopupMenu(e) => write_locale!(f,
                "ポップアップメニューの作成に失敗: {}",
                "Failed to create popup menu: {}",
                e
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontFamily {
    pub name: String,
    pub size: i32
}
impl FontFamily {
    pub fn new(name: &str, size: i32) -> Self {
        Self {name: name.into(), size}
    }
    fn new2(name: Option<String>, size: Option<i32>) -> Self {
        let name = name.unwrap_or("Yu Gothic UI".into());
        let size = size.unwrap_or(15);
        Self::new(&name, size)
    }
    pub fn as_handle(&self) -> UWindowResult<HFONT> {
        Window::create_font(&self.name, self.size)
    }
}
impl Default for FontFamily {
    fn default() -> Self {
        Self::new("Yu Gothic UI", 15)
    }
}
impl From<(Option<String>, Option<i32>)> for FontFamily {
    fn from((name, size): (Option<String>, Option<i32>)) -> Self {
        Self::new2(name, size)
    }
}

pub trait WparamExt {
    fn hi_word(&self) -> usize;
    fn lo_word(&self) -> usize;
}

impl WparamExt for WPARAM {
    fn hi_word(&self) -> usize {
        (self.0 & 0xFFFF0000) / 0x10000
    }
    fn lo_word(&self) -> usize {
        self.0 & 0xFFFF
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemError {
    code: u32,
    msg: String,
}
impl SystemError {
    fn new() -> Self {
        unsafe {
            let code = GetLastError().0;
            let mut buf = [0; 512];
            FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM|FORMAT_MESSAGE_IGNORE_INSERTS,
                std::ptr::null(),
                code,
                SUBLANG_DEFAULT << 10 | LANG_NEUTRAL,
                PWSTR::from_raw(buf.as_mut_ptr()),
                buf.len() as u32,
                std::ptr::null()
            );
            let msg = from_wide_string(&buf);
            Self { code, msg }
        }
    }
}
impl std::fmt::Display for SystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.msg)
    }
}