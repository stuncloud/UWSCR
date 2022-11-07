use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::error::evaluator::{UErrorMessage, UErrorKind};
use crate::winapi::{from_ansi_bytes, to_wide_string, attach_console, free_console, WString, PcwstrExt};
use windows::{
    w,
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
                OSVERSIONINFOEXW, GetVersionExW,
                SYSTEM_INFO, GetNativeSystemInfo,
            },
            Diagnostics::Debug::{
                PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_INTEL,
            },
            Threading::{
                STARTUPINFOW, PROCESS_INFORMATION,
                STARTF_USESHOWWINDOW, NORMAL_PRIORITY_CLASS,
                CREATE_NEW_CONSOLE, CREATE_NO_WINDOW,
                CreateProcessW, WaitForSingleObject, GetExitCodeProcess,
                WaitForInputIdle,
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
    }
};

use std::{thread, time};
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
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}

pub fn is_64bit_os() -> Option<bool> {
    let mut lpsysteminfo = SYSTEM_INFO::default();
    let arch = unsafe {
        GetNativeSystemInfo(&mut lpsysteminfo);
        lpsysteminfo.Anonymous.Anonymous.wProcessorArchitecture
    };
    match arch {
        PROCESSOR_ARCHITECTURE_AMD64 => Some(true),
        PROCESSOR_ARCHITECTURE_INTEL => Some(false),
        _ => None,
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
    let obj = match FromPrimitive::from_i32(t).unwrap_or(KindOfOsResultType::KIND_OF_OS) {
        KindOfOsResultType::IS_64BIT_OS => {
            let is_x64_os = is_64bit_os()
                .ok_or(builtin_func_error(UErrorMessage::UnsupportedArchitecture))?;
            Object::Bool(is_x64_os)
        },
        KindOfOsResultType::OSVER_MAJOR => Object::Num(osnum[1]),
        KindOfOsResultType::OSVER_MINOR => Object::Num(osnum[2]),
        KindOfOsResultType::OSVER_BUILD => Object::Num(osnum[3]),
        KindOfOsResultType::OSVER_PLATFORM => Object::Num(osnum[4]),
        KindOfOsResultType::KIND_OF_OS => Object::Num(osnum[0]),
    };
    Ok(BuiltinFuncReturnValue::Result(obj))
}

pub fn env(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let env_var = args.get_as_string(0, None)?;
    let env_val = std::env::var(env_var).unwrap_or_default();
    Ok(BuiltinFuncReturnValue::Result(Object::String(env_val)))
}

pub fn shell_execute(cmd: String, params: Option<String>) -> bool {
    unsafe {
        let hinstance = ShellExecuteW(
            HWND::default(),
            w!("open"),
            cmd.to_wide_null_terminated().to_pcwstr(),
            params.unwrap_or_default().to_wide_null_terminated().to_pcwstr(),
            PCWSTR::null(),
            SW_SHOWNORMAL
        );
        hinstance.0 > 32
    }
}

fn create_process(cmd: String) -> BuiltInResult<PROCESS_INFORMATION> {
    unsafe {
        let mut si = STARTUPINFOW::default();
        si.cb = mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESHOWWINDOW;
        si.wShowWindow = SW_SHOW.0 as u16;
        let mut pi = PROCESS_INFORMATION::default();
        let mut command = to_wide_string(&cmd);

        let r = CreateProcessW(
            PCWSTR::null(),
            PWSTR(command.as_mut_ptr()),
            None,
            None,
            false,
            NORMAL_PRIORITY_CLASS,
            None,
            PCWSTR::null(),
            &mut si,
            &mut pi
        );
        if r.as_bool() {
            WaitForInputIdle(pi.hProcess, 1000);
            Ok(pi)
        } else {
            Err(builtin_func_error(UErrorMessage::FailedToCreateProcess))
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
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
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
    let pi = create_process(cmd)?;
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
            Ok(BuiltinFuncReturnValue::Result(Object::Num(exit.into())))
        } else {
            // idを返す
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
            if ph.hwnd.0 > 0 {
                let id = window_control::get_next_id();
                window_control::set_new_window(id, ph.hwnd, true);
                Ok(BuiltinFuncReturnValue::Result(Object::Num(id.into())))
            } else {
                Ok(BuiltinFuncReturnValue::Result(Object::Num(-1.0)))
            }
        }
    }
}

pub fn shexec(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let cmd = args.get_as_string(0, None)?;
    let params = args.get_as_string(1, None).map_or(None, |s| Some(s));
    Ok(BuiltinFuncReturnValue::Result(Object::Bool(shell_execute(cmd, params))))
}

pub fn task(mut args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let obj = args.get_as_object(0, None)?;
    let arguments = args.take_argument(1);
    match obj {
        Object::Function(f) |
        Object::AsyncFunction(f) => Ok(BuiltinFuncReturnValue::Task(f, arguments)),
        _ => Err(builtin_func_error(UErrorMessage::BuiltinArgIsNotFunction))
    }
}

pub fn wait_task(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let task = args.get_as_task(0)?;
    let mut handle = task.handle.lock().unwrap();
    let result = match handle.take().unwrap().join() {
        Ok(res) => res.map(|o| BuiltinFuncReturnValue::Result(o)),
        Err(e) => {
            Err(UError::new(
                UErrorKind::TaskError,
                UErrorMessage::TaskEndedIncorrectly(format!("{:?}", e))
            ))
        }
    };
    result.map_err(|e| e.into())
}

pub fn wmi_query(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let wql = args.get_as_string(0, None)?;
    let name_space = args.get_as_string_or_empty(1)?;
    let conn = unsafe {
        let com_lib = wmi::COMLibrary::assume_initialized();
        if let Some(namespace_path) = name_space {
            wmi::WMIConnection::with_namespace_path(&namespace_path, com_lib)?
        } else {
            wmi::WMIConnection::new(com_lib)?
        }
    };
    let result: Vec<Map<String, Value>> = conn.raw_query(&wql)?;
    let obj = result
        .into_iter()
        .map(|m| {
            let value = Value::Object(m);
            Object::UObject(UObject::new(value))
        })
        .collect();
    Ok(BuiltinFuncReturnValue::Result(Object::Array(obj)))
}

impl From<wmi::WMIError> for BuiltinFuncError {
    fn from(e: wmi::WMIError) -> Self {
        Self::new_with_kind(
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
    Ok(BuiltinFuncReturnValue::Result(result))
}

fn run_powershell(shell: ShellType, args: &BuiltinFuncArgs) -> BuiltinFuncResult {
    let command = args.get_as_string(0, None)?;
    // falseが渡されたら終了を待つ
    let wait = ! args.get_as_bool(1, Some(false))?;
    let (show, minimize) = match args.get_as_three_state(2, Some(ThreeState::False))? {
        ThreeState::True => (true, false),
        ThreeState::False => (false, false),
        ThreeState::Other => (true, true),
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
    Ok(BuiltinFuncReturnValue::Result(result))
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

impl From<std::io::Error> for BuiltinFuncError {
    fn from(e: std::io::Error) -> Self {
        Self::UError(e.into())
    }
}

pub fn _attachconsole(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let attach = args.get_as_bool(0, None)?;
    let result = if attach {
        attach_console()
    } else {
        free_console()
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Bool(result)))
}
