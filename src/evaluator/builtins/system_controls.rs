use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::error::evaluator::{UErrorMessage, UErrorKind};
use windows::{
    core::Handle,
    Win32::{
        Foundation::{
            BOOL, PWSTR, HWND, LPARAM,
            CloseHandle
        },
        System::{
            WindowsProgramming::{
                INFINITE,
            },
            SystemInformation::{
                OSVERSIONINFOEXW,
                GetVersionExW,
            },
            Threading::{
                STARTUPINFOW, PROCESS_INFORMATION,
                STARTF_USESHOWWINDOW, NORMAL_PRIORITY_CLASS,
                CreateProcessW, WaitForSingleObject, GetExitCodeProcess,
                GetCurrentProcess,
                WaitForInputIdle, IsWow64Process,
            },
            SystemServices::{
                VER_NT_WORKSTATION,
            }
        },
        UI::{
            WindowsAndMessaging::{
                SM_SERVERR2,
                SW_SHOWNORMAL, SW_SHOW,
                EnumWindows, GetWindowThreadProcessId, GetSystemMetrics,
            },
            Shell::{
                ShellExecuteW,
            }
        },
        Security::SECURITY_ATTRIBUTES,
    }
};
use crate::winapi::to_wide_string;

use std::{ptr::null_mut, thread, time};
use std::mem;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use serde_json::{Map, Value};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("sleep", 1, sleep);
    sets.add("kindofos", 1, kindofos);
    sets.add("env", 1, env);
    sets.add("exec", 6, exec);
    sets.add("shexec", 2, shexec);
    sets.add("task", 21, task);
    sets.add("waittask", 1, wait_task);
    sets.add("wmi", 2, wmi_query);
    sets
}

pub fn sleep(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let sec = get_num_argument_value(&args, 0, None)?;
    if sec >= 0.0 {
        thread::sleep(time::Duration::from_secs_f64(sec));
    }
    Ok(Object::Empty)
}

pub fn is_64bit_os(f_name: String) -> Result<bool, UError> {
    let arch = std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or("unknown".to_string());
    match arch.as_str() {
        "AMD64" => Ok(true),
        "x86" => {
            let mut b = false.into();
            unsafe {
                IsWow64Process(GetCurrentProcess(), &mut b);
            }
            Ok(b.as_bool())
        },
        _ => Err(builtin_func_error(UErrorMessage::UnknownArchitecture(arch), f_name))
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum OsKind {
    OS_WIN2000      = 12,
    OS_WINXP        = 13,
    OS_WINSRV2003   = 14,
    OS_WINSRV2003R2 = 15,
    OS_WINVISTA     = 20,
    OS_WINSRV2008   = 21,
    OS_WIN7         = 22,
    OS_WINSRV2008R2 = 27,
    OS_WIN8         = 23,
    OS_WINSRV2012   = 24,
    OS_WIN81        = 25,
    OS_WINSRV2012R2 = 26,
    OS_WIN10        = 30,
    OS_WINSRV2016   = 31,
    OS_WIN11        = 32,
    OS_UNKNOWN      = 0,
}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum KindOfOsResultType {
    KIND_OF_OS     = 0,
    IS_64BIT_OS    = 1,
    OSVER_MAJOR    = 2,
    OSVER_MINOR    = 3,
    OSVER_BUILD    = 4,
    OSVER_PLATFORM = 5,
}


pub fn get_os_kind() -> Vec<f64> {
    let mut info = OSVERSIONINFOEXW::default();
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
    let p_info = <*mut _>::cast(&mut info);
    unsafe {
        GetVersionExW(p_info);
    }
    let mut res = vec![];
    let win11_build_version = 21996;
    let num = match info.dwMajorVersion {
        10 => if info.dwBuildNumber < win11_build_version {
            // Windows 10
            if info.wProductType == VER_NT_WORKSTATION as u8 {
                    OsKind::OS_WIN10
                } else {
                    OsKind::OS_WINSRV2016
                }
            } else {
                // ビルド番号が21996以降はWindows 11
                OsKind::OS_WIN11
            },
        6 => match info.dwMinorVersion {
            3 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsKind::OS_WIN81
            } else {
                OsKind::OS_WINSRV2012R2
            },
            2 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsKind::OS_WIN8
            } else {
                OsKind::OS_WINSRV2012
            },
            1 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsKind::OS_WIN7
            } else {
                OsKind::OS_WINSRV2008R2
            },
            0 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsKind::OS_WINVISTA
            } else {
                OsKind::OS_WINSRV2008
            },
            _ => OsKind::OS_UNKNOWN
        },
        5 => match info.dwMinorVersion {
            2 => if unsafe{GetSystemMetrics(SM_SERVERR2)} != 0 {
                OsKind::OS_WINSRV2003R2
            } else {
                OsKind::OS_WINSRV2003
            },
            1 => OsKind::OS_WINXP,
            0 => OsKind::OS_WIN2000,
            _ => OsKind::OS_UNKNOWN
        },
        _ => OsKind::OS_UNKNOWN
    };
    res.push(num as u8 as f64);
    res.push(info.dwMajorVersion.into());
    res.push(info.dwMinorVersion.into());
    res.push(info.dwBuildNumber.into());
    res.push(info.dwPlatformId.into());
    res
}

