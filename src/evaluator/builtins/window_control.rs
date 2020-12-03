use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::builtins::window_low::get_current_pos;
use crate::evaluator::builtins::system_controls::is_64bit_os;
use crate::evaluator::environment::NamedObject;

use std::fmt;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};
use std::thread;
use std::mem;

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

#[derive(Clone)]
struct WindowControl {
    next_id: Arc<Mutex<i32>>,
    windows: Arc<Mutex<HashMap<i32, HWND>>>
}

fn window_singlton() -> Box<WindowControl> {
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

fn get_next_id() -> i32 {
    let s = window_singlton();
    let mut next_id = s.next_id.lock().unwrap();
    let id = next_id.clone();
    *next_id += 1;

    id
}

fn set_new_window(key:i32, handle: HWND) {
    let s = window_singlton();
    let mut list = s.windows.lock().unwrap();
    list.insert(key, handle);
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

pub fn set_builtins(vec: &mut Vec<NamedObject>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("getid", 4, getid),
        ("idtohnd", 1, idtohnd),
        ("hndtoid", 1, hndtoid),
        ("clkitem", 5, clkitem),
        ("ctrlwin", 2, ctrlwin),
        ("status", 22, status),
        ("acw", 5, acw),
        ("monitor", 2, monitor),
    ];
    for (name, arg_len, func) in funcs {
        vec.push(NamedObject::new_builtin_func(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func)));
    }
    let str_constant = vec![
        ("GET_ACTIVE_WIN"    , GET_ACTIVE_WIN),
        ("GET_FROMPOINT_WIN" , GET_FROMPOINT_WIN),
        ("GET_FROMPOINT_OBJ" , GET_FROMPOINT_OBJ),
        ("GET_THISUWSC_WIN"  , GET_THISUWSC_WIN),
        ("GET_LOGPRINT_WIN"  , GET_LOGPRINT_WIN),
        ("GET_BALLOON_WIN"   , GET_BALLOON_WIN),
        ("GET_FUKIDASI_WIN"  , GET_FUKIDASI_WIN),
        ("GET_FORM_WIN"      , GET_FORM_WIN),
        ("GET_FORM_WIN2"     , GET_FORM_WIN2),
        ("GET_SCHEDULE_WIN"  , GET_SCHEDULE_WIN),
        ("GET_STOPFORM_WIN"  , GET_STOPFORM_WIN),
    ];
    for (key, value) in str_constant {
        vec.push(NamedObject::new_builtin_const(key.to_ascii_uppercase(), Object::String(value.to_string())));
    }
    let num_constant = vec![
        // ctrlwin
        ("CLOSE", CLOSE),
        ("CLOSE2", CLOSE2),
        ("ACTIVATE", ACTIVATE),
        ("HIDE", HIDE),
        ("SHOW", SHOW),
        ("MIN", MIN),
        ("MAX", MAX),
        ("NORMAL", NORMAL),
        ("TOPMOST", TOPMOST),
        ("NOTOPMOST", NOTOPMOST),
        ("TOPNOACTV", TOPNOACTV),
        // status
        ("ST_ALL", ST_ALL),
        ("ST_TITLE", ST_TITLE),
        ("ST_CLASS", ST_CLASS),
        ("ST_X", ST_X),
        ("ST_Y", ST_Y),
        ("ST_WIDTH", ST_WIDTH),
        ("ST_HEIGHT", ST_HEIGHT),
        ("ST_CLX", ST_CLX),
        ("ST_CLY", ST_CLY),
        ("ST_CLWIDTH", ST_CLWIDTH),
        ("ST_CLHEIGHT", ST_CLHEIGHT),
        ("ST_PARENT", ST_PARENT),
        ("ST_ICON", ST_ICON),
        ("ST_MAXIMIZED", ST_MAXIMIZED),
        ("ST_VISIBLE", ST_VISIBLE),
        ("ST_ACTIVE", ST_ACTIVE),
        ("ST_BUSY", ST_BUSY),
        ("ST_ISID", ST_ISID),
        ("ST_WIN64", ST_WIN64),
        ("ST_PATH", ST_PATH),
        ("ST_PROCESS", ST_PROCESS),
        ("ST_MONITOR", ST_MONITOR),
        // monitor
        ("MON_X", MON_X),
        ("MON_Y", MON_Y),
        ("MON_WIDTH", MON_WIDTH),
        ("MON_HEIGHT", MON_HEIGHT),
        ("MON_NAME", MON_NAME),
        ("MON_ISMAIN", MON_ISMAIN),
        ("MON_WORK_X", MON_WORK_X),
        ("MON_WORK_Y", MON_WORK_Y),
        ("MON_WORK_WIDTH", MON_WORK_WIDTH),
        ("MON_WORK_HEIGHT", MON_WORK_HEIGHT),
        ("MON_ALL", MON_ALL),
    ];
    for (key, value) in num_constant {
        vec.push(NamedObject::new_builtin_const(key.to_ascii_uppercase(), Object::Num(value.into())));
    }
}

