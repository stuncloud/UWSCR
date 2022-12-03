mod acc;
mod clkitem;
mod win32;
mod monitor;

use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::builtins::{
    window_low,
    system_controls::is_64bit_os,
    text_control::ErrConst,
};
pub use monitor::Monitor;

#[cfg(feature="chkimg")]
use crate::{
    settings::USETTINGS,
    evaluator::builtins::chkimg::{ChkImg, ScreenShot},
};

use windows::{
    Win32::{
        Foundation::{
            MAX_PATH,
            BOOL, HANDLE, HINSTANCE,
            HWND, WPARAM, LPARAM, POINT, RECT,
            CloseHandle,
        },
        System::{
            Threading::{
                PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
                OpenProcess, WaitForInputIdle, IsWow64Process,
            },
            ProcessStatus::K32GetModuleFileNameExW,
            SystemServices::CF_BITMAP,
            DataExchange::{
                OpenClipboard, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
            }
        },
        UI::{
            WindowsAndMessaging::{
                SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
                SW_SHOWNORMAL, SW_SHOW, SW_HIDE, SW_MINIMIZE, SW_MAXIMIZE,
                WINDOWPLACEMENT,
                WM_CLOSE, WM_DESTROY, HWND_TOPMOST, HWND_NOTOPMOST,
                WindowFromPoint, GetParent, IsWindowVisible, GetClientRect,
                GetForegroundWindow, GetWindowTextW, GetClassNameW, EnumWindows,
                IsWindow, PostMessageW, SetForegroundWindow, ShowWindow,
                SetWindowPos, GetWindowRect, MoveWindow, GetWindowPlacement,
                GetWindowThreadProcessId, IsIconic, IsHungAppWindow,
                EnumChildWindows, GetMenu, GetSystemMenu,
                GetCursorInfo, CURSORINFO,
            },
            HiDpi::{
                GetDpiForWindow,
            },
            Input::KeyboardAndMouse::{
                SendInput, INPUT
            },
        },
        Graphics::{
            Gdi::{
                ClientToScreen,
                GetDC, ReleaseDC, DeleteDC,
                GetPixel,
                HBITMAP,
                SelectObject, DeleteObject,
                CreateCompatibleDC,
            },
            Dwm::{
                DWMWA_EXTENDED_FRAME_BOUNDS,
                DwmIsCompositionEnabled, DwmGetWindowAttribute,
            },
        }
    },
};

use std::{ffi::c_void, fmt};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use strum_macros::{EnumString, EnumProperty, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;
use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct WindowControl {
    next_id: Arc<Mutex<i32>>,
    windows: Arc<Mutex<HashMap<i32, HWND>>>
}

static WINDOW_CONTROL_SINGLETON: Lazy<WindowControl> = Lazy::new(||{
    WindowControl {
        next_id: Arc::new(Mutex::new(1)),
        windows: Arc::new(Mutex::new(HashMap::new()))
    }
});

pub fn get_next_id() -> i32 {
    let mut next_id = WINDOW_CONTROL_SINGLETON.next_id.lock().unwrap();
    let id = next_id.clone();
    *next_id += 1;

    id
}

pub fn set_new_window(id: i32, handle: HWND, to_zero: bool) {
    let mut list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    list.insert(id, handle);
    if to_zero {
        list.insert(0, handle);
    }
}

fn set_id_zero(hwnd: HWND) {
    let mut list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    list.insert(0, hwnd);
}

#[derive(PartialEq, Clone, Debug)]
pub struct Window {
    pub id: i32
}

impl Eq for Window {}
impl Hash for Window {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("getid", 4, getid);
    sets.add("idtohnd", 1, idtohnd);
    sets.add("hndtoid", 1, hndtoid);
    sets.add("clkitem", 5, clkitem);
    sets.add("ctrlwin", 2, ctrlwin);
    sets.add("status", 22, status);
    sets.add("acw", 6, acw);
    sets.add("monitor", 2, monitor);
    #[cfg(feature="chkimg")]
    sets.add("chkimg", 7, chkimg);
    sets.add("getallwin", 1, getallwin);
    sets.add("getctlhnd", 3, getctlhnd);
    sets.add("&&getitem", 6, getitem);
    sets.add("posacc", 4, posacc);
    sets.add("muscur", 0, muscur);
    sets.add("peekcolor", 4, peekcolor);
    sets.add("sckey", 36, sckey);
    sets.add("setslider", 4, setslider);
    sets.add("getslider", 3, getslider);
    sets
}

// GETID
#[allow(non_camel_case_types)]
#[derive(Debug, EnumVariantNames)]
pub enum SpecialWindowId {
    GET_ACTIVE_WIN,    // __GET_ACTIVE_WIN__
    GET_FROMPOINT_WIN, // __GET_FROMPOINT_WIN__
    GET_FROMPOINT_OBJ, // __GET_FROMPOINT_OBJ__
    GET_THISUWSC_WIN,  // __GET_THISUWSC_WIN__
    GET_LOGPRINT_WIN,  // __GET_LOGPRINT_WIN__
    GET_BALLOON_WIN,   // __GET_BALLOON_WIN__
    GET_FUKIDASI_WIN,  // __GET_FUKIDASI_WIN__
    GET_FORM_WIN,      // __GET_FORM_WIN__
    GET_FORM_WIN2,     // __GET_FORM_WIN2__
    GET_SCHEDULE_WIN,  // __GET_SCHEDULE_WIN__
    GET_STOPFORM_WIN,  // __GET_STOPFORM_WIN__
}

pub fn getid(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let title = args.get_as_string(0, None)?;
    let hwnd = match title.as_str() {
        "__GET_ACTIVE_WIN__" => unsafe {
            GetForegroundWindow()
        },
        "__GET_FROMPOINT_WIN__" => get_hwnd_from_mouse_point(true)?,
        "__GET_FROMPOINT_OBJ__" => get_hwnd_from_mouse_point(false)?,
        "__GET_THISUWSC_WIN__" => {
            HWND::default()
        },
        "__GET_LOGPRINT_WIN__" => {
            return Ok(BuiltinFuncReturnValue::GetLogPrintWinId)
        },
        "__GET_BALLOON_WIN__" => {
            return Ok(BuiltinFuncReturnValue::BalloonID)
        },
        "__GET_FORM_WIN__" => {
            HWND::default()
        },
        "__GET_FORM_WIN2__" => {
            HWND::default()
        },
        "__GET_SCHEDULE_WIN__" => {
            HWND::default()
        },
        "__GET_STOPFORM_WIN__" => {
            HWND::default()
        },
        _ => {
            let class_name = args.get_as_string(1, Some("".into()))?;
            let wait = args.get_as_num(2, Some(0.0))?;
            let _mdi_title = args.get_as_string(3, Some("".into()))?;
            find_window(title, class_name, wait)?
        },
    };
    if hwnd.0 > 0 {
        let id = get_id_from_hwnd(hwnd);
        // if id == -1.0 {
        //     let new_id = get_next_id();
        //     set_new_window(new_id, hwnd, false);
        //     id = new_id as f64;
        // }
        return Ok(BuiltinFuncReturnValue::Result(Object::Num(id)))
    } else {
        return Ok(BuiltinFuncReturnValue::Result(Object::Num(-1.0)))
    }
}

const MAX_NAME_SIZE: usize = 512;

#[derive(Debug)]
struct TargetWindow {
    hwnd: HWND,
    title: String,
    class_name: String,
    found: bool,
}

unsafe extern "system"
fn callback_find_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut title_buffer = [0; MAX_NAME_SIZE];
    let mut class_buffer = [0; MAX_NAME_SIZE];
    // let target = &mut *(lparam as *mut TargetWindow) as &mut TargetWindow;
    let target = &mut *(lparam.0 as *mut TargetWindow);

    let len = GetWindowTextW(hwnd, &mut title_buffer);
    let title = String::from_utf16_lossy(&title_buffer[..len as usize]);
    match title.to_ascii_lowercase().find(target.title.to_ascii_lowercase().as_str()) {
        Some(_) => {
            let len = GetClassNameW(hwnd, &mut class_buffer);
            let class = String::from_utf16_lossy(&class_buffer[..len as usize]);

            match class.to_ascii_lowercase().find(target.class_name.to_ascii_lowercase().as_str()) {
                Some(_) => {
                    target.title = title;
                    target.class_name = class;
                    target.hwnd = hwnd;
                    target.found = true;
                    return false.into();
                },
                None => ()
            }
        },
        None => ()
    }
    true.into() // 次のウィンドウへ
}

