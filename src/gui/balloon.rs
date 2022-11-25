use super::*;

static BALLOON_CLASS: OnceCell<UWindowResult<String>> = OnceCell::new();
static BALLOON_MARGIN: i32 = 10;

#[derive(Debug, Clone)]
pub struct Balloon {
    hwnd: HWND,
    bg_color: HBRUSH,
    font_color: COLORREF,
    text: String,
    hfont: HFONT,
    x: Option<i32>,
    y: Option<i32>,
}

impl Balloon {
    pub fn new(text: &str, x: Option<i32>, y: Option<i32>, font: Option<FontFamily>, font_color: Option<u32>, bg_color: Option<u32>) -> UWindowResult<Self> {
        let colorref = COLORREF(bg_color.unwrap_or(0x00FFFF));
        let bg_color = Window::create_solid_brush(colorref);
        let font_color = COLORREF(font_color.unwrap_or(0x000000));
        // let mut m = BALLOON.lock().unwrap();
        let class_name = Window::get_class_name("UWSCR.Balloon", &BALLOON_CLASS, Some(Self::wndproc))?;
        let hwnd = Self::create(text, &class_name)?;
        let hfont = font.unwrap_or_default().as_handle()?;
        let balloon = Self { hwnd, bg_color, font_color, text: text.to_string(), hfont, x, y };
        // balloon.draw(x, y);
        // balloon.show();
        Ok(balloon)
    }
    fn create(_text: &str, class_name: &str) -> UWindowResult<HWND> {
        Window::create_window(
            None,
            class_name,
            "",
            // WS_EX_TOPMOST|WS_EX_TOOLWINDOW,
            WS_EX_TOOLWINDOW,
            WS_VISIBLE|WS_POPUP|WS_BORDER,
            0,
            0,
            10,
            10,
            None
        )
    }
    pub fn redraw(&mut self, new: Balloon) {
        self.bg_color = new.bg_color;
        self.font_color = new.font_color;
        self.hfont = new.hfont;
        self.text = new.text.to_owned();
        self.x = new.x;
        self.y = new.y;
        self.draw();
    }
    pub fn draw(&self) {
        unsafe {
            let size = self.text.lines().map(|line| {
                Window::get_string_size(line, self.hwnd, Some(self.hfont))
            }).reduce(|s1, s2| {
                let cx = s1.cx.max(s2.cx);
                let cy = s1.cy + s2.cy;
                SIZE { cx, cy }
            }).unwrap();
            let x = self.x.unwrap_or(0);
            let y = self.y.unwrap_or(0);
            Window::move_window(self.hwnd, x, y, size.cx + BALLOON_MARGIN*2, size.cy + BALLOON_MARGIN*2);
            let mut ps = PAINTSTRUCT::default();
            let mut tm = TEXTMETRICW::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            // 背景色
            FillRect(hdc, &ps.rcPaint, self.bg_color);
            // 文字
            let obj = SelectObject(hdc, self.hfont);
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, self.font_color);
            GetTextMetricsW(hdc, &mut tm);
            let x = BALLOON_MARGIN;
            let mut y = BALLOON_MARGIN;
            for line in self.text.lines() {
                let size = Window::get_string_size(line, self.hwnd, Some(self.hfont));
                let lpstring = line.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
                // let lpstring = PCWSTR(wide.as_ptr());
                // let c = wide.len() as i32;
                TextOutW(hdc, x, y, &lpstring);
                y += size.cy;
            }
            SelectObject(hdc, obj);
            EndPaint(self.hwnd, &ps);
        }
    }
    pub fn close(&self) {
        unsafe { DestroyWindow(self.hwnd()) };
    }
}

impl Drop for Balloon {
    fn drop(&mut self) {
        self.close();
    }
}

impl UWindow<()> for Balloon {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(hwnd.0));
                LRESULT(0)
            },
            msg => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}