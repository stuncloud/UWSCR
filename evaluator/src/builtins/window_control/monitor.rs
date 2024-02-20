use util::winapi::from_wide_string;

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{RECT, POINT, LPARAM, BOOL, HWND},
        UI::{
            WindowsAndMessaging::MONITORINFOF_PRIMARY,
            HiDpi::{
                GetDpiForMonitor, MDT_DEFAULT
            },
        },
        Graphics::Gdi::{
            HMONITOR, HDC,
            EnumDisplayMonitors,
            MonitorFromPoint, MONITOR_DEFAULTTONEAREST,
            MONITORINFOEXW, MONITORINFO, GetMonitorInfoW,
            EnumDisplayDevicesW, DISPLAY_DEVICEW,
            MonitorFromWindow,
            EnumDisplaySettingsW, ENUM_CURRENT_SETTINGS
        }
    }
};
use std::mem::size_of;

#[derive(Debug)]
pub struct Monitor {
    info: MONITORINFO,
    name: String,
    primary: bool,
    index: u32,
    // device: DISPLAY_DEVICEW,
    devmode: DEVMODE,
    /// 表示スケール (%)
    scaling: f64,
    /// dpi
    dpi: f64,
    hmonitor: HMONITOR,
}

impl Monitor {
    /// モニタの数を得る
    pub fn get_count() -> u32 {
        let mut counter = 0_u32;
        Self::enum_display_monitors(Self::callback_count, &mut counter);
        counter
    }
    /// モニタ番号からモニタを得る (0から)
    pub fn from_index(index: u32) -> Option<Self> {
        let mut data = (HMONITOR::default(), index);
        Self::enum_display_monitors(Self::callback_get_handle, &mut data);
        Self::new(data.0, Some(index))
    }
    /// 座標からモニタを得る
    pub fn from_point(x: i32, y: i32) -> Option<Self> {
        unsafe {
            let pt = POINT { x, y };
            let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            Self::new(hmonitor, None)
        }
    }
    /// HWNDからモニタを得る
    pub fn from_hwnd(hwnd: HWND) -> Option<Self> {
        unsafe {
            let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            Self::new(hmonitor, None)
        }
    }
    // 各種情報を得る
    pub fn x(&self) -> i32 {
        self.info.rcMonitor.left
    }
    pub fn y(&self) -> i32 {
        self.info.rcMonitor.top
    }
    pub fn width(&self) -> i32 {
        // self.info.rcMonitor.right - self.info.rcMonitor.left
        self.devmode.dmPelsWidth as i32
    }
    pub fn height(&self) -> i32 {
        // self.info.rcMonitor.bottom - self.info.rcMonitor.top
        self.devmode.dmPelsHeight as i32
    }
    pub fn work_x(&self) -> i32 {
        self.info.rcWork.left
    }
    pub fn work_y(&self) -> i32 {
        self.info.rcWork.top
    }
    pub fn work_width(&self) -> i32 {
        let width = self.info.rcWork.right - self.info.rcWork.left;
        (width as f64 * self.scaling) as i32
    }
    pub fn work_height(&self) -> i32 {
        let height = self.info.rcWork.bottom - self.info.rcWork.top;
        (height as f64 * self.scaling) as i32
    }
    pub fn is_primary(&self) -> bool {
        self.primary
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn dpi(&self) -> f64 {
        self.dpi
    }
    pub fn scaling(&self) -> u32 {
        (self.scaling * 100.0) as u32
    }
    pub fn index(&self) -> u32 {
        self.index
    }
    fn get_dpi(hmonitor: HMONITOR) -> Option<f64> {
        unsafe {
            let mut dpix = 0;
            GetDpiForMonitor(hmonitor, MDT_DEFAULT, &mut dpix, &mut 0).ok()?;
            Some(dpix as f64)
        }
    }
    pub fn handle(&self) -> HMONITOR {
        self.hmonitor
    }

    fn new(hmonitor: HMONITOR, index: Option<u32>) -> Option<Self> {
        if hmonitor.is_invalid() {
            None
        } else {
            let miex = Self::get_monitor_info(hmonitor)?;
            let index = index.unwrap_or(Self::get_index(hmonitor));
            let device = Self::get_display_device(&miex.szDevice)?;
            let name = from_wide_string(&device.DeviceString);
            let devmode = Self::get_monitor_settings(&miex.szDevice)?;
            let dpi = Self::get_dpi(hmonitor)?;
            // スケーリングの計算
            let scaling = dpi / 96.0;
            let monitor = Self {
                info: miex.monitorInfo,
                name,
                primary: miex.monitorInfo.dwFlags == MONITORINFOF_PRIMARY,
                index,
                // device,
                devmode,
                scaling,
                dpi,
                hmonitor,
            };
            Some(monitor)
        }
    }
    fn get_monitor_settings(name: &[u16; 32]) -> Option<DEVMODE> {
        unsafe {
            let mut dm = DEVMODE::default();
            dm.dmSize = size_of::<DEVMODE>() as u16;
            let lpszdevicename = HSTRING::from_wide(name).ok()?;
            let lpdevmode = <*mut _>::cast(&mut dm);
            if EnumDisplaySettingsW(&lpszdevicename, ENUM_CURRENT_SETTINGS, lpdevmode).as_bool() {
                Some(dm)
            } else {
                None
            }
        }
    }
    fn get_monitor_info(hmonitor: HMONITOR) -> Option<MONITORINFOEXW> {
        unsafe {
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            let lpmi = <*mut _>::cast(&mut mi);
            if GetMonitorInfoW(hmonitor, lpmi).as_bool() {
                Some(mi)
            } else {
                None
            }
        }
    }
    fn get_display_device(szdevice: &[u16; 32]) -> Option<DISPLAY_DEVICEW> {
        unsafe {
            let mut dd = DISPLAY_DEVICEW::default();
            dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            let lpdevice = HSTRING::from_wide(szdevice).ok()?;
            if EnumDisplayDevicesW(&lpdevice, 0, &mut dd, 0).as_bool() {
                Some(dd)
            } else {
                None
            }
        }
    }
    fn get_index(hmonitor: HMONITOR) -> u32 {
        let mut data = (hmonitor, 0);
        Self::enum_display_monitors(Self::callback_get_index, &mut data);
        data.1
    }
    /// EnumDisplayMonitorsラッパー
    fn enum_display_monitors<T>(callback: unsafe extern "system" fn(HMONITOR, HDC, *mut RECT, LPARAM) -> BOOL, data: &mut T) -> bool {
        unsafe {
            let dwdata = LPARAM(data as *mut T as isize);
            EnumDisplayMonitors(None, None, Some(callback), dwdata).as_bool()
        }
    }
    /// モニタ数カウント用コールバック関数
    unsafe extern "system"
    fn callback_count(_: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let counter = &mut *(lparam.0 as *mut u32);
        *counter += 1;
        true.into()
    }
    /// インデックス取得用コールバック関数
    unsafe extern "system"
    fn callback_get_index(hmonitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut (HMONITOR, u32));
        if hmonitor == data.0 {
            return false.into();
        } else {
            data.1 += 1;
        }
        true.into()
    }
    /// HMONITOR取得用コールバック関数
    unsafe extern "system"
    fn callback_get_handle(hmonitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut (HMONITOR, u32));
        if data.1 == 0 {
            data.0 = hmonitor;
            false.into()
        } else {
            data.1 -= 1;
            true.into()
        }
    }
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct DEVMODE {
    pub dmDeviceName: [u16; 32],
    pub dmSpecVersion: u16,
    pub dmDriverVersion: u16,
    pub dmSize: u16,
    pub dmDriverExtra: u16,
    pub dmFields: u32,
    // pub Anonymous1: DEVMODEW_0,
    pub dmPosition: POINT,
    pub dmDisplayOrientation: u32,
    pub dmDisplayFixedOutput: u32,

    pub dmColor: i16,
    pub dmDuplex: i16,
    pub dmYResolution: i16,
    pub dmTTOption: i16,
    pub dmCollate: i16,
    pub dmFormName: [u16; 32],
    pub dmLogPixels: u16,
    pub dmBitsPerPel: u32,
    pub dmPelsWidth: u32,
    pub dmPelsHeight: u32,
    // pub Anonymous2: DEVMODEW_1,
    pub dmDisplayFlags: u32,
    pub dmNup: u32,

    pub dmDisplayFrequency: u32,
    pub dmICMMethod: u32,
    pub dmICMIntent: u32,
    pub dmMediaType: u32,
    pub dmDitherType: u32,
    pub dmReserved1: u32,
    pub dmReserved2: u32,
    pub dmPanningWidth: u32,
    pub dmPanningHeight: u32,
}