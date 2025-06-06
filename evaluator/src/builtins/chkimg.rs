use std::ffi::c_void;
use std::sync::{
    OnceLock,
    mpsc::channel,
};
use std::fs::File;
use std::io::Read;

use crate::error::{UError, UErrorKind, UErrorMessage};
use super::window_control::{ImgConst, Monitor};
use util::clipboard::Clipboard;
use util::winapi::WindowsResultExt;

use opencv::prelude::MatTraitConstManual;

use windows::{
    core::{Result as Win32Result, Error as Win32Error, IInspectable, ComInterface},
    Win32::{
        Foundation::{HWND, RECT, POINT, E_FAIL},
        Graphics::{
            Gdi::{
                SRCCOPY, CAPTUREBLT, DIB_RGB_COLORS,
                BITMAPINFO, BITMAPINFOHEADER,
                GetDC, GetWindowDC, ReleaseDC, DeleteDC, SelectObject, DeleteObject, GetDIBits,
                StretchBlt,
                CreateCompatibleDC, CreateCompatibleBitmap,
                CreateDIBitmap, CBM_INIT,
                ClientToScreen, IntersectRect,
                RedrawWindow, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW, RDW_ALLCHILDREN,
                HMONITOR,
            },
            Dwm::{
                DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS,
            },
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, D3D11_SDK_VERSION, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                ID3D11Texture2D, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11_CPU_ACCESS_READ,
                ID3D11Resource, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
            },
            Dxgi::{DXGI_ERROR_UNSUPPORTED, IDXGIDevice},
        },
        UI::WindowsAndMessaging::{
            SM_CYVIRTUALSCREEN, SM_CXVIRTUALSCREEN,
            SM_YVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            GetSystemMetrics,
            GetClientRect, GetWindowRect,
            GetWindow, GW_HWNDPREV,
            IsWindowVisible, IsIconic,
        },
        System::WinRT::{
            RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED,
            Graphics::Capture::IGraphicsCaptureItemInterop,
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
        },
    },

    Graphics::{
        Capture::{GraphicsCaptureItem, Direct3D11CaptureFramePool},
        DirectX::{DirectXPixelFormat, Direct3D11::IDirect3DDevice},
    },
    Foundation::TypedEventHandler,
};

use opencv::{
    core::{self as opencv_core, Mat, MatTrait, MatTraitConst, MatExprTraitConst, Vector, Point, Vec3b},
    imgcodecs, imgproc,
};

impl From<opencv::Error> for UError {
    fn from(e: opencv::Error) -> Self {
        Self::new(UErrorKind::OpenCvError, UErrorMessage::Any(e.message))
    }
}



#[derive(Debug)]
pub struct MatchedPoint {
    pub score: f64,
    pub x: i32,
    pub y: i32
}
impl MatchedPoint {
    pub fn new(x: i32, y: i32, score: f64) -> Self {
        Self {x, y, score}
    }
}
impl std::fmt::Display for MatchedPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {}, y: {} score: {}", self.x, self.y, self.score)
    }
}
pub type MatchedPoints = Vec<MatchedPoint>;
pub type ChkImgResult<T> = Result<T, UError>;

#[derive(Debug)]
pub struct SearchImage {
    image: Mat,
    width: i32,
    height: i32,
    offset_x: i32,
    offset_y: i32,
    gray_scale: bool,
}
impl SearchImage {
    pub fn from_screenshot(ss: ScreenShot, gray_scale: bool) -> ChkImgResult<Self> {
        let (width, height) = Self::get_width_height(&ss.data);
        Ok(Self {
            image: ss.data,
            width,
            height,
            offset_x: ss.left,
            offset_y: ss.top,
            gray_scale,
        })
    }
    // pub fn _from_file(path: &str) -> ChkImgResult<Self> {
    //     let image = imgcodecs::imread(path, imgcodecs::IMREAD_GRAYSCALE)?;
    //     let size = image.mat_size();
    //     Ok(Self {
    //         image,
    //         width: *size.get(0).unwrap(),
    //         height: *size.get(1).unwrap(),
    //         offset_x: 0,
    //         offset_y: 0
    //     })
    // }
    fn read_file(&self, path: &str) -> ChkImgResult<Mat> {
        let mut buf = vec![];
        let mut f = File::open(path)?;
        f.read_to_end(&mut buf)?;
        let buf = Vector::from_slice(&buf);
        let flags = if self.gray_scale {imgcodecs::IMREAD_GRAYSCALE} else {imgcodecs::IMREAD_COLOR};
        let mat = imgcodecs::imdecode(&buf, flags)?;
        Ok(mat)
    }
    /// (幅, 高さ)
    fn get_width_height(mat: &Mat) -> (i32, i32) {
        let width = mat.cols();
        let height = mat.rows();
        (width, height)
    }
    pub fn search(&self, path: &str, score: f64, max_count: Option<u8>, method: i32) -> ChkImgResult<MatchedPoints> {
        let templ = self.read_file(path)?;

        let (templ_width, templ_height) = Self::get_width_height(&templ);

        // テンプレートサイズが対象画像より大きい場合は即終了
        if self.width < templ_width || self.height < templ_height {
            return Ok(Vec::new());
        }

        // マッチング
        let mut result = Mat::default();
        if self.gray_scale {
            let mut gray = Mat::default();
            imgproc::cvt_color(&self.image, &mut gray, imgproc::COLOR_RGB2GRAY, 0)?;
            imgproc::match_template(&gray, &templ, &mut result, method, &opencv_core::no_array())?;
        } else {
            let mut image = Mat::default();
            imgproc::cvt_color(&self.image, &mut image, imgproc::COLOR_RGBA2RGB, 0)?;
            imgproc::match_template(&image, &templ, &mut result, method, &opencv_core::no_array())?;
        };

        // 検索範囲のマスク
        let cols = self.width - templ_width + 1;
        let rows = self.height - templ_height + 1;
        let mut mask = opencv_core::Mat::ones(rows, cols, opencv_core::CV_8UC1)?.to_mat()?;

        // 戻り値
        let mut matches = vec![];

        let counter = max_count.unwrap_or(10);
        for _i in 0..counter {
            // スコア
            let mut max_val = 0.0;
            // 座標
            let mut max_loc = opencv_core::Point::default();
            opencv_core::min_max_loc(
                &result,
                None,
                Some(&mut max_val),
                None,
                Some(&mut max_loc),
                &mask
            )?;
            // 指定スコアを下回っていたら終了
            if max_val < score {
                break;
            }
            let matched = MatchedPoint::new(
                max_loc.x + self.offset_x,
                max_loc.y + self.offset_y,
                max_val
            );
            matches.push(matched);

            // 検索が完了した部分のマスクを外す
            let lower_x = 0.max(max_loc.x);
            let lower_y = 0.max(max_loc.y);
            let upper_x = cols.min(max_loc.x + templ_width);
            let upper_y = rows.min(max_loc.y + templ_height);
            for x in lower_x..upper_x {
                for y in lower_y..upper_y {
                    let offset = y * cols + x;
                    let v = mask.at_mut::<u8>(offset)?;
                    *v = 0;
                }
            }
        }

        Ok(matches)
    }
}

