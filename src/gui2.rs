pub mod msgbox;
pub mod slctbox;
pub mod input;
pub mod popupmenu;
pub mod print;
pub mod balloon;
pub mod form;

pub use msgbox::*;
pub use slctbox::*;
pub use input::*;
pub use popupmenu::*;
pub use print::*;
pub use balloon::*;
pub use form::*;

use crate::write_locale;
use crate::error::{CURRENT_LOCALE, Locale};

use windows::{
    core::{self, w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{HWND, WPARAM, LPARAM, LRESULT, SIZE, POINT, RECT},
        UI::{
            WindowsAndMessaging::{
                self as wm,
                WINDOW_STYLE, WS_CHILD, WS_VISIBLE, WS_TABSTOP,  GetWindowRect,
            },
            Input::KeyboardAndMouse as km,
            Controls::{
                InitCommonControlsEx, INITCOMMONCONTROLSEX,
                ICC_LINK_CLASS,
                WC_BUTTONW, WC_STATICW, WC_LINK,
            },
        },
        Graphics::{
            Gdi,
            Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
        },
        System::LibraryLoader::GetModuleHandleW,
    }
};
use std::ffi::c_void;
use std::sync::{Once, OnceLock};

static INIT_COMMON_CONTROL: Once = Once::new();

type WindowProc = unsafe extern "system" fn(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
pub type UWindowResult<T> = std::result::Result<T, UWindowError>;
#[derive(Debug, Clone, PartialEq)]
pub enum UWindowError {
    Win32(core::Error),
    ClassRegistrationError(String),
    CreateFontError(String),
    CreateWindowError,
    SlctboxGotTooManyItems,
    SlctItemOutOfBounds,
    PopupMenuCreateError,
    PopupMenuAppendError,
}
impl std::fmt::Display for UWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UWindowError::Win32(e) => write!(f, "{e}"),
            UWindowError::ClassRegistrationError(c) => write_locale!(f,
                "ウィンドウクラスの作成に失敗: {c}",
                "Failed to create window class: {c}"
            ),
            UWindowError::CreateFontError(font) => write_locale!(f,
                "フォント名が不正: {font}",
                "Invalid font name: {font}"
            ),
            UWindowError::CreateWindowError => write_locale!(f,
                "ウィンドウの作成に失敗",
                "Failed to create window"
            ),
            UWindowError::SlctboxGotTooManyItems => write_locale!(f,
                "slctboxの要素数が多すぎます",
                "Too many items given to slctbox"
            ),
            UWindowError::SlctItemOutOfBounds => write_locale!(f,
                "インデックスが不正です",
                "Index is out of bounds"
            ),
            UWindowError::PopupMenuCreateError => write_locale!(f,
                "ポップアップメニューの作成に失敗しました",
                "Failed to create popup menu"
            ),
            UWindowError::PopupMenuAppendError => write_locale!(f,
                "ポップアップメニューに要素を追加できませんでした",
                "Failed to append item to popup menu"
            ),
        }
    }
}
impl From<core::Error> for UWindowError {
    fn from(err: core::Error) -> Self {
        Self::Win32(err)
    }
}

pub struct ClassName(PCWSTR);
impl Default for ClassName {
    fn default() -> Self {
        Self(PCWSTR::null())
    }
}
pub struct WindowSize(i32, i32, i32, i32);
impl Default for WindowSize {
    fn default() -> Self {
        Self(wm::CW_USEDEFAULT, wm::CW_USEDEFAULT, wm::CW_USEDEFAULT, wm::CW_USEDEFAULT)
    }
}