// GETID
const GET_ACTIVE_WIN: &str    = "__GET_ACTIVE_WIN__";
const GET_FROMPOINT_WIN: &str = "__GET_FROMPOINT_WIN__";
const GET_FROMPOINT_OBJ: &str = "__GET_FROMPOINT_OBJ__";
const GET_THISUWSC_WIN: &str  = "__GET_THISUWSC_WIN__";
const GET_LOGPRINT_WIN: &str  = "__GET_LOGPRINT_WIN__";
const GET_BALLOON_WIN: &str   = "__GET_FUKIDASI_WIN__";
const GET_FUKIDASI_WIN: &str  = "__GET_FUKIDASI_WIN__";
const GET_FORM_WIN: &str      = "__GET_FORM_WIN__";
const GET_FORM_WIN2: &str     = "__GET_FORM_WIN2__";
const GET_SCHEDULE_WIN: &str  = "__GET_SCHEDULE_WIN__";
const GET_STOPFORM_WIN: &str  = "__GET_STOPFORM_WIN__";

pub fn getid(args: Vec<Object>) -> Object {
    let hwnd = match args[0].clone() {
        Object::String(title) => {
            match title.as_str() {
                GET_ACTIVE_WIN => {
                    unsafe {
                        winuser::GetForegroundWindow()
                    }
                },
                GET_FROMPOINT_WIN => match get_hwnd_from_mouse_point(true) {
                    Ok(h) => h,
                    Err(Object::Error(err)) => return builtin_func_error("getid", err.as_str()),
                    Err(_) => return builtin_func_error("getid", "unknown error"),
                },
                GET_FROMPOINT_OBJ => match get_hwnd_from_mouse_point(false) {
                        Ok(h) => h,
                        Err(Object::Error(err)) => return builtin_func_error("getid", err.as_str()),
                        Err(_) => return builtin_func_error("getid", "unknown error"),
                },
                GET_THISUWSC_WIN => {
                    null_mut()
                },
                GET_LOGPRINT_WIN => {
                    null_mut()
                },
                GET_BALLOON_WIN => {
                    null_mut()
                },
                GET_FORM_WIN => {
                    null_mut()
                },
                GET_FORM_WIN2 => {
                    null_mut()
                },
                GET_SCHEDULE_WIN => {
                    null_mut()
                },
                GET_STOPFORM_WIN => {
                    null_mut()
                },
                _ => {
                    let class_name =if args.len() >= 2 {
                        match &args[1] {
                            Object::String(name) => name.clone(),
                            Object::Num(n) => n.to_string(),
                            _ => return Object::Num(-1.0)
                        }
                    } else {
                        "".to_string()
                    };
                    let wait = if args.len() >= 3 {
                        match args[2] {
                            Object::Num(sec) => sec,
                            _ => 0.0 // uwscの初期値
                        }
                    } else {
                        0.0 // uwscの初期値
                    };
                    let _mdi_title = if args.len() >= 4 {
                        match &args[3] {
                            Object::String(mdi) => Some(mdi.clone()),
                            _ => None
                        }
                    } else {
                        None
                    };
                    match find_window(title, class_name, wait) {
                        Ok(h) => h,
                        Err(e) => return Object::Error(e)
                    }
                },
            }
        },
        _ => return Object::Error(format!("string required for title"))
    };
    if hwnd != null_mut() {
        let mut id = get_id_from_hwnd(hwnd);
        if id == -1.0 {
            let new_id = get_next_id();
            set_new_window(new_id, hwnd);
            id = new_id as f64;
        }
        return Object::Num(id)
    } else {
        return Object::Num(-1.0)
    }
}

const MAX_NAME_SIZE: usize = 512;