struct WinRTInit;
impl WinRTInit {
    fn new() -> Win32Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_SINGLETHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for WinRTInit {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
thread_local! {
    static WINRT_INIT: OnceLock<Win32Result<WinRTInit>> = const { OnceLock::new() };
}

pub enum CaptureItem {
    Window(HWND),
    Monitor(HMONITOR),
}


#[derive(Debug)]
pub struct ScreenShot {
    pub data: Mat,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}
impl std::fmt::Display for ScreenShot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "ScreenShot left: {}, top: {}, width: {}, height: {}",
            self.left, self.top, self.width, self.height,
        )
    }
}
pub type ScreenShotResult = Result<ScreenShot, UError>;
impl ScreenShot {
    unsafe fn new(hwnd: Option<&HWND>, left: i32, top: i32, width: i32, height: i32) -> ScreenShotResult {
        unsafe {
            let hdc = match hwnd {
                Some(hwnd) => GetWindowDC(*hwnd),
                None => GetDC(None),
            };
            let hdc_compat = CreateCompatibleDC(hdc);
            if hdc_compat.is_invalid() {
                ReleaseDC(hwnd, hdc);
                return Err(UError::new(
                    UErrorKind::ScreenShotError,
                    UErrorMessage::GdiError("CreateCompatibleDC".into())
                ));
            }

            let hbmp = CreateCompatibleBitmap(hdc, width, height);
            let mut info = BITMAPINFO::default();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = width;
            info.bmiHeader.biHeight = -height; // 上下反転させる
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;

            let hobj = SelectObject(hdc_compat, hbmp);
            if hobj.is_invalid() {
                ReleaseDC(hwnd, hdc);
                DeleteDC(hdc_compat);
                DeleteObject(hbmp);
                return Err(UError::new(
                    UErrorKind::ScreenShotError,
                    UErrorMessage::GdiError("SelectObject".into())
                ));
            }

            let res = StretchBlt(
                hdc_compat,
                0,
                0,
                width,
                height,
                hdc,
                left,
                top,
                width,
                height,
                SRCCOPY| CAPTUREBLT
            );
            if ! res.as_bool() {
                ReleaseDC(hwnd, hdc);
                DeleteDC(hdc_compat);
                DeleteObject(hbmp);
                return Err(UError::new(
                    UErrorKind::ScreenShotError,
                    UErrorMessage::GdiError("StretchBlt".into())
                ));
            }

            let mut data = Mat::new_rows_cols(height, width, opencv_core::CV_8UC4)?;
            let pdata = data.data_mut() as *mut c_void;
            GetDIBits(
                hdc_compat,
                hbmp,
                0,
                height as u32,
                Some(pdata),
                &mut info,
                DIB_RGB_COLORS
            );

            // cleanup
            ReleaseDC(hwnd, hdc);
            DeleteDC(hdc_compat);
            DeleteObject(hbmp);

            Ok(ScreenShot {data, left, top, width, height})
        }
    }


