use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::environment::NamedObject;

use std::{thread, time};

use winapi::{
    um::{
        winuser,
        wow64apiset,
        processthreadsapi,
        sysinfoapi,
        winnt,
    },
    shared::{
        minwindef::{
            FALSE
        }
    }
};

pub fn set_builtins(vec: &mut Vec<NamedObject>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("sleep", 1, sleep),
        ("kindofos", 1, kindofos),
        ("env", 1, env),
    ];
    for (name, arg_len, func) in funcs {
        vec.push(NamedObject::new_builtin_func(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func)));
    }
    let num_constant = vec![
        ("OS_WIN2000", OS_WIN2000),
        ("OS_WINXP", OS_WINXP),
        ("OS_WINSRV2003", OS_WINSRV2003),
        ("OS_WINSRV2003R2", OS_WINSRV2003R2),
        ("OS_WINVISTA", OS_WINVISTA),
        ("OS_WINSRV2008", OS_WINSRV2008),
        ("OS_WIN7", OS_WIN7),
        ("OS_WINSRV2008R2", OS_WINSRV2008R2),
        ("OS_WIN8", OS_WIN8),
        ("OS_WINSRV2012", OS_WINSRV2012),
        ("OS_WIN81", OS_WIN81),
        ("OS_WINSRV2012R2", OS_WINSRV2012R2),
        ("OS_WIN10", OS_WIN10),
        ("OS_WINSRV2016", OS_WINSRV2016),
        ("OSVER_MAJOR", OSVER_MAJOR),
        ("OSVER_MINOR", OSVER_MINOR),
        ("OSVER_BUILD", OSVER_BUILD),
        ("OSVER_PLATFORM", OSVER_PLATFORM),
    ];
    for (key, value) in num_constant {
        vec.push(NamedObject::new_builtin_const(key.to_ascii_uppercase(), Object::Num(value.into())));
    }
}

pub fn sleep(args: Vec<Object>) -> Object {
    match args[0] {
        Object::Num(n) => {
            if n > 0.0 {
                thread::sleep(time::Duration::from_secs_f64(n));
            }
        },
        _ => return builtin_func_error("sleep", format!("bad argument: {}", args[0]).as_str())
    }
    Object::Empty
}

pub fn is_64bit_os() -> Result<bool, String> {
    let arch = std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or("unknown".to_string());
    match arch.as_str() {
        "AMD64" => Ok(true),
        "x86" => {
            let mut b = FALSE;
            unsafe {
                wow64apiset::IsWow64Process(processthreadsapi::GetCurrentProcess(), &mut b);
            }
            Ok(b != FALSE)
        },
        _ => Err(arch)
    }
}
const OS_WIN2000      :u8 = 12;
const OS_WINXP        :u8 = 13;
const OS_WINSRV2003   :u8 = 14;
const OS_WINSRV2003R2 :u8 = 15;
const OS_WINVISTA     :u8 = 20;
const OS_WINSRV2008   :u8 = 21;
const OS_WIN7         :u8 = 22;
const OS_WINSRV2008R2 :u8 = 27;
const OS_WIN8         :u8 = 23;
const OS_WINSRV2012   :u8 = 24;
const OS_WIN81        :u8 = 25;
const OS_WINSRV2012R2 :u8 = 26;
const OS_WIN10        :u8 = 30;
const OS_WINSRV2016   :u8 = 31;

const OSVER_MAJOR    : u8 = 2;
const OSVER_MINOR    : u8 = 3;
const OSVER_BUILD    : u8 = 4;
const OSVER_PLATFORM : u8 = 5;

pub fn get_os_num() -> Vec<f64> {
    let mut info: winnt::OSVERSIONINFOEXW = unsafe{std::mem::zeroed()};
    info.dwOSVersionInfoSize = std::mem::size_of::<winnt::OSVERSIONINFOEXW>() as u32;
    let p_info = <*mut _>::cast(&mut info);
    unsafe {
        sysinfoapi::GetVersionExW(p_info);
    }
    let mut res = vec![];
    let num = match info.dwMajorVersion {
        10 => if info.wProductType == winnt::VER_NT_WORKSTATION {
            OS_WIN10
        } else {
            OS_WINSRV2016
        },
        6 => match info.dwMinorVersion {
            3 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OS_WIN81
            } else {
                OS_WINSRV2012R2
            },
            2 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OS_WIN8
            } else {
                OS_WINSRV2012
            },
            1 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OS_WIN7
            } else {
                OS_WINSRV2008R2
            },
            0 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OS_WINVISTA
            } else {
                OS_WINSRV2008
            },
            _ => 0
        },
        5 => match info.dwMinorVersion {
            2 => if unsafe{winuser::GetSystemMetrics(winuser::SM_SERVERR2)} != 0 {
                OS_WINSRV2003R2
            } else {
                OS_WINSRV2003
            },
            1 => OS_WINXP,
            0 => OS_WIN2000,
            _ => 0
        },
        _ => 0
    };
    res.push(num.into());
    res.push(info.dwMajorVersion.into());
    res.push(info.dwMinorVersion.into());
    res.push(info.dwBuildNumber.into());
    res.push(info.dwPlatformId.into());
    res
}

pub fn kindofos(args: Vec<Object>) -> Object {
    let t = get_bool_or_int_argument_value::<u8>(&args, 0, Some(0)).unwrap_or(0);
    let osnum = get_os_num();
    match t {
        1 => is_64bit_os().map_or_else(
            |e| builtin_func_error("kindofos", format!("Architecture: {}", e).as_str()),
            |b| Object::Bool(b)
        ),
        OSVER_MAJOR => Object::Num(osnum[1]),
        OSVER_MINOR => Object::Num(osnum[2]),
        OSVER_BUILD => Object::Num(osnum[3]),
        OSVER_PLATFORM => Object::Num(osnum[4]),
        _ => Object::Num(osnum[0]),
    }
}

pub fn env(args: Vec<Object>) -> Object {
    match get_string_argument_value(&args, 0, None) {
        Ok(s) => Object::String(std::env::var(s).unwrap_or("".to_string())),
        Err(e) => builtin_func_error("env", e.as_str())
    }
}