fn buffer_to_string( buffer: &[u16] ) -> Result<String, String> {
    buffer.iter()
        .position(|wch| wch == &0)
        .ok_or("String : Can't find zero terminator !".to_owned())
        .and_then(
            |ix| String::from_utf16( &buffer[..ix] )
            .map_err(|e| e.to_string())
        )
}

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

fn find_window(title: String, class_name: String, timeout: f64) -> Result<HWND, String> {
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
            Some(e) => return Err(e.clone()),
            None => Ok(target.hwnd)
        }
    }
}

fn get_hwnd_from_mouse_point(toplevel: bool) -> Result<HWND, Object> {
    unsafe {
        let point = match get_current_pos() {
            Ok(p) => p,
            Err(err) => return Err(err)
        };
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
pub fn idtohnd(args: Vec<Object>) -> Object {
    match args[0] {
        Object::Num(id) => {
            if id < 0.0 {
                return Object::Num(0.0);
            }
            let h = get_hwnd_from_id(id as i32);
            if h == null_mut() {
                Object::Num(0.0)
            } else {
                unsafe {
                    if winuser::IsWindow(h) != 0 {
                        Object::Num(h as i32 as f64)
                    } else {
                        Object::Num(0.0)
                    }
                }
            }
        },
        _ => builtin_func_error("idtohnd", "invalid argumant")
    }
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
pub fn hndtoid(args: Vec<Object>) -> Object {
    match args[0] {
        Object::Num(h) => {
            let hwnd = h as i32 as HWND;
            let id = get_id_from_hwnd(hwnd);
            Object::Num(id)
        },
        _ => builtin_func_error("hndtoid", "invalid argumant")
    }
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
pub fn acw(args: Vec<Object>) -> Object {
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    let hwnd = match args[0] {
        Object::Num(n) => {
            let id = n as i32;
            match list.get(&id) {
                Some(h) => *h,
                None => return Object::Empty
            }
        },
        _ => return builtin_func_error("acw", format!("bad argument: {}", args[0]).as_str())
    };
    let rect = get_window_size(hwnd);
    let x = get_non_float_argument_value(&args, 1, Some(*rect.get(&ST_X).unwrap())).unwrap_or(0);
    let y = get_non_float_argument_value(&args, 2, Some(*rect.get(&ST_Y).unwrap())).unwrap_or(0);
    let w = get_non_float_argument_value(&args, 3, Some(*rect.get(&ST_WIDTH).unwrap())).unwrap_or(0);
    let h = get_non_float_argument_value(&args, 4, Some(*rect.get(&ST_HEIGHT).unwrap())).unwrap_or(0);
    let ms= get_non_float_argument_value(&args, 5, Some(0)).unwrap_or(0);
    thread::sleep(Duration::from_millis(ms));
    set_window_size(hwnd, x, y, w, h);
    Object::Empty
}


// CLKITEM
pub fn clkitem(args: Vec<Object>) -> Object {
    Object::Bool(args.len() > 0)
}

// CTRLWIN

const CLOSE: u8     = 2;
const CLOSE2: u8    = 3;
const ACTIVATE: u8  = 1;
const HIDE: u8      = 4;
const SHOW: u8      = 5;
const MIN: u8       = 6;
const MAX: u8       = 7;
const NORMAL: u8    = 8;
const TOPMOST: u8   = 9;
const NOTOPMOST: u8 = 10;
const TOPNOACTV: u8 = 11;

pub fn ctrlwin(args: Vec<Object>) -> Object {
    let id = match get_non_float_argument_value(&args, 0, Some(-2)) {
        Ok(n) => n,
        Err(err) => return builtin_func_error("ctrlwin", err.as_str())
    };
    if id == -2 {
        return builtin_func_error("ctrlwin", "id required")
    }
    let hwnd = get_hwnd_from_id(id);
    if hwnd == null_mut() {
        return Object::Empty;
    }
    let cmd = match get_non_float_argument_value::<u8>(&args, 1, Some(0)) {
        Ok(n) => n,
        Err(err) => return builtin_func_error("ctrlwin", err.as_str())
    };
    match cmd {
        0 => return builtin_func_error("ctrlwin", "command required"),
        CLOSE => unsafe {
            winuser::PostMessageA(hwnd, winuser::WM_CLOSE, 0, 0);
        },
        CLOSE2 => unsafe {
            winuser::PostMessageA(hwnd, winuser::WM_DESTROY, 0, 0);
        },
        ACTIVATE => unsafe {
            winuser::SetForegroundWindow(hwnd);
        },
        HIDE => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_HIDE);
        },
        SHOW => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_SHOW);
        },
        MIN => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_MINIMIZE);
        },
        MAX => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_MAXIMIZE);
        },
        NORMAL => unsafe {
            winuser::ShowWindow(hwnd, winuser::SW_SHOWNORMAL);
        },
        TOPMOST => unsafe {
            winuser::SetWindowPos(
                hwnd,
                winuser::HWND_TOPMOST,
                0, 0, 0, 0,
                winuser::SWP_NOMOVE | winuser::SWP_NOSIZE
            );
        },
        NOTOPMOST => unsafe {
            winuser::SetWindowPos(
                hwnd,
                winuser::HWND_NOTOPMOST,
                0, 0, 0, 0,
                winuser::SWP_NOMOVE | winuser::SWP_NOSIZE
            );
        },
        TOPNOACTV => unsafe {
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
    Object::Empty
}