#[derive(Default, Debug)]
pub struct DialogResult<T: Default> {
    pub result: T,
    pub point: POINT,
}
impl<T: Default> DialogResult<T> {
    fn new(result: T, point: POINT) -> Self {
        DialogResult { result, point }
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    title: HSTRING,
    class_name: ClassName,
    parent: Option<HWND>,
    ex_style: wm::WINDOW_EX_STYLE,
    style: wm::WINDOW_STYLE,
    size: WindowSize,
    menu: wm::HMENU,
    lpparam: Option<*const c_void>,
}
impl WindowBuilder {
    pub fn new(title: &str, class_name: PCWSTR) -> Self {
        Self {
            title: HSTRING::from(title),
            class_name: ClassName(class_name),
            ..Default::default()
        }
    }
    pub fn parent(mut self, parent: HWND) -> Self {
        self.parent = Some(parent);
        self
    }
    pub fn ex_style(mut self, style: wm::WINDOW_EX_STYLE) -> Self {
        self.ex_style = style;
        self
    }
    pub fn style(mut self, style: wm::WINDOW_STYLE) -> Self {
        self.style = style;
        self
    }
    pub fn size(mut self, x: Option<i32>, y: Option<i32>, width: Option<i32>, height: Option<i32>) -> Self {
        self.size = WindowSize(
            x.unwrap_or(wm::CW_USEDEFAULT),
            y.unwrap_or(wm::CW_USEDEFAULT),
            width.unwrap_or(wm::CW_USEDEFAULT),
            height.unwrap_or(wm::CW_USEDEFAULT),
        );
        self
    }
    pub fn menu(mut self, menu: isize) -> Self {
        self.menu = wm::HMENU(menu);
        self
    }
    pub fn lpparam(mut self, ptr: *const c_void) -> Self {
        self.lpparam = Some(ptr);
        self
    }
    pub fn build(self) -> UWindowResult<HWND> {
        unsafe {
            let WindowSize(x, y, nwidth, nheight) = self.size;
            let hwnd = wm::CreateWindowExW(
                self.ex_style,
                self.class_name.0,
                &self.title,
                self.style,
                x, y, nwidth, nheight,
                self.parent.as_ref(),
                self.menu,
                GetModuleHandleW(None)?,
                self.lpparam,
            );
            if hwnd.0 == 0 {
                Err(UWindowError::CreateWindowError)
            } else {
                Ok(hwnd)
            }
        }
    }
}


#[derive(Debug, Clone)]
pub struct FontFamily {
    pub name: HSTRING,
    pub size: i32
}
impl FontFamily {
    pub fn new(name: &str, size: i32) -> Self {
        Self {name: HSTRING::from(name), size}
    }
    pub fn create(&self) -> UWindowResult<Gdi::HFONT> {
        unsafe {
            let hfont = Gdi::CreateFontW(
                self.size,
                0,
                0,
                0,
                Gdi::FW_DONTCARE.0 as i32,
                0,
                0,
                0,
                Gdi::CHARSET_UNICODE.0,
                Gdi::OUT_TT_PRECIS.0 as u32,
                Gdi::CLIP_DEFAULT_PRECIS.0 as u32,
                Gdi::DEFAULT_QUALITY.0 as u32,
                Gdi::FF_DONTCARE.0 as u32,
                &self.name
            );
            (! hfont.is_invalid())
                .then_some(hfont)
                .ok_or(UWindowError::CreateFontError(self.name.to_string()))
        }
    }
}
impl Default for FontFamily {
    fn default() -> Self {
        Self::new("Yu Gothic UI", 20)
    }
}
impl From<(Option<String>, Option<i32>)> for FontFamily {
    fn from(value: (Option<String>, Option<i32>)) -> Self {
        let mut font = Self::default();
        if let Some(name) = value.0 {
            font.name = HSTRING::from(name);
        }
        if let Some(size) = value.1 {
            font.size = size;
        }
        font
    }
}

pub trait UWindow<T> {
    const CLASS_NAME: PCWSTR;

    fn create_window(title: &str) -> UWindowResult<HWND>;
    fn draw(&self) -> UWindowResult<()>;
    fn hwnd(&self) -> HWND;
    fn font(&self) -> Gdi::HFONT;

