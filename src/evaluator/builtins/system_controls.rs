use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

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
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("sleep", 1, sleep);
    sets.add("kindofos", 1, kindofos);
    sets.add("env", 1, env);
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
            let mut b = FALSE;
            unsafe {
                wow64apiset::IsWow64Process(processthreadsapi::GetCurrentProcess(), &mut b);
            }
            Ok(b != FALSE)
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
    let mut info: winnt::OSVERSIONINFOEXW = unsafe{std::mem::zeroed()};
    info.dwOSVersionInfoSize = std::mem::size_of::<winnt::OSVERSIONINFOEXW>() as u32;
    let p_info = <*mut _>::cast(&mut info);
    unsafe {
        sysinfoapi::GetVersionExW(p_info);
    }
    let mut res = vec![];
    let num = match info.dwMajorVersion {
        10 => if info.wProductType == winnt::VER_NT_WORKSTATION {
            OsNumber::OS_WIN10 as u8 as f64
        } else {
            OsNumber::OS_WINSRV2016 as u8 as f64
        },
        6 => match info.dwMinorVersion {
            3 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OsNumber::OS_WIN81 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2012R2 as u8 as f64
            },
            2 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OsNumber::OS_WIN8 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2012 as u8 as f64
            },
            1 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OsNumber::OS_WIN7 as u8 as f64
            } else {
                OsNumber::OS_WINSRV2008R2 as u8 as f64
            },
            0 => if info.wProductType == winnt::VER_NT_WORKSTATION {
                OsNumber::OS_WINVISTA as u8 as f64
            } else {
                OsNumber::OS_WINSRV2008 as u8 as f64
            },
            _ => 0.0
        },
        5 => match info.dwMinorVersion {
            2 => if unsafe{winuser::GetSystemMetrics(winuser::SM_SERVERR2)} != 0 {
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