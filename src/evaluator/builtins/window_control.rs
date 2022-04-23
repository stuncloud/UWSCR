use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::builtins::window_low;
use crate::evaluator::builtins::system_controls::is_64bit_os;
#[cfg(feature="chkimg")]
use crate::evaluator::builtins::chkimg::{ChkImg, ScreenShot};

use windows::{
    core::{PCWSTR},
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
        },
        UI::{
            WindowsAndMessaging::{
                SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
                SW_SHOWNORMAL, SW_SHOW, SW_HIDE, SW_MINIMIZE, SW_MAXIMIZE,
                WINDOWPLACEMENT,
                WM_CLOSE, WM_DESTROY, HWND_TOPMOST, HWND_NOTOPMOST,
                MONITORINFOF_PRIMARY,
                WindowFromPoint, GetParent, IsWindowVisible, GetClientRect,
                GetForegroundWindow, GetWindowTextW, GetClassNameW, EnumWindows,
                IsWindow, PostMessageW, SetForegroundWindow, ShowWindow,
                SetWindowPos, GetWindowRect, MoveWindow, GetWindowPlacement,
                GetWindowThreadProcessId, IsIconic, IsHungAppWindow,
            },
            HiDpi::{
                GetDpiForWindow,
            },
        },
        Graphics::{
            Gdi::{
                HMONITOR, HDC, DISPLAY_DEVICEW, MONITORINFOEXW,
                MONITOR_DEFAULTTONEAREST,
                MapWindowPoints, MonitorFromWindow, EnumDisplayMonitors,
                EnumDisplayDevicesW, GetMonitorInfoW,
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
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};
use std::thread;
use std::mem;
use std::ptr;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
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
        "__GET_FROMPOINT_WIN__" => get_hwnd_from_mouse_point(true, args.name())?,
        "__GET_FROMPOINT_OBJ__" => get_hwnd_from_mouse_point(false, args.name())?,
        "__GET_THISUWSC_WIN__" => {
            HWND::default()
        },
        "__GET_LOGPRINT_WIN__" => {
            return Ok(Object::SpecialFuncResult(SpecialFuncResultType::GetLogPrintWinId))
        },
        "__GET_BALLOON_WIN__" => {
            return Ok(Object::SpecialFuncResult(SpecialFuncResultType::BalloonID))
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
        return Ok(Object::Num(id))
    } else {
        return Ok(Object::Num(-1.0))
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
fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
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
            EnumWindows(Some(enum_window_proc), LPARAM(lparam));
            if target.found {
                break
            }
            if limit.is_some() && now.elapsed() >= limit.unwrap() {
                break;
            }
        }
        let h = get_process_handle_from_hwnd(target.hwnd)?;
        WaitForInputIdle(h, 1000); // 入力可能になるまで最大1秒待つ
        CloseHandle(h);
        Ok(target.hwnd)
    }
}

fn get_hwnd_from_mouse_point(toplevel: bool, name: String) -> BuiltInResult<HWND> {
    unsafe {
        let point = window_low::get_current_pos(name)?;
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
        return Ok(Object::Num(0.0));
    }
    let h = get_hwnd_from_id(id);
    if h.0 > 0 {
        unsafe {
            if IsWindow(h).as_bool() {
                return Ok(Object::Num(h.0 as f64));
            }
        }
    }
    Ok(Object::Num(0.0))
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
    Ok(Object::Num(id))
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
        return Ok(Object::Empty);
    }
    let x = args.get_as_int(1, None).ok();
    let y = args.get_as_int(2, None).ok();
    let w = args.get_as_int(3, None).ok();
    let h = args.get_as_int(4, None).ok();
    let ms= args.get_as_int(5, Some(0)).unwrap_or(0);
    thread::sleep(Duration::from_millis(ms));
    set_window_size(hwnd, x, y, w, h)?;
    set_id_zero(hwnd);
    Ok(Object::Empty)
}


// CLKITEM
pub fn clkitem(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::Bool(args.len() > 0))
}

// CTRLWIN
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
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
    UNKNOWN_CTRLWIN_CMD = -1,
}