// STATUS

const ST_ALL       :u8 = 0;
const ST_TITLE     :u8 = 9;
const ST_CLASS     :u8 = 14;
const ST_X         :u8 = 1;
const ST_Y         :u8 = 2;
const ST_WIDTH     :u8 = 3;
const ST_HEIGHT    :u8 = 4;
const ST_CLX       :u8 = 5;
const ST_CLY       :u8 = 6;
const ST_CLWIDTH   :u8 = 7;
const ST_CLHEIGHT  :u8 = 8;
const ST_PARENT    :u8 = 16;
const ST_ICON      :u8 = 10;
const ST_MAXIMIZED :u8 = 11;
const ST_VISIBLE   :u8 = 12;
const ST_ACTIVE    :u8 = 13;
const ST_BUSY      :u8 = 15;
const ST_ISID      :u8 = 21;
const ST_WIN64     :u8 = 19;
const ST_PATH      :u8 = 17;
const ST_PROCESS   :u8 = 18;
const ST_MONITOR   :u8 = 20;

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
    ret.insert(ST_X, rect.left);
    ret.insert(ST_Y, rect.top);
    ret.insert(ST_WIDTH, rect.right - rect.left);
    ret.insert(ST_HEIGHT, rect.bottom - rect.top);
    ret
}

fn set_window_size(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
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
        ret.insert(ST_CLX, point.x);
        ret.insert(ST_CLY, point.y);
        ret.insert(ST_CLWIDTH, rect.right - rect.left);
        ret.insert(ST_CLHEIGHT, rect.bottom - rect.top);
    }
    ret
}

fn get_window_text(hwnd: HWND) -> Object {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        winuser::GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        buffer_to_string(&buffer).map_or_else(
            |e| builtin_func_error("status", e.as_str()),
            |s| Object::String(s)
        )
    }
}