pub fn kindofos(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = get_bool_or_int_argument_value(&args, 0, Some(0)).unwrap_or(0);
    let osnum = get_os_kind();
    match FromPrimitive::from_i32(t).unwrap_or(KindOfOsResultType::KIND_OF_OS) {
        KindOfOsResultType::IS_64BIT_OS => Ok(Object::Bool(is_64bit_os(args.name())?)),
        KindOfOsResultType::OSVER_MAJOR => Ok(Object::Num(osnum[1])),
        KindOfOsResultType::OSVER_MINOR => Ok(Object::Num(osnum[2])),
        KindOfOsResultType::OSVER_BUILD => Ok(Object::Num(osnum[3])),
        KindOfOsResultType::OSVER_PLATFORM => Ok(Object::Num(osnum[4])),
        KindOfOsResultType::KIND_OF_OS => Ok(Object::Num(osnum[0])),
    }
}

pub fn env(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let env_var = get_string_argument_value(&args, 0, None)?;
    Ok(Object::String(std::env::var(env_var).unwrap_or("".to_string())))
}

pub fn shell_execute(cmd: String, params: Option<String>) -> bool {
    unsafe {
        let hinstance = ShellExecuteW(
            HWND::default(),
            PWSTR(to_wide_string("open").as_mut_ptr()),
            PWSTR(to_wide_string(&cmd).as_mut_ptr()),
            if params.is_some() {
                PWSTR(to_wide_string(&params.unwrap()).as_mut_ptr())
            } else {
                PWSTR::default()
            },
            PWSTR::default(),
            SW_SHOWNORMAL.0 as i32
        );
        hinstance.0 > 32
    }
}

fn create_process(cmd: String, name: String) -> Result<PROCESS_INFORMATION, UError> {
    unsafe {
        let mut si = STARTUPINFOW::default();
        si.cb = mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESHOWWINDOW;
        si.wShowWindow = SW_SHOW.0 as u16;
        let mut pi = PROCESS_INFORMATION::default();
        let mut command = to_wide_string(&cmd);

        let r = CreateProcessW(
            PWSTR::default(),
            PWSTR(command.as_mut_ptr()),
            &mut SECURITY_ATTRIBUTES::default(),
            &mut SECURITY_ATTRIBUTES::default(),
            false,
            NORMAL_PRIORITY_CLASS,
            null_mut(),
            PWSTR::default(),
            &mut si,
            &mut pi
        );
        if r.as_bool() {
            WaitForInputIdle(pi.hProcess, 1000);
            Ok(pi)
        } else {
            Err(builtin_func_error(UErrorMessage::FailedToCreateProcess, name))
        }
    }
}

