use crate::evaluator::object::*;
use crate::evaluator::builtins::builtin_func_error;
use crate::evaluator::builtins::window_low::get_current_pos;

use std::fmt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};
use std::thread;

use std::ptr::null_mut;
use winapi::{
    um::{
        winuser,
    },
    shared::{
        windef::{HWND, RECT},
        minwindef::{LPARAM, BOOL, TRUE, FALSE},
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

pub fn set_builtin_constant(map: &mut HashMap<String, Object>) {
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
    ];
    for (key, value) in str_constant {
        map.insert(
            key.to_ascii_uppercase(),
            Object::BuiltinConst(Box::new(Object::String(value.to_string())))
        );
    }
}

pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("getid", 4, getid),
        ("idtohnd", 1, idtohnd),
        ("hndtoid", 1, hndtoid),
        ("clkitem", 5, clkitem),
        ("acw", 5, acw),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
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

pub fn getid(args: Vec<Object>) -> Object {
    let hwnd = match args[0].clone() {
        Object::String(title) => {
            match title.as_str() {
                GET_ACTIVE_WIN => {
                    unsafe {
println!("にゃーん");
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
        let id = get_next_id();
        set_new_window(id, hwnd);
        return Object::Num(id as f64)
    } else {
        return Object::Num(-1.0)
    }
}

const MAX_BUFFER_SIZE: usize = 512;
type TBuffer = [u16; MAX_BUFFER_SIZE];
fn buffer_to_string( buffer: &TBuffer ) -> Result<String, String> {
    buffer.iter()
        .position(|wch| wch == &0)
        .ok_or("String : Can't find zero terminator !".to_owned())
        .and_then(|ix| String::from_utf16( &buffer[..ix] )
        .map_err(|e| e.to_string()))
}

fn find_window(title: String, class_name: String, timeout: f64) -> Result<HWND, String> {
    static mut TITLE: String = String::new();
    static mut CLASSNAME: String = String::new();
    static mut HANDLE: HWND = null_mut();
    static mut ERR: Option<String> = None;
    unsafe {
        TITLE = title;
        CLASSNAME = class_name;
    }
    unsafe extern "system"
    fn enum_window_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        let mut title_buffer: TBuffer = [0; MAX_BUFFER_SIZE];
        let mut class_buffer: TBuffer = [0; MAX_BUFFER_SIZE];
        winuser::GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        match buffer_to_string(&title_buffer) {
            Ok(t) => match t.find(TITLE.as_str()) {
                Some(_) => {
                    winuser::GetClassNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as i32);
                    match buffer_to_string(&class_buffer) {
                        Ok(c) => match c.find(CLASSNAME.as_str()) {
                            Some(_) => {
                                HANDLE = hwnd;
                                return FALSE;
                            },
                            None => ()
                        },
                        Err(e) => {
                            ERR = Some(e);
                            return FALSE; // 終わる
                        },
                    }
                },
                None => ()
            },
            Err(e) => {
                ERR = Some(e);
                return FALSE; // 終わる
            },
        }
        TRUE // 次のウィンドウへ
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
            winuser::EnumWindows(Some(enum_window_proc), 0);
            if HANDLE != null_mut() {
                break
            }
            if limit.is_some() && now.elapsed() >= limit.unwrap() {
                break;
            }
        }
        match &ERR {
            Some(e) => return Err(e.clone()),
            None => Ok(HANDLE)
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
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    match args[0] {
        Object::Num(id) => {
            if id < 0.0 {
                return Object::Num(0.0);
            }
            match list.get(&(id as i32)) {
                Some(h) => {
                    unsafe {
                        if winuser::IsWindow(*h) != 0 {
                            Object::Num(*h as i32 as f64)
                        } else {
                            Object::Num(0.0)
                        }
                    }
                },
                None => Object::Num(0.0)
            }
        },
        _ => builtin_func_error("idtohnd", "invalid argumant")
    }
}

// HNDTOID
pub fn hndtoid(args: Vec<Object>) -> Object {
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    match args[0] {
        Object::Num(h) => {
            let hwnd = h as i32 as HWND;
            let id = list.iter().find_map(
                |(key, &val)| if val == hwnd {
                    Some(*key as f64)
                } else {
                    Some(-1.0)
                }
            ).unwrap();
            Object::Num(id)
        },
        _ => builtin_func_error("hndtoid", "invalid argumant")
    }
}

// ACW
pub fn acw(args: Vec<Object>) -> Object {
    let s = window_singlton();
    let list = s.windows.lock().unwrap();
    let hwnd = match args[0] {
        Object::Num(n) => {
            let id = n as i32;
            match list.get(&id) {
                Some(h) => h,
                None => return Object::Empty
            }
        },
        _ => return builtin_func_error("acw", format!("bad argument: {}", args[0]).as_str())
    };
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        if winuser::GetWindowRect(*hwnd, &mut rect) == FALSE {
            return builtin_func_error("acw", "failed to get window rect");
        }
    }
    let x = if args.len() >= 2 {
        match args[1] {
            Object::Num(n) => n as i32,
            _ => return builtin_func_error("acw", format!("bad argument: {}", args[1]).as_str())
        }
    } else {
        rect.left
    };
    let y = if args.len() >= 3 {
        match args[2] {
            Object::Num(n) => n as i32,
            _ => return builtin_func_error("acw", format!("bad argument: {}", args[2]).as_str())
        }
    } else {
        rect.top
    };
    let cx = if args.len() >= 4 {
        match args[3] {
            Object::Num(n) => (n as i32) + x,
            _ => return builtin_func_error("acw", format!("bad argument: {}", args[3]).as_str())
        }
    } else {
        rect.right
    };
    let cy = if args.len() >= 5 {
        match args[4] {
            Object::Num(n) => (n as i32) + y,
            _ => return builtin_func_error("acw", format!("bad argument: {}", args[4]).as_str())
        }
    } else {
        rect.bottom
    };
    let ms= if args.len() >= 6 {
        match args[5] {
            Object::Num(n) => n as u64,
            _ => return builtin_func_error("acw", format!("bad argument: {}", args[5]).as_str())
        }
    } else {
        0
    };
    unsafe {
        thread::sleep(Duration::from_millis(ms));
        if winuser::SetWindowPos(*hwnd, winuser::HWND_TOP, x, y, cx, cy, 0) == FALSE {
            return builtin_func_error("acw", "setwindowpos failed");
        }
    }
    Object::Empty
}


// CLKITEM
pub fn clkitem(args: Vec<Object>) -> Object {
    Object::Bool(args.len() > 0)
}
