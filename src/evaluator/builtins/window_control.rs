use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::builtins::window_low::get_current_pos;
use crate::evaluator::builtins::system_controls::is_64bit_os;
use crate::evaluator::UError;
use crate::winapi_util::buffer_to_string;

use std::fmt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};
use std::thread;
use std::mem;
use std::rc::Rc;
use std::cell::RefCell;

use std::ptr::{null_mut};

use winapi::{
    um::{
        winuser,
        dwmapi,
        processthreadsapi,
        // libloaderapi,
        handleapi,
        psapi,
        winnt::{
            PROCESS_QUERY_INFORMATION,
            PROCESS_VM_READ,
            HANDLE
        },
        wingdi,
        wow64apiset,
    },
    shared::{
        windef::{HWND, RECT, POINT, HMONITOR, HDC, LPRECT, },
        minwindef::{
            LPARAM, BOOL, TRUE, FALSE, LPVOID, DWORD, MAX_PATH
        },
    },
};
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

#[derive(Clone)]
pub struct WindowControl {
    next_id: Arc<Mutex<i32>>,
    windows: Arc<Mutex<HashMap<i32, HWND>>>
}

pub fn window_singlton() -> Box<WindowControl> {
    static mut SINGLETON: Option<Box<WindowControl>> = None;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once( || {
            let singleton = WindowControl {
                next_id: Arc::new(Mutex::new(1)),
                windows: Arc::new(Mutex::new(HashMap::new()))
            };
            SINGLETON = Some(Box::new(singleton));
        });
        SINGLETON.clone().unwrap()
    }
}

pub fn get_next_id() -> i32 {
    let s = window_singlton();
    let mut next_id = s.next_id.lock().unwrap();
    let id = next_id.clone();
    *next_id += 1;

    id
}

pub fn set_new_window(id: i32, handle: HWND, to_zero: bool) {
    let s = window_singlton();
    let mut list = s.windows.lock().unwrap();
    list.insert(id, handle);
    if to_zero {
        list.insert(0, handle);
    }
}

fn set_id_zero(hwnd: HWND) {
    let s = window_singlton();
    let mut list = s.windows.lock().unwrap();
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
    let title = get_string_argument_value(&args, 0, None)?;
    let hwnd = match title.as_str() {
        "__GET_ACTIVE_WIN__" => {
            unsafe {
                winuser::GetForegroundWindow()
            }
        },
        "__GET_FROMPOINT_WIN__" => get_hwnd_from_mouse_point(true, "getid")?,
        "__GET_FROMPOINT_OBJ__" => get_hwnd_from_mouse_point(false, "getid")?,
        "__GET_THISUWSC_WIN__" => {
            null_mut()
        },
        "__GET_LOGPRINT_WIN__" => {
            null_mut()
        },
        "__GET_BALLOON_WIN__" => {
            null_mut()
        },
        "__GET_FORM_WIN__" => {
            null_mut()
        },
        "__GET_FORM_WIN2__" => {
            null_mut()
        },
        "__GET_SCHEDULE_WIN__" => {
            null_mut()
        },
        "__GET_STOPFORM_WIN__" => {
            null_mut()
        },
        _ => {
            let class_name = get_string_argument_value(&args, 1, Some("".into()))?;
            let wait = get_num_argument_value(&args, 2, Some(0.0))?;
            let _mdi_title = get_string_argument_value(&args, 3, Some("".into()))?;
            find_window(title, class_name, wait, args.name())?
        },
    };
    if hwnd != null_mut() {
        let mut id = get_id_from_hwnd(hwnd);
        if id == -1.0 {
            let new_id = get_next_id();
            set_new_window(new_id, hwnd, false);
            id = new_id as f64;
        }
        return Ok(Object::Num(id))
    } else {
        return Ok(Object::Num(-1.0))
    }
}

const MAX_NAME_SIZE: usize = 512;

struct TargetWindow {
    hwnd: HWND,
    title: String,
    class_name: String,
    found: bool,
    err: Option<String>,
}