fn find_window(title: String, class_name: String, timeout: f64) -> windows::core::Result<HWND> {
    let mut target = TargetWindow {
        hwnd: HWND::default(),
        title,
        class_name,
        found: false,
    };
    let now = Instant::now();
    let limit = if timeout < 0.0 {
        // 0以下なら無限待ち
        None
    } else if timeout == 0.0 {
        // デフォルト値
        // 0.1～10秒まで状況や負荷により自動判断
        let auto_detected_timeout = 1.0;
        Some(Duration::from_secs_f64(auto_detected_timeout))
    } else {
        Some(Duration::from_secs_f64(timeout))
    };
    unsafe {
        let lparam = &mut target as *mut TargetWindow as isize;
        loop {
            EnumWindows(Some(callback_find_window), LPARAM(lparam));
            if target.found {
                let h = get_process_handle_from_hwnd(target.hwnd)?;
                WaitForInputIdle(h, 1000); // 入力可能になるまで最大1秒待つ
                CloseHandle(h);
                break;
            }
            if limit.is_some() && now.elapsed() >= limit.unwrap() {
                break;
            }
        }
        Ok(target.hwnd)
    }
}

fn get_hwnd_from_mouse_point(toplevel: bool) -> BuiltInResult<HWND> {
    unsafe {
        let point = window_low::get_current_pos()?;
        let mut hwnd = WindowFromPoint(point);
        if toplevel {
            loop {
                let parent = GetParent(hwnd);
                if parent.0 == 0 || ! IsWindowVisible(parent).as_bool(){
                    break;
                } else {
                    hwnd = parent;
                }
            }
        }
        Ok(hwnd)
    }
}

// IDTOHND
pub fn idtohnd(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int::<i32>(0, None)?;
    if id < 0 {
        return Ok(BuiltinFuncReturnValue::Result(Object::Num(0.0)));
    }
    let h = get_hwnd_from_id(id);
    if h.0 > 0 {
        unsafe {
            if IsWindow(h).as_bool() {
                return Ok(BuiltinFuncReturnValue::Result(Object::Num(h.0 as f64)));
            }
        }
    }
    Ok(BuiltinFuncReturnValue::Result(Object::Num(0.0)))
}

pub fn get_hwnd_from_id(id: i32) -> HWND {
    let list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    match list.get(&id) {
        Some(h) => *h,
        None => HWND::default()
    }
}

// HNDTOID
pub fn hndtoid(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let h = args.get_as_int::<isize>(0, None)?;
    let hwnd = HWND(h);
    let id = get_id_from_hwnd(hwnd);
    Ok(BuiltinFuncReturnValue::Result(Object::Num(id)))
}

pub fn get_id_from_hwnd(hwnd: HWND) -> f64 {
    let id = {
        let list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
        list.iter().find_map(
            |(key, &val)| if val == hwnd {
                Some(*key as f64)
            } else {
                None
            }
        )
    };
    // リストにない場合は新たなIDを発行する
    // hwndが無効なら-1
    match id {
        Some(id) => id,
        None => if unsafe { IsWindow(hwnd).as_bool() } {
            let new_id = get_next_id();
            set_new_window(new_id, hwnd, false);
            new_id as f64
        } else {
            -1.0
        }
    }
}

