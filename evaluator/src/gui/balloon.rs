use super::*;
use util::settings::USETTINGS;

use windows::Win32::Foundation::COLORREF;

use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

pub struct Balloon {
    hwnd: HWND,
    font: GdiObject<Gdi::HFONT>,
    /// 背景色
    back_color: COLORREF,
    /// 文字色
    fore_color: COLORREF,
    message: String,
    shape: Shape,
    transparency: Transparency,
    x: i32,
    y: i32,
}

impl Drop for Balloon {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl Balloon {
    const DEFAULT_FORE_COLOR: u32 = 0x000000;
    const DEFAULT_BACK_COLOR: u32 = 0x00FFFF;

    #[allow(clippy::too_many_arguments)]
    pub fn new(message: &str, x: i32, y: i32, font: Option<FontFamily>, fore_color: Option<u32>, back_color: Option<u32>, shape: u8, transparency: i16) -> UWindowResult<Self> {
        let hwnd = Self::create_window("UWSCR")?;
        let fore_color = COLORREF(fore_color.unwrap_or(Self::DEFAULT_FORE_COLOR));
        let back_color = COLORREF(back_color.unwrap_or(Self::DEFAULT_BACK_COLOR));
        let message = message.to_string();
        let shape = Shape::from(shape);
        let transparency = Transparency::from(transparency);
        let font = font.unwrap_or_default().create(hwnd)?;
        let balloon = Self { hwnd, font, back_color, fore_color, message, shape, transparency, x, y };

        balloon.draw()?;

        Ok(balloon)
    }