unsafe extern "system"
fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut title_buffer = [0; MAX_NAME_SIZE];
    let mut class_buffer = [0; MAX_NAME_SIZE];
    let target = &mut *(lparam as *mut TargetWindow) as &mut TargetWindow;
    winuser::GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
    match buffer_to_string(&title_buffer) {
        Ok(t) => match t.to_ascii_lowercase().find(target.title.to_ascii_lowercase().as_str()) {
            Some(_) => {
                winuser::GetClassNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as i32);
                match buffer_to_string(&class_buffer) {
                    Ok(c) => match c.to_ascii_lowercase().find(target.class_name.to_ascii_lowercase().as_str()) {
                        Some(_) => {
                            target.title = t;
                            target.class_name = c;
                            target.hwnd = hwnd;
                            target.found = true;
                            return FALSE;
                        },
                        None => ()
                    },
                    Err(e) => {
                        target.err = Some(e);
                        return FALSE; // 終わる
                    },
                }
            },
            None => ()
        },
        Err(e) => {
            target.err = Some(e);
            return FALSE; // 終わる
        },
    }
    TRUE // 次のウィンドウへ
}

fn find_window(title: String, class_name: String, timeout: f64, name: &str) -> Result<HWND, UError> {
    let mut target = TargetWindow {
        hwnd: null_mut(),
        title,
        class_name,
        found: false,
        err: None
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
        loop {
            winuser::EnumWindows(Some(enum_window_proc), &mut target as *mut TargetWindow as LPARAM);
            if target.found {
                break
            }
            if limit.is_some() && now.elapsed() >= limit.unwrap() {
                break;
            }
        }
        match target.err {
            Some(e) => return Err(builtin_func_error(name, e)),
            None => {
                let h = get_process_handle_from_hwnd(target.hwnd);
                winuser::WaitForInputIdle(h, 1000); // 入力可能になるまで最大1秒待つ
                handleapi::CloseHandle(h);
                Ok(target.hwnd)
            }
        }
    }
}