// ACW
pub fn acw(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int::<i32>(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd.0 == 0 {
        return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
    }
    let x = args.get_as_int(1, None).ok();
    let y = args.get_as_int(2, None).ok();
    let w = args.get_as_int(3, None).ok();
    let h = args.get_as_int(4, None).ok();
    let ms= args.get_as_int(5, Some(0)).unwrap_or(0);
    thread::sleep(Duration::from_millis(ms));
    set_window_size(hwnd, x, y, w, h);
    set_id_zero(hwnd);
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}


// CLKITEM
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum ClkConst {
    CLK_BTN       = 1,
    CLK_LIST      = 2,
    CLK_MENU      = 4,
    CLK_TAB       = 8,
    #[strum(props(alias="CLK_TREEVEW"))]
    CLK_TREEVIEW  = 16,
    #[strum(props(alias="CLK_LSTVEW"))]
    CLK_LISTVIEW  = 32,
    CLK_TOOLBAR   = 64,
    CLK_LINK      = 128,
    CLK_SHORT     = 256,
    CLK_BACK      = 512,
    #[strum(props(alias="CLK_MUSMOVE"))]
    CLK_MOUSEMOVE = 1024,
    CLK_RIGHTCLK  = 4096,
    CLK_LEFTCLK   = 2048,
    CLK_DBLCLK    = 8192,
    CLK_FROMLAST  = 65536,
    CLK_ACC       = 32768,
    CLK_API       = 536870912,
    CLK_UIA       = 1073741824,
    CLK_HWND      = 262144,
}

pub fn clkitem(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    // let name = args.get_as_string(1, None)?;
    let names = args.get_as_string_array(1)?;
    let clk_const = args.get_as_int(2, Some(0_usize))?;
    let check = args.get_as_three_state(3, Some(ThreeState::True))?;
    let order = args.get_as_int(4, Some(1))?;
    let order = if order < 1 {1_u32} else {order as u32};

    let hwnd = get_hwnd_from_id(id);

    let name = if names.len() > 1 {
        names.join("\t")
    } else {
        names[0].to_string()
    };

    let ci = clkitem::ClkItem::new(name, clk_const, order);
    let result = ci.click(hwnd, check);

    Ok(BuiltinFuncReturnValue::Result(result))
}

// CTRLWIN
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum CtrlWinCmd {
    CLOSE     = 2,
    CLOSE2    = 3,
    ACTIVATE  = 1,
    HIDE      = 4,
    SHOW      = 5,
    MIN       = 6,
    MAX       = 7,
    NORMAL    = 8,
    TOPMOST   = 9,
    NOTOPMOST = 10,
    TOPNOACTV = 11,
}

pub fn ctrlwin(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd.0 == 0 {
        return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
    }
    if let Some(cmd) = args.get_as_const(1, true)? {
        match cmd {
            CtrlWinCmd::CLOSE => unsafe {
                PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            CtrlWinCmd::CLOSE2 => unsafe {
                PostMessageW(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
            },
            CtrlWinCmd::ACTIVATE => unsafe {
                SetForegroundWindow(hwnd);
            },
            CtrlWinCmd::HIDE => unsafe {
                ShowWindow(hwnd, SW_HIDE);
            },
            CtrlWinCmd::SHOW => unsafe {
                ShowWindow(hwnd, SW_SHOW);
            },
            CtrlWinCmd::MIN => unsafe {
                ShowWindow(hwnd, SW_MINIMIZE);
            },
            CtrlWinCmd::MAX => unsafe {
                ShowWindow(hwnd, SW_MAXIMIZE);
            },
            CtrlWinCmd::NORMAL => unsafe {
                ShowWindow(hwnd, SW_SHOWNORMAL);
            },
            CtrlWinCmd::TOPMOST => unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE
                );
            },
            CtrlWinCmd::NOTOPMOST => unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE
                );
            },
            CtrlWinCmd::TOPNOACTV => unsafe {
                for h in vec![HWND_TOPMOST, HWND_NOTOPMOST] {
                    SetWindowPos(
                        hwnd,
                        h,
                        0, 0, 0, 0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                    );
                }
            },
        }
    }
    set_id_zero(hwnd);
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}

// STATUS
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, PartialEq, Clone, Copy)]
pub enum StatusEnum {
    ST_ALL       = 0,
    ST_TITLE     = 9,
    ST_CLASS     = 14,
    ST_X         = 1,
    ST_Y         = 2,
    ST_WIDTH     = 3,
    ST_HEIGHT    = 4,
    ST_CLX       = 5,
    ST_CLY       = 6,
    ST_CLWIDTH   = 7,
    ST_CLHEIGHT  = 8,
    ST_PARENT    = 16,
    ST_ICON      = 10,
    ST_MAXIMIZED = 11,
    ST_VISIBLE   = 12,
    ST_ACTIVE    = 13,
    ST_BUSY      = 15,
    ST_ISID      = 21,
    ST_WIN64     = 19,
    ST_PATH      = 17,
    ST_PROCESS   = 18,
    ST_MONITOR   = 20,
    ST_WX        = 101,
    ST_WY        = 102,
    ST_WWIDTH    = 103,
    ST_WHEIGHT   = 104,
}

struct WindowSize(i32, i32, i32, i32); // x, y, with, height
impl WindowSize {
    fn x(&self) -> i32 {
        self.0
    }
    fn y(&self) -> i32 {
        self.1
    }
    fn width(&self) -> i32 {
        self.2
    }
    fn height(&self) -> i32 {
        self.3
    }
}

fn get_window_size(h: HWND) -> WindowSize {
    unsafe {
        let mut rect = RECT::default();
        let pvattribute = &mut rect as *mut RECT as *mut c_void;
        let cbattribute = std::mem::size_of::<RECT>() as u32;
        if DwmIsCompositionEnabled().unwrap_or(BOOL(0)).as_bool() {
            // 見た目のRectを取る
            if DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_err() {
                // 失敗時はGetWindowRect
                GetWindowRect(h, &mut rect);
            }
        } else {
            // AEROがオフならGetWindowRect
            GetWindowRect(h, &mut rect);
        };
        WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
    }
}