    pub fn get_screen(left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>) -> ScreenShotResult {
        unsafe {
            // キャプチャ範囲を確定
            let vs_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vs_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vs_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vs_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            let (left, width) = match (left, right) {
                (None, None) => (vs_x, vs_w),
                (None, Some(r)) => (vs_x, r - vs_x),
                (Some(l), None) => (l, vs_w - (l - vs_x)),
                (Some(l), Some(r)) => (l, r - l),
            };
            let (top, height) = match (top, bottom) {
                (None, None) => (vs_y, vs_h),
                (None, Some(b)) => (vs_y, b - vs_y),
                (Some(t), None) => (t, vs_h - (t - vs_y)),
                (Some(t), Some(b)) => (t, b - t),
            };

            let ss = Self::new(None, left, top, width, height)?;
            Ok(ss)
        }
    }
    pub fn get_screen_wh(left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>) -> ScreenShotResult {
        unsafe {
            // キャプチャ範囲を確定
            let vs_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vs_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vs_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vs_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            let (left, width) = match (left, width) {
                (None, None) => (vs_x, vs_w),
                (None, Some(w)) => (vs_x, w),
                (Some(l), None) => (l, vs_w - (l - vs_x)),
                (Some(l), Some(w)) => (l, w),
            };
            let (top, height) = match (top, height) {
                (None, None) => (vs_y, vs_h),
                (None, Some(h)) => (vs_y, h),
                (Some(t), None) => (t, vs_h - (t - vs_y)),
                (Some(t), Some(h)) => (t, h),
            };

            Self::new(None, left, top, width, height)
        }
    }
    fn is_window_shown(hwnd: HWND) -> bool {
        unsafe {
            let mut prev = hwnd;
            let Ok(rect) = Self::get_visible_rect(hwnd) else {
                return false;
            };
            let mut dest = RECT::default();
            let mut known_hwnd = vec![];
            loop {
                prev = GetWindow(prev, GW_HWNDPREV);
                if prev.0 == 0 || known_hwnd.contains(&prev) {
                    break true;
                } else {
                    known_hwnd.push(prev);
                    if IsWindowVisible(prev).as_bool() {
                        if let Ok(prev_rect) = Self::get_visible_rect(prev) {
                            if IntersectRect(&mut dest, &rect, &prev_rect).as_bool() {
                                break false;
                            }
                        }
                    }
                }
            }
        }
    }
    fn get_visible_rect(hwnd: HWND) -> Result<RECT, UError> {
        unsafe {
            let mut rect = RECT::default();
            let pvattribute = &mut rect as *mut RECT as *mut c_void;
            let cbattribute = std::mem::size_of::<RECT>() as u32;
            DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute)?;
            Ok(rect)
        }
    }
    fn get_window_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            rect
        }
    }
    fn get_client_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            rect
        }
    }
    fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> (i32, i32) {
        unsafe {
            let mut point = POINT { x, y };
            ClientToScreen(hwnd, &mut point);
            (point.x, point.y)
        }
    }
    fn window_lr_to_wh(left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>) -> (Option<i32>, Option<i32>) {
        let width = match (left, right) {
            (None, None) => None,
            (None, Some(r)) => Some(r),
            (Some(_), None) => None,
            (Some(l), Some(r)) => Some(r - l),
        };
        let height = match (top, bottom) {
            (None, None) => None,
            (None, Some(r)) => Some(r),
            (Some(_), None) => None,
            (Some(t), Some(b)) => Some(b - t),
        };
        (width, height)
    }
    pub fn get_window(hwnd: HWND, left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>, client: bool, style: ImgConst) -> ScreenShotResult {
        let (width, height) = Self::window_lr_to_wh(left, top, right, bottom);
        Self::get_window_wh(hwnd, left, top, width, height, client, style)
    }
    pub fn get_window_wh(hwnd: HWND, left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>, client: bool, style: ImgConst) -> ScreenShotResult {
        unsafe {
            let is_fore = match style {
                ImgConst::IMG_AUTO => Self::is_window_shown(hwnd),
                ImgConst::IMG_FORE => true,
                ImgConst::IMG_BACK => false,
            };

            let dx = left.unwrap_or(0);
            let dy = top.unwrap_or(0);

            let (left, top, width, height) = if client {
                let crect = Self::get_client_rect(hwnd);
                let mut point = POINT {
                    x: left.unwrap_or(crect.left),
                    y: top.unwrap_or(crect.top),
                };
                let width = width.unwrap_or(crect.right - crect.left - point.x);
                let height = height.unwrap_or(crect.bottom - crect.top - point.y);
                // スクリーン座標を得る
                ClientToScreen(hwnd, &mut point);
                if is_fore {
                    // IMG_FOREならスクリーン座標を返す
                    (point.x, point.y, width, height)
                } else {
                    // IMG_BACKならクライアント座標へのオフセットを返す
                    let vrect = Self::get_visible_rect(hwnd)?;
                    let wrect = Self::get_window_rect(hwnd);
                    let left = point.x - vrect.left + (vrect.left - wrect.left);
                    let top = point.y - vrect.top + (vrect.top - wrect.top);
                    (left, top, width, height)
                }
            } else {
                let (margin_x, margin_y, rect_w, rect_h) = if is_fore {
                    let rect = Self::get_visible_rect(hwnd)?;
                    (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
                } else {
                    let vrect = Self::get_visible_rect(hwnd)?;
                    let wrect = Self::get_window_rect(hwnd);
                    (vrect.left - wrect.left, vrect.top - wrect.top, vrect.right - vrect.left, vrect.bottom - vrect.top)
                };
                let left = left.unwrap_or(0);
                let top = top.unwrap_or(0);
                let width = width.unwrap_or(rect_w - left);
                let height = height.unwrap_or(rect_h - top);
                (left + margin_x, top + margin_y, width, height)
            };

            if ! is_fore {
                // IMG_BACKの場合は再描画する
                let mut flags = RDW_INVALIDATE|RDW_UPDATENOW|RDW_ALLCHILDREN;
                if ! client {
                    flags |= RDW_FRAME;
                }
                RedrawWindow(hwnd, None, None, flags);
            }
            let hwnd = if is_fore {None} else {Some(&hwnd)};
            let mut ss = Self::new(hwnd, left, top, width, height)?;
            ss.left = dx;
            ss.top = dy;
            Ok(ss)
        }
    }
    pub fn save(&self, filename: Option<&str>) -> ChkImgResult<()> {
        if ! self.data.empty() {
            let vector = opencv_core::Vector::new();
            let default = format!("chkimg_ss_{}_{}.png", self.width, self.height);
            let filename = filename.unwrap_or(&default);
            imgcodecs::imwrite(filename, &self.data, &vector)?;
        }
        Ok(())
    }
    pub fn save_to(&self, filename: &str, jpg_quality: Option<i32>, png_compression: Option<i32>) -> ChkImgResult<()> {
        let mut params = opencv_core::Vector::new();
        if let Some(val) = jpg_quality {
            params.push(imgcodecs::IMWRITE_JPEG_QUALITY);
            params.push(val);
        }
        if let Some(val) = png_compression {
            params.push(imgcodecs::IMWRITE_PNG_COMPRESSION);
            params.push(val);
        }
        imgcodecs::imwrite(filename, &self.data, &params)?;
        Ok(())
    }
    pub fn to_clipboard(&self) -> ChkImgResult<()> {
        unsafe {
            let mut info = BITMAPINFO::default();
            let size = self.data.size()?;
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = size.width;
            info.bmiHeader.biHeight = -size.height; // 上下反転
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;

            let hdc = GetDC(None);
            if hdc.is_invalid() {
                return Err(UError::new(
                    UErrorKind::ScreenShotError,
                    UErrorMessage::GdiError("GetDC".into())
                ));
            }
            let pjbits = Some(self.data.data() as *const c_void);
            let hbmp = CreateDIBitmap(hdc, Some(&info.bmiHeader), CBM_INIT as u32, pjbits, Some(&info), DIB_RGB_COLORS);

            if let Ok(cb) = Clipboard::new() {
                cb.set_bmp(hbmp);
            }
            ReleaseDC(None, hdc);
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.data.empty()
    }

    fn crop_image(mat: &Mat, x: i32, y: i32, width: i32, height: i32) -> opencv::Result<Mat> {
        let roi = opencv_core::Rect { x, y, width, height };
        Mat::roi(mat, roi)
    }

    /* Windows Graphics Capture API */

    pub fn get_screen_wgcapi(monitor: u32, left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>) -> ScreenShotResult {
        let monitor = Monitor::from_index(monitor)
            .ok_or(UError::new(UErrorKind::ScreenShotError, UErrorMessage::MonitorNotFound))?;
        let crop_flg = left.is_some() || top.is_some() || right.is_some() || bottom.is_some();

        let x = left.unwrap_or(0);
        let y = top.unwrap_or(0);
        let width = right.map(|right| right - x ).unwrap_or(monitor.width() - x);
        let height = bottom.map(|bottom| bottom - y).unwrap_or(monitor.height() - y);
        let left = monitor.x() + x;
        let top = monitor.y() + y;

        let mat = Self::capture(CaptureItem::Monitor(monitor.handle()))?;

        let data = if crop_flg {
            Self::crop_image(&mat, x, y, width, height)?
        } else {
            mat
        };

        let ss = Self { data, left, top, width, height };
        Ok(ss)
    }
    pub fn get_window_wgcapi(hwnd: HWND, left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>, client: bool) -> ScreenShotResult {
        let (width, height) = Self::window_lr_to_wh(left, top, right, bottom);
        Self::get_window_wgcapi_wh(hwnd, left, top, width, height, client)
    }
    pub fn get_window_wgcapi_wh(hwnd: HWND, left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>, client: bool) -> ScreenShotResult {
        let mat = Self::capture(CaptureItem::Window(hwnd))?;
        let crop_flg = left.is_some() || top.is_some() || width.is_some() || height.is_some();

        // クライアント領域の切り出し
        let mat = if client {
            let crect = Self::get_client_rect(hwnd);
            let vrect = Self::get_visible_rect(hwnd)?;

            let (cx, cy) = Self::client_to_screen(hwnd, crect.left, crect.top);

            let cx = cx - vrect.left;
            let cy = cy - vrect.top;
            let cw = crect.right - crect.left;
            let ch = crect.bottom - crect.top;
            // クライアント領域を切り出す
            Self::crop_image(&mat, cx, cy, cw, ch)?
        } else {
            mat
        };
        let mat_w = mat.cols();
        let mat_h = mat.rows();

        let (x, width) = match (left, width) {
            (None, None) => (0, mat_w),
            (None, Some(w)) => (0, w.min(mat_w)),
            (Some(l), None) => (l, mat_w - l),
            (Some(l), Some(w)) => (l, w.min(mat_w - l)),
        };
        let (y, height) = match (top, height) {
            (None, None) => (0, mat_h),
            (None, Some(h)) => (0, h.min(mat_h)),
            (Some(t), None) => (t, mat_h - t),
            (Some(t), Some(h)) => (t, h.min(mat_h - t)),
        };
        let data = if crop_flg {
            Self::crop_image(&mat, x, y, width, height)?
        } else {
            mat
        };

        let left = left.unwrap_or(0);
        let top = top.unwrap_or(0);
        let ss = ScreenShot { data, left, top, width, height };

        Ok(ss)

    }

    fn capture(item: CaptureItem) -> Result<Mat, UError> {
        WINRT_INIT.with(|once| {
            match once.get_or_init(WinRTInit::new) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.clone()),
            }
        })?;
        unsafe {
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
                .err_hint("core::factory")?;
            let item: GraphicsCaptureItem = match item {
                CaptureItem::Window(hwnd) => {
                    interop.CreateForWindow(hwnd).err_hint("CreateForWindow")?
                },
                CaptureItem::Monitor(hmonitor) => {
                    interop.CreateForMonitor(hmonitor).err_hint("CreateForMonitor")?
                },
            };

            let d3d_device = Self::create_d3d_device().err_hint("create_d3d_device")?;
            let context = d3d_device.GetImmediateContext().err_hint("GetImmediateContext")?;

            let texture = {
                let size = item.Size().err_hint("GraphicsCaptureItem::Size")?;

                let zero_sized = size.Height == 0 || size.Width == 0;

                let dxgidevice: IDXGIDevice = d3d_device.cast()?;
                let device: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgidevice)
                    .err_hint("CreateDirect3D11DeviceFromDXGIDevice")?
                    .cast()
                    .err_hint("cast<IDirect3DDevice>")?;

                let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 1, size)
                    .err_hint("Direct3D11CaptureFramePool::CreateFreeThreaded")
                    .map_err(|e| {
                        if e.is_invalid_arg_error() && zero_sized {
                            // E_INVALIDARGかつサイズ0の場合は注意喚起エラー
                            UError::new(UErrorKind::CaptureError, UErrorMessage::ExplorerMayBeSuspended)
                        } else {
                            e.into()
                        }
                    })?;
                let session = frame_pool.CreateCaptureSession(&item)
                    .err_hint("CreateCaptureSession")?;

                // キャプチャ時に枠を消す
                // 失敗するのだけれどダメ元でやる
                let _ = session.SetIsBorderRequired(false);

                let (s, r) = channel();
                let handler = TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                    move |frame_pool, _| {
                        let pool = frame_pool.as_ref().unwrap();
                        let frame = pool.TryGetNextFrame()?;
                        s.send(frame).map_err(|_| Win32Error::from(E_FAIL))?;
                        Ok(())
                    }
                });
                let token = frame_pool.FrameArrived(&handler).err_hint("FrameArrived")?;
                session.StartCapture().err_hint("StartCapture")?;

                let frame = r.recv().map_err(|_| Win32Error::from(E_FAIL)).err_hint("recv(Direct3D11CaptureFrame)")?;
                let access: IDirect3DDxgiInterfaceAccess = frame.Surface()
                    .err_hint("Surface")?
                    .cast()
                    .err_hint("cast<IDirect3DDxgiInterfaceAccess>")?;
                let source: ID3D11Texture2D = access.GetInterface().err_hint("GetInterface<ID3D11Texture2D>")?;

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                source.GetDesc(&mut desc);
                desc.BindFlags = 0;
                desc.MiscFlags = 0;
                desc.Usage = D3D11_USAGE_STAGING;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

                let mut texture = None;
                d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))
                    .err_hint("CreateTexture2D")?;
                let texture = texture.unwrap();

                context.CopyResource(
                    Some(&texture.cast().err_hint("texture.cast")?),
                    Some(&source.cast().err_hint("source.cast")?)
                );

                // 後始末
                frame_pool.RemoveFrameArrived(token).err_hint("RemoveFrameArrived")?;
                frame_pool.Close().err_hint("Direct3D11CaptureFramePool::Close")?;
                session.Close().err_hint("GraphicsCaptureSession::Close")?;

                texture
            };

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);

            let resource: ID3D11Resource = texture.cast().err_hint("cast<ID3D11Resource>")?;
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();

            context.Map(Some(&resource), 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .err_hint("ID3D11DeviceContext::Map")?;

            let slice = std::slice::from_raw_parts(mapped.pData as *const u8, (desc.Height * mapped.RowPitch) as usize);

            let bytes_per_pixel = 4;
            let mut bits = vec![0_u8; (desc.Width * desc.Height * bytes_per_pixel) as usize];
            let desc_width_bytes = desc.Width * bytes_per_pixel;
            for row in 0..desc.Height {
                let data_begin = (row * desc_width_bytes) as usize;
                let data_end = ((row + 1) * desc_width_bytes) as usize;
                let slice_begin = (row * mapped.RowPitch) as usize;
                let slice_end = slice_begin + desc_width_bytes as usize;
                bits[data_begin..data_end].copy_from_slice(&slice[slice_begin..slice_end]);
            }

            context.Unmap(Some(&resource), 0);

            let width = desc.Width as i32;
            let height = desc.Height as i32;

            let mut data = Mat::new_rows_cols(height, width, opencv_core::CV_8UC4)?;
            let pdata = data.data_mut();
            std::ptr::copy_nonoverlapping(bits.as_ptr(), pdata, bits.len());

            Ok(data)
        }
    }
    fn create_d3d_device() -> Win32Result<ID3D11Device> {
        let device = match Self::create_d3d_device_with_type(D3D_DRIVER_TYPE_HARDWARE) {
            Ok(device) => device,
            Err(err) => {
                if err.code() == DXGI_ERROR_UNSUPPORTED {
                    Self::create_d3d_device_with_type(D3D_DRIVER_TYPE_WARP)
                } else {
                    Err(err)
                }?
            },
        };
        Ok(device.unwrap())
    }
    fn create_d3d_device_with_type(drivertype: D3D_DRIVER_TYPE) -> Win32Result<Option<ID3D11Device>> {
        let mut device = None;
        unsafe {
            D3D11CreateDevice(None, drivertype, None, D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, None)
                .map(|_| device)
        }
    }

    pub fn is_window_capturable(hwnd: HWND) -> bool {
        unsafe {
            IsWindowVisible(hwnd).as_bool() & ! IsIconic(hwnd).as_bool()
        }
    }
}