fn get_hwnd_from_mouse_point(toplevel: bool, name: &str) -> Result<HWND, UError> {
    unsafe {
        let point = get_current_pos(name)?;
        let mut hwnd = winuser::WindowFromPoint(point);
        if toplevel {
            loop {
                let parent = winuser::GetParent(hwnd);
                if parent == null_mut() || winuser::IsWindowVisible(parent) == FALSE{
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
    let id = get_non_float_argument_value::<i32>(&args, 0, None)?;
    if id < 0 {
        return Ok(Object::Num(0.0));
    }
    let h = get_hwnd_from_id(id);
    if h != null_mut() {
        unsafe {
            if winuser::IsWindow(h) != 0 {
                return Ok(Object::Num(h as i32 as f64));
            }
        }
    }
    Ok(Object::Num(0.0))
}

fn get_hwnd_from_id(id: i32) -> HWND {
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    match list.get(&id) {
        Some(h) => *h,
        None => null_mut()
    }
}

// HNDTOID
pub fn hndtoid(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let h = get_non_float_argument_value::<i32>(&args, 0, None)?;
    let hwnd = h as i32 as HWND;
    let id = get_id_from_hwnd(hwnd);
    Ok(Object::Num(id))
}

fn get_id_from_hwnd(hwnd: HWND) -> f64 {
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    list.iter().find_map(
        |(key, &val)| if val == hwnd {
            Some(*key as f64)
        } else {
            None
        }
    ).or_else(||Some(-1.0)).unwrap()
}

// ACW
pub fn acw(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = get_non_float_argument_value::<i32>(&args, 0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd == null_mut() {
        return Ok(Object::Empty);
    }
    let x = get_non_float_argument_value(&args, 1, None).ok();
    let y = get_non_float_argument_value(&args, 2, None).ok();
    let w = get_non_float_argument_value(&args, 3, None).ok();
    let h = get_non_float_argument_value(&args, 4, None).ok();
    let ms= get_non_float_argument_value(&args, 5, Some(0)).unwrap_or(0);
    thread::sleep(Duration::from_millis(ms));
    set_window_size(hwnd, x, y, w, h);
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
    let id = get_non_float_argument_value(&args, 0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd == null_mut() {
        return Ok(Object::Empty);
    }
    let cmd = get_non_float_argument_value(&args, 1, None)?;
    match FromPrimitive::from_i32(cmd).unwrap_or(CtrlWinCmd::UNKNOWN_CTRLWIN_CMD) {
        CtrlWinCmd::CLOSE => unsafe {
            winuser::PostMessageA(hwnd, winuser::WM_CLOSE, 0, 0);
        },
        CtrlWinCmd::CLOSE2 => unsafe {
            winuser::PostMessageA(hwnd, winuser::WM_DESTROY, 0, 0);
        },
        CtrlWinCmd::ACTIVATE => unsafe {
            winuser::SetForegroundWindow(hwnd);
        },
        CtrlWinCmd::HIDE => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_HIDE);
        },
        CtrlWinCmd::SHOW => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_SHOW);
        },
        CtrlWinCmd::MIN => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_MINIMIZE);
        },
        CtrlWinCmd::MAX => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_MAXIMIZE);
        },
        CtrlWinCmd::NORMAL => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_SHOWNORMAL);
        },
        CtrlWinCmd::TOPMOST => unsafe {
            winuser::SetWindowPos(
                hwnd,
                winuser::HWND_TOPMOST,
                0, 0, 0, 0,
                winuser::SWP_NOMOVE | winuser::SWP_NOSIZE
            );
        },
        CtrlWinCmd::NOTOPMOST => unsafe {
            winuser::SetWindowPos(
                hwnd,
                winuser::HWND_NOTOPMOST,
                0, 0, 0, 0,
                winuser::SWP_NOMOVE | winuser::SWP_NOSIZE
            );
        },
        CtrlWinCmd::TOPNOACTV => unsafe {
            for h in vec![winuser::HWND_TOPMOST, winuser::HWND_NOTOPMOST] {
                winuser::SetWindowPos(
                    hwnd,
                    h,
                    0, 0, 0, 0,
                    winuser::SWP_NOMOVE | winuser::SWP_NOSIZE | winuser::SWP_NOACTIVATE
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
    UNKNOWN_STATUS = -1,
}

fn get_window_size(h: HWND) -> HashMap<u8, i32> {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    let mut ret = HashMap::new();
    unsafe {
        let mut aero_enabled = FALSE;
        dwmapi::DwmIsCompositionEnabled(&mut aero_enabled);
        if aero_enabled == FALSE {
            // AEROがオフならGetWindowRect
            winuser::GetWindowRect(h, &mut rect);
        } else {
            dwmapi::DwmGetWindowAttribute(
                h,
                dwmapi::DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut RECT as LPVOID,
                mem::size_of::<RECT>() as u32
            );
        };
    }
    ret.insert(StatusEnum::ST_X as u8, rect.left);
    ret.insert(StatusEnum::ST_Y as u8, rect.top);
    ret.insert(StatusEnum::ST_WIDTH as u8, rect.right - rect.left);
    ret.insert(StatusEnum::ST_HEIGHT as u8, rect.bottom - rect.top);
    ret
}

pub fn set_window_size(hwnd: HWND, x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) {
    let default_rect = get_window_size(hwnd);
    let x = x.unwrap_or(*default_rect.get(&(StatusEnum::ST_X as u8)).unwrap());
    let y = y.unwrap_or(*default_rect.get(&(StatusEnum::ST_Y as u8)).unwrap());
    let w = w.unwrap_or(*default_rect.get(&(StatusEnum::ST_WIDTH as u8)).unwrap());
    let h = h.unwrap_or(*default_rect.get(&(StatusEnum::ST_HEIGHT as u8)).unwrap());
    unsafe {
        let mut rect1: RECT= mem::zeroed();
        let mut rect2: RECT= mem::zeroed();
        let mut dx = 0;
        let mut dy = 0;
        let mut dw = 0;
        let mut dh = 0;
        let mut aero_enabled = FALSE;
        dwmapi::DwmIsCompositionEnabled(&mut aero_enabled);
        if aero_enabled == TRUE {
            dwmapi::DwmGetWindowAttribute(
                hwnd,
                dwmapi::DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect1 as *mut RECT as LPVOID,
                mem::size_of::<RECT>() as u32
            );
            winuser::GetWindowRect(hwnd, &mut rect2);
            dx = rect2.left - rect1.left;
            dy = rect2.top - rect1.top;
            dw = -dx + rect2.right - rect1.right;
            dh = -dy + rect2.bottom - rect1.bottom;
        };
        winuser::MoveWindow(hwnd, x + dx, y + dy, w + dw, h + dh, TRUE);
    }
}

fn get_client_size(h: HWND) -> HashMap<u8, i32> {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    let mut ret = HashMap::new();
    unsafe {
        winuser::GetClientRect(h, &mut rect);
        let mut point = POINT {x: rect.left, y: rect.top};
        winuser::MapWindowPoints(h, null_mut(), &mut point, 1);
        ret.insert(StatusEnum::ST_CLX as u8, point.x);
        ret.insert(StatusEnum::ST_CLY as u8, point.y);
        ret.insert(StatusEnum::ST_CLWIDTH as u8, rect.right - rect.left);
        ret.insert(StatusEnum::ST_CLHEIGHT as u8, rect.bottom - rect.top);
    }
    ret
}

fn get_window_text(hwnd: HWND) -> BuiltinFuncResult {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        winuser::GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        buffer_to_string(&buffer).map_or_else(
            |e| Err(builtin_func_error("status", e)),
            |s| Ok(Object::String(s))
        )
    }
}

fn get_class_name(hwnd: HWND) -> BuiltinFuncResult {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        winuser::GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        buffer_to_string(&buffer).map_or_else(
            |e| Err(builtin_func_error("status", e)),
            |s| Ok(Object::String(s))
        )
    }
}