pub fn ctrlwin(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd.0 == 0 {
        return Ok(Object::Empty);
    }
    let cmd = args.get_as_int(1, None)?;
    match FromPrimitive::from_i32(cmd).unwrap_or(CtrlWinCmd::UNKNOWN_CTRLWIN_CMD) {
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
        _ => (),
    };
    set_id_zero(hwnd);
    Ok(Object::Empty)
}

// STATUS
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
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
    UNKNOWN_STATUS = -1,
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

fn get_window_size(h: HWND) -> BuiltInResult<WindowSize> {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        // let mut aero_enabled = false.into();
        // let _ = DwmIsCompositionEnabled(&mut aero_enabled);
        let aero_enabled = DwmIsCompositionEnabled()?;
        if ! aero_enabled.as_bool() {
            // AEROがオフならGetWindowRect
            GetWindowRect(h, &mut rect);
        } else {
            let _ = DwmGetWindowAttribute(
                h,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut c_void,
                mem::size_of::<RECT>() as u32
            );
        };
    }
    Ok(WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
}

fn get_window_rect(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        GetWindowRect(h, &mut rect);
    }
    WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
}

pub fn set_window_size(hwnd: HWND, x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) -> BuiltInResult<()> {
    let default_rect = get_window_size(hwnd)?;

    let x = x.unwrap_or(default_rect.x());
    let y = y.unwrap_or(default_rect.y());
    let w = w.unwrap_or(default_rect.width());
    let h = h.unwrap_or(default_rect.height());
    unsafe {
        // let mut aero_enabled = false.into();
        // let _ = DwmIsCompositionEnabled(&mut aero_enabled);
        let aero_enabled = DwmIsCompositionEnabled()?;

        if aero_enabled.as_bool() {
            let mut drect: RECT= mem::zeroed();
            let mut wrect: RECT= mem::zeroed();

            // 一旦移動する
            MoveWindow(hwnd, x, y, w, h, false);

            // ウィンドウのDPIを得る
            let w_dpi = GetDpiForWindow(&hwnd);
            let dpi_factor = w_dpi as f64 / 96.0;

            // 見た目のRectを取る
            let _ = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut drect as *mut _ as *mut c_void,
                mem::size_of::<RECT>() as u32
            );
            // 実際のRectを取る
            GetWindowRect(hwnd, &mut wrect);

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
        } else {
            MoveWindow(hwnd, x, y, w, h, true);
        }
    }
    Ok(())
}


fn get_client_size(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        GetClientRect(h, &mut rect);
        let point = POINT {x: rect.left, y: rect.top};
        MapWindowPoints(h, HWND::default(), &mut [point]);
        WindowSize(
            point.x,
            point.y,
            rect.right - rect.left,
            rect.bottom - rect.top
        )
    }
}

fn get_window_text(hwnd: HWND) -> BuiltinFuncResult {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetWindowTextW(hwnd, &mut buffer);
        let s = String::from_utf16_lossy(&buffer[..len as usize]);
        Ok(Object::String(s))
    }
}

fn get_class_name(hwnd: HWND) -> BuiltinFuncResult {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        GetClassNameW(hwnd, &mut buffer);
        let name = String::from_utf16_lossy(&buffer);
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
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid
    }
}

fn is_process_64bit(hwnd: HWND) -> BuiltinFuncResult {
    if ! is_64bit_os("status".into()).unwrap_or(true) {
        // 32bit OSなら必ずfalse
        return Ok(Object::Bool(false));
    }
    let h = get_process_handle_from_hwnd(hwnd)?;
    let mut b = false.into();
    unsafe {
        IsWow64Process(h, &mut b);
        Ok(Object::Bool(b.into()))
    }
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

fn get_process_path_from_hwnd(hwnd: HWND) -> BuiltinFuncResult {
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
    let h = unsafe {
        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
    };
    get_monitor_count(h)
}


fn get_status_result(hwnd: HWND, st: u8) -> BuiltinFuncResult {
    let stat = FromPrimitive::from_u8(st).unwrap_or(StatusEnum::UNKNOWN_STATUS);
    let obj = match stat {
        StatusEnum::ST_TITLE => get_window_text(hwnd)?,
        StatusEnum::ST_CLASS => get_class_name(hwnd)?,
        StatusEnum::ST_X |
        StatusEnum::ST_Y |
        StatusEnum::ST_WIDTH |
        StatusEnum::ST_HEIGHT => {
            let wsize = get_window_size(hwnd)?;
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
        _ => Object::Bool(false) // 定数以外を受けた場合false
    };
    Ok(obj)
}

fn get_all_status(hwnd: HWND) -> BuiltinFuncResult {
    let mut stats = HashTbl::new(true, false);
    stats.insert((StatusEnum::ST_TITLE as u8).to_string(), get_window_text(hwnd)?);
    stats.insert((StatusEnum::ST_CLASS as u8).to_string(), get_class_name(hwnd)?);
    let wsize = get_window_size(hwnd)?;
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
    Ok(Object::HashTbl(Arc::new(Mutex::new(stats))))
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
            let cmd = args.get_as_int::<u8>(i, None)?;
            let value = get_status_result(hwnd, cmd)?;
            stats.insert(cmd.to_string(), value);
            i += 1;
        }
        Ok(Object::HashTbl(Arc::new(Mutex::new(stats))))
    } else {
        let cmd = args.get_as_int::<u8>(1, None)?;
        if cmd == StatusEnum::ST_ALL as u8 {
            Ok(get_all_status(hwnd)?)
        } else {
            Ok(get_status_result(hwnd, cmd)?)
        }
    }
}

