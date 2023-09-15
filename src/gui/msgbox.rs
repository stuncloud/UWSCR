use super::{Window, UWindow, Child, UWindowResult, UWindowError, FontFamily, WparamExt};
use crate::write_locale;
use crate::error::{CURRENT_LOCALE, Locale};

use windows::Win32::{
        Foundation::{
            HWND,WPARAM,LPARAM,LRESULT,
            SIZE, RECT,
        },
        UI::{
            WindowsAndMessaging::{
                MSG,
                WM_DESTROY, WM_COMMAND, WM_KEYDOWN, WM_KEYUP, WM_QUIT,
                BM_CLICK,
                WS_OVERLAPPED, WS_SYSMENU,
                WS_EX_TOPMOST,
                BN_CLICKED,
                KF_REPEAT,
                SM_CXSCREEN, SM_CYSCREEN,
                DestroyWindow,
                DefWindowProcW,
                SendMessageW, GetMessageW, TranslateMessage, DispatchMessageW, PostMessageW,
                GetSystemMetrics,
            },
            Input::KeyboardAndMouse::{
                VIRTUAL_KEY, VK_TAB, VK_ESCAPE, VK_RETURN, VK_SHIFT, VK_RIGHT, VK_LEFT,
                SetFocus,
            },
        },
    };
use std::{ops::{Add, BitOr, BitAnd}, fmt::Display};
use once_cell::sync::OnceCell;

static MSGBOX_CLASS: OnceCell<Result<String, UWindowError>> = OnceCell::new();

#[derive(Debug)]
pub struct Msgbox {
    hwnd: HWND,
    buttons: Vec<Child>,
}
impl Display for Msgbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}, {} buttons", self.hwnd, self.buttons.len())
    }
}

static WIDTH: i32 = 192;
static HEIGHT: i32 = 108;
static MAX_WIDTH: i32 = 1980;
static MAX_HEIGHT: i32 = 1080;
static MARGIN_X: i32 = 16;
static MARGIN_Y: i32 = 16;