fn get_parent(hwnd: HWND) -> Object {
    unsafe {
        let parent = winuser::GetParent(hwnd);
        Object::Num(get_id_from_hwnd(parent))
    }
}

fn is_maximized(hwnd: HWND)-> Object {
    let mut wp = winuser::WINDOWPLACEMENT {
        length: 0,
        flags: 0,
        showCmd: 0,
        ptMinPosition: POINT {x: 0, y: 0},
        ptMaxPosition: POINT {x: 0, y: 0},
        rcNormalPosition: RECT {left: 0, top: 0, right: 0, bottom: 0},
    };
    unsafe {
        winuser::GetWindowPlacement(hwnd, &mut wp);
        Object::Bool(wp.showCmd == winuser::SW_MAXIMIZE as u32)
    }
}

fn is_active_window(hwnd: HWND) -> Object {
    unsafe {
        Object::Bool(winuser::GetForegroundWindow() == hwnd)
    }
}

fn get_process_id_from_hwnd(hwnd: HWND) -> u32 {
    let mut pid: DWORD = 0;
    unsafe {
        winuser::GetWindowThreadProcessId(hwnd, &mut pid);
        pid
    }
}

fn is_process_64bit(hwnd: HWND) -> Object {
    if ! is_64bit_os("status").unwrap_or(true) {
        // 32bit OSなら必ずfalse
        return Object::Bool(false);
    }
    let h = get_process_handle_from_hwnd(hwnd);
    let mut b = FALSE;
    unsafe {
        wow64apiset::IsWow64Process(h, &mut b);
        Object::Bool(b == FALSE)
    }
}

fn get_process_handle_from_hwnd(hwnd: HWND) -> HANDLE {
    let pid = get_process_id_from_hwnd(hwnd);
    unsafe {
        processthreadsapi::OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            FALSE, pid
        )
    }
}

fn get_process_path_from_hwnd(hwnd: HWND) -> BuiltinFuncResult {
    let mut buffer = [0; MAX_PATH];
    unsafe {
        let handle = get_process_handle_from_hwnd(hwnd);
        psapi::GetModuleFileNameExW(handle, null_mut(), buffer.as_mut_ptr(), MAX_PATH as u32);
        handleapi::CloseHandle(handle);
    }
    buffer_to_string(&buffer).map_or_else(
        |e| Err(builtin_func_error("status", e)),
        |s| Ok(Object::String(s))
    )
}