pub struct CheckColor {
    lower: Vec3b,
    upper: Vec3b,
}
impl CheckColor {
    pub fn new(color: [u8; 3], threshold: Option<[u8; 3]>) -> Self {
        let (lower, upper) = match threshold {
            Some(t) => {
                let lower = Vec3b::from([
                    color[0].saturating_sub(t[0]),
                    color[1].saturating_sub(t[1]),
                    color[2].saturating_sub(t[2]),
                ]);
                let upper = Vec3b::from([
                    color[0].saturating_add(t[0]),
                    color[1].saturating_add(t[1]),
                    color[2].saturating_add(t[2]),
                ]);

                (lower, upper)
            },
            None => {
                let lower = Vec3b::from(color);
                let upper = Vec3b::from(color);
                (lower, upper)
            },
        };
        Self { lower, upper }
    }
    pub fn new_from_bgr(color: u32, threshold: Option<[u8; 3]>) -> Self {
        let color = [
            ((color & 0xFF0000) >> 16) as u8,
            ((color & 0xFF00) >> 8) as u8,
            (color & 0xFF) as u8,
        ];
        Self::new(color, threshold)
    }
    pub fn search(&self, ss: &ScreenShot) -> Result<Vec<ColorFound>, UError> {

        let mut bgr = Mat::default();
        imgproc::cvt_color(&ss.data, &mut bgr, imgproc::COLOR_RGBA2RGB, 0)?;

        let mut mask = Mat::default();
        opencv_core::in_range(&bgr, &self.lower, &self.upper, &mut mask)?;

        let mut indices: Vector<Point> = Vector::new();
        opencv_core::find_non_zero(&mask, &mut indices)?;

        let points = indices.into_iter()
            .map(|p| {
                let x = p.x + ss.left;
                let y = p.y + ss.top;
                let vec3b = bgr.at_2d::<opencv_core::Vec3b>(p.y, p.x)?;
                let color = vec3b.0;
                Ok(ColorFound::new(x, y, color))
            })
            .collect::<Result<Vec<_>, opencv::Error>>()?;

        Ok(points)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorFound {
    pub x: i32,
    pub y: i32,
    pub color: [u8; 3]
}
impl ColorFound {
    fn new(x: i32, y: i32, color: [u8; 3]) -> Self {
        Self { x, y, color }
    }
}
impl std::fmt::Display for ColorFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}, {:?}]", self.x, self.y, self.color)
    }
}