// monitor
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum MonitorEnum {
    MON_X           = 0,
    MON_Y           = 1,
    MON_WIDTH       = 2,
    MON_HEIGHT      = 3,
    MON_PRIMARY     = 4,
    MON_NAME        = 5,
    MON_WORK_X      = 10,
    MON_WORK_Y      = 11,
    MON_WORK_WIDTH  = 12,
    MON_WORK_HEIGHT = 13,
    MON_ALL         = 20,
    UNKNOWN_MONITOR_CMD = -1,
}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum MonitorEnumAlias {
    MON_ISMAIN     = 4,
}

#[derive(Debug)]
struct Monitor {
    count: usize,
    handle: HMONITOR,
    index: usize,
}

unsafe extern "system"
fn callback_count_monitor(h: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
    let m = &mut *(lparam.0 as *mut Monitor);
    if m.handle == h {
        return false.into()
    }
    m.count += 1;
    true.into()
}
// nullを渡すと全モニタ数、モニタのハンドルを渡すとそのインデックスを返す
fn get_monitor_count(handle: HMONITOR) -> Object {
    unsafe {
        let mut monitor = Monitor {
            count: 0,
            handle,
            index: 0,
        };
        let lparam = &mut monitor as *mut Monitor as isize;

        EnumDisplayMonitors(
            HDC::default(),
            ptr::null() as *const RECT,
            Some(callback_count_monitor),
            LPARAM(lparam)
        );
        Object::Num(monitor.count as f64)
    }
}

unsafe extern "system"
fn callback_get_handle_by_index(h: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
    let m = &mut *(lparam.0 as *mut Monitor);
    if m.count == m.index {
        m.handle = h;
        return false.into();
    }
    m.count += 1;
    true.into()
}

fn get_monitor_handle_by_index(i: usize) -> HMONITOR {
    unsafe {
        let mut monitor = Monitor {
            count: 0,
            handle: HMONITOR::default(),
            index: i,
        };
        EnumDisplayMonitors(
            HDC::default(),
            ptr::null() as *const RECT,
            Some(callback_get_handle_by_index),
            LPARAM(&mut monitor as *mut Monitor as isize)
        );
        monitor.handle
    }
}

fn get_monitor_name(name: &[u16]) -> Object {
    let mut dd = DISPLAY_DEVICEW::default();
    dd.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
    unsafe {
        EnumDisplayDevicesW(PCWSTR(name.as_ptr()), 0, &mut dd, 0);
    }
    Object::String(
        String::from_utf16_lossy(&dd.DeviceString)
    )
}

