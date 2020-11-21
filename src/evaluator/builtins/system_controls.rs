use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use std::collections::HashMap;
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

pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("sleep", 1, sleep),
        ("kindofos", 1, kindofos),
        ("env", 1, env),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
    }
}

pub fn set_builtin_constant(map: &mut HashMap<String, Object>) {
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
    ];
    for (key, value) in num_constant {
        map.insert(
            key.to_ascii_uppercase(),
            Object::BuiltinConst(Box::new(Object::Num(value.into())))
        );
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

pub fn get_os_num() -> Object {
    let mut info: winnt::OSVERSIONINFOEXW = unsafe{std::mem::zeroed()};
    info.dwOSVersionInfoSize = std::mem::size_of::<winnt::OSVERSIONINFOEXW>() as u32;
    let p_info = <*mut _>::cast(&mut info);
    unsafe {
        sysinfoapi::GetVersionExW(p_info);
    }
    match info.dwMajorVersion {
        10 => if info.wProductType == winnt::VER_NT_WORKSTATION {
            Object::Num(OS_WIN10.into())
        } else {
            Object::Num(OS_WINSRV2016.into())
        },
        6 => match info.dwMinorVersion {
            3 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                Object::Num(OS_WIN81.into())
            } else {
                Object::Num(OS_WINSRV2012R2.into())
            },
            2 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                Object::Num(OS_WIN8.into())
            } else {
                Object::Num(OS_WINSRV2012.into())
            },
            1 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                Object::Num(OS_WIN7.into())
            } else {
                Object::Num(OS_WINSRV2008R2.into())
            },
            0 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                Object::Num(OS_WINVISTA.into())
            } else {
                Object::Num(OS_WINSRV2008.into())
            },
            _ => Object::Num(0.0)
        },
        5 => match info.dwMinorVersion {
            2 => if unsafe{winuser::GetSystemMetrics(winuser::SM_SERVERR2)} != 0 {
                Object::Num(OS_WINSRV2003R2.into())
            } else {
                Object::Num(OS_WINSRV2003.into())
            },
            1 => Object::Num(OS_WINXP.into()),
            0 => Object::Num(OS_WIN2000.into()),
            _ => Object::Num(0.0)
        },
        _ => Object::Num(0.0)
    }
}

pub fn kindofos(args: Vec<Object>) -> Object {
    let flg = match get_bool_argument_value(&args, 0, Some(false)) {
        Ok(b) => b,
        Err(e) => return builtin_func_error("kindofos", e.as_str())
    };
    if flg {
        is_64bit_os().map_or_else(
            |e| builtin_func_error("kindofos", format!("Architecture: {}", e).as_str()),
            |b| Object::Bool(b)
        )
    } else {
        get_os_num()
    }
}

pub fn env(args: Vec<Object>) -> Object {
    match get_string_argument_value(&args, 0, None) {
        Ok(s) => Object::String(std::env::var(s).unwrap_or("".to_string())),
        Err(e) => builtin_func_error("env", e.as_str())
    }
}