impl Msgbox {
    pub fn new(title: &str, message: &str, btn_type: MsgBoxButton, font: Option<FontFamily>, selected: Option<MsgBoxButton>, x: Option<i32>, y: Option<i32>) -> UWindowResult<Self> {
        let hwnd = Self::create(title)?;
        let hfont = font.unwrap_or_default().as_handle()?;
        let mut height = MARGIN_Y * 2;
        let (pad_x, pad_y) = Window::get_window_margin(hwnd);

        // メッセージ表示部分
        let label = Window::set_label(hwnd, message, MARGIN_X, MARGIN_Y, Some(hfont), None)?;
        height += label.size.cy;

        // ボタン類
        let btop = label.size.cy + MARGIN_Y*2;

        let mut buttons = vec![];
        // let mut bwidth = MARGIN_X * 2;
        let mut bheight = 0;
        let mut bleft = 0;
        let panel = Window::create_panel(hwnd, None, Some(Self::subclass), Some(200))?;

        let btn_type = btn_type.is_zero_then_default();
        let mut focus = false;
        for btn in [BTN_YES, BTN_NO, BTN_OK, BTN_CANCEL, BTN_ABORT, BTN_RETRY, BTN_IGNORE] {
            if btn_type.includes(btn) {
                let button = Window::set_button(panel, &btn.to_string(), bleft, 0, btn.0, None, None)?;
                match selected {
                    Some(b) => if b.includes(btn) {
                        Window::focus(button.hwnd);
                        focus = true;
                    }
                    None => {}
                }
                bheight = bheight.max(button.size.cy);
                // bwidth += button.size.cx;
                bleft += button.size.cx + MARGIN_X;
                buttons.push(button);
            }
        }
        if ! focus {
            Window::focus(buttons[0].hwnd);
        }
        let bwidth = bleft;
        height += bheight + pad_y + MARGIN_Y;

        let width = Self::calculate_width(label.size.cx, bwidth, pad_x);
        let size = SIZE { cx: width, cy: height.max(HEIGHT).min(MAX_HEIGHT) };

        if x.is_none() | y.is_none() {
            let (center_x, center_y) = Self::calculate_pos(size);
            let x = x.unwrap_or(center_x);
            let y = y.unwrap_or(center_y);
            Window::set_window_pos(hwnd, x, y, size, None);
        } else {
            Window::move_window(hwnd, x.unwrap(), y.unwrap(), width, height);
        };
        let bleft = Window::calculate_center(width, bwidth);
        Window::move_window(panel, bleft, btop, bwidth, bheight);
        let msgbox = Self {
            hwnd, buttons
        };
        Ok(msgbox)
    }
    fn create(title: &str) -> UWindowResult<HWND> {
        let class_name = Window::get_class_name("UWSCR.MsgBox", &MSGBOX_CLASS, Some(Self::wndproc))?;
        Window::create_window(
            None,
            &class_name,
            title,
            WS_EX_TOPMOST,
            WS_OVERLAPPED|WS_SYSMENU,
            0, 0, WIDTH, HEIGHT,
            None
        )
    }
    fn calculate_width(panel_width: i32, buttons_width: i32, padding: i32) -> i32 {
        let mut new_width = WIDTH;
        // クライアント領域の外側の幅
        let tx = MARGIN_X * 2 + padding;

        // パネルサイズ
        new_width = new_width.max(panel_width + tx);
        // ボタン類のサイズ
        new_width = new_width.max(buttons_width + tx);

        // サイズ上限を越えないようにする
        new_width = new_width.min(MAX_WIDTH);
        new_width
    }
    fn calculate_pos(size: SIZE) -> (i32, i32) {
        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let x = screen_w / 2 - size.cx / 2;
            let y = screen_h / 2 - size.cy / 2;
            (x, y)
        }
    }

    pub fn move_btn_focus(&self, current: HWND, shift: bool) {
        unsafe {
            if let Some(pos) =  self.buttons.iter().position(|b|b.hwnd == current) {
                let new_pos = if shift {pos as isize -1} else {pos as isize +1};
                let i = if new_pos < 0 {
                    self.buttons.len() - 1
                } else if new_pos >= self.buttons.len() as isize {
                    0
                } else {
                    new_pos as usize
                };
                SetFocus(self.buttons[i].hwnd);
            }
        }
    }
}
type MsgBoxResult = (MsgBoxButton, i32, i32);
impl UWindow<MsgBoxResult> for Msgbox {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn message_loop(&self) -> UWindowResult<MsgBoxResult> {
        unsafe {
            let mut msg = MSG::default();
            let mut shift_flg = false;
            let mut rect = RECT::default();
            let clicked = loop {
                if GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                    rect = Window::get_window_rect(self.hwnd);
                    match msg.message {
                        WM_COMMAND => match msg.wParam.hi_word() as u32{
                            BN_CLICKED => {
                                let id = msg.wParam.lo_word() as i32;
                                // SendMessageW(self.hwnd, WM_CLOSE, WPARAM(1), LPARAM(0));
                                break MsgBoxButton(id);
                            }
                            _ => {}
                        },
                        WM_KEYDOWN => {
                            let key = msg.wParam.0 as u16;
                            match VIRTUAL_KEY(key) {
                                VK_ESCAPE => break BTN_CANCEL,
                                VK_RETURN => {
                                    SendMessageW(msg.hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
                                },
                                VK_TAB |
                                VK_RIGHT |
                                VK_LEFT => if (msg.lParam.0 as u32 & KF_REPEAT * 0x10000) > 0 {
                                    // 繰り返しフラグが立ってたらフォーカス移動
                                    let flg = match VIRTUAL_KEY(key) {
                                        VK_RIGHT => false,
                                        VK_LEFT => true,
                                        _ => shift_flg
                                    };
                                    self.move_btn_focus(msg.hwnd, flg);
                                },
                                VK_SHIFT => shift_flg = true,
                                _ => {}
                            }
                        },
                        WM_KEYUP => {
                            let key = msg.wParam.0 as u16;
                            match VIRTUAL_KEY(key) {
                                VK_TAB => self.move_btn_focus(msg.hwnd, shift_flg),
                                VK_RIGHT => self.move_btn_focus(msg.hwnd, false),
                                VK_LEFT => self.move_btn_focus(msg.hwnd, true),
                                VK_SHIFT => shift_flg = false,
                                _ => {}
                            }
                        },
                        _ => {}
                    };
                } else {
                    // WM_QUITを送ってきたのが自身ならループを抜ける
                    if self.hwnd.0 == msg.lParam.0 {
                        break BTN_CANCEL;
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };
            let _ = DestroyWindow(self.hwnd);
            // UnregisterClassW(MSGBOX_CLASS.to_string(), HINSTANCE::default());
            Ok((clicked, rect.left, rect.top))
        }
    }
    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                // LPARAMでhwndも伝える
                let _ = PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(hwnd.0));
                LRESULT(0)
            },
            WM_COMMAND => {
                let _ = PostMessageW(HWND(0), umsg, wparam, lparam);
                LRESULT(0)
            },
            msg => {
                // println!("[debug] hwnd: {:?}, umsg: {}, wparam: {:?}, lparam: {:?}", &hwnd, &umsg, &wparam, &lparam);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct MsgBoxButton(pub i32);
pub const BTN_YES   : MsgBoxButton = MsgBoxButton(4);
pub const BTN_NO    : MsgBoxButton = MsgBoxButton(8);
pub const BTN_OK    : MsgBoxButton = MsgBoxButton(1);
pub const BTN_CANCEL: MsgBoxButton = MsgBoxButton(2);
pub const BTN_ABORT : MsgBoxButton = MsgBoxButton(16);
pub const BTN_RETRY : MsgBoxButton = MsgBoxButton(32);
pub const BTN_IGNORE: MsgBoxButton = MsgBoxButton(64);

impl MsgBoxButton {
    fn is_zero_then_default(self) -> Self {
        if self.0 == 0 {
            BTN_OK
        } else {
            self
        }
    }
    fn includes(&self, btn: MsgBoxButton) -> bool {
        (*self & btn) == btn
    }
}

impl Default for MsgBoxButton {
    fn default() -> Self {
        BTN_OK
    }
}

impl From<MsgBoxButton> for i32 {
    fn from(b: MsgBoxButton) -> Self {
        b.0
    }
}
impl From<i32> for MsgBoxButton {
    fn from(n: i32) -> Self {
        MsgBoxButton(n)
    }
}

impl Add for MsgBoxButton {
    type Output = MsgBoxButton;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl BitOr for MsgBoxButton {
    type Output = MsgBoxButton;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for MsgBoxButton {
    type Output = MsgBoxButton;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::fmt::Display for MsgBoxButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &BTN_YES => write_locale!(f, "はい(&Y)", "&Yes"),
            &BTN_NO => write_locale!(f, "いいえ(&N)", "&No"),
            &BTN_OK => write!(f, "OK"),
            &BTN_CANCEL => write_locale!(f, "キャンセル(&C)", "&Cancel"),
            &BTN_ABORT => write_locale!(f, "中止(&A)", "&Abort"),
            &BTN_RETRY => write_locale!(f, "再試行(&R)", "&Retry"),
            &BTN_IGNORE => write_locale!(f, "無視(&I)", "&Ignore"),
            _ => write!(f, "にゃーん")
        }
    }
}