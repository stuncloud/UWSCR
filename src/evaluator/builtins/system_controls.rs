mod lockhard;
mod sensor;
mod sound;

use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::Evaluator;
use crate::error::evaluator::{UErrorMessage, UErrorKind};
use crate::winapi::{from_ansi_bytes, to_wide_string, attach_console, free_console, WString, PcwstrExt};
use crate::evaluator::builtins::window_control::get_hwnd_from_id;
use windows::{
    w,
    core::{PWSTR,PCWSTR},
    Win32::{
        Foundation::{
            BOOL, HWND, LPARAM, WPARAM,
            CloseHandle,
            FILETIME,
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
                GetSystemTimes,
            },
            SystemServices::{
                VER_NT_WORKSTATION,
            }
        },
        UI::{
            Input::{
                Ime::{
                    ImmGetDefaultIMEWnd,
                    IME_CMODE_KATAKANA,
                },
                KeyboardAndMouse::{
                    GetKeyState,
                    VK_NUMLOCK, VK_CAPITAL, VK_SCROLL,
                }
            },
            WindowsAndMessaging::{
                SM_SERVERR2,
                SW_SHOWNORMAL, SW_SHOW,
                EnumWindows, GetWindowThreadProcessId, GetSystemMetrics,
                GetForegroundWindow,
                SendMessageW, WM_IME_CONTROL,
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
    sets.add("lockhard", 1, lockhard);
    sets.add("lockhardex", 2, lockhardex);
    sets.add("cpuuserate", 0, cpuuserate);
    sets.add("sensor", 1, sensor);
    sets.add("sound", 3, sound);
    sets.add("beep", 3, beep);
    sets.add("getkeystate", 2, getkeystate);
    // sets.add("attachconsole", 1, attachconsole);
    sets
}

pub fn sleep(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let sec = args.get_as_num(0, None)?;
    if sec >= 0.0 {
        thread::sleep(time::Duration::from_secs_f64(sec));
    }
    Ok(Object::Empty)
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
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
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
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
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

pub fn kindofos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    Ok(obj)
}

pub fn env(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let env_var = args.get_as_string(0, None)?;
    let env_val = std::env::var(env_var).unwrap_or_default();
    Ok(Object::String(env_val))
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

pub fn exec(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

pub fn shexec(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let cmd = args.get_as_string(0, None)?;
    let params = args.get_as_string(1, None).map_or(None, |s| Some(s));
    let shell_result = shell_execute(cmd, params);
    Ok(shell_result.into())
}

pub fn task(evaluator: &mut Evaluator, mut args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let obj = args.get_as_object(0, None)?;
    let arguments = args.take_argument(1);
    match obj {
        Object::Function(func) |
        Object::AsyncFunction(func) => {
            let task = evaluator.new_task(func, arguments);
            let obj = if args.is_await() {
                evaluator.await_task(task)?
            } else {
                Object::Task(task)
            };
            Ok(obj)
        },
        _ => Err(builtin_func_error(UErrorMessage::BuiltinArgIsNotFunction))
    }
}

pub fn wait_task(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let task = args.get_as_task(0)?;
    let mut handle = task.handle.lock().unwrap();
    match handle.take().unwrap().join() {
        Ok(res) => res.map_err(|err| BuiltinFuncError::UError(err)),
        Err(e) => {
            Err(BuiltinFuncError::new_with_kind(
                UErrorKind::TaskError,
                UErrorMessage::TaskEndedIncorrectly(format!("{:?}", e))
            ))
        }
    }
}

pub fn wmi_query(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    Ok(Object::Array(obj))
}

impl From<wmi::WMIError> for BuiltinFuncError {
    fn from(e: wmi::WMIError) -> Self {
        Self::new_with_kind(
            UErrorKind::WmiError,
            UErrorMessage::Any(e.to_string())
        )
    }
}

pub fn doscmd(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    Ok(result)
}

pub fn powershell(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    run_powershell(ShellType::PowerShell, &args)
}

pub fn pwsh(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

pub fn _attachconsole(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let attach = args.get_as_bool(0, None)?;
    let result = if attach {
        attach_console()
    } else {
        free_console()
    };
    Ok(result.into())
}

pub fn lockhard(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let flg = args.get_as_bool(0, Some(false))?;
    let result = lockhard::lock(flg);
    Ok(result.into())
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum LockHardExConst {
    #[default]
    LOCK_ALL      = 0,
    LOCK_KEYBOARD = 1,
    LOCK_MOUSE    = 2,
}

pub fn lockhardex(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int_or_empty(0)?;
    let result = if let Some(id) = id {
        let mode = args.get_as_const(1, false)?.unwrap_or_default();
        let hwnd = if id == 0 {
            None
        } else {
            Some(super::window_control::get_hwnd_from_id(id))
        };
        lockhard::lock_ex(hwnd, mode)
    } else {
        lockhard::free_ex()
    };
    Ok(result.into())
}

pub fn cpuuserate(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let rate = unsafe {
        let time = |ft: FILETIME| {
            (ft.dwHighDateTime as i64) << 32 | ft.dwLowDateTime as i64
        };
        let diff = |ft1: FILETIME, ft2: FILETIME| {
            time(ft2) - time(ft1)
        };
        let mut idle1 = FILETIME::default();
        let mut kernel1 = FILETIME::default();
        let mut user1 = FILETIME::default();
        let mut idle2 = FILETIME::default();
        let mut kernel2 = FILETIME::default();
        let mut user2 = FILETIME::default();
        GetSystemTimes(Some(&mut idle1), Some(&mut kernel1), Some(&mut user1));
        thread::sleep(time::Duration::from_secs(1));
        GetSystemTimes(Some(&mut idle2), Some(&mut kernel2), Some(&mut user2));
        let total = diff(kernel2, kernel1) + diff(user2, user1);
        let idle = diff(idle2, idle1);
        let usage = 1.0 - idle as f64 / total as f64;
        usage * 100.0
    };
    Ok(rate.into())
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum SensorConst {
    /// 人が存在した場合に True
    SNSR_Biometric_HumanPresense    = 1,
    /// 人との距離(メートル)
    SNSR_Biometric_HumanProximity   = 2,
    /// 静電容量(ファラド)
    SNSR_Electrical_Capacitance     = 5,
    /// 電気抵抗(オーム)
    SNSR_Electrical_Resistance      = 6,
    /// 誘導係数(ヘンリー)
    SNSR_Electrical_Inductance      = 7,
    /// 電流(アンペア)
    SNSR_Electrical_Current         = 8,
    /// 電圧(ボルト)
    SNSR_Electrical_Voltage         = 9,
    /// 電力(ワット)
    SNSR_Electrical_Power           = 10,
    /// 気温(セ氏)
    SNSR_Environmental_Temperature  = 15,
    /// 気圧(バール)
    SNSR_Environmental_Pressure     = 16,
    /// 湿度(パーセンテージ)
    SNSR_Environmental_Humidity     = 17,
    /// 風向(度数)
    SNSR_Environmental_WindDirection= 18,
    /// 風速(メートル毎秒)
    SNSR_Environmental_WindSpeed    = 19,
    /// 照度(ルクス)
    SNSR_Light_Lux                  = 20,
    /// 光色温度(ケルビン)
    SNSR_Light_Temperature          = 21,
    /// 力(ニュートン)
    SNSR_Mechanical_Force           = 25,
    /// 絶対圧(パスカル)
    SNSR_Mechanical_AbsPressure     = 26,
    /// ゲージ圧(パスカル)
    SNSR_Mechanical_GaugePressure   = 27,
    /// 重量(キログラム)
    SNSR_Mechanical_Weight          = 28,
    /// X/Y/Z軸 加速度(ガル)
    SNSR_Motion_AccelerationX       = 30,
    SNSR_Motion_AccelerationY       = 31,
    SNSR_Motion_AccelerationZ       = 32,
    /// X/Y/Z軸 角加速度(度毎秒毎秒)
    SNSR_Motion_AngleAccelX         = 33,
    SNSR_Motion_AngleAccelY         = 34,
    SNSR_Motion_AngleAccelZ         = 35,
    /// 速度(メートル毎秒)
    SNSR_Motion_Speed               = 36,
    /// RFIDタグの40ビット値
    SNSR_Scanner_RFIDTag            = 40,
    /// バーコードデータを表す文字列
    SNSR_Scanner_BarcodeData        = 41,
    /// X/Y/Z 軸角(度)
    SNSR_Orientation_TiltX          = 45,
    SNSR_Orientation_TiltY          = 46,
    SNSR_Orientation_TiltZ          = 47,
    /// X/Y/Z 距離(メートル)
    SNSR_Orientation_DistanceX      = 48,
    SNSR_Orientation_DistanceY      = 49,
    SNSR_Orientation_DistanceZ      = 50,
    /// 磁北基準未補正コンパス方位
    SNSR_Orientation_MagHeading     = 51,
    /// 真北基準未補正コンパス方位
    SNSR_Orientation_TrueHeading    = 52,
    /// 磁北基準補正済みコンパス方位
    SNSR_Orientation_CompMagHeading = 53,
    /// 真北基準補正済みコンパス方位
    SNSR_Orientation_CompTrueHeading= 54,
    /// 海抜(メートル)
    SNSR_Location_Altitude          = 60,
    /// 緯度(度数)
    SNSR_Location_Latitude          = 61,
    /// 経度(度数)
    SNSR_Location_Longitude         = 62,
    /// スピード(ノット)
    SNSR_Location_Speed             = 63,
}

pub fn sensor(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let Some(category) = args.get_as_const::<SensorConst>(0, true)? else {
        return Ok(Object::Empty);
    };
    let obj = sensor::Sensor::new(category).get_as_object();
    Ok(obj)
}

pub fn sound(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let name = args.get_as_string_or_empty(0)?;
    let sync = args.get_as_bool(1, Some(false))?;
    let device = args.get_as_int(2, Some(0))?;
    let device = if device < 0 {0} else {device} as u32;
    match name {
        Some(name) => sound::play_sound(&name, sync, device),
        None => sound::stop_sound(),
    }
    Ok(Object::Empty)
}

pub fn beep(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let duration = args.get_as_int(0, Some(300u32))?;
    let freq = args.get_as_int(1, Some(2000u32))?.min(32767).max(37);
    let count = args.get_as_int(2, Some(1u32))?.max(1);
    sound::beep(duration, freq, count);
    Ok(Object::Empty)
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, PartialEq)]
pub enum ToggleKey {
    TGL_NUMLOCK    = 10000,
    TGL_CAPSLOCK   = 10001,
    TGL_SCROLLLOCK = 10002,
    TGL_KANALOCK   = 10003,
    TGL_IME        = 10004,
}
const IMC_GETOPENSTATUS: WPARAM = WPARAM(5);
const IMC_GETCONVERSIONMODE: WPARAM = WPARAM(1);

pub fn getkeystate(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let code = args.get_as_int(0, None)?;
    let id = args.get_as_int(1, Some(0))?;
    let state = get_key_state(code, id);
    Ok(state.into())
}
fn get_key_state(code: i32, id: i32) -> bool {
    unsafe {
        let (code, toggle) = match FromPrimitive::from_i32(code) {
            Some(tk) => {
                let code = match tk {
                    ToggleKey::TGL_NUMLOCK => VK_NUMLOCK,
                    ToggleKey::TGL_CAPSLOCK => VK_CAPITAL,
                    ToggleKey::TGL_SCROLLLOCK => VK_SCROLL,
                    tk => {
                        let hwnd = if id < 1 {
                            // 0以下はアクティブウィンドウ
                            GetForegroundWindow()
                        } else {
                            get_hwnd_from_id(id)
                        };
                        let hime = ImmGetDefaultIMEWnd(hwnd);
                        if tk == ToggleKey::TGL_KANALOCK {
                            // TGL_KANALOCK
                            let mode = SendMessageW(hime, WM_IME_CONTROL, IMC_GETCONVERSIONMODE, LPARAM(0)).0;
                            return (mode & IME_CMODE_KATAKANA.0 as isize) > 0;
                        } else {
                            // TGL_IME
                            let state = SendMessageW(hime, WM_IME_CONTROL, IMC_GETOPENSTATUS, LPARAM(0)).0;
                            return state != 0;
                        }
                    }
                }.0 as i32;
                (code, true)
            },
            None => (code, false),
        };
        let key_state = GetKeyState(code) as i32;
        if toggle {
            (key_state & 0x0001) > 0
        } else {
            (key_state & 0x8000) > 0
        }
    }
}