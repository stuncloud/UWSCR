use std::{ffi::c_void, io::Read};
use std::fs::File;

use crate::error::{UError, UErrorKind, UErrorMessage};
use super::window_control::ImgConst;
use super::clipboard::Clipboard;

use opencv::prelude::MatTraitConstManual;
use windows::Win32::{
        Foundation::{HWND, RECT, POINT},
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
            },
            Dwm::{
                DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS,
            }
        },
        UI::WindowsAndMessaging::{
                SM_CYVIRTUALSCREEN, SM_CXVIRTUALSCREEN,
                SM_YVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                GetSystemMetrics,
                GetClientRect, GetWindowRect,
                GetWindow, GW_HWNDPREV,
                IsWindowVisible,
            },
    };

use opencv::{
    core::{self, Mat, MatTrait, MatTraitConst, MatExprTraitConst, Vector},
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
    offset_y: i32
}
impl ChkImg {
    pub fn from_screenshot(ss: ScreenShot) -> ChkImgResult<Self> {
        let size = ss.data.mat_size();
        Ok(Self {
            image: ss.data,
            width: *size.get(0).unwrap(),
            height: *size.get(1).unwrap(),
            offset_x: ss.left,
            offset_y: ss.top,
        })
    }
    pub fn _from_file(path: &str) -> ChkImgResult<Self> {
        let image = imgcodecs::imread(path, imgcodecs::IMREAD_GRAYSCALE)?;
        let size = image.mat_size();
        Ok(Self {
            image,
            width: *size.get(0).unwrap(),
            height: *size.get(1).unwrap(),
            offset_x: 0,
            offset_y: 0
        })
    }
    pub fn search(&self, path: &str, score: f64, max_count: Option<u8>) -> ChkImgResult<MatchedPoints> {
        let buf= {
            let mut buf = vec![];
            let mut f = File::open(path)?;
            f.read_to_end(&mut buf)?;
            Vector::from_slice(buf.as_slice())
        };
        let templ = imgcodecs::imdecode(&buf, imgcodecs::IMREAD_GRAYSCALE)?;
        let templ_width = *templ.mat_size().get(0)
            .ok_or(UError::new(UErrorKind::OpenCvError, UErrorMessage::FailedToLoadImageFile(path.into())))?;
        let templ_height = *templ.mat_size().get(1)
            .ok_or(UError::new(UErrorKind::OpenCvError, UErrorMessage::FailedToLoadImageFile(path.into())))?;

        // マッチング
        let mut result = Mat::default();
        imgproc::match_template(&self.image, &templ, &mut result, imgproc::TM_CCOEFF_NORMED, &core::no_array())?;

        // 検索範囲のマスク
        let rows = self.width - templ_width + 1;
        let cols = self.height - templ_height + 1;
        let mut mask = core::Mat::ones(rows, cols, core::CV_8UC1)?.to_mat()?;

        // 戻り値
        let mut matches = vec![];

        let counter = max_count.unwrap_or(10);
        for _ in 0..counter {
            // スコア
            let mut max_val = 0.0;
            // 座標
            let mut max_loc = core::Point::default();
            core::min_max_loc(
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

        let mut data = Mat::new_rows_cols(height, width, core::CV_8UC4)?;
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
    unsafe fn new_window(hwnd: Option<&HWND>, left: i32, top: i32, width: i32, height: i32, dx: i32, dy: i32) -> ScreenShotResult {
        let mut ss = Self::new(hwnd, left, top, width, height)?;
        ss.left = dx;
        ss.top = dy;
        Ok(ss)
    }
    pub fn to_gray(&mut self) -> Result<(), UError>{
        let mut data = Mat::default();
        imgproc::cvt_color(&self.data, &mut data, imgproc::COLOR_RGB2GRAY, 0)?;
        self.data = data;
        Ok(())
    }
    pub fn get_screen(left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>) -> ScreenShotResult {
        unsafe {
            // キャプチャ範囲を確定
            let left = left.unwrap_or(GetSystemMetrics(SM_XVIRTUALSCREEN));
            let top = top.unwrap_or(GetSystemMetrics(SM_YVIRTUALSCREEN));
            let mut width = GetSystemMetrics(SM_CXVIRTUALSCREEN);

            if right.is_some() {
                width = right.unwrap() - left;
            } else {
                width -= left;
            }
            let mut height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if bottom.is_some() {
                height = bottom.unwrap() - top;
            } else {
                height -= left
            }

            let mut ss = Self::new(None, left, top, width, height)?;
            ss.to_gray()?;
            Ok(ss)
        }
    }
    pub fn get_screen2(left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>) -> ScreenShotResult {
        unsafe {
            // キャプチャ範囲を確定
            let left = left.unwrap_or(GetSystemMetrics(SM_XVIRTUALSCREEN));
            let top = top.unwrap_or(GetSystemMetrics(SM_YVIRTUALSCREEN));
            let width = width.unwrap_or(GetSystemMetrics(SM_CXVIRTUALSCREEN));
            let height = height.unwrap_or(GetSystemMetrics(SM_CYVIRTUALSCREEN));

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
    pub fn get_window(hwnd: HWND, left: Option<i32>, top: Option<i32>, width: Option<i32>, height: Option<i32>, client: bool, style: ImgConst) -> ScreenShotResult {
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
            Self::new_window(hwnd, left, top, width, height, dx, dy)
        }
    }
    pub fn save(&self, filename: Option<&str>) -> ChkImgResult<()> {
        let vector = core::Vector::new();
        let default = format!("chkimg_ss_{}_{}.png", self.width, self.height);
        let filename = filename.unwrap_or(&default);
        imgcodecs::imwrite(filename, &self.data, &vector)?;
        Ok(())
    }
    pub fn save_to(&self, filename: &str, jpg_quality: Option<i32>, png_compression: Option<i32>) -> ChkImgResult<()> {
        let mut params = core::Vector::new();
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
}