struct ProcessHwnd {
    pid: u32,
    hwnd: HWND,
}

unsafe extern "system"
fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ph = &mut *(lparam.0 as *mut ProcessHwnd);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == ph.pid {
        ph.hwnd = hwnd;
        false.into()
    } else {
        true.into()
    }
}

pub fn exec(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let cmd = get_string_argument_value(&args, 0, None)?;
    let sync = get_bool_argument_value(&args, 1, Some(false))?;
    let process = create_process(cmd, args.name());
    if process.is_err() {
        return Ok(Object::Num(-1.0));
    }
    let pi = process.unwrap();
    unsafe{
        let mut ph = ProcessHwnd{pid: pi.dwProcessId, hwnd: HWND::default()};
        EnumWindows(Some(enum_window_proc), LPARAM(&mut ph as *mut ProcessHwnd as isize));
        let x = get_non_float_argument_value(&args, 2, None).ok();
        let y = get_non_float_argument_value(&args, 3, None).ok();
        let w = get_non_float_argument_value(&args, 4, None).ok();
        let h = get_non_float_argument_value(&args, 5, None).ok();
        window_control::set_window_size(ph.hwnd, x, y, w, h)?;
        if sync {
            // 同期する場合は終了コード
            let mut exit: u32 = 0;
            WaitForSingleObject(pi.hProcess, INFINITE);
            GetExitCodeProcess(pi.hProcess, &mut exit);
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
            Ok(Object::Num(exit.into()))
        } else {
            // idを返す
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
            if ! ph.hwnd.is_invalid() {
                let id = window_control::get_next_id();
                window_control::set_new_window(id, ph.hwnd, true);
                Ok(Object::Num(id.into()))
            } else {
                Ok(Object::Num(-1.0))
            }
        }
    }
}

pub fn shexec(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let cmd = get_string_argument_value(&args, 0, None)?;
    let params = get_string_argument_value(&args, 1, None).map_or(None, |s| Some(s));
    Ok(Object::Bool(shell_execute(cmd, params)))
}

pub fn task(mut args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let obj = get_any_argument_value(&args, 0, None)?;
    let arguments = args.get_args_from(1);
    match obj {
        Object::Function(_, _, _, _, _) |
        Object::AsyncFunction(_, _, _, _, _) => Ok(Object::SpecialFuncResult(
            SpecialFuncResultType::Task(Box::new(obj), arguments)
        )),
        _ => Err(builtin_func_error(UErrorMessage::BuiltinArgIsNotFunction, args.name()))
    }
}

pub fn wait_task(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let task = get_task_argument_value(&args, 0)?;
    let mut handle = task.handle.lock().unwrap();
    let result = match handle.take().unwrap().join() {
        Ok(res) => res,
        Err(e) => {
            Err(UError::new(
                UErrorKind::TaskError,
                UErrorMessage::TaskEndedIncorrectly(format!("{:?}", e))
            ))
        }
    };
    result
}

pub fn wmi_query(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let wql = get_string_argument_value(&args, 0, None)?;
    let name_space = get_string_or_empty_argument(&args, 1, Some(None))?;
    let namespace_path = name_space.as_deref();
    let conn = unsafe {
        wmi::WMIConnection::with_initialized_com(namespace_path)?
    };
    let result: Vec<Map<String, Value>> = conn.raw_query(&wql)?;
    let obj = result
        .into_iter()
        .map(|m| {
            let value = Value::Object(m);
            Object::UObject(Arc::new(Mutex::new(value)))
        })
        .collect();
    Ok(Object::Array(obj))
}

impl From<wmi::WMIError> for UError {
    fn from(e: wmi::WMIError) -> Self {
        Self::new(
            UErrorKind::WmiError,
            UErrorMessage::Any(e.to_string())
        )
    }
}