    fn new_solid_brush(color: COLORREF) -> GdiObject<Gdi::HBRUSH> {
        unsafe {
            GdiObject::new(Gdi::CreateSolidBrush(color))
        }
    }
    fn get_border_brush(&self) -> GdiObject<Gdi::HBRUSH> {
        let mut border = self.back_color;
        let COLORREF(color) = &mut border;
        if *color == u32::MAX {
            *color -= 1;
        } else {
            *color += 1;
        }
        Self::new_solid_brush(border)
    }
    fn new_rect_rgn(x2: i32, y2: i32) -> GdiObject<Gdi::HRGN> {
        unsafe {
            let hrgn = Gdi::CreateRectRgn(0, 0, x2, y2);
            GdiObject::new(hrgn)
        }
    }
    fn new_polygon_rgn(pptl: &[POINT; 6]) -> GdiObject<Gdi::HRGN> {
        unsafe {
            let hrgn = Gdi::CreatePolygonRgn(pptl, Gdi::ALTERNATE);
            GdiObject::new(hrgn)
        }
    }
    fn new_round_rect_rgn(x2: i32, y2: i32, w: i32, h: i32) -> GdiObject<Gdi::HRGN> {
        unsafe {
            let hrgn = Gdi::CreateRoundRectRgn(0, 0, x2, y2, w, h);
            GdiObject::new(hrgn)
        }
    }
    unsafe fn set_transparent(&self) {
        unsafe {
            let ex = Self::get_window_long(self.hwnd, wm::GWL_EXSTYLE);
            let dwnewlong = ex | wm::WS_EX_LAYERED.0 as isize;
            Self::set_window_long(self.hwnd, wm::GWL_EXSTYLE, dwnewlong);
            match self.transparency {
                Transparency::None => {},
                Transparency::Alpha(alpha) => {
                    let _ = wm::SetLayeredWindowAttributes(self.hwnd, COLORREF::default(), alpha, wm::LWA_ALPHA);
                },
                Transparency::NoBackground |
                Transparency::NoBackgroundAndBorder => {
                    let _ = wm::SetLayeredWindowAttributes(self.hwnd, self.back_color, 0, wm::LWA_COLORKEY);
                },
            }
        }
    }
    unsafe fn get_poly_points(&self, w_margin: &mut i32, h_margin: &mut i32) -> ([POINT; 6], POINT) {
        let rect = self.get_rect().unwrap_or_default();
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let mut top_left = POINT { x: 0, y: 0 };
        let mut top_right = POINT { x: w, y: 0 };
        let bottom_left = POINT { x: 0, y: h };
        let mut bottom_right = POINT { x: w, y: h };
        // 嘴の先の座標
        let mut beak_top = POINT { x: 0, y: 0 };
        // 嘴の根本左側の座標
        let mut beak_lbottom = POINT { x: 0, y: 0 };
        // 嘴の根本右側の座標
        let mut beak_rbottom = POINT { x: 0, y: 0 };
        let pptl = match self.shape {
            Shape::Upward(_) => {
                let beak_height = *h_margin;
                let beak_width = beak_height / 2;
                *h_margin += beak_height /2;

                top_right.y += beak_height;

                beak_top.x = beak_width;
                beak_top.y = 0;
                beak_lbottom.x = 0;
                beak_lbottom.y = beak_height;
                beak_rbottom.x = beak_width;
                beak_rbottom.y = beak_height;
                [beak_lbottom, beak_top, beak_rbottom, top_right, bottom_right, bottom_left]
            },
            Shape::Downward(_) => {
                let beak_height = *h_margin;
                let beak_width = beak_height / 2;
                *h_margin -= beak_height /2;

                bottom_right.y -= beak_height;

                beak_top.x = bottom_left.x + beak_width;
                beak_top.y = bottom_left.y;
                beak_lbottom.x = bottom_left.x + beak_width;
                beak_lbottom.y = bottom_left.y - beak_height;
                beak_rbottom.x = bottom_left.x;
                beak_rbottom.y = bottom_left.y - beak_height;
                [top_left, top_right, bottom_right, beak_lbottom, beak_top, beak_rbottom]
            },
            Shape::Leftward(_) => {
                let beak_height = *w_margin;
                let beak_width = beak_height / 2;
                *w_margin += beak_height / 2;

                top_left.x += beak_height;

                beak_top.x = bottom_left.x;
                beak_top.y = bottom_left.y - beak_width;
                beak_lbottom.x = bottom_left.x + beak_height;
                beak_lbottom.y = bottom_left.y;
                beak_rbottom.x = bottom_left.x + beak_height;
                beak_rbottom.y = bottom_left.y - beak_width;
                [top_left, top_right, bottom_right, beak_lbottom, beak_top, beak_rbottom]
            },
            Shape::Rightward(_) => {
                let beak_height = *w_margin;
                let beak_width = beak_height / 2;
                *w_margin -= beak_height / 2;

                top_right.x -= beak_height;

                beak_top.x = bottom_right.x;
                beak_top.y = bottom_right.y - beak_width;
                beak_lbottom.x = bottom_right.x - beak_height;
                beak_lbottom.y = bottom_right.y - beak_width;
                beak_rbottom.x = bottom_right.x - beak_height;
                beak_rbottom.y = bottom_right.y;
                [top_left, top_right, beak_lbottom, beak_top, beak_rbottom, bottom_left]
            },
            Shape::Default |
            Shape::Round => unreachable!(),
        };
        (pptl, beak_top)
    }
}

impl UWindow<()> for Balloon {
    const CLASS_NAME: PCWSTR = w!("UWSCR.Balloon");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_window_class(&REGISTER_CLASS)?;
        let style = {
            let s = USETTINGS.lock().unwrap();
            if s.options.fix_balloon {
                wm::WS_EX_TOOLWINDOW|wm::WS_EX_NOACTIVATE|wm::WS_EX_TOPMOST
            } else {
                wm::WS_EX_APPWINDOW|wm::WS_EX_TOOLWINDOW|wm::WS_EX_NOACTIVATE|wm::WS_EX_TOPMOST
            }
        };
        WindowBuilder::new(title, Self::CLASS_NAME)
            .style(WS_VISIBLE|wm::WS_POPUP)
            .ex_style(style)
            .build()
    }

