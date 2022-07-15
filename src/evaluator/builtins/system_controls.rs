use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::error::evaluator::{UErrorMessage, UErrorKind};
use crate::winapi::{from_ansi_bytes, to_wide_string, attach_console, free_console};
use windows::{
    core::{PWSTR,PCWSTR},
    Win32::{
        Foundation::{
            BOOL, HWND, LPARAM,
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
                CREATE_NEW_CONSOLE, CREATE_NO_WINDOW,
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

use std::{ptr::null_mut, thread, time};
use std::mem;
use std::process::Command;
use std::os::windows::process::CommandExt;

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
    sets.add("doscmd", 4, doscmd);
    sets.add("powershell", 4, powershell);
    sets.add("pwsh", 4, pwsh);
    // sets.add("attachconsole", 1, attachconsole);
    sets
}

pub fn sleep(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let sec = args.get_as_num(0, None)?;
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
    let t = args.get_as_bool_or_int(0, Some(0)).unwrap_or(0);
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
    let env_var = args.get_as_string(0, None)?;
    Ok(Object::String(std::env::var(env_var).unwrap_or("".to_string())))
}

pub fn shell_execute(cmd: String, params: Option<String>) -> bool {
    unsafe {
        let hinstance = ShellExecuteW(
            HWND::default(),
            "open",
            cmd,
            params.unwrap_or_default(),
            PCWSTR::default(),
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
            PCWSTR::default(),
            PWSTR(command.as_mut_ptr()),
            &mut SECURITY_ATTRIBUTES::default(),
            &mut SECURITY_ATTRIBUTES::default(),
            false,
            NORMAL_PRIORITY_CLASS,
            null_mut(),
            PCWSTR::default(),
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
    let cmd = args.get_as_string(0, None)?;
    let sync = args.get_as_bool(1, Some(false))?;
    let process = create_process(cmd, args.name());
    if process.is_err() {
        return Ok(Object::Num(-1.0));
    }
    let pi = process.unwrap();
    unsafe{
        let mut ph = ProcessHwnd{pid: pi.dwProcessId, hwnd: HWND::default()};
        EnumWindows(Some(enum_window_proc), LPARAM(&mut ph as *mut ProcessHwnd as isize));
        let x = args.get_as_int(2, None).ok();
        let y = args.get_as_int(3, None).ok();
        let w = args.get_as_int(4, None).ok();
        let h = args.get_as_int(5, None).ok();
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
            if ph.hwnd.0 > 0 {
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
    let cmd = args.get_as_string(0, None)?;
    let params = args.get_as_string(1, None).map_or(None, |s| Some(s));
    Ok(Object::Bool(shell_execute(cmd, params)))
}

pub fn task(mut args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let obj = args.get_as_object(0, None)?;
    let arguments = args.take_argument(1);
    match obj {
        Object::Function(f) |
        Object::AsyncFunction(f) => Ok(Object::SpecialFuncResult(
            SpecialFuncResultType::Task(f, arguments)
        )),
        _ => Err(builtin_func_error(UErrorMessage::BuiltinArgIsNotFunction, args.name()))
    }
}

pub fn wait_task(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let task = args.get_as_task(0)?;
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
    let wql = args.get_as_string(0, None)?;
    let name_space = args.get_as_string_or_empty(1)?;
    let namespace_path = name_space.as_deref();
    let conn = unsafe {
        wmi::WMIConnection::with_initialized_com(namespace_path)?
    };
    let result: Vec<Map<String, Value>> = conn.raw_query(&wql)?;
    let obj = result
        .into_iter()
        .map(|m| {
            let value = Value::Object(m);
            Object::UObject(UObject::new(value))
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

pub fn doscmd(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let command = args.get_as_string(0, None)?;
    // falseが渡されたら終了を待つ
    let wait = ! args.get_as_bool(1, Some(false))?;
    let show = args.get_as_bool(2, Some(false))?;
    let option = if args.get_as_bool(3, Some(false))? {
        ShellOption::CmdUnicode
    } else {
        ShellOption::CmdAnsi
    };
    let shell = Shell {
        shell: ShellType::Cmd,
        minimize: false,
        wait, show, option,
        command
    };
    let result = match shell.run()? {
        Some(out) => Object::String(out),
        None => Object::Empty
    };
    Ok(result)
}

fn run_powershell(shell: ShellType, args: &BuiltinFuncArgs) -> BuiltinFuncResult {
    let command = args.get_as_string(0, None)?;
    // falseが渡されたら終了を待つ
    let wait = ! args.get_as_bool(1, Some(false))?;
    let (show, minimize) = match args.get_as_bool_or_int::<i32>(2, Some(0))? {
        0 => (false, false),
        2 => (true, true),
        _ => (true, false)
    };
    let option = if args.get_as_bool(3, Some(false))? {
        ShellOption::PsNoProfile
    } else {
        ShellOption::None
    };
    let shell = Shell { shell, wait, show, minimize, option, command };
    let result = match shell.run()? {
        Some(out) => Object::String(out),
        None => Object::Empty
    };
    Ok(result)
}

pub fn powershell(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    run_powershell(ShellType::PowerShell, &args)
}

pub fn pwsh(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    run_powershell(ShellType::Pwsh, &args)
}

#[derive(PartialEq)]
enum ShellType {
    Cmd,
    PowerShell,
    Pwsh
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cmd => write!(f, "cmd"),
            Self::PowerShell => write!(f, "powershell"),
            Self::Pwsh => write!(f, "pwsh"),
        }
    }
}

struct Shell {
    shell: ShellType,
    wait: bool,
    show: bool,
    minimize: bool,
    option: ShellOption,
    command: String
}

#[derive(PartialEq)]
enum ShellOption {
    CmdAnsi,
    CmdUnicode,
    PsNoProfile,
    None
}

impl std::fmt::Display for ShellOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellOption::CmdAnsi => write!(f, "/A"),
            ShellOption::CmdUnicode => write!(f, "/U"),
            ShellOption::PsNoProfile => write!(f, "-NoProfile"),
            ShellOption::None => write!(f, ""),
        }
    }
}

impl Shell {
    fn run(&self) -> Result<Option<String>, std::io::Error> {
        let mut shell = Command::new(self.shell.to_string());
        if self.shell == ShellType::Cmd {
            // doscmd
            shell.raw_arg(&self.option.to_string());
            shell.raw_arg("/C");
            shell.raw_arg(&self.command);

        } else {
            // powershell, pwsh
            if self.option == ShellOption::PsNoProfile {
                shell.arg(&self.option.to_string());
            }
            let command = format!(
                "[console]::OutputEncoding = [System.Text.Encoding]::UTF8;{}",
                &self.command
            );
            shell.args([
                "-Nologo",
                "-OutputFormat",
                "Text",
                "-EncodedCommand",
                &Self::to_base64(&command)
            ]);
        }
        if self.show {
            shell.creation_flags(CREATE_NEW_CONSOLE.0);
            if self.minimize {
                // 最小化処理
                shell.args(["-WindowStyle", "Minimized"]);
            }
            if self.wait {
                shell.status()?;
            } else {
                shell.spawn()?;
            }
            Ok(None)
        } else {
            shell.creation_flags(CREATE_NO_WINDOW.0);
            if self.wait {
                let output = shell.output()?;
                let out_raw = if self.shell == ShellType::Cmd {
                    if output.stderr.len()> 0 {
                        output.stderr
                    } else {
                        output.stdout
                    }
                } else {
                    output.stdout
                };
                let out_string = match self.option {
                    ShellOption::CmdUnicode => Self::unicode_output_to_string(&out_raw),
                    ShellOption::CmdAnsi => from_ansi_bytes(&out_raw),
                    _ => String::from_utf8(out_raw).unwrap_or_default()
                };
                Ok(Some(out_string))
            } else {
                shell.spawn()?;
                Ok(None)
            }
        }
    }

    fn unicode_output_to_string(u8: &[u8]) -> String {
        let u16: Vec<u16> = u8
            .chunks_exact(2)
            .into_iter()
            .map(|a| u16::from_ne_bytes([a[0], a[1]]))
            .collect();
        String::from_utf16_lossy(&u16)
    }
    fn to_base64(command: &str) -> String {
        let wide = command.encode_utf16().collect::<Vec<u16>>();
        let bytes = wide.into_iter().map(|u| u.to_ne_bytes()).flatten().collect::<Vec<u8>>();
        base64::encode(bytes)
    }
}

pub fn _attachconsole(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let attach = args.get_as_bool(0, None)?;
    let result = if attach {
        attach_console()
    } else {
        free_console()
    };
    Ok(Object::Bool(result))
}