fn get_monitor_index_from_hwnd(hwnd: HWND) -> Object {
    let h = unsafe {
        winuser::MonitorFromWindow(hwnd, winuser::MONITOR_DEFAULTTONEAREST)
    };
    get_monitor_count(h)
}


fn get_status_result(hwnd: HWND, st: u8) -> BuiltinFuncResult {
    let obj = match FromPrimitive::from_u8(st).unwrap_or(StatusEnum::UNKNOWN_STATUS) {
        StatusEnum::ST_TITLE => get_window_text(hwnd)?,
        StatusEnum::ST_CLASS => get_class_name(hwnd)?,
        StatusEnum::ST_X |
        StatusEnum::ST_Y |
        StatusEnum::ST_WIDTH |
        StatusEnum::ST_HEIGHT => Object::Num(*get_window_size(hwnd).get(&st).unwrap_or(&0) as f64),
        StatusEnum::ST_CLX |
        StatusEnum::ST_CLY |
        StatusEnum::ST_CLWIDTH |
        StatusEnum::ST_CLHEIGHT => Object::Num(*get_client_size(hwnd).get(&st).unwrap_or(&0) as f64),
        StatusEnum::ST_PARENT => get_parent(hwnd),
        StatusEnum::ST_ICON => unsafe {
            Object::Bool(winuser::IsIconic(hwnd) == TRUE)
        },
        StatusEnum::ST_MAXIMIZED => is_maximized(hwnd),
        StatusEnum::ST_VISIBLE => unsafe {
            Object::Bool(winuser::IsWindowVisible(hwnd) == TRUE)
        },
        StatusEnum::ST_ACTIVE => is_active_window(hwnd),
        StatusEnum::ST_BUSY => unsafe {
            Object::Bool(winuser::IsHungAppWindow(hwnd) == TRUE)
        },
        StatusEnum::ST_ISID => unsafe {
            Object::Bool(winuser::IsWindow(hwnd) == TRUE)
        },
        StatusEnum::ST_WIN64 => is_process_64bit(hwnd),
        StatusEnum::ST_PATH => get_process_path_from_hwnd(hwnd)?,
        StatusEnum::ST_PROCESS => Object::Num(get_process_id_from_hwnd(hwnd) as f64),
        StatusEnum::ST_MONITOR => get_monitor_index_from_hwnd(hwnd),
        _ => Object::Bool(false) // 定数以外を受けた場合false
    };
    Ok(obj)
}

