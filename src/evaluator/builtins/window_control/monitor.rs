use crate::winapi::{from_wide_string};

use windows::{
    core::{HSTRING},
    Win32::{
        Foundation::{RECT, POINT, LPARAM, BOOL, HWND},
        UI::{
            WindowsAndMessaging::{MONITORINFOF_PRIMARY},
            HiDpi::{
                GetDpiForMonitor, MDT_EFFECTIVE_DPI,
            },
        },
        Graphics::{
            Gdi::{
                HMONITOR, HDC,
                EnumDisplayMonitors,
                MonitorFromPoint, MONITOR_DEFAULTTONEAREST,
                MONITORINFOEXW, MONITORINFO, GetMonitorInfoW,
                EnumDisplayDevicesW, DISPLAY_DEVICEW,
                MonitorFromWindow,
            },
        }
    }
};
use std::mem::size_of;

pub struct Monitor {
    handle: HMONITOR,
    info: MONITORINFO,
    name: [u16; 32],
    primary: bool,
    index: u32,
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
    pub fn _from_point(x: i32, y: i32) -> Option<Self> {
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
        self.info.rcMonitor.right - self.info.rcMonitor.left
    }
    pub fn height(&self) -> i32 {
        self.info.rcMonitor.bottom - self.info.rcMonitor.top
    }
    pub fn work_x(&self) -> i32 {
        self.info.rcWork.left
    }
    pub fn work_y(&self) -> i32 {
        self.info.rcWork.top
    }
    pub fn work_width(&self) -> i32 {
        self.info.rcWork.right - self.info.rcWork.left
    }
    pub fn work_height(&self) -> i32 {
        self.info.rcWork.bottom - self.info.rcWork.top
    }
    pub fn is_primary(&self) -> bool {
        self.primary
    }
    pub fn name(&self) -> Option<String> {
        let dd = self.get_display_device()?;
        let name = from_wide_string(&dd.DeviceString);
        Some(name)
    }
    pub fn dpi(&self) -> Option<f64> {
        unsafe {
            let mut dpix = 0;
            GetDpiForMonitor(self.handle, MDT_EFFECTIVE_DPI, &mut dpix, &mut 0).ok()?;
            Some(dpix as f64)
        }
    }
    pub fn scaling(&self) -> Option<f64> {
        Some(0.0)
    }
    pub fn index(&self) -> u32 {
        self.index
    }

    fn new(hmonitor: HMONITOR, index: Option<u32>) -> Option<Self> {
        if hmonitor.is_invalid() {
            None
        } else {
            let miex = Self::get_monitor_info(hmonitor)?;
            let index = index.unwrap_or(Self::get_index(hmonitor));
            let me = Self {
                handle: hmonitor,
                info: miex.monitorInfo,
                name: miex.szDevice,
                primary: miex.monitorInfo.dwFlags == MONITORINFOF_PRIMARY,
                index,
            };
            Some(me)
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
    fn get_display_device(&self) -> Option<DISPLAY_DEVICEW> {
        unsafe {
            let mut dd = DISPLAY_DEVICEW::default();
            dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            let lpdevice = HSTRING::from_wide(&self.name);
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