fn get_class_name(hwnd: HWND) -> Object {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        winuser::GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        buffer_to_string(&buffer).map_or_else(
            |e| builtin_func_error("status", e.as_str()),
            |s| Object::String(s)
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
    if ! is_64bit_os().unwrap_or(true) {
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

fn get_process_path_from_hwnd(hwnd: HWND) -> Object {
    let mut buffer = [0; MAX_PATH];
    unsafe {
        let handle = get_process_handle_from_hwnd(hwnd);
        psapi::GetModuleFileNameExW(handle, null_mut(), buffer.as_mut_ptr(), MAX_PATH as u32);
        handleapi::CloseHandle(handle);
    }
    buffer_to_string(&buffer).map_or_else(
        |e| builtin_func_error("status", e.as_str()),
        |s| Object::String(s)
    )
}

fn get_monitor_index_from_hwnd(hwnd: HWND) -> Object {
    let h = unsafe {
        winuser::MonitorFromWindow(hwnd, winuser::MONITOR_DEFAULTTONEAREST)
    };
    get_monitor_count(h)
}


fn get_status_result(hwnd: HWND, st: u8) -> Object {
    match st {
        ST_TITLE => get_window_text(hwnd),
        ST_CLASS => get_class_name(hwnd),
        ST_X |
        ST_Y |
        ST_WIDTH |
        ST_HEIGHT => Object::Num(*get_window_size(hwnd).get(&st).unwrap_or(&0) as f64),
        ST_CLX |
        ST_CLY |
        ST_CLWIDTH |
        ST_CLHEIGHT => Object::Num(*get_client_size(hwnd).get(&st).unwrap_or(&0) as f64),
        ST_PARENT => get_parent(hwnd),
        ST_ICON => unsafe {
            Object::Bool(winuser::IsIconic(hwnd) == TRUE)
        },
        ST_MAXIMIZED => is_maximized(hwnd),
        ST_VISIBLE => unsafe {
            Object::Bool(winuser::IsWindowVisible(hwnd) == TRUE)
        },
        ST_ACTIVE => is_active_window(hwnd),
        ST_BUSY => unsafe {
            Object::Bool(winuser::IsHungAppWindow(hwnd) == TRUE)
        },
        ST_ISID => unsafe {
            Object::Bool(winuser::IsWindow(hwnd) == TRUE)
        },
        ST_WIN64 => is_process_64bit(hwnd),
        ST_PATH => get_process_path_from_hwnd(hwnd),
        ST_PROCESS => Object::Num(get_process_id_from_hwnd(hwnd) as f64),
        ST_MONITOR => get_monitor_index_from_hwnd(hwnd),
        _ => Object::Bool(false) // 定数以外を受けた場合false
    }
}

fn get_all_status(hwnd: HWND) -> Object {
    let mut stats = BTreeMap::new();
    stats.insert(ST_TITLE.to_string(), get_window_text(hwnd));
    stats.insert(ST_CLASS.to_string(), get_class_name(hwnd));
    let rect = get_window_size(hwnd);
    stats.insert(ST_X.to_string(), Object::Num(*rect.get(&ST_X).unwrap_or(&0) as f64));
    stats.insert(ST_Y.to_string(), Object::Num(*rect.get(&ST_Y).unwrap_or(&0) as f64));
    stats.insert(ST_WIDTH.to_string(), Object::Num(*rect.get(&ST_WIDTH).unwrap_or(&0) as f64));
    stats.insert(ST_HEIGHT.to_string(), Object::Num(*rect.get(&ST_HEIGHT).unwrap_or(&0) as f64));
    let crect = get_client_size(hwnd);
    stats.insert(ST_CLX.to_string(), Object::Num(*crect.get(&ST_CLX).unwrap_or(&0) as f64));
    stats.insert(ST_CLY.to_string(), Object::Num(*crect.get(&ST_CLY).unwrap_or(&0) as f64));
    stats.insert(ST_CLWIDTH.to_string(), Object::Num(*crect.get(&ST_CLWIDTH).unwrap_or(&0) as f64));
    stats.insert(ST_CLHEIGHT.to_string(), Object::Num(*crect.get(&ST_CLHEIGHT).unwrap_or(&0) as f64));
    stats.insert(ST_PARENT.to_string(), get_parent(hwnd));
    stats.insert(ST_ICON.to_string(), unsafe{ Object::Bool(winuser::IsIconic(hwnd) == TRUE) });
    stats.insert(ST_MAXIMIZED.to_string(), is_maximized(hwnd));
    stats.insert(ST_VISIBLE.to_string(), unsafe{ Object::Bool(winuser::IsWindowVisible(hwnd) == TRUE) });
    stats.insert(ST_ACTIVE.to_string(), is_active_window(hwnd));
    stats.insert(ST_BUSY.to_string(), unsafe{ Object::Bool(winuser::IsHungAppWindow(hwnd) == TRUE) });
    stats.insert(ST_ISID.to_string(), unsafe{ Object::Bool(winuser::IsWindow(hwnd) == TRUE) });
    stats.insert(ST_WIN64.to_string(), is_process_64bit(hwnd));
    stats.insert(ST_PATH.to_string(), get_process_path_from_hwnd(hwnd));
    stats.insert(ST_PROCESS.to_string(), Object::Num(get_process_id_from_hwnd(hwnd) as f64));
    stats.insert(ST_MONITOR.to_string(), get_monitor_index_from_hwnd(hwnd));
    Object::SortedHash(stats, false)
}

pub fn status(args: Vec<Object>) -> Object {
    let hwnd = match get_non_float_argument_value(&args, 0, None) {
        Ok(id) => get_hwnd_from_id(id),
        Err(e) => return Object::Error(e)
    };
    if args.len() > 2 {
        let mut i = 1;
        // let mut stats = vec![Object::Empty; 22];
        let mut stats = BTreeMap::new();
        while i < args.len() {
            let (cmd, value) = get_non_float_argument_value::<u8>(&args, i, None).map_or_else(
                |e| (0, Object::Error(e)),
                |cmd| (cmd, get_status_result(hwnd, cmd))
            );
            match value {
                Object::Error(_) => return value,
                _ => stats.insert(cmd.to_string(), value)
            };
            i += 1;
        }
        Object::SortedHash(stats, false)
    } else {
        get_non_float_argument_value::<u8>(&args, 1, None).map_or_else(
            |e| Object::Error(e),
            |cmd| if cmd == ST_ALL {
                get_all_status(hwnd)
            } else {
                get_status_result(hwnd, cmd)
            }
        )
    }
}

// monitor

const MON_X: u8           = 0;
const MON_Y: u8           = 1;
const MON_WIDTH: u8       = 2;
const MON_HEIGHT: u8      = 3;
const MON_NAME: u8        = 5;
const MON_ISMAIN: u8      = 7;
const MON_WORK_X: u8      = 10;
const MON_WORK_Y: u8      = 11;
const MON_WORK_WIDTH: u8  = 12;
const MON_WORK_HEIGHT: u8 = 13;
const MON_ALL: u8         = 20;

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

pub fn monitor(args: Vec<Object>) -> Object {
    if args.len() == 0 {
        return get_monitor_count(null_mut());
    }
    let index = match get_non_float_argument_value::<usize>(&args, 0, None) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("monitor", e.as_str())
    };
    let h = get_monitor_handle_by_index(index);
    if h == null_mut() {
        return Object::Bool(false);
    };
    let mut miex: winuser::MONITORINFOEXW = unsafe {mem::zeroed()};
    miex.cbSize = mem::size_of::<winuser::MONITORINFOEXW>() as u32;
    let p_miex = <*mut _>::cast(&mut miex);
    unsafe {
        if winuser::GetMonitorInfoW(h, p_miex) == FALSE {
            return builtin_func_error("monitor", "failed to get monitor information");
        }
    }
    match get_non_float_argument_value::<u8>(&args, 1, Some(MON_ALL)) {
        Ok(mon) => {
            let value = match mon {
                MON_ALL => {
                    let mut map = BTreeMap::new();
                    map.insert(MON_X.to_string(), Object::Num(miex.rcMonitor.left.into()));
                    map.insert(MON_Y.to_string(), Object::Num(miex.rcMonitor.top.into()));
                    map.insert(MON_WIDTH.to_string(), Object::Num((miex.rcMonitor.right - miex.rcMonitor.left).into()));
                    map.insert(MON_HEIGHT.to_string(), Object::Num((miex.rcMonitor.bottom - miex.rcMonitor.top).into()));
                    map.insert(MON_NAME.to_string(), get_monitor_name(&miex.szDevice));
                    map.insert(MON_ISMAIN.to_string(), Object::Bool(miex.dwFlags == winuser::MONITORINFOF_PRIMARY));
                    map.insert(MON_WORK_X.to_string(), Object::Num(miex.rcWork.left.into()));
                    map.insert(MON_WORK_Y.to_string(), Object::Num(miex.rcWork.top.into()));
                    map.insert(MON_WORK_WIDTH.to_string(), Object::Num((miex.rcWork.right - miex.rcWork.left).into()));
                    map.insert(MON_WORK_HEIGHT.to_string(), Object::Num((miex.rcWork.bottom - miex.rcWork.top).into()));
                    return Object::SortedHash(map, false);
                },
                MON_X => miex.rcMonitor.left,
                MON_Y => miex.rcMonitor.top,
                MON_WIDTH => miex.rcMonitor.right - miex.rcMonitor.left,
                MON_HEIGHT => miex.rcMonitor.bottom - miex.rcMonitor.top,
                MON_NAME => return get_monitor_name(&miex.szDevice),
                MON_ISMAIN => return Object::Bool(miex.dwFlags == winuser::MONITORINFOF_PRIMARY),
                MON_WORK_X => miex.rcWork.left,
                MON_WORK_Y => miex.rcWork.top,
                MON_WORK_WIDTH => miex.rcWork.right - miex.rcWork.left,
                MON_WORK_HEIGHT => miex.rcWork.bottom - miex.rcWork.top,
                _ => return Object::Bool(false)
            };
            Object::Num(value as f64)
        },
        Err(e) => builtin_func_error("monitor", e.as_str())
    }
}