fn get_all_status(hwnd: HWND) -> BuiltinFuncResult {
    let mut stats = HashTbl::new(true, false);
    stats.insert((StatusEnum::ST_TITLE as u8).to_string(), get_window_text(hwnd)?);
    stats.insert((StatusEnum::ST_CLASS as u8).to_string(), get_class_name(hwnd)?);
    let rect = get_window_size(hwnd);
    stats.insert((StatusEnum::ST_X as u8).to_string(), Object::Num(*rect.get(&(StatusEnum::ST_X as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_Y as u8).to_string(), Object::Num(*rect.get(&(StatusEnum::ST_Y as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_WIDTH as u8).to_string(), Object::Num(*rect.get(&(StatusEnum::ST_WIDTH as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_HEIGHT as u8).to_string(), Object::Num(*rect.get(&(StatusEnum::ST_HEIGHT as u8)).unwrap_or(&0) as f64));
    let crect = get_client_size(hwnd);
    stats.insert((StatusEnum::ST_CLX as u8).to_string(), Object::Num(*crect.get(&(StatusEnum::ST_CLX as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_CLY as u8).to_string(), Object::Num(*crect.get(&(StatusEnum::ST_CLY as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_CLWIDTH as u8).to_string(), Object::Num(*crect.get(&(StatusEnum::ST_CLWIDTH as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_CLHEIGHT as u8).to_string(), Object::Num(*crect.get(&(StatusEnum::ST_CLHEIGHT as u8)).unwrap_or(&0) as f64));
    stats.insert((StatusEnum::ST_PARENT as u8).to_string(), get_parent(hwnd));
    stats.insert((StatusEnum::ST_ICON as u8).to_string(), unsafe{ Object::Bool(winuser::IsIconic(hwnd) == TRUE) });
    stats.insert((StatusEnum::ST_MAXIMIZED as u8).to_string(), is_maximized(hwnd));
    stats.insert((StatusEnum::ST_VISIBLE as u8).to_string(), unsafe{ Object::Bool(winuser::IsWindowVisible(hwnd) == TRUE) });
    stats.insert((StatusEnum::ST_ACTIVE as u8).to_string(), is_active_window(hwnd));
    stats.insert((StatusEnum::ST_BUSY as u8).to_string(), unsafe{ Object::Bool(winuser::IsHungAppWindow(hwnd) == TRUE) });
    stats.insert((StatusEnum::ST_ISID as u8).to_string(), unsafe{ Object::Bool(winuser::IsWindow(hwnd) == TRUE) });
    stats.insert((StatusEnum::ST_WIN64 as u8).to_string(), is_process_64bit(hwnd));
    stats.insert((StatusEnum::ST_PATH as u8).to_string(), get_process_path_from_hwnd(hwnd)?);
    stats.insert((StatusEnum::ST_PROCESS as u8).to_string(), Object::Num(get_process_id_from_hwnd(hwnd) as f64));
    stats.insert((StatusEnum::ST_MONITOR as u8).to_string(), get_monitor_index_from_hwnd(hwnd));
    Ok(Object::HashTbl(Rc::new(RefCell::new(stats))))
}

pub fn status(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let hwnd = get_hwnd_from_id(
        get_non_float_argument_value(&args, 0, None)?
    );
    if args.len() > 2 {
        let mut i = 1;
        // let mut stats = vec![Object::Empty; 22];
        let mut stats = HashTbl::new(true, false);
        while i < args.len() {
            let cmd = get_non_float_argument_value::<u8>(&args, i, None)?;
            let value = get_status_result(hwnd, cmd)?;
            stats.insert(cmd.to_string(), value);
            i += 1;
        }
        Ok(Object::HashTbl(Rc::new(RefCell::new(stats))))
    } else {
        let cmd = get_non_float_argument_value::<u8>(&args, 1, None)?;
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

struct Monitor {
    count: usize,
    handle: HMONITOR,
    index: usize,
}

// nullを渡すと全モニタ数、モニタのハンドルを渡すとそのインデックスを返す
fn get_monitor_count(handle: HMONITOR) -> Object {
    unsafe extern "system"
    fn monitor_enum_proc(h: HMONITOR, _: HDC, _: LPRECT, lparam: LPARAM) -> BOOL {
        let m = &mut *(lparam as *mut Monitor) as &mut Monitor;
        if m.handle == h {
            return FALSE;
        }
        m.count += 1;
        TRUE
    }
    unsafe {
        let mut monitor = Monitor {
            count: 0,
            handle,
            index: 0,
        };
        winuser::EnumDisplayMonitors(
            null_mut(),
            null_mut(),
            Some(monitor_enum_proc),
            &mut monitor as *mut Monitor as LPARAM
        );
        Object::Num(monitor.count as f64)
    }
}

fn get_monitor_handle_by_index(i: usize) -> HMONITOR {
    unsafe extern "system"
    fn monitor_enum_proc(h: HMONITOR, _: HDC, _: LPRECT, lparam: LPARAM) -> BOOL {
        let m = &mut *(lparam as *mut Monitor) as &mut Monitor;
        if m.count == m.index {
            m.handle = h;
            return FALSE;
        }
        m.count += 1;
        TRUE
    }
    unsafe {
        let mut monitor = Monitor {
            count: 0,
            handle: null_mut(),
            index: i,
        };
        winuser::EnumDisplayMonitors(
            null_mut(),
            null_mut(),
            Some(monitor_enum_proc),
            &mut monitor as *mut Monitor as LPARAM
        );
        monitor.handle
    }
}

fn get_monitor_name(name: &[u16]) -> Object {
    let mut dd: wingdi::DISPLAY_DEVICEW = unsafe {mem::zeroed()};
    dd.cb = mem::size_of::<wingdi::DISPLAY_DEVICEW>() as u32;
    unsafe {
        winuser::EnumDisplayDevicesW(name.as_ptr(), 0, &mut dd, 0);
    }
    Object::String(
        buffer_to_string(&dd.DeviceString).map_or("".to_string(), |s| s)
    )
}

pub fn monitor(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if args.len() == 0 {
        return Ok(get_monitor_count(null_mut()));
    }
    let index = get_non_float_argument_value::<usize>(&args, 0, None)?;
    let h = get_monitor_handle_by_index(index);
    if h == null_mut() {
        return Ok(Object::Bool(false));
    };
    let mut miex: winuser::MONITORINFOEXW = unsafe {mem::zeroed()};
    miex.cbSize = mem::size_of::<winuser::MONITORINFOEXW>() as u32;
    let p_miex = <*mut _>::cast(&mut miex);
    unsafe {
        if winuser::GetMonitorInfoW(h, p_miex) == FALSE {
            return Err(builtin_func_error("monitor", "failed to get monitor information"));
        }
    }
    let cmd = get_non_float_argument_value::<u8>(&args, 1, Some(MonitorEnum::MON_ALL as u8))?;
    let value = match FromPrimitive::from_u8(cmd).unwrap_or(MonitorEnum::UNKNOWN_MONITOR_CMD) {
        MonitorEnum::MON_ALL => {
            let mut map = HashTbl::new(false, true);
            map.insert((MonitorEnum::MON_X as u8).to_string(), Object::Num(miex.rcMonitor.left.into()));
            map.insert((MonitorEnum::MON_Y as u8).to_string(), Object::Num(miex.rcMonitor.top.into()));
            map.insert((MonitorEnum::MON_WIDTH as u8).to_string(), Object::Num((miex.rcMonitor.right - miex.rcMonitor.left).into()));
            map.insert((MonitorEnum::MON_HEIGHT as u8).to_string(), Object::Num((miex.rcMonitor.bottom - miex.rcMonitor.top).into()));
            map.insert((MonitorEnum::MON_NAME as u8).to_string(), get_monitor_name(&miex.szDevice));
            map.insert((MonitorEnum::MON_PRIMARY as u8).to_string(), Object::Bool(miex.dwFlags == winuser::MONITORINFOF_PRIMARY));
            map.insert((MonitorEnum::MON_WORK_X as u8).to_string(), Object::Num(miex.rcWork.left.into()));
            map.insert((MonitorEnum::MON_WORK_Y as u8).to_string(), Object::Num(miex.rcWork.top.into()));
            map.insert((MonitorEnum::MON_WORK_WIDTH as u8).to_string(), Object::Num((miex.rcWork.right - miex.rcWork.left).into()));
            map.insert((MonitorEnum::MON_WORK_HEIGHT as u8).to_string(), Object::Num((miex.rcWork.bottom - miex.rcWork.top).into()));
            return Ok(Object::HashTbl(Rc::new(RefCell::new(map))));
        },
        MonitorEnum::MON_X => miex.rcMonitor.left,
        MonitorEnum::MON_Y => miex.rcMonitor.top,
        MonitorEnum::MON_WIDTH => miex.rcMonitor.right - miex.rcMonitor.left,
        MonitorEnum::MON_HEIGHT => miex.rcMonitor.bottom - miex.rcMonitor.top,
        MonitorEnum::MON_NAME => return Ok(get_monitor_name(&miex.szDevice)),
        MonitorEnum::MON_PRIMARY => return Ok(Object::Bool(miex.dwFlags == winuser::MONITORINFOF_PRIMARY)),
        MonitorEnum::MON_WORK_X => miex.rcWork.left,
        MonitorEnum::MON_WORK_Y => miex.rcWork.top,
        MonitorEnum::MON_WORK_WIDTH => miex.rcWork.right - miex.rcWork.left,
        MonitorEnum::MON_WORK_HEIGHT => miex.rcWork.bottom - miex.rcWork.top,
        _ => return Ok(Object::Bool(false))
    };
    Ok(Object::Num(value as f64))
}