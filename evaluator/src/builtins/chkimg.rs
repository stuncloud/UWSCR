use std::{ffi::c_void, io::Read};
use std::fs::File;
use std::sync::{
    OnceLock,
    mpsc::channel,
};

use crate::error::{UError, UErrorKind, UErrorMessage};
use super::window_control::{ImgConst, Monitor};
use super::clipboard::Clipboard;

use opencv::prelude::MatTraitConstManual;

use windows::{
    core::{Result as Win32Result, Error as Win32Error, IInspectable, ComInterface},
    Win32::{
        Foundation::{HWND, RECT, POINT, E_FAIL,},
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
            IsWindowVisible,
        },
        System::WinRT::{
            RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED,
            Graphics::Capture::IGraphicsCaptureItemInterop,
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
        }
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
pub struct ChkImg {
    image: Mat,
    width: i32,
    height: i32,
    offset_x: i32,
    offset_y: i32,
    gray_scale: bool,
}
impl ChkImg {
    pub fn from_screenshot(ss: ScreenShot, gray_scale: bool) -> ChkImgResult<Self> {
        let size = ss.data.mat_size();
        Ok(Self {
            image: ss.data,
            width: *size.get(0).unwrap(),
            height: *size.get(1).unwrap(),
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
    pub fn search(&self, path: &str, score: f64, max_count: Option<u8>) -> ChkImgResult<MatchedPoints> {
        let buf= {
            let mut buf = vec![];
            let mut f = File::open(path)?;
            f.read_to_end(&mut buf)?;
            Vector::from_slice(buf.as_slice())
        };
        let templ = if self.gray_scale {
            imgcodecs::imdecode(&buf, imgcodecs::IMREAD_GRAYSCALE)?
        } else {
            imgcodecs::imdecode(&buf, imgcodecs::IMREAD_UNCHANGED)?
        };
        let templ_width = *templ.mat_size().get(0)
            .ok_or(UError::new(UErrorKind::OpenCvError, UErrorMessage::FailedToLoadImageFile(path.into())))?;
        let templ_height = *templ.mat_size().get(1)
            .ok_or(UError::new(UErrorKind::OpenCvError, UErrorMessage::FailedToLoadImageFile(path.into())))?;


        // マッチング
        let mut result = Mat::default();
        if self.gray_scale {
            let mut gray = Mat::default();
            imgproc::cvt_color(&self.image, &mut gray, imgproc::COLOR_RGB2GRAY, 0)?;
            imgproc::match_template(&gray, &templ, &mut result, imgproc::TM_CCOEFF_NORMED, &opencv_core::no_array())?;
        } else {
            imgproc::match_template(&self.image, &templ, &mut result, imgproc::TM_CCOEFF_NORMED, &opencv_core::no_array())?;
        };


        // 検索範囲のマスク
        let rows = self.width - templ_width + 1;
        let cols = self.height - templ_height + 1;
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
            let lower_x = 0.max(max_loc.x - templ_width / 2 + 1);
            let lower_y = 0.max(max_loc.y - templ_height/ 2 + 1);
            let upper_x = cols.min(max_loc.x + templ_width / 2);
            let upper_y = rows.min(max_loc.y + templ_height/ 2);
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
    static WINRT_INIT: OnceLock<Win32Result<WinRTInit>> = OnceLock::new();
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
        let vector = opencv_core::Vector::new();
        let default = format!("chkimg_ss_{}_{}.png", self.width, self.height);
        let filename = filename.unwrap_or(&default);
        imgcodecs::imwrite(filename, &self.data, &vector)?;
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

    fn crop_image(mat: &Mat, x: i32, y: i32, width: i32, height: i32) -> opencv::Result<Mat> {
        let roi = opencv_core::Rect { x, y, width, height };
        Mat::roi(&mat, roi)
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
        let mat_w = mat.cols();
        let mat_h = mat.rows();
        let crop_flg = left.is_some() || top.is_some() || width.is_some() || height.is_some();


        let (x, width) = match (left, width) {
            (None, None) => (0, mat_w),
            (None, Some(w)) => (0, w),
            (Some(l), None) => (l, mat_w - l),
            (Some(l), Some(w)) => (l, w),
        };
        let (y, height) = match (top, height) {
            (None, None) => (0, mat_h),
            (None, Some(h)) => (0, h),
            (Some(t), None) => (t, mat_h - t),
            (Some(t), Some(h)) => (t, h),
        };

        let data = if crop_flg {
            Self::crop_image(&mat, x, y, width, height)?
        } else {
            mat
        };

        let mut left = left.unwrap_or(0);
        let mut top = top.unwrap_or(0);

        let ss = if client {
            let crect = Self::get_client_rect(hwnd);
            let vrect = Self::get_visible_rect(hwnd)?;

            let (cx, cy) = Self::client_to_screen(hwnd, crect.left, crect.top);

            let cx = cx - vrect.left;
            let cy = cy - vrect.top;
            let cw = crect.right - crect.left;
            let ch = crect.bottom - crect.top;
            // クライアント領域を切り出す
            let data = Self::crop_image(&data, cx, cy, cw, ch)?;

            // 切り出した分オフセットを補正
            left += cx;
            top += cy;

            ScreenShot { data, left, top, width, height }
        } else {
            ScreenShot { data, left, top, width, height }
        };

        Ok(ss)

    }

    fn capture(item: CaptureItem) -> Result<Mat, UError> {
        WINRT_INIT.with(|once| {
            match once.get_or_init(|| WinRTInit::new()) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.clone()),
            }
        })?;
        unsafe {
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = match item {
                CaptureItem::Window(hwnd) => {
                    interop.CreateForWindow(hwnd)?
                },
                CaptureItem::Monitor(hmonitor) => {
                    interop.CreateForMonitor(hmonitor)?
                },
            };

            let d3d_device = Self::create_d3d_device()?;
            let context = d3d_device.GetImmediateContext()?;

            let texture = {
                let size = item.Size()?;

                let dxgidevice: IDXGIDevice = d3d_device.cast()?;
                let device: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgidevice)?.cast()?;

                let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 1, size)?;
                let session = frame_pool.CreateCaptureSession(&item)?;

                let (s, r) = channel();
                let handler = TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                    move |frame_pool, _| {
                        let pool = frame_pool.as_ref().unwrap();
                        let frame = pool.TryGetNextFrame()?;
                        s.send(frame).map_err(|_| Win32Error::from(E_FAIL))?;
                        Ok(())
                    }
                });
                frame_pool.FrameArrived(&handler)?;
                session.StartCapture()?;

                let frame = r.recv().map_err(|_| Win32Error::from(E_FAIL))?;
                let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
                let source: ID3D11Texture2D = access.GetInterface()?;

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                source.GetDesc(&mut desc);
                desc.BindFlags = 0;
                desc.MiscFlags = 0;
                desc.Usage = D3D11_USAGE_STAGING;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

                let mut texture = None;
                d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))?;
                let texture = texture.unwrap();

                context.CopyResource(Some(&texture.cast()?), Some(&source.cast()?));

                session.Close()?;
                frame_pool.Close()?;

                texture
            };

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);

            let resource: ID3D11Resource = texture.cast()?;
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();

            context.Map(Some(&resource), 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

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
    pub fn search(&self, ss: &ScreenShot) -> Result<Vec<(i32, i32, (u8, u8, u8))>, UError> {
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
                let color = (vec3b[0], vec3b[1], vec3b[2]);
                Ok((x, y, color))
            })
            .collect::<Result<Vec<_>, opencv::Error>>()?;
        Ok(points)
    }
}