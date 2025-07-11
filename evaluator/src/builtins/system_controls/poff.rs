
use windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM, LUID, HANDLE, BOOLEAN},
        System::{
            Power::SetSuspendState,
            Shutdown::{
                ExitWindowsEx,
                SHUTDOWN_REASON, SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_OTHER, SHTDN_REASON_FLAG_PLANNED,
                EXIT_WINDOWS_FLAGS, EWX_LOGOFF, EWX_POWEROFF, EWX_SHUTDOWN, EWX_REBOOT, EWX_FORCE,
            },
            Threading::{
                OpenProcessToken, GetCurrentProcess,
            },
        },
        Security::{
            LookupPrivilegeValueW, AdjustTokenPrivileges,
            TOKEN_PRIVILEGES, LUID_AND_ATTRIBUTES,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
            SE_SHUTDOWN_NAME,
        },
        UI::WindowsAndMessaging::{
            SendMessageW,
            WM_SYSCOMMAND,
            SC_MONITORPOWER,
        },
        Graphics::Gdi::SC_SCREENSAVE,
    };

pub fn hibernate() {
    unsafe {
        let t = BOOLEAN::from(true);
        let f = BOOLEAN::from(false);
        SetSuspendState(t, f, f);
    }
}

pub fn suspend() {
    unsafe {
        let f = BOOLEAN::from(false);
        SetSuspendState(f, f, f);
    }
}

pub fn power_off(force: bool) {
    exit_windows_ex(EWX_POWEROFF, force)
}
pub fn shutdown(force: bool) {
    exit_windows_ex(EWX_SHUTDOWN, force)
}
pub fn reboot(force: bool) {
    exit_windows_ex(EWX_REBOOT, force)
}
pub fn sign_out(force: bool) {
    unsafe {
        let uflags = if force {EWX_LOGOFF|EWX_FORCE} else {EWX_LOGOFF};
        let dwreason = SHUTDOWN_REASON(0);
        let _ = ExitWindowsEx(uflags, dwreason);
    }
}

fn exit_windows_ex(flags: EXIT_WINDOWS_FLAGS, force: bool) {
    unsafe {
        // SE_SHUTDOWN_NAME 特権を得る
        let processhandle = GetCurrentProcess();
        let mut tokenhandle = HANDLE::default();
        if OpenProcessToken(processhandle, TOKEN_ADJUST_PRIVILEGES|TOKEN_QUERY, &mut tokenhandle).is_err() {
            return;
        }
        let mut lpluid = LUID::default();
        let _ = LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut lpluid);
        let laa = LUID_AND_ATTRIBUTES {
            Luid: lpluid,
            Attributes: SE_PRIVILEGE_ENABLED,
        };
        let tkp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [laa],
        };

        if AdjustTokenPrivileges(tokenhandle, false, Some(&tkp), 0, None, None).is_err() {
            return;
        }

        let uflags = if force {flags|EWX_FORCE} else {EWX_LOGOFF};
        let dwreason = SHTDN_REASON_MAJOR_OTHER|SHTDN_REASON_MINOR_OTHER|SHTDN_REASON_FLAG_PLANNED;
        let _ = ExitWindowsEx(uflags, dwreason);
    }
}

pub fn monitor_save() {
    unsafe {
        SendMessageW(HWND(-1), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(1));
    }
}
pub fn monitor_off() {
    unsafe {
        SendMessageW(HWND(-1), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(2));
    }
}
pub fn monitor_on() {
    unsafe {
        SendMessageW(HWND(-1), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(-1));
    }
}
pub fn screen_saver() {
    unsafe {
        SendMessageW(HWND(-1), WM_SYSCOMMAND, WPARAM(SC_SCREENSAVE as usize), LPARAM(0));
    }
}