    fn init() {
        INIT_COMMON_CONTROL.call_once(|| unsafe {
            let icce = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_LINK_CLASS,
            };
            InitCommonControlsEx(&icce);
        });
    }
    fn fix_aero_rect(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) -> (i32, i32, i32, i32) {
        unsafe {
            let mut drect = RECT::default();
            let pvattribute = &mut drect as *mut RECT as *mut c_void;
            let cbattribute = std::mem::size_of::<RECT>() as u32;
            if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_ok() {
                let mut wrect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut wrect);
                let x = x - (drect.left - wrect.left);
                let y = y - (drect.top - wrect.top);
                let w = width - ((drect.right - drect.left) - (wrect.right - wrect.left));
                let h = height - ((drect.bottom - drect.top) - (wrect.bottom - wrect.top));
                (x, y, w, h)
            } else {
                (x, y, width, height)
            }
        }
    }
    fn move_to(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            let (x, y, width, height) = Self::fix_aero_rect(self.hwnd(), x, y, width, height);
            let _ = wm::MoveWindow(self.hwnd(), x, y, width, height, true);
        }
    }
    fn activate(&self) {
        unsafe {
            km::SetFocus(self.hwnd());
            wm::SetForegroundWindow(self.hwnd());
        }
    }
    fn get_monitor_center() -> POINT {
        unsafe {
            let active = wm::GetForegroundWindow();
            let hmonitor = Gdi::MonitorFromWindow(active, Gdi::MONITOR_DEFAULTTONEAREST);
            let mut mi = Gdi::MONITORINFO::default();
            mi.cbSize = std::mem::size_of_val(&mi) as u32;
            Gdi::GetMonitorInfoW(hmonitor, &mut mi);
            POINT {
                x: (mi.rcWork.right - mi.rcWork.left) / 2 + mi.rcWork.left,
                y: (mi.rcWork.bottom - mi.rcWork.top) / 2 + mi.rcWork.top
            }
        }
    }
    fn get_center_pos(width: i32, height: i32) -> POINT {
        let mut center = Self::get_monitor_center();
        center.x -= width / 2;
        center.y -= height / 2;
        center
    }
    fn get_rect(&self) -> Option<RECT> {
        unsafe {
            let mut rect = RECT::default();
            wm::GetWindowRect(self.hwnd(), &mut rect)
                .and_then(|_| Ok(rect))
                .ok()
        }
    }
    fn get_pos(&self) -> Option<POINT> {
        self.get_rect()
            .map(|r| POINT { x: r.left, y: r.top })
    }
    fn title_bar_height(&self) -> i32 {
        unsafe {
            wm::GetSystemMetrics(wm::SM_CYCAPTION)
        }
    }
    fn get_client_wh(&self) -> (i32, i32) {
        unsafe {
            let mut rect = RECT::default();
            let _ = wm::GetClientRect(self.hwnd(), &mut rect);
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            (w, h)
        }
    }
    fn show(&self) {
        unsafe {
            wm::ShowWindow(self.hwnd(), wm::SW_SHOW);
        }
    }
    fn hide(&self) {
        unsafe {
            wm::ShowWindow(self.hwnd(), wm::SW_HIDE);
        }
    }
    fn message_loop(&self) -> UWindowResult<T>
        where T: Default
    {
        unsafe {
            let mut msg = wm::MSG::default();
            let hwnd = HWND::default();
            let result = loop {
                match wm::GetMessageW(&mut msg, hwnd, 0, 0).0 {
                    -1 => {
                        break Err(UWindowError::Win32(core::Error::from_win32()));
                    },
                    0 => {
                        break Ok(Default::default());
                    },
                    _ => match msg.message {
                        _ => {}
                    }
                };
                if ! wm::IsDialogMessageW(self.hwnd(), &msg).as_bool() {
                    wm::TranslateMessage(&msg);
                    wm::DispatchMessageW(&msg);
                }
            };
            result
        }
    }
    fn register_window_class(once: &OnceLock<UWindowResult<()>>) -> UWindowResult<()> {
        unsafe {
            once.get_or_init(|| {
                let hinstance = GetModuleHandleW(None)
                    .map(|hmod| hmod.into())?;
                let wc = wm::WNDCLASSEXW {
                    cbSize: std::mem::size_of::<wm::WNDCLASSEXW>() as u32,
                    style: wm::CS_HREDRAW|wm::CS_VREDRAW,
                    lpfnWndProc: Some(Self::wndproc),
                    hInstance: hinstance,
                    hIcon: wm::LoadIconW(hinstance, PCWSTR(1 as _))?,
                    lpszClassName: Self::CLASS_NAME,
                    ..Default::default()
                };
                match wm::RegisterClassExW(&wc) {
                    0 => Err(UWindowError::ClassRegistrationError(Self::CLASS_NAME.to_string().unwrap_or_default())),
                    _ => Ok(())
                }
            }).clone()
        }
    }
    fn register_dlg_class(once: &OnceLock<UWindowResult<()>>) -> UWindowResult<()> {
        unsafe {
            once.get_or_init(|| {
                let hinstance = GetModuleHandleW(None)
                    .map(|hmod| hmod.into())?;
                let wc = wm::WNDCLASSEXW {
                    cbSize: std::mem::size_of::<wm::WNDCLASSEXW>() as u32,
                    style: wm::CS_HREDRAW|wm::CS_VREDRAW,
                    lpfnWndProc: Some(Self::dlgproc),
                    hInstance: hinstance,
                    hIcon: wm::LoadIconW(hinstance, PCWSTR(1 as _))?,
                    lpszClassName: Self::CLASS_NAME,
                    cbWndExtra: wm::DLGWINDOWEXTRA as i32,
                    ..Default::default()
                };
                match wm::RegisterClassExW(&wc) {
                    0 => Err(UWindowError::ClassRegistrationError(Self::CLASS_NAME.to_string().unwrap_or_default())),
                    _ => Ok(())
                }
            }).clone()
        }
    }
    fn set_dlgsubproc(hwnd: HWND, subproc: WindowProc) {
        unsafe {
            let dwnewlong = subproc as *const WindowProc as isize;
            Self::set_window_long(hwnd, wm::GWLP_WNDPROC, dwnewlong);
        }
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            msg => wm::DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
    unsafe extern "system"
    fn dlgproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            wm::WM_CLOSE => {
                let _ = wm::DestroyWindow(hwnd);
                LRESULT(0)
            },
            msg => wm::DefDlgProcW(hwnd, msg, wparam, lparam)
        }
    }
    #[cfg(target_pointer_width="32")]
    unsafe fn set_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> isize {
        wm::SetWindowLongW(hwnd, nindex, dwnewlong as i32) as isize
    }
    #[cfg(target_pointer_width="64")]
    unsafe fn set_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> isize {
        wm::SetWindowLongPtrW(hwnd, nindex, dwnewlong)
    }
    #[cfg(target_pointer_width="32")]
    unsafe fn get_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX) -> isize {
        wm::GetWindowLongW(hwnd, nindex) as isize
    }
    #[cfg(target_pointer_width="64")]
    unsafe fn get_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX) -> isize {
        wm::GetWindowLongPtrW(hwnd, nindex)
    }
    #[cfg(target_pointer_width="32")]
    unsafe fn set_class_long(hwnd: HWND, nindex: wm::GET_CLASS_LONG_INDEX, dwnewlong: isize) -> usize {
        wm::SetClassLongW(hwnd, nindex, dwnewlong as i32) as usize
    }
    #[cfg(target_pointer_width="64")]
    unsafe fn set_class_long(hwnd: HWND, nindex: wm::GET_CLASS_LONG_INDEX, dwnewlong: isize) -> usize {
        wm::SetClassLongPtrW(hwnd, nindex, dwnewlong)
    }

    fn set_font(&self, hwnd: HWND, text: &str) -> SIZE {
        unsafe {
            let font = self.font();
            // フォントを適用
            wm::SendMessageW(hwnd, wm::WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

            // テキスト全体のSIZEを返す
            self.get_text_size(hwnd, text)
        }
    }
    fn get_text_size(&self, hwnd: HWND, text: &str) -> SIZE {
        unsafe {
            let hfont = self.font();
            let mut size = SIZE::default();
            let hdc = Gdi::GetDC(hwnd);
            let old = Gdi::SelectObject(hdc, hfont);
            for line in text.lines() {
                let hstring = HSTRING::from(line);
                let thw = Gdi::GetTabbedTextExtentW(hdc, hstring.as_wide(), Some(&[]));
                let tw = thw & 0xFFFF;
                let th = (thw & 0xFFFF0000) >> 16;
                size.cx = size.cx.max(tw as i32);
                size.cy += th as i32;
            }
            Gdi::SelectObject(hdc, old);
            Gdi::ReleaseDC(hwnd, hdc);
            size
        }
    }

    fn destroy(&self) {
        unsafe {
            let _ = wm::DestroyWindow(self.hwnd());
        }
    }

    /* パーツ */
    fn set_static(&self, title: &str, x: i32, y: i32) -> UWindowResult<ChildCtl<Static>> {
        let hwnd = WindowBuilder::new(title, WC_STATICW)
            .style(WS_CHILD|WS_VISIBLE)
            // .style(WS_CHILD|WS_VISIBLE|WINDOW_STYLE(SS_CENTER.0))
            // .ex_style(wm::WS_EX_STATICEDGE)
            .parent(self.hwnd())
            .build()?;
        let size = self.set_font(hwnd, title);
        let mut child = ChildCtl::new(hwnd, None, self.hwnd(), Static);
        child.move_to(x, y, Some(size.cx), Some(size.cy));
        Ok(child)
    }
    fn set_static_with_link(&self, title: &str, x: i32, y: i32) -> UWindowResult<ChildCtl<Static>> {
        let finder = linkify::LinkFinder::new();
        let mut links = finder.links(title).map(|link| link.as_str()).collect::<Vec<_>>();
        if links.is_empty() {
            self.set_static(title, x, y)
        } else {
            links.sort();
            links.dedup();
            let mut text_with_link = title.to_string();
            for link in links {
                let to = format!("<A HREF=\"{0}\">{0}</A>", link);
                text_with_link = text_with_link.replace(link, &to);
            }
            let hwnd = WindowBuilder::new(&text_with_link, WC_LINK)
                .style(WS_CHILD|WS_VISIBLE)
                .parent(self.hwnd())
                .build()?;
            let size = self.set_font(hwnd, &title);
            let mut child = ChildCtl::new(hwnd, None, self.hwnd(), Static);
            child.move_to(x, y, Some(size.cx), Some(size.cy));
            Ok(child)
        }
    }
    fn set_button(&self, title: &str, x: i32, y: i32, id: isize, default: bool, min_width: i32) -> UWindowResult<ChildCtl<Button>> {
        let parent = self.hwnd();
        let btn_style = if default {
            WINDOW_STYLE(wm::BS_DEFPUSHBUTTON as u32)
        } else {
            WINDOW_STYLE(wm::BS_PUSHBUTTON as u32)
        };
        let hwnd = WindowBuilder::new(title, WC_BUTTONW)
            .style(WS_CHILD|WS_VISIBLE|WS_TABSTOP|btn_style)
            .parent(parent)
            .menu(id)
            .build()?;
        let size = self.set_font(hwnd, title);
        let button = Button(default);
        let mut child = ChildCtl::new(hwnd, Some(id), self.hwnd(), button);
        let nwidth = min_width.max(size.cx + 8);
        let nheight = size.cy + 4;
        child.move_to(x, y, Some(nwidth), Some(nheight));
        Ok(child)
    }
}