/* chkimg */

use std::num::NonZeroUsize;
use std::path::Path;
use std::cmp::Ordering;
use image::{load_from_memory, DynamicImage, GenericImageView, RgbImage};
use rayon::iter::{IntoParallelIterator, ParallelIterator};


#[derive(PartialEq)]
pub struct Image {
    inner: DynamicImage,
}
impl Image {
    fn new(buffer: &[u8]) -> ChkImgResult<Self> {
        let inner = load_from_memory(buffer)
            .map_err(|e| UError::new(
                UErrorKind::Any("Image Error".into()),
                UErrorMessage::Any(e.to_string()))
            )?;
        Ok(Self { inner })
    }
    /// ファイルから自身を作成
    pub fn from_file<P: AsRef<Path>>(path: P) -> ChkImgResult<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Self::new(&buffer)
    }
    pub fn from_clipboard() -> Option<Self> {
        use clipboard_rs::{ClipboardContext, common::RustImage, Clipboard};
        let ctx = ClipboardContext::new().ok()?;
        let image = ctx.get_image()
            .inspect_err(|e| {dbg!(e);})
            .ok()?;
        image.get_dynamic_image().ok()
            .map(|inner| Self { inner })
    }
    pub(super) fn to_rgb8(&self) -> RgbImage {
        self.inner.to_rgb8()
    }

    fn first_row(rgb: &RgbImage) -> &[u8] {
        rgb.as_raw()
            .chunks_exact(rgb.width() as usize * 3)
            .next()
            .unwrap()
    }
    fn last_row(rgb: &RgbImage) -> &[u8] {
        rgb.as_raw()
            .rchunks_exact(rgb.width() as usize * 3)
            .next()
            .unwrap()
    }
    fn left_rgb(row: &[u8]) -> &[u8] {
        row.chunks_exact(3).next().unwrap()
    }
    fn right_rgb(row: &[u8]) -> &[u8] {
        row.rchunks_exact(3).next().unwrap()
    }

    /// 左上の色
    pub fn left_top(&self) -> RgbColor {
        let rgb = self.to_rgb8();
        let bytes = rgb.as_raw();
        RgbColor::from(&bytes[0..=2])
    }
    /// 左上の色
    pub fn right_top(&self) -> RgbColor {
        let rgb = self.to_rgb8();
        let pixel = Self::right_rgb(Self::first_row(&rgb));
        pixel.into()
    }
    /// 左下の色
    pub fn left_bottom(&self) -> RgbColor {
        let rgb = self.to_rgb8();
        let pixel = Self::left_rgb(Self::last_row(&rgb));
        pixel.into()
    }
    /// 右下の色
    pub fn right_bottom(&self) -> RgbColor {
        let rgb = self.to_rgb8();
        let pixel = Self::right_rgb(Self::last_row(&rgb));
        pixel.into()
    }

}
impl From<RgbImage> for Image {
    fn from(value: RgbImage) -> Self {
        Self { inner: DynamicImage::ImageRgb8(value) }
    }
}
impl PartialOrd for Image {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (i_w, i_h) = self.inner.dimensions();
        let (o_w, o_h) = other.inner.dimensions();
        if i_w > o_w && i_h > o_h {
            Some(Ordering::Greater)
        } else if i_w == o_w && i_h == o_h {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Less)
        }
    }
}
/// 画像マッチ座標
#[derive(Debug, PartialEq, Clone, Default)]
pub struct MatchLocation {
    /// マッチしたX座標
    pub x: i32,
    /// マッチしたY座標
    pub y: i32,
}
impl MatchLocation {
    pub(crate) fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
/// RGB
#[derive(Debug)]
pub struct RgbColor([u8; 3]);
impl From<&[u8]> for RgbColor {
    /// ## panics
    /// スライスの長さが3未満だとpanicする
    fn from(slice: &[u8]) -> Self {
        if slice.len() < 3 {
            panic!("Length of &[u8] must be 3");
        } else {
            Self([slice[0], slice[1], slice[2]])
        }
    }
}
impl From<[u8; 3]> for RgbColor {
    fn from(rgb: [u8; 3]) -> Self {
        Self(rgb)
    }
}

/// chkimg風探索の結果
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum ChkimgLikeMatches {
    /// n番目指定
    Nth(Option<MatchLocation>),
    /// 全探索
    All(Vec<MatchLocation>),
}
/// 探索結果の形式
pub enum ResultType {
    Nth(NonZeroUsize),
    All,
}
impl From<i32> for ResultType {
    fn from(value: i32) -> Self {
        match value {
            0 | -1 => Self::All,
            n => Self::Nth(NonZeroUsize::new(n as usize).unwrap())
        }
    }
}
impl Default for ResultType {
    fn default() -> Self {
        Self::Nth(NonZeroUsize::new(1).unwrap())
    }
}
/// 探索方法
#[derive(Debug)]
pub enum SearchMethod {
    /// 完全一致
    Exact,
    /// 色幅マッチ
    Threshold(RgbColor),
    /// 透過色マッチ
    Transparent(RgbColor),
    /// 色幅かつ透過色あり\
    /// (閾値, 透過色)
    ThresholdTransparent(RgbColor, RgbColor),
    /// 色無視マッチ\
    /// 前のピクセルと比べて違いがあったかどうかで判定
    /// - 探す画像の前後ピクセルが一致の場合対象も前後一致であれば形としては一致
    /// - 探す画像の前後ピクセルが不一致の場合対象も前後不一致であれば形としては一致
    Shape
}
impl SearchMethod {
    pub fn new(method: i32, threshold: u32, target: &Image) -> Self {
        let threshold = Self::rgb_from_threshold(threshold);
        match method {
            1 => threshold.map(|t| Self::ThresholdTransparent(t, target.left_top()))
                .unwrap_or(Self::Transparent(target.left_top())),
            2 => threshold.map(|t| Self::ThresholdTransparent(t, target.right_top()))
                .unwrap_or(Self::Transparent(target.right_top())),
            3 => threshold.map(|t| Self::ThresholdTransparent(t, target.left_bottom()))
                .unwrap_or(Self::Transparent(target.left_bottom())),
            4 => threshold.map(|t| Self::ThresholdTransparent(t, target.right_bottom()))
                .unwrap_or(Self::Transparent(target.right_bottom())),
            -1 => Self::Shape,
            _ => threshold.map(Self::Threshold)
                .unwrap_or(Self::Exact),
        }
    }
    fn rgb_from_threshold(threshold: u32) -> Option<RgbColor> {
        let r = match threshold & 15 {
            1 => 2,
            3 => 4,
            7 => 8,
            15 => 16,
            _ => 0
        };
        let g = match threshold & 3840 {
            256 => 2,
            768 => 4,
            1792 => 8,
            3840 => 16,
            _ => 0,
        };
        let b = match threshold & 983040 {
            65536 => 2,
            196608 => 4,
            458752 => 8,
            983040 => 16,
            _ => 0,
        };
        (r > 0 || g > 0 || b > 0).then_some(RgbColor([r, g, b]))
    }
    /// メソッドに従いマッチ判定を行う
    fn matches(&self, row_slice: &[u8], target_slice: &[u8]) -> bool {
        // RGBのイテレータにする
        let row_iter = row_slice.chunks_exact(3);
        let target_iter = target_slice.chunks_exact(3);
        match self {
            SearchMethod::Exact => {
                row_slice.eq(target_slice)
            },
            SearchMethod::Threshold(threshold) => {
                row_iter.zip(target_iter)
                    .all(|(color, target)| {
                        threshold.in_range(color, target)
                    })
            },
            SearchMethod::Transparent(transparent) => {
                row_iter.zip(target_iter)
                    .all(|(color, target)| {
                        // targetが透過色ならtrue
                        transparent.eq(target) ||
                        // 透過色以外なら色が一致するかを確認
                        color.eq(target)
                    })
                },
            Self::ThresholdTransparent(threshold, transparent) => {
                row_iter.zip(target_iter)
                    .all(|(color, target)| {
                        // targetが透過色ならtrue
                        transparent.eq(target) ||
                        // 透過色以外なら色幅一致するかを確認
                        threshold.in_range(color, target)
                    })
            }
            SearchMethod::Shape => {
                let mut row_iter = row_iter;
                let mut target_iter = target_iter;
                if let (Some(row_first), Some(target_first)) = (row_iter.next(), target_iter.next()) {
                    target_iter.zip(row_iter)
                        .try_fold((target_first, row_first), |prev, cur| {
                            // - targetの前後が一致かつ対象の前後も一致
                            // - targetの前後が不一致かつ対象の前後も不一致
                            // の場合に続行
                            prev.0.eq(cur.0)
                                .eq(&prev.1.eq(cur.1))
                                .then_some(cur)
                        })
                        // 最後まで一致であればSomeとなり、trueを返す
                        .is_some()
                } else {
                    // いずれも最初の色がないというのはunreachableな気もするが一応falseを返す
                    false
                }
            },
        }
    }
}
/// chkimg風画像探索
pub struct ChkimgLikeImageMatcher {
    captured: ScreenShot,
    target: RgbImage,
}
impl ChkimgLikeImageMatcher {
    pub fn new(mut captured: ScreenShot, target: &Image) -> ChkImgResult<Self> {
        let target = target.to_rgb8();

        let _ = std::fs::write("D:\\work\\uwscr_test\\target.txt", format!("{target:#?}"));

        let mut rgb = Mat::default();
        imgproc::cvt_color(&captured.data, &mut rgb, imgproc::COLOR_BGR2RGB, 0)?;
        captured.data = rgb;
        Ok(Self { captured, target, })
    }
    /// 画像探索
    pub fn find<RT>(&self, method: SearchMethod, rtype: RT) -> ChkimgLikeMatches
    where RT: Into<ResultType>,
    {
        let rows = (self.captured.height as u32 - self.target.height() + 1) as usize;

        let matcher = MatcherInner {
            method: &method,
            captured: self.captured.data.data_bytes().unwrap(),
            width: self.captured.width as usize * 3,
            target: self.target.as_raw(),
            window_size: self.target.width() as usize * 3,
            target_rows: self.target.height() as usize,
            offset_x: self.captured.left,
            offset_y: self.captured.top,
        };

        match rtype.into() {
            ResultType::Nth(nth) => {
                let n = nth.get().saturating_sub(1);
                let found = (0..rows)
                    .flat_map(|row| matcher.get_matched(row))
                    .nth(n);
                ChkimgLikeMatches::Nth(found)
            },
            ResultType::All => {
                let found = (0..rows)
                    .flat_map(|row| matcher.get_matched(row))
                    .collect();
                ChkimgLikeMatches::All(found)
            },
        }
    }
}

struct MatcherInner<'a> {
    method: &'a SearchMethod,
    captured: &'a [u8],
    width: usize,
    target: &'a [u8],
    window_size: usize,
    target_rows: usize,
    offset_x: i32,
    offset_y: i32,
}
impl MatcherInner<'_> {
    fn get_matched(&self, row: usize) -> Vec<MatchLocation> {
        let first_target_slice = &self.target[0..self.window_size];

        // 列の探索範囲は行の最初からtargetが収まる範囲まで
        let from = row * self.width;
        let to = from + self.width - self.window_size + 1;
        let captured_row_slice = &self.captured[from..to];

        let windows = captured_row_slice
            // target幅でwindowにしていく
            .windows(self.window_size)
            // 列インデックスを追加
            .enumerate()
            // 画像の列はインデックス÷3なのでstepを入れる
            .step_by(3);

        windows
            .filter_map(move |(col, row_slice)| {
                // targetの最初の行がマッチするかどうか
                self.method.matches(row_slice, first_target_slice)
                    .then_some(col)
            })
            .filter(move |col| {
            // targetの2行目以降全体とのマッチを判定
            (1..self.target_rows).into_par_iter()
                .all(|t_row| {
                    let t_from = t_row * self.window_size;
                    let t_to = t_from + self.window_size;
                    // targetの行スライス
                    let target_slice = &self.target[t_from..t_to];
                    let c_from = (row + t_row) * self.width + col;
                    let c_to = c_from + self.window_size;
                    // captuerdの該当部分の行スライス
                    let row_slice = &self.captured[c_from..c_to];
                    self.method.matches(row_slice, target_slice)
                })
            })
            .map(move |col| {
                // 画像列はインデックス÷3 (RGB幅)
                let x = (col/3) as i32 + self.offset_x;
                let y = row as i32 + self.offset_y;
                MatchLocation::new(x, y)
            })
            .collect()
    }
}

impl RgbColor {
    /// 自身のRGB値をそれぞれ閾値としてtargetからの色範囲にcolorが含まれるかを返す\
    /// 閾値が0の場合は直接比較\
    /// 条件は (target - 閾値) < color < (target + 閾値)
    fn in_range(&self, color: &[u8], target: &[u8]) -> bool {
        self.0.iter().enumerate()
            .all(|(i, t)| {
                if *t == 0 {
                    target[i].eq(&color[i])
                } else {
                    ((target[i].saturating_sub(*t)+1)..(target[i].saturating_add(*t)))
                        .contains(&color[i])
                }
            })
    }
}

impl PartialEq<[u8]> for RgbColor {
    fn eq(&self, other: &[u8]) -> bool {
        self.0.eq(other)
    }
}