fn get_window_rect(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        GetWindowRect(h, &mut rect);
    }
    WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
}

pub fn set_window_size(hwnd: HWND, x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) {
    let default_rect = get_window_size(hwnd);

    let x = x.unwrap_or(default_rect.x());
    let y = y.unwrap_or(default_rect.y());
    let w = w.unwrap_or(default_rect.width());
    let h = h.unwrap_or(default_rect.height());
    unsafe {
        MoveWindow(hwnd, x, y, w, h, true);
        if DwmIsCompositionEnabled().unwrap_or(BOOL(0)).as_bool() {
            // ウィンドウのDPIを得る
            let w_dpi = GetDpiForWindow(hwnd);
            let dpi_factor = w_dpi as f64 / 96.0;

            // 見た目のRectを取る
            let mut drect = RECT::default();
            let pvattribute = &mut drect as *mut RECT as *mut c_void;
            let cbattribute = std::mem::size_of::<RECT>() as u32;
            if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_ok() {
                // 実際のRectを取る
                let mut wrect = RECT::default();
                GetWindowRect(hwnd, &mut wrect);

                // 見た目と実際の差分から最適な移動位置を得る
                let fix= |o, v| {
                    let d = dpi_factor * 100.0;
                    let t = ((v as f64 / d) * 100.0).round();
                    o - t as i32
                };
                let new_x = fix(x, drect.left - wrect.left);
                let new_y = fix(y, drect.top - wrect.top);
                let new_w = fix(w, (drect.right - drect.left) - (wrect.right - wrect.left));
                let new_h = fix(h, (drect.bottom - drect.top) - (wrect.bottom - wrect.top));
                // 移動し直し
                MoveWindow(hwnd, new_x, new_y, new_w, new_h, true);
            }
        }
    }
}


fn get_client_size(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        GetClientRect(h, &mut rect);
        let mut point = POINT {x: rect.left, y: rect.top};
        ClientToScreen(h, &mut point);
        WindowSize(
            point.x,
            point.y,
            rect.right - rect.left,
            rect.bottom - rect.top
        )
    }
}

fn get_window_text(hwnd: HWND) -> BuiltInResult<Object> {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetWindowTextW(hwnd, &mut buffer);
        let s = String::from_utf16_lossy(&buffer[..len as usize]);
        Ok(Object::String(s))
    }
}

fn get_class_name(hwnd: HWND) -> BuiltInResult<Object> {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetClassNameW(hwnd, &mut buffer);
        let name = String::from_utf16_lossy(&buffer[..len as usize]);
        Ok(Object::String(name))
    }
}

fn get_parent(hwnd: HWND) -> Object {
    unsafe {
        let parent = GetParent(hwnd);
        Object::Num(get_id_from_hwnd(parent))
    }
}

fn is_maximized(hwnd: HWND)-> Object {
    let mut wp = WINDOWPLACEMENT::default();
    unsafe {
        GetWindowPlacement(hwnd, &mut wp);
        Object::Bool(wp.showCmd == SW_MAXIMIZE)
    }
}

fn is_active_window(hwnd: HWND) -> Object {
    unsafe {
        Object::Bool(GetForegroundWindow() == hwnd)
    }
}

fn get_process_id_from_hwnd(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

fn is_process_64bit(hwnd: HWND) -> BuiltInResult<Object> {
    if ! is_64bit_os().unwrap_or(true) {
        // 32bit OSなら必ずfalse
        return Ok(Object::Bool(false));
    }
    let h = get_process_handle_from_hwnd(hwnd)?;
    let mut is_wow64 = false.into();
    unsafe {
        IsWow64Process(h, &mut is_wow64);
    }
    let is_64 = ! is_wow64.as_bool();
    Ok(Object::Bool(is_64))
}

fn get_process_handle_from_hwnd(hwnd: HWND) -> windows::core::Result<HANDLE> {
    let pid = get_process_id_from_hwnd(hwnd);
    unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false, pid
        )
    }
}

fn get_process_path_from_hwnd(hwnd: HWND) -> BuiltInResult<Object> {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        let handle = get_process_handle_from_hwnd(hwnd)?;
        K32GetModuleFileNameExW(handle, HINSTANCE::default(), &mut buffer);
        CloseHandle(handle);
    }
    let path = String::from_utf16_lossy(&buffer);
    Ok(Object::String(path))
}

fn get_monitor_index_from_hwnd(hwnd: HWND) -> Object {
    match Monitor::from_hwnd(hwnd) {
        Some(m) => m.index().into(),
        None => Object::Empty,
    }
}