pub fn monitor(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if args.len() == 0 {
        return Ok(get_monitor_count(HMONITOR::default()));
    }
    let index = args.get_as_int::<usize>(0, None)?;
    let h = get_monitor_handle_by_index(index);
    if h.is_invalid() {
        return Ok(Object::Bool(false));
    };
    let mut miex = MONITORINFOEXW::default();
    miex.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
    let p_miex = <*mut _>::cast(&mut miex);
    unsafe {
        if ! GetMonitorInfoW(h, p_miex).as_bool() {
            return Err(builtin_func_error(UErrorMessage::UnableToGetMonitorInfo, args.name()));
        }
    }
    let mi = miex.monitorInfo;
    let cmd = args.get_as_int::<u8>(1, Some(MonitorEnum::MON_ALL as u8))?;
    let value = match FromPrimitive::from_u8(cmd).unwrap_or(MonitorEnum::UNKNOWN_MONITOR_CMD) {
        MonitorEnum::MON_ALL => {
            let mut map = HashTbl::new(true, false);
            map.insert((MonitorEnum::MON_X as u8).to_string(), Object::Num(mi.rcMonitor.left.into()));
            map.insert((MonitorEnum::MON_Y as u8).to_string(), Object::Num(mi.rcMonitor.top.into()));
            map.insert((MonitorEnum::MON_WIDTH as u8).to_string(), Object::Num((mi.rcMonitor.right - mi.rcMonitor.left).into()));
            map.insert((MonitorEnum::MON_HEIGHT as u8).to_string(), Object::Num((mi.rcMonitor.bottom - mi.rcMonitor.top).into()));
            map.insert((MonitorEnum::MON_NAME as u8).to_string(), get_monitor_name(&miex.szDevice));
            map.insert((MonitorEnum::MON_PRIMARY as u8).to_string(), Object::Bool(mi.dwFlags == MONITORINFOF_PRIMARY));
            map.insert((MonitorEnum::MON_WORK_X as u8).to_string(), Object::Num(mi.rcWork.left.into()));
            map.insert((MonitorEnum::MON_WORK_Y as u8).to_string(), Object::Num(mi.rcWork.top.into()));
            map.insert((MonitorEnum::MON_WORK_WIDTH as u8).to_string(), Object::Num((mi.rcWork.right - mi.rcWork.left).into()));
            map.insert((MonitorEnum::MON_WORK_HEIGHT as u8).to_string(), Object::Num((mi.rcWork.bottom - mi.rcWork.top).into()));
            return Ok(Object::HashTbl(Arc::new(Mutex::new(map))));
        },
        MonitorEnum::MON_X => mi.rcMonitor.left,
        MonitorEnum::MON_Y => mi.rcMonitor.top,
        MonitorEnum::MON_WIDTH => mi.rcMonitor.right - mi.rcMonitor.left,
        MonitorEnum::MON_HEIGHT => mi.rcMonitor.bottom - mi.rcMonitor.top,
        MonitorEnum::MON_NAME => return Ok(get_monitor_name(&miex.szDevice)),
        MonitorEnum::MON_PRIMARY => return Ok(Object::Bool(mi.dwFlags == MONITORINFOF_PRIMARY)),
        MonitorEnum::MON_WORK_X => mi.rcWork.left,
        MonitorEnum::MON_WORK_Y => mi.rcWork.top,
        MonitorEnum::MON_WORK_WIDTH => mi.rcWork.right - mi.rcWork.left,
        MonitorEnum::MON_WORK_HEIGHT => mi.rcWork.bottom - mi.rcWork.top,
        _ => return Ok(Object::Bool(false))
    };
    Ok(Object::Num(value as f64))
}

#[cfg(feature="chkimg")]
pub fn chkimg(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let save_ss = {
        let singleton = usettings_singleton(None);
        let settings = singleton.0.lock().unwrap();
        settings.chkimg.save_ss
    };
    let default_score = 95;
    let path = args.get_as_string(0, None)?;
    let score = args.get_as_int::<i32>(1, Some(default_score))?;
    if score < 1 && score > 100 {
        return Err(builtin_func_error(UErrorMessage::GivenNumberIsOutOfRange(1.0, 100.0), args.name()));
    }
    let score = score as f64 / 100.0;
    let count = args.get_as_int::<u8>(2, Some(5))?;
    let left = args.get_as_int_or_empty(3, Some(None))?;
    let top = args.get_as_int_or_empty(4, Some(None))?;
    let right = args.get_as_int_or_empty(5, Some(None))?;
    let bottom = args.get_as_int_or_empty(6, Some(None))?;

    let ss = ScreenShot::get(None, left, top, right, bottom)?;
    if save_ss {
        ss.save("chkimg_ss.png")?;
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
    Ok(Object::Array(arr))
}