    fn draw(&self) -> UWindowResult<()> {
        unsafe {
            let hfont = self.font();
            let size = self.get_text_size(self.hwnd, &self.message, hfont.as_object());
            let mut w_margin = ((size.cx as f64 * 0.05) as i32).max(10);
            let width = size.cx + w_margin * 2;
            let mut h_margin = ((size.cy as f64 * 0.1) as i32).max(15);
            let height = size.cy + h_margin * 2;
            self.move_to(self.x, self.y, width, height);

            let hdc = Gdi::GetDC(self.hwnd);

            // リージョンの作成
            let (hrgn, beak_point) = match self.shape {
                Shape::Default => {
                    let hrgn = Self::new_rect_rgn(width, height);
                    (hrgn, None)
                },
                Shape::Upward(b) |
                Shape::Downward(b) |
                Shape::Leftward(b) |
                Shape::Rightward(b) => {
                    let (pptl, point) = self.get_poly_points(&mut w_margin, &mut h_margin);
                    let hrgn = Self::new_polygon_rgn(&pptl);
                    (hrgn, b.then_some(point))
                },
                Shape::Round => {
                    let rect = self.get_rect().unwrap_or_default();
                    let x2 = rect.right - rect.left;
                    let y2 = rect.bottom - rect.top;
                    let l = (x2.max(y2) as f64 * 0.05) as i32;
                    let hrgn = Self::new_round_rect_rgn(x2, y2, l, l);
                    (hrgn, None)
                },
            };

            // 背景色
            let hbr = Self::new_solid_brush(self.back_color);
            Gdi::FillRgn(hdc, hrgn.as_handle(), hbr.as_handle());
            // 枠
            if self.transparency.border() {
                let hbr = self.get_border_brush();
                Gdi::FrameRgn(hdc, hrgn.as_handle(), hbr.as_handle(), 1, 1);
            }

            // 文字
            let old = Gdi::SelectObject(hdc, hfont.as_object());
            Gdi::SetBkMode(hdc, Gdi::TRANSPARENT);
            Gdi::SetTextColor(hdc, self.fore_color);
            let x = w_margin;
            let mut y = h_margin;
            for line in self.message.lines() {
                let size = self.get_text_size(self.hwnd, line, hfont.as_object());
                let hstring = HSTRING::from(line);
                Gdi::TabbedTextOutW(hdc, x, y, hstring.as_wide(), Some(&[]), x);
                y += size.cy;
            }
            Gdi::SelectObject(hdc, old);
            Gdi::ReleaseDC(self.hwnd, hdc);

            // 指定座標を嘴先にする
            if let Some(point) = beak_point {
                let x = self.x - point.x;
                let y = self.y - point.y;
                self.move_to(x, y, width, height);
            }

            // 透過
            self.set_transparent();
        }
        Ok(())
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn font(&self) -> &GdiObject<Gdi::HFONT> {
        &self.font
    }

    fn register_window_class(once: &OnceLock<UWindowResult<()>>) -> UWindowResult<()> {
        unsafe {
            once.get_or_init(|| {
                let hinstance = GetModuleHandleW(None)
                    .map(|hmod| hmod.into())?;
                let wc = wm::WNDCLASSEXW {
                    cbSize: std::mem::size_of::<wm::WNDCLASSEXW>() as u32,
                    // style: wm::CS_HREDRAW|wm::CS_VREDRAW,
                    lpfnWndProc: Some(Self::wndproc),
                    hInstance: hinstance,
                    // hIcon: wm::LoadIconW(hinstance, PCWSTR(1 as _))?,
                    lpszClassName: Self::CLASS_NAME,
                    hCursor: wm::LoadCursorW(None, wm::IDC_ARROW)?,
                    ..Default::default()
                };
                match wm::RegisterClassExW(&wc) {
                    0 => Err(UWindowError::ClassRegistrationError(Self::CLASS_NAME.to_string().unwrap_or_default())),
                    _ => Ok(())
                }
            }).clone()
        }
    }
}

enum Transparency {
    None,
    Alpha(u8),
    NoBackground,
    NoBackgroundAndBorder,
}
impl Transparency {
    fn border(&self) -> bool {
        match self {
            Transparency::None |
            Transparency::Alpha(_) |
            Transparency::NoBackground => true,
            Transparency::NoBackgroundAndBorder => false,
        }
    }
}
impl From<i16> for Transparency {
    fn from(value: i16) -> Self {
        match value {
            1..=255 => Self::Alpha(value as u8),
            -1 => Self::NoBackground,
            -2 => Self::NoBackgroundAndBorder,
            _ => Self::None
        }
    }
}

/// Balloonの形
///
/// 嘴方向のboolは
/// - true : 指定座標を嘴の先にする
/// - false: 指定座標を左上にする
#[derive(Debug)]
enum Shape {
    /// 通常
    Default,
    /// 1: 嘴上向き
    Upward(bool),
    /// 2: 嘴下向き
    Downward(bool),
    /// 3: 嘴左向き
    Leftward(bool),
    /// 4: 嘴右向き
    Rightward(bool),
    /// 9: 角丸
    Round,
}
impl From<u8> for Shape {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Upward(false),
            241 => Self::Upward(true),
            2 => Self::Downward(false),
            242 => Self::Downward(true),
            3 => Self::Leftward(false),
            243 => Self::Leftward(true),
            4 => Self::Rightward(false),
            244 => Self::Rightward(true),
            9 => Self::Round,
            _ => Self::Default,
        }
    }
}

impl GdiObjectImpl for Gdi::HBRUSH {
    fn as_inner(&self) -> isize {
        self.0
    }
}
impl GdiObjectImpl for Gdi::HRGN {
    fn as_inner(&self) -> isize {
        self.0
    }
}