fn get_status_result(hwnd: HWND, stat: StatusEnum) -> BuiltInResult<Object> {
    let obj = match stat {
        StatusEnum::ST_TITLE => get_window_text(hwnd)?,
        StatusEnum::ST_CLASS => get_class_name(hwnd)?,
        StatusEnum::ST_X |
        StatusEnum::ST_Y |
        StatusEnum::ST_WIDTH |
        StatusEnum::ST_HEIGHT => {
            let wsize = get_window_size(hwnd);
            match stat {
                StatusEnum::ST_X => Object::Num(wsize.x() as f64),
                StatusEnum::ST_Y => Object::Num(wsize.y() as f64),
                StatusEnum::ST_WIDTH => Object::Num(wsize.width() as f64),
                StatusEnum::ST_HEIGHT => Object::Num(wsize.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_CLX |
        StatusEnum::ST_CLY |
        StatusEnum::ST_CLWIDTH |
        StatusEnum::ST_CLHEIGHT => {
            let csize = get_client_size(hwnd);
            match stat {
                StatusEnum::ST_CLX => Object::Num(csize.x() as f64),
                StatusEnum::ST_CLY => Object::Num(csize.y() as f64),
                StatusEnum::ST_CLWIDTH => Object::Num(csize.width() as f64),
                StatusEnum::ST_CLHEIGHT => Object::Num(csize.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_PARENT => get_parent(hwnd),
        StatusEnum::ST_ICON => unsafe {
            Object::Bool(IsIconic(hwnd).as_bool())
        },
        StatusEnum::ST_MAXIMIZED => is_maximized(hwnd),
        StatusEnum::ST_VISIBLE => unsafe {
            Object::Bool(IsWindowVisible(hwnd).as_bool())
        },
        StatusEnum::ST_ACTIVE => is_active_window(hwnd),
        StatusEnum::ST_BUSY => unsafe {
            Object::Bool(IsHungAppWindow(hwnd).as_bool())
        },
        StatusEnum::ST_ISID => unsafe {
            Object::Bool(IsWindow(hwnd).as_bool())
        },
        StatusEnum::ST_WIN64 => is_process_64bit(hwnd)?,
        StatusEnum::ST_PATH => get_process_path_from_hwnd(hwnd)?,
        StatusEnum::ST_PROCESS => Object::Num(get_process_id_from_hwnd(hwnd) as f64),
        StatusEnum::ST_MONITOR => get_monitor_index_from_hwnd(hwnd),
        StatusEnum::ST_WX |
        StatusEnum::ST_WY |
        StatusEnum::ST_WWIDTH |
        StatusEnum::ST_WHEIGHT => {
            let size = get_window_rect(hwnd);
            match stat {
                StatusEnum::ST_WX => Object::Num(size.x() as f64),
                StatusEnum::ST_WY => Object::Num(size.y() as f64),
                StatusEnum::ST_WWIDTH => Object::Num(size.width() as f64),
                StatusEnum::ST_WHEIGHT => Object::Num(size.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_ALL => Object::Empty
    };
    Ok(obj)
}

fn get_all_status(hwnd: HWND) -> BuiltinFuncResult {
    let mut stats = HashTbl::new(true, false);
    stats.insert((StatusEnum::ST_TITLE as u8).to_string(), get_window_text(hwnd)?);
    stats.insert((StatusEnum::ST_CLASS as u8).to_string(), get_class_name(hwnd)?);
    let wsize = get_window_size(hwnd);
    stats.insert((StatusEnum::ST_X as u8).to_string(), Object::Num(wsize.x() as f64));
    stats.insert((StatusEnum::ST_Y as u8).to_string(), Object::Num(wsize.y() as f64));
    stats.insert((StatusEnum::ST_WIDTH as u8).to_string(), Object::Num(wsize.width() as f64));
    stats.insert((StatusEnum::ST_HEIGHT as u8).to_string(), Object::Num(wsize.height() as f64));
    let csize = get_client_size(hwnd);
    stats.insert((StatusEnum::ST_CLX as u8).to_string(), Object::Num(csize.x() as f64));
    stats.insert((StatusEnum::ST_CLY as u8).to_string(), Object::Num(csize.y() as f64));
    stats.insert((StatusEnum::ST_CLWIDTH as u8).to_string(), Object::Num(csize.width() as f64));
    stats.insert((StatusEnum::ST_CLHEIGHT as u8).to_string(), Object::Num(csize.height() as f64));
    stats.insert((StatusEnum::ST_PARENT as u8).to_string(), get_parent(hwnd));
    stats.insert((StatusEnum::ST_ICON as u8).to_string(), unsafe{ Object::Bool(IsIconic(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_MAXIMIZED as u8).to_string(), is_maximized(hwnd));
    stats.insert((StatusEnum::ST_VISIBLE as u8).to_string(), unsafe{ Object::Bool(IsWindowVisible(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_ACTIVE as u8).to_string(), is_active_window(hwnd));
    stats.insert((StatusEnum::ST_BUSY as u8).to_string(), unsafe{ Object::Bool(IsHungAppWindow(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_ISID as u8).to_string(), unsafe{ Object::Bool(IsWindow(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_WIN64 as u8).to_string(), is_process_64bit(hwnd)?);
    stats.insert((StatusEnum::ST_PATH as u8).to_string(), get_process_path_from_hwnd(hwnd)?);
    stats.insert((StatusEnum::ST_PROCESS as u8).to_string(), Object::Num(get_process_id_from_hwnd(hwnd) as f64));
    stats.insert((StatusEnum::ST_MONITOR as u8).to_string(), get_monitor_index_from_hwnd(hwnd));
    Ok(BuiltinFuncReturnValue::Result(Object::HashTbl(Arc::new(Mutex::new(stats)))))
}

pub fn status(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let hwnd = get_hwnd_from_id(
        args.get_as_int(0, None)?
    );
    if args.len() > 2 {
        let mut i = 1;
        // let mut stats = vec![Object::Empty; 22];
        let mut stats = HashTbl::new(true, false);
        while i < args.len() {
            if let Some(cmd) = args.get_as_const::<StatusEnum>(i, true)? {
                let value = get_status_result(hwnd, cmd)?;
                let name = (cmd as u8).to_string();
                stats.insert(name, value);
            }
            i += 1;
        }
        Ok(BuiltinFuncReturnValue::Result(Object::HashTbl(Arc::new(Mutex::new(stats)))))
    } else {
        if let Some(cmd) = args.get_as_const::<StatusEnum>(1, true)?{
            if cmd == StatusEnum::ST_ALL {
                Ok(get_all_status(hwnd)?)
            } else {
                let st = get_status_result(hwnd, cmd)?;
                Ok(BuiltinFuncReturnValue::Result(st))
            }
        } else {
            Ok(BuiltinFuncReturnValue::Result(Object::Empty))
        }
    }
}

// monitor
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum MonitorEnum {
    MON_X           = 0,
    MON_Y           = 1,
    MON_WIDTH       = 2,
    MON_HEIGHT      = 3,
    #[strum(props(alias="MON_ISMAIN"))]
    MON_PRIMARY     = 4,
    MON_NAME        = 5,
    MON_WORK_X      = 10,
    MON_WORK_Y      = 11,
    MON_WORK_WIDTH  = 12,
    MON_WORK_HEIGHT = 13,
    MON_DPI         = 15,
    MON_SCALING     = 16,
    MON_ALL         = 20,
}
impl fmt::Display for MonitorEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}", ToPrimitive::to_f64(self).unwrap_or_default())
    }
}

pub fn monitor(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if args.len() == 0 {
        let count = Monitor::get_count();
        Ok(BuiltinFuncReturnValue::Result(count.into()))
    } else {
        let index = args.get_as_int(0, None)?;
        let Some(monitor) = Monitor::from_index(index) else {
            return Ok(BuiltinFuncReturnValue::Result(false.into()))
        };
        let mon_enum = args.get_as_const::<MonitorEnum>(1, false)?
            .unwrap_or(MonitorEnum::MON_ALL);
        let obj = match mon_enum {
            MonitorEnum::MON_X => monitor.x().into(),
            MonitorEnum::MON_Y => monitor.y().into(),
            MonitorEnum::MON_WIDTH => monitor.width().into(),
            MonitorEnum::MON_HEIGHT => monitor.height().into(),
            MonitorEnum::MON_PRIMARY => monitor.is_primary().into(),
            MonitorEnum::MON_NAME => monitor.name().into(),
            MonitorEnum::MON_WORK_X => monitor.work_x().into(),
            MonitorEnum::MON_WORK_Y => monitor.work_y().into(),
            MonitorEnum::MON_WORK_WIDTH => monitor.work_width().into(),
            MonitorEnum::MON_WORK_HEIGHT => monitor.work_height().into(),
            MonitorEnum::MON_DPI => monitor.dpi().unwrap_or(0.0).into(),
            MonitorEnum::MON_SCALING => monitor.scaling().into(),
            MonitorEnum::MON_ALL => {
                let mut map = HashTbl::new(false, false);
                map.insert(MonitorEnum::MON_X.to_string(), monitor.x().into());
                map.insert(MonitorEnum::MON_Y.to_string(), monitor.y().into());
                map.insert(MonitorEnum::MON_WIDTH.to_string(), monitor.width().into());
                map.insert(MonitorEnum::MON_HEIGHT.to_string(), monitor.height().into());
                map.insert(MonitorEnum::MON_PRIMARY.to_string(), monitor.is_primary().into());
                map.insert(MonitorEnum::MON_NAME.to_string(), monitor.name().into());
                map.insert(MonitorEnum::MON_WORK_X.to_string(), monitor.work_x().into());
                map.insert(MonitorEnum::MON_WORK_Y.to_string(), monitor.work_y().into());
                map.insert(MonitorEnum::MON_WORK_WIDTH.to_string(), monitor.work_width().into());
                map.insert(MonitorEnum::MON_WORK_HEIGHT.to_string(), monitor.work_height().into());
                map.insert(MonitorEnum::MON_DPI.to_string(), monitor.dpi().unwrap_or(0.0).into());
                map.insert(MonitorEnum::MON_SCALING.to_string(), monitor.scaling().into());
                Object::HashTbl(Arc::new(Mutex::new(map)))
            },
        };
        Ok(BuiltinFuncReturnValue::Result(obj))
    }
}

#[cfg(feature="chkimg")]
pub fn chkimg(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let save_ss = {
        let settings = USETTINGS.lock().unwrap();
        settings.chkimg.save_ss
    };
    let default_score = 95;
    let path = args.get_as_string(0, None)?;
    let score = args.get_as_int::<i32>(1, Some(default_score))?;
    if score < 1 && score > 100 {
        return Err(builtin_func_error(UErrorMessage::GivenNumberIsOutOfRange(1.0, 100.0)));
    }
    let score = score as f64 / 100.0;
    let count = args.get_as_int::<u8>(2, Some(5))?;
    let left = args.get_as_int_or_empty(3)?;
    let top = args.get_as_int_or_empty(4)?;
    let right = args.get_as_int_or_empty(5)?;
    let bottom = args.get_as_int_or_empty(6)?;

    let ss = ScreenShot::get(None, left, top, right, bottom)?;
    if save_ss {
        ss.save(None)?;
    }
    let chk = ChkImg::from_screenshot(ss)?;
    let result = chk.search(&path, score, Some(count))?;
    let arr = result
                            .into_iter()
                            .map(|m| {
                                let vec = vec![
                                    Object::Num(m.x as f64),
                                    Object::Num(m.y as f64),
                                    Object::Num(m.score * 100.0)
                                ];
                                Object::Array(vec)
                            })
                            .collect::<Vec<_>>();
    Ok(BuiltinFuncReturnValue::Result(Object::Array(arr)))
}

unsafe extern "system"
fn callback_getallwin(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = lparam.0 as *mut HwndList;
    match hwnd {
        HWND(0) => false.into(),
        h => {
            (*list).0.push(h);
            true.into()
        },
    }
}

#[derive(Debug)]
struct HwndList(Vec<HWND>);

pub fn getallwin(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = match args.get_as_int_or_empty::<i32>(0)? {
        Some(id) => match get_hwnd_from_id(id) {
            HWND(0) => return Ok(BuiltinFuncReturnValue::Result(Object::Array(vec![]))),
            h => Some(h)
        },
        None => None,
    };
    let id_list = unsafe {
        let mut list = HwndList(vec![]);
        let lparam = LPARAM(&mut list as *mut HwndList as isize);
        match target {
            Some(h) => EnumChildWindows(h, Some(callback_getallwin), lparam),
            None => EnumWindows(Some(callback_getallwin), lparam),
        };

        list.0.into_iter()
            .map(|h| {
                let id = get_id_from_hwnd(h);
                Object::Num(id)
            })
            .collect()
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Array(id_list)))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumVariantNames)]
pub enum GetHndConst {
    GET_MENU_HND,   // __GET_MENU_HND__
    GET_SYSMENU_HND // __GET_SYSMENU_HND__
}

unsafe extern "system"
fn callback_getctlhnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctlhnd = &mut *(lparam.0 as *mut CtlHnd);
    let pat = ctlhnd.target.to_ascii_lowercase();

    let mut buffer = [0; MAX_NAME_SIZE];
    let len = GetWindowTextW(hwnd, &mut buffer);
    let title = String::from_utf16_lossy(&buffer[..len as usize]);
    if let Some(_) = title.to_ascii_lowercase().find(&pat) {
        ctlhnd.order -= 1;
        if ctlhnd.order == 0 {
            ctlhnd.hwnd = hwnd;
            return false.into()
        }
    } else {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetClassNameW(hwnd, &mut buffer);
        let name = String::from_utf16_lossy(&buffer[..len as usize]);
        if let Some(_) = name.to_ascii_lowercase().find(&pat) {
            ctlhnd.order-= 1;
            if ctlhnd.order == 0 {
                ctlhnd.hwnd = hwnd;
                return false.into()
            }
        }
    }
    true.into()
}

struct CtlHnd{target: String, hwnd: HWND, order: u32}

pub fn getctlhnd(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let parent = get_hwnd_from_id(id);
    let target = args.get_as_string(1, None)?;

    let hwnd = match target.to_ascii_uppercase().as_str() {
        "__GET_MENU_HND__" => unsafe {
            let menu = GetMenu(parent);
            menu.0 as f64
        },
        "__GET_SYSMENU_HND__" => unsafe {
            let menu = GetSystemMenu(parent, false);
            menu.0 as f64
        },
        _ => {
            let n = args.get_as_int(2, Some(1))?;
            let order = if n < 1 {1_u32} else {n as u32};

            let mut ctlhnd = CtlHnd {target, hwnd: HWND::default(), order};
            let lparam = LPARAM(&mut ctlhnd as *mut CtlHnd as isize);
            unsafe {
                EnumChildWindows(parent, Some(callback_getctlhnd), lparam);
            }
            ctlhnd.hwnd.0 as f64
        }
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Num(hwnd)))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum GetItemConst {
    ITM_BTN       = 1,
    ITM_LIST      = 2,
    ITM_TAB       = 8,
    ITM_MENU      = 4,
    #[strum(props(alias="ITM_TREEVEW"))]
    ITM_TREEVIEW  = 16,
    #[strum(props(alias="ITM_LSTVEW"))]
    ITM_LISTVIEW  = 32,
    ITM_EDIT      = 131072,
    ITM_STATIC    = 262144,
    ITM_STATUSBAR = 524288,
    ITM_TOOLBAR   = 64,
    ITM_LINK      = 128,
    ITM_ACCCLK    = 4194304,
    ITM_ACCCLK2   = 272629760,
    ITM_ACCTXT    = 8388608,
    ITM_ACCEDIT   = 16777216,
    ITM_FROMLAST  = 65536,
    ITM_BACK      = 512,
    // UObject (json) ですべての要素を返す
    ITM_API_ALL   = 1024,
    ITM_ACC_ALL   = 2097152,
    ITM_UIA_ALL   = 536870912,
}

pub fn getitem(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let hwnd = get_hwnd_from_id(id);
    match acc::Acc::from_hwnd(hwnd) {
        Some(acc) => {
            let tree = acc.get_all_children();
            println!("{:#?}", tree);
        },
        None => {},
    }
    Ok(BuiltinFuncReturnValue::Result(Object::default()))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum AccConst {
    ACC_ACC         = 1,
    ACC_API         = 2,
    ACC_NAME        = 3,
    ACC_VALUE       = 4,
    ACC_ROLE        = 5,
    ACC_STATE       = 6,
    ACC_DESCRIPTION = 7,
    ACC_LOCATION    = 8,
    ACC_BACK        = 512,
}

pub fn posacc(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let hwnd = get_hwnd_from_id(id);
    let clx = args.get_as_int(1, None::<i32>)?;
    let cly = args.get_as_int(2, None::<i32>)?;
    let mode = args.get_as_int(3, Some(0_u16))?;
    let mode = if (mode & AccConst::ACC_BACK as u16) > 0 {
        // ACC_BACKを除去
        mode - AccConst::ACC_BACK as u16
    } else {
        // ACC_BACKがないので対象ウィンドウをアクティブにする
        unsafe { SetForegroundWindow(hwnd); }
        mode
    };
    let obj = match acc::Acc::from_point(hwnd, clx, cly) {
        Some(acc) => match mode {
            0 => {
                match acc.get_name().map(|name|name.into()) {
                    Some(obj) => obj,
                    None => acc.get_api_text().map(|api| api.into()).unwrap_or_default(),
                }
            }
            1 | 3 => {
                acc.get_name().map(|name|name.into()).unwrap_or_default()
            },
            2 => {
                acc.get_api_text().map(|api| api.into()).unwrap_or_default()
            },
            4 => {
                acc.get_value().map(|val| val.into()).unwrap_or_default()
            },
            5 => {
                acc.get_role_text().map(|role| role.into()).unwrap_or_default()
            },
            6 => {
                let vec2obj = |vec: Vec<String>| {
                    let arr = vec.into_iter()
                        .map(|text| text.into())
                        .collect();
                    Object::Array(arr)
                };
                acc.get_state_texts().map(vec2obj).unwrap_or_default()
            },
            7 => {
                acc.get_description().map(|desc| desc.into()).unwrap_or_default()
            },
            8 => {
                let vec2obj = |vec: Vec<i32>| {
                    let arr = vec.into_iter()
                        .map(|n| n.into())
                        .collect();
                    Object::Array(arr)
                };
                acc.get_location(hwnd).map(vec2obj).unwrap_or_default()
            },
            _ => Object::Empty
        },
        None => Object::Empty,
    };
    Ok(BuiltinFuncReturnValue::Result(obj))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum CurConst {
    CUR_APPSTARTING = 1,
    CUR_ARROW       = 2,
    CUR_CROSS       = 3,
    CUR_HAND        = 4,
    CUR_HELP        = 5,
    CUR_IBEAM       = 6,
    CUR_NO          = 8,
    CUR_SIZEALL     = 10,
    CUR_SIZENESW    = 11,
    CUR_SIZENS      = 12,
    CUR_SIZENWSE    = 13,
    CUR_SIZEWE      = 14,
    CUR_UPARROW     = 15,
    CUR_WAIT        = 16,
}

pub fn muscur(_: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = unsafe {
        let mut pci = CURSORINFO::default();
        pci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        GetCursorInfo(&mut pci);
        pci.hCursor.0
    };
    let cursor = match id {
        65563 => CurConst::CUR_APPSTARTING,
        65541 => CurConst::CUR_ARROW,
        65547 => CurConst::CUR_CROSS,
        65569 => CurConst::CUR_HAND,
        65565 => CurConst::CUR_HELP,
        65543 => CurConst::CUR_IBEAM,
        65561 => CurConst::CUR_NO,
        65559 => CurConst::CUR_SIZEALL,
        65553 => CurConst::CUR_SIZENESW,
        65557 => CurConst::CUR_SIZENS,
        65551 => CurConst::CUR_SIZENWSE,
        65555 => CurConst::CUR_SIZEWE,
        65549 => CurConst::CUR_UPARROW,
        65545 => CurConst::CUR_WAIT,
        _ => return Ok(BuiltinFuncReturnValue::Result(Object::Num(0.0))),
    };
    let n = cursor as i32 as f64;
    Ok(BuiltinFuncReturnValue::Result(Object::Num(n)))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum ColConst {
    #[default]
    COL_BGR = 0,
    COL_RGB = 3,
    COL_R   = 4,
    COL_G   = 5,
    COL_B   = 6,
}

pub fn peekcolor(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let x = args.get_as_int(0, None::<i32>)?;
    let y = args.get_as_int(1, None::<i32>)?;
    let colconst = args.get_as_const::<ColConst>(2, false)?.unwrap_or_default();
    let clipboard = args.get_as_bool(3, Some(false))?;
    unsafe {
        let bgr = if clipboard {
            if IsClipboardFormatAvailable(CF_BITMAP.0).as_bool() && OpenClipboard(HWND(0)).as_bool() {
                let h = GetClipboardData(CF_BITMAP.0)?;
                let hbitmap = HBITMAP(h.0);
                let hdc = CreateCompatibleDC(None);
                let old = SelectObject(hdc, hbitmap);
                let colorref = GetPixel(hdc, x, y);
                SelectObject(hdc, old);
                CloseHandle(h);
                DeleteObject(hbitmap);
                DeleteDC(hdc);
                CloseClipboard();
                colorref.0
            } else {
                0xFFFFFFFF
            }
        } else {
            let hdc = GetDC(None);
            let colorref = GetPixel(hdc, x, y);
            ReleaseDC(None, hdc);
            colorref.0
        };
        if bgr > 0xFFFFFF {
            Ok(BuiltinFuncReturnValue::Result(Object::Num(-1.0)))
        } else {
            let r = |c: u32| c & 0xFF;
            let g = |c: u32| (c >> 8) & 0xFF;
            let b = |c: u32| (c >> 16) & 0xFF;
            let color = match colconst {
                ColConst::COL_BGR => bgr,
                ColConst::COL_RGB => {
                    r(bgr) << 16 |
                    g(bgr) << 8 |
                    b(bgr)
                },
                ColConst::COL_R => r(bgr),
                ColConst::COL_G => g(bgr),
                ColConst::COL_B => b(bgr),
            };
            Ok(BuiltinFuncReturnValue::Result(Object::Num(color as f64)))
        }
    }
}

pub fn sckey(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let hwnd = get_hwnd_from_id(id);
    let keys = args.get_sckey_codes(1)?;
    let pinputs: Vec<INPUT> = SCKeyCode::codes_to_input(keys);
    unsafe {
        if hwnd.0 != 0 {
            SetForegroundWindow(hwnd);
        }
        SendInput(&pinputs, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(BuiltinFuncReturnValue::Result(Object::default()))
}

struct Slider {
    slider: win32::Slider,
}

impl Slider {
    fn new(hwnd: HWND, nth: u32) -> Option<Self> {
        win32::Win32::get_slider(hwnd, nth)
            .map(|slider| Self { slider })
    }
    fn get(&self, param: SldConst) -> i32 {

        match param {
            SldConst::SLD_POS => self.slider.get_pos(),
            SldConst::SLD_MIN => self.slider.get_min(),
            SldConst::SLD_MAX => self.slider.get_max(),
            SldConst::SLD_PAGE => self.slider.get_page(),
            SldConst::SLD_BAR => self.slider.get_bar(),
            SldConst::SLD_X => self.slider.get_point().0,
            SldConst::SLD_Y => self.slider.get_point().1,
        }
    }
    fn set(&self, pos: i32, smooth: bool) -> bool {
        self.slider.set_pos(pos, smooth)
    }
}

pub fn setslider(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    let value = args.get_as_int(1, None)?;
    let nth = args.get_as_int(2, Some(1))?;
    let smooth = args.get_as_bool(3, Some(true))?;

    let result = if let Some(slider) = Slider::new(hwnd, nth) {
        slider.set(value, smooth)
    } else {
        false
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Bool(result)))

}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum SldConst {
    #[default]
    SLD_POS  = 0,
    SLD_MIN  = 1,
    SLD_MAX  = 2,
    SLD_PAGE = 3,
    SLD_BAR  = 4,
    SLD_X    = 5,
    SLD_Y    = 6,
}
pub fn getslider(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    let nth = args.get_as_int(1, Some(1))?;
    let param = args.get_as_const(2, false)?.unwrap_or_default();

    if let Some(slider) = Slider::new(hwnd, nth) {
        let val = slider.get(param);
        Ok(BuiltinFuncReturnValue::Result(Object::Num(val as f64)))
    } else {
        let error_value = Object::Num(ErrConst::ERR_VALUE as i32 as f64);
        Ok(BuiltinFuncReturnValue::Result(error_value))
    }
}