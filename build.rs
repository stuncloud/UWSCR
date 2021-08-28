use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        // res.set_icon("test.ico");

        let desc = match std::env::var("TARGET").unwrap().as_str() {
            "x86_64-pc-windows-msvc" => {
                res.set_icon(r#".\icons\UWSC\ico\MAINICON_0016-0256_light.ico"#);
                "UWSCR x64"
            },
            "i686-pc-windows-msvc" => {
                res.set_icon(r#".\icons\UWSC\ico\MAINICON_0016-0256_dark.ico"#);
                "UWSCR x86"
            },
            _ => "UWSCR"
        };
        res.set("FileDescription", &desc);
        res.set("LegalCopyright", "Joey Takahashi a.k.a. stuncloud");
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <!-- Windows 10 -->
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
            <!-- Windows 8.1 -->
            <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
            <!-- Windows 8 -->
            <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
            <!-- Windows 7 -->
            <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
            <!-- Windows Vista -->
            <supportedOS Id="{e2011457-1546-43c5-a5fe-008deee3d3f0}"/>
        </application>
    </compatibility>
    <asmv3:application>
        <asmv3:windowsSettings>
            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware> <!-- legacy -->
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2,permonitor</dpiAwareness> <!-- falls back to pm if pmv2 is not available -->
        </asmv3:windowsSettings>
    </asmv3:application>
</assembly>
        "#);
        res.compile().unwrap();
    }
    // windows-rs
    windows::build!(
        Windows::Win32::Foundation::{
            PSTR, PWSTR, BOOL, HANDLE, MAX_PATH, HINSTANCE,
            HWND, WPARAM, LPARAM, POINT, RECT, BSTR,
            DISP_E_MEMBERNOTFOUND,
            CloseHandle,
        },
        Windows::Win32::System::WindowsProgramming::{
            INFINITE,
        },
        Windows::Win32::System::Threading::{
            PROCESS_CREATION_FLAGS, PROCESS_ACCESS_RIGHTS, STARTUPINFOW, PROCESS_INFORMATION,
            STARTUPINFOW_FLAGS,
            OpenProcess, CreateProcessW, WaitForSingleObject, GetExitCodeProcess, GetCurrentProcess,
            WaitForInputIdle, IsWow64Process,
        },
        Windows::Win32::System::SystemInformation::{
            OSVERSIONINFOEXW,
            GetVersionExW, GetSystemDirectoryW, GetWindowsDirectoryW
        },
        Windows::Win32::System::SystemServices::{
            VER_NT_WORKSTATION,
        },
        Windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW,
        Windows::Win32::UI::KeyboardAndMouseInput::{
            KEYBD_EVENT_FLAGS,
            keybd_event, MapVirtualKeyW,
        },
        Windows::Win32::UI::WindowsAndMessaging::{
            WM_CLOSE, WM_DESTROY, HWND_TOPMOST, HWND_NOTOPMOST,
            SHOW_WINDOW_CMD, SET_WINDOW_POS_FLAGS, WINDOWPLACEMENT, WINDOWPLACEMENT_FLAGS,
            MONITORINFOF_PRIMARY, SYSTEM_METRICS_INDEX,
            GetCursorPos,
            WindowFromPoint, GetParent, IsWindowVisible, GetClientRect,
            GetForegroundWindow, GetWindowTextW, GetClassNameW, EnumWindows,
            IsWindow, PostMessageW, SetForegroundWindow, ShowWindow,
            SetWindowPos, GetWindowRect, MoveWindow, GetWindowPlacement,
            GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsHungAppWindow,
            GetSystemMetrics,
        },
        Windows::Win32::UI::Shell::{
            CSIDL_APPDATA,
            ShellExecuteW, SHGetSpecialFolderPathW,
        },
        Windows::Win32::UI::HiDpi::{
            GetDpiForWindow,
        },
        Windows::Win32::Storage::FileSystem::{
            GetFullPathNameW
        },
        Windows::Win32::Graphics::Gdi::{
            MONITOR_FROM_FLAGS, HMONITOR, HDC, DISPLAY_DEVICEW, MONITORINFOEXW, MONITORINFO,
            GET_DEVICE_CAPS_INDEX,
            MapWindowPoints, MonitorFromWindow, EnumDisplayMonitors,
            EnumDisplayDevicesW, GetMonitorInfoW, GetDC, GetDeviceCaps,
        },
        Windows::Win32::Graphics::Dwm::{
            DWMWINDOWATTRIBUTE,
            DwmIsCompositionEnabled, DwmGetWindowAttribute,
        },
        Windows::Win32::Globalization::{
            CP_ACP, WC_COMPOSITECHECK,
            WideCharToMultiByte, MultiByteToWideChar,
        },
        Windows::Win32::Security::{
            SECURITY_ATTRIBUTES
        },
        Windows::Win32::System::Com::{
            COINIT, CLSCTX,
            CLSIDFromProgID, CoInitializeEx, CoUninitialize, CoCreateInstance,
        },
        Windows::Win32::System::OleAutomation::{
            DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPATCH_METHOD,
            VARIANT, DISPPARAMS, EXCEPINFO, VARENUM,
            DISPID_PROPERTYPUT, SAFEARRAY, SAFEARRAYBOUND,
            IDispatch, VariantChangeType, VariantInit,
            SafeArrayCreate, SafeArrayGetElement, SafeArrayPutElement,
        },
    );
}