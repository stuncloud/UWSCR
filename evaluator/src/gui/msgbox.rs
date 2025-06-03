use super::*;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::HWND,
        UI::{
            WindowsAndMessaging::{
                WS_SYSMENU,
                WS_EX_TOPMOST,
                SW_NORMAL,
            },
            Controls::{
                NM_CLICK, NM_RETURN,
                NMHDR, NMLINK,
            },
            Shell::ShellExecuteW,
        },
        Graphics::Gdi,
    },

};
use std::ops::{BitOr, BitAnd};
use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();
pub struct MsgBox {
    hwnd: HWND,
    hfont: Gdi::HFONT,
    x: Option<i32>,
    y: Option<i32>,
    buttons: MsgBoxButton,
    defbtn: Option<MsgBoxButton>,
    message: String,
    enable_link: bool,
}
impl MsgBox {
    const BTN_MIN_WIDTH: i32 = 70;
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: &str,
        message: &str,
        x: Option<i32>,
        y: Option<i32>,
        buttons: MsgBoxButton,
        defbtn: Option<MsgBoxButton>,
        font: Option<FontFamily>,
        enable_link: bool
    ) -> UWindowResult<Self> {
        let hwnd = Self::create_window(title)?;
        let hfont = font.unwrap_or_default().create()?;
        let msgbox = Self { hwnd, hfont, x, y, buttons, defbtn, message: message.into(), enable_link };
        msgbox.draw()?;
        msgbox.activate();
        Ok(msgbox)
    }
    fn add_buttons(&self) -> UWindowResult<Vec<ChildCtl<Button>>> {
        let mut buttons = vec![];
        let defbtn = match self.defbtn {
            Some(btn) => if self.buttons.includes(&btn) {
                btn
            } else {
                self.buttons.first()
            },
            None => self.buttons.first()
        };
        for mbb in [BTN_YES,BTN_NO,BTN_OK,BTN_CANCEL,BTN_ABORT,BTN_RETRY,BTN_IGNORE] {
            let default = mbb == defbtn;
            if self.buttons.includes(&mbb) {
                let mut btn = self.set_button(&mbb.to_string(), 0, 0, mbb.id(), default, Self::BTN_MIN_WIDTH)?;
                if default {
                    btn.class.set_default();
                }
                buttons.push(btn);
            }
        }
        Ok(buttons)
    }
    fn open_url(url: &str) {
        unsafe {
            let lpfile = HSTRING::from(url);
            ShellExecuteW(None, w!("open"), &lpfile, None, None, SW_NORMAL);
        }
    }
}
impl UWindow<DialogResult<MsgBoxButton>> for MsgBox {
    const CLASS_NAME: PCWSTR = w!("UWSCR.MsgBox");

    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn font(&self) -> Gdi::HFONT {
        self.hfont
    }

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_dlg_class(&REGISTER_CLASS)?;
        let style = WS_SYSMENU;
        let ex_style = WS_EX_TOPMOST;
        WindowBuilder::new(title, Self::CLASS_NAME)
            .style(style)
            .ex_style(ex_style)
            .build()
    }

    fn draw(&self) -> UWindowResult<()> {
        let margin = 16;
        let btn_gap = 5;
        let label_btn_gap = 10;

        let label = if self.enable_link {
            self.set_static_with_link(&self.message, margin, margin)?
        } else {
            self.set_static(&self.message, margin, margin)?
        };
        let mut btns = self.add_buttons()?;
        let btns_width = btns.iter().map(|b| b.width()).reduce(|a, b| a + b + btn_gap).unwrap();
        let btns_height = btns[0].height();

        let width = label.width().max(btns_width) + margin * 2;
        let height = label.height() + label_btn_gap + btns_height + margin * 2 + self.title_bar_height();
        let (x, y) = if self.x.is_none() || self.y.is_none() {
            let center = Self::get_center_pos(width, height);
            let x = self.x.unwrap_or(center.x);
            let y = self.y.unwrap_or(center.y);
            (x, y)
        } else {
            (self.x.unwrap(), self.y.unwrap())
        };
        self.move_to(x, y, width+ margin, height+ label_btn_gap);
        let (cw, _) = self.get_client_wh();

        let btn_y = margin + label.height() + label_btn_gap;
        let mut btn_x = if label.width() > btns_width {
            cw / 2 - btns_width / 2
        } else {
            margin
        };
        for btn in &mut btns {
            btn.move_to(btn_x, btn_y, None, None);
            if btn.class.is_default() {
                btn.focus();
            }
            btn_x += btn.width() + btn_gap;
        }
        self.show();
        Ok(())
    }

    fn message_loop(&self) -> UWindowResult<DialogResult<MsgBoxButton>> {
        unsafe {
            let mut msg = wm::MSG::default();
            let hwnd = HWND::default();
            let point = self.get_pos().unwrap_or_default();

            loop {
                let point = self.get_pos().unwrap_or(point);
                match wm::GetMessageW(&mut msg, hwnd, 0, 0).0 {
                    -1 => {
                        break Err(UWindowError::Win32(core::Error::from_win32()));
                    },
                    0 => {
                        if msg.hwnd == self.hwnd {
                            let res = DialogResult { result: MsgBoxButton::default(), point };
                            break Ok(res);
                        }
                    },
                    _ => {
                        match msg.message {
                            wm::WM_NOTIFY => {
                                let nmhdr = *(msg.lParam.0 as *const NMHDR);
                                match nmhdr.code {
                                    NM_CLICK |
                                    NM_RETURN => {
                                        let nmlink = *(msg.lParam.0 as *const NMLINK);
                                        if let Ok(url) = PCWSTR::from_raw(nmlink.item.szUrl.as_ptr()).to_string() {
                                            Self::open_url(&url);
                                        }
                                    },
                                    _ => {}
                                }
                            },
                            wm::WM_COMMAND => {
                                if msg.wParam.hi_word() as u32 == wm::BN_CLICKED {
                                    let id = msg.wParam.lo_word() as i32;
                                    let res = DialogResult { result: MsgBoxButton(id), point };
                                    self.destroy();
                                    break Ok(res);
                                }
                            }
                            _ => {},
                        }
                    }
                };
                if ! wm::IsDialogMessageW(self.hwnd(), &msg).as_bool() {
                    wm::TranslateMessage(&msg);
                    wm::DispatchMessageW(&msg);
                }
            }
        }
    }
    unsafe extern "system"
    fn dlgproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                wm::WM_COMMAND |
                wm::WM_NOTIFY => {
                    let _ = wm::PostMessageW(HWND(0), msg, wparam, lparam);
                    LRESULT(0)
                },
                wm::WM_CLOSE => {
                    // let _ = wm::DestroyWindow(hwnd);
                    let _ = wm::PostMessageW(HWND(0), wm::WM_COMMAND, WPARAM(BTN_CANCEL.0 as usize), None);
                    LRESULT(0)
                },
                msg => wm::DefDlgProcW(hwnd, msg, wparam, lparam)
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
    fn includes(&self, btn: &MsgBoxButton) -> bool {
        (self.0 & btn.0) == btn.0
    }
    fn id(&self) -> isize {
        self.0 as isize
    }
    fn first(&self) -> Self {
        [BTN_YES,BTN_NO,BTN_OK,BTN_CANCEL,BTN_ABORT,BTN_RETRY,BTN_IGNORE].into_iter()
            .find(|btn| self.includes(btn))
            .unwrap_or(Self(0))
    }
}

impl Default for MsgBoxButton {
    fn default() -> Self {
        BTN_CANCEL
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
        match *self {
            BTN_YES => write!(f, "はい(&Y)"),
            BTN_NO => write!(f, "いいえ(&N)"),
            BTN_OK => write!(f, "&OK"),
            BTN_CANCEL => write!(f, "キャンセル(&C)"),
            BTN_ABORT => write!(f, "中止(&A)"),
            BTN_RETRY => write!(f, "再試行(&R)"),
            BTN_IGNORE => write!(f, "無視(&I)"),
            _ => write!(f, "にゃーん")
        }
    }
}