use std::{ffi::c_void, io::Read};
use std::fs::File;

use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Gdi::{
                ROP_CODE, SRCCOPY, CAPTUREBLT, DIB_RGB_COLORS,
                BITMAPINFO, BITMAPINFOHEADER,
                GetDC, ReleaseDC, DeleteDC, SelectObject, DeleteObject, GetDIBits,
                StretchBlt,
                CreateCompatibleDC, CreateCompatibleBitmap,
            }
        },
        UI::{
            WindowsAndMessaging::{
                SM_CYVIRTUALSCREEN, SM_CXVIRTUALSCREEN,
                SM_YVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                GetSystemMetrics,
            },
            // HiDpi::{
            //     GetThreadDpiAwarenessContext,
            //     SetThreadDpiAwarenessContext,
            //     DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
            //     DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            //     DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
            //     DPI_AWARENESS_CONTEXT_UNAWARE,
            //     DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED,
            // }
        },
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
    pub fn get(hwnd: Option<HWND>, left: Option<i32>, top: Option<i32>, right: Option<i32>, bottom: Option<i32>) -> ScreenShotResult {
        unsafe {
            // let context = GetThreadDpiAwarenessContext();
            // let c  = match context {
            //     DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE => "DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE",
            //     DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 => "DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2",
            //     DPI_AWARENESS_CONTEXT_SYSTEM_AWARE => "DPI_AWARENESS_CONTEXT_SYSTEM_AWARE",
            //     DPI_AWARENESS_CONTEXT_UNAWARE => "DPI_AWARENESS_CONTEXT_UNAWARE",
            //     DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED => "DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED",
            //     _ => "",
            // };
            // println!("\u{001b}[33m[debug] context: {c}\u{001b}[0m");
            // SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            // キャプチャ範囲を確定
            let left = left.unwrap_or(GetSystemMetrics(SM_XVIRTUALSCREEN));
            let top = top.unwrap_or(GetSystemMetrics(SM_YVIRTUALSCREEN));
            let mut width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            // SetThreadDpiAwarenessContext(context);
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

            // ディスプレイ全体のHDCを取得
            let hdc = GetDC(hwnd);
            let hdc_compat = CreateCompatibleDC(hdc);
            if hdc_compat.is_invalid() {
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
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0)
            );
            if ! res.as_bool() {
                return Err(UError::new(
                    UErrorKind::ScreenShotError,
                    UErrorMessage::GdiError("BitBlt".into())
                ));
            }

            let mut mat = Mat::new_rows_cols(height, width, core::CV_8UC4)?;
            let pmat = mat.data_mut() as *mut c_void;
            GetDIBits(
                hdc_compat,
                hbmp,
                0,
                height as u32,
                Some(pmat),
                &mut info,
                DIB_RGB_COLORS
            );

            // convert to gray image
            let mut data = Mat::default();
            imgproc::cvt_color(&mat, &mut data, imgproc::COLOR_RGB2GRAY, 0)?;

            // cleanup
            ReleaseDC(None, hdc);
            DeleteDC(hdc_compat);
            DeleteObject(hbmp);

            Ok(ScreenShot {data, left, top, width, height})
        }
    }
    pub fn save(&self, filename: Option<&str>) -> ChkImgResult<()> {
        let vector = core::Vector::new();
        let default = format!("chkimg_ss_{}_{}.png", self.width, self.height);
        let filename = filename.unwrap_or(&default);
        imgcodecs::imwrite(filename, &self.data, &vector)?;
        Ok(())
    }
}