pub struct ChildCtl<T: ChildClass + Sized> {
    hwnd: HWND,
    rect: RECT,
    crect: RECT,
    menu: Option<isize>,
    class: T,
    main: HWND,
}
impl<T: ChildClass> std::fmt::Debug for ChildCtl<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildCtl")
        .field("hwnd", &self.hwnd)
        .field("rect", &self.rect)
        .field("crect", &self.crect)
        .field("menu", &self.menu)
        .field("class", &std::any::type_name::<T>().to_string())
        .field("main", &self.main)
        .finish()
    }
}
impl<T: ChildClass> ChildCtl<T> {
    fn new(hwnd: HWND, menu: Option<isize>, main: HWND, class: T) -> Self{
        Self {
            hwnd,
            rect: Self::get_screen_rect(hwnd, main),
            crect: Self::get_client_rect(hwnd),
            menu,
            class,
            main
        }
    }

    fn width(&self) -> i32 {
        self.rect.right - self.rect.left
    }
    fn height(&self) -> i32 {
        self.rect.bottom - self.rect.top
    }
    fn get_screen_rect(hwnd: HWND, parent: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            let _ = wm::GetWindowRect(hwnd, &mut rect);
            let mut point = POINT {
                x: rect.left,
                y: rect.top,
            };
            Gdi::ScreenToClient(parent, &mut point);
            rect.right -= rect.left - point.x;
            rect.bottom -= rect.top - point.y;
            rect.left = point.x;
            rect.top = point.y;
            rect
        }
    }
    fn get_client_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            let _ = wm::GetClientRect(hwnd, &mut rect);
            rect
        }
    }

    fn move_to(&mut self, x: i32, y: i32, width: Option<i32>, height: Option<i32>) {
        unsafe {
            let nwidth = width.unwrap_or(self.width());
            let nheight = height.unwrap_or(self.height());
            let _ = wm::MoveWindow(self.hwnd, x, y, nwidth, nheight, true);
            self.rect = Self::get_screen_rect(self.hwnd, self.main);
            self.crect = Self::get_client_rect(self.hwnd);
        }
    }
    fn focus(&self) {
        unsafe {
            km::SetFocus(self.hwnd);
        }
    }
}
pub trait ChildClass {}

pub struct Static;
impl ChildClass for Static {}
pub struct Button(bool);
impl ChildClass for Button {}
impl Button {
    fn set_default(&mut self) {
        self.0 = true;
    }
    fn is_default(&self) -> bool {
        self.0
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

trait LparamExt {
    fn hi_word(&self) -> u32;
    fn lo_word(&self) -> u32;
}
impl LparamExt for LPARAM {
    fn hi_word(&self) -> u32 {
        let hi = (self.0 >> 16) & 0xFFFF;
        hi as u32
    }

    fn lo_word(&self) -> u32 {
        let lo = self.0 & 0xFFFF;
        lo as u32
    }
}