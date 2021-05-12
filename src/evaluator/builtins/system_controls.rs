use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::winapi::bindings::{
    Windows::Win32::WindowsProgramming::{
        INFINITE,
        PROCESS_CREATION_FLAGS, OSVERSIONINFOEXW,
        CloseHandle, GetVersionExW,
    },
    Windows::Win32::SystemServices::{
        BOOL, PWSTR,
        SECURITY_ATTRIBUTES, STARTUPINFOW, PROCESS_INFORMATION, STARTUPINFOW_FLAGS,
        VER_NT_WORKSTATION,
        CreateProcessW, WaitForInputIdle, WaitForSingleObject, GetExitCodeProcess,
        IsWow64Process, GetCurrentProcess,
    },
    Windows::Win32::WindowsAndMessaging::{
        HWND, LPARAM, SHOW_WINDOW_CMD, SYSTEM_METRICS_INDEX,
        EnumWindows, GetWindowThreadProcessId, GetSystemMetrics,
    },
    Windows::Win32::Shell::{
        ShellExecuteW,
    },
};

use std::{ptr::null_mut, thread, time};
use std::mem;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("sleep", 1, sleep);
    sets.add("kindofos", 1, kindofos);
    sets.add("env", 1, env);
    sets.add("exec", 6, exec);
    sets.add("shexec", 2, shexec);
    sets
}

pub fn sleep(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let sec = get_num_argument_value(&args, 0, None)?;
    if sec >= 0.0 {
        thread::sleep(time::Duration::from_secs_f64(sec));
    }
    Ok(Object::Empty)
}

pub fn is_64bit_os(f_name: &str) -> Result<bool, UError> {
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
        _ => Err(builtin_func_error(f_name, format!("unknown architecture: {}", arch)))
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum OsNumber {
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


pub fn get_os_num() -> Vec<f64> {
    let mut info: OSVERSIONINFOEXW = unsafe{std::mem::zeroed()};
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
    let p_info = <*mut _>::cast(&mut info);
    unsafe {
        GetVersionExW(p_info);
    }
    let mut res = vec![];
    let num = match info.dwMajorVersion {
        10 => if info.wProductType == VER_NT_WORKSTATION as u8 {
            OsNumber::OS_WIN10 as u8 as f64
        } else {
            OsNumber::OS_WINSRV2016 as u8 as f64
        },
        6 => match info.dwMinorVersion {
            3 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsNumber::OS_WIN81 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2012R2 as u8 as f64
            },
            2 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsNumber::OS_WIN8 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2012 as u8 as f64
            },
            1 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsNumber::OS_WIN7 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2008R2 as u8 as f64
            },
            0 => if info.wProductType == VER_NT_WORKSTATION as u8 {
                OsNumber::OS_WINVISTA as u8 as f64
            } else {
                OsNumber::OS_WINSRV2008 as u8 as f64
            },
            _ => 0.0
        },
        5 => match info.dwMinorVersion {
            2 => if unsafe{GetSystemMetrics(SYSTEM_METRICS_INDEX::SM_SERVERR2)} != 0 {
                OsNumber::OS_WINSRV2003R2 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2003 as u8 as f64
            },
            1 => OsNumber::OS_WINXP as u8 as f64,
            0 => OsNumber::OS_WIN2000 as u8 as f64,
            _ => 0.0
        },
        _ => 0.0
    };
    res.push(num.into());
    res.push(info.dwMajorVersion.into());
    res.push(info.dwMinorVersion.into());
    res.push(info.dwBuildNumber.into());
    res.push(info.dwPlatformId.into());
    res
}

pub fn kindofos(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = get_bool_or_int_argument_value(&args, 0, Some(0)).unwrap_or(0);
    let osnum = get_os_num();
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

pub fn to_wide_string(str: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect()
}

pub fn shell_execute(cmd: String, params: Option<String>) -> bool {
    unsafe {
        let hinstance = ShellExecuteW(
            HWND::NULL,
            PWSTR(to_wide_string("open").as_mut_ptr()),
            PWSTR(to_wide_string(cmd.as_str()).as_mut_ptr()),
            if params.is_some() {
                PWSTR(to_wide_string(params.unwrap().as_str()).as_mut_ptr())
            } else {
                PWSTR::NULL
            },
            PWSTR::NULL,
            SHOW_WINDOW_CMD::SW_SHOWNORMAL.0 as i32
        );
        hinstance.0 > 32
    }
}

fn create_process(cmd: String, name: &str) -> Result<PROCESS_INFORMATION, UError> {
    unsafe {
        let mut si: STARTUPINFOW = mem::zeroed();
        si.cb = mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTUPINFOW_FLAGS::STARTF_USESHOWWINDOW;
        si.wShowWindow = SHOW_WINDOW_CMD::SW_SHOW.0 as u16;
        let mut pi: PROCESS_INFORMATION = mem::zeroed();
        let mut command = to_wide_string(cmd.as_str());

        let r = CreateProcessW(
            PWSTR::NULL,
            PWSTR(command.as_mut_ptr()),
            &mut SECURITY_ATTRIBUTES::default(),
            &mut SECURITY_ATTRIBUTES::default(),
            false,
            PROCESS_CREATION_FLAGS::NORMAL_PRIORITY_CLASS,
            null_mut(),
            PWSTR::NULL,
            &mut si,
            &mut pi
        );
        if r.as_bool() {
            WaitForInputIdle(pi.hProcess, 1000);
            Ok(pi)
        } else {
            Err(builtin_func_error(name, "failed to create process"))
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
        let mut ph = ProcessHwnd{pid: pi.dwProcessId, hwnd: HWND::NULL};
        EnumWindows(Some(enum_window_proc), LPARAM(&mut ph as *mut ProcessHwnd as isize));
        let x = get_non_float_argument_value(&args, 2, None).ok();
        let y = get_non_float_argument_value(&args, 3, None).ok();
        let w = get_non_float_argument_value(&args, 4, None).ok();
        let h = get_non_float_argument_value(&args, 5, None).ok();
        window_control::set_window_size(ph.hwnd, x, y, w, h);
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
            if ! ph.hwnd.is_null() {
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