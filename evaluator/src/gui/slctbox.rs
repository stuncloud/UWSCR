use super::*;

use windows::{
    core::HSTRING,
    Win32::{
        UI::{
            WindowsAndMessaging::{
                self as wm,
                WS_SYSMENU, WS_EX_TOPMOST, WS_VSCROLL,
            },
            Controls::{
                PROGRESS_CLASSW, WC_BUTTONW, WC_COMBOBOXW, WC_LISTBOXW,
                PBM_SETRANGE32, PBM_SETPOS, BST_CHECKED
            }
        },
        Graphics::Gdi,
    }
};

use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

#[derive(Debug)]
pub struct Slctbox {
    hwnd: HWND,
    message: Option<String>,
    hfont: Gdi::HFONT,
    ctl_type: SlctCtlType,
    ret_type: SlctReturnType,
    progress: Option<f64>,
    items: Vec<String>,
    x: Option<i32>,
    y: Option<i32>,
}

impl Slctbox {
    const ITEM_MIN_WIDTH: i32 = 250;
    const OK_WIDTH: i32 = 200;
    const CTL_GAP: i32 = 10;
    const BUTTON_GAP: i32 = 5;
    const MAX_WINDOW_HEIGHT: i32 = 1000;
    const ID_OK: i32 = -1;
    const ID_CANCEL: i32 = 2;
    const ID_LISTBOX: i32 = -13;
    const ID_COMBOBOX: i32 = -13;
    const ID_PROGRESS: i32 = -14;
    const CTL_ID_FIRST: usize = 100;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: &str,
        message: Option<String>,
        r#type: SlctType,
        items: Vec<String>,
        progress: Option<f64>,
        font: Option<FontFamily>,
        x: Option<i32>,
        y: Option<i32>,
    ) -> UWindowResult<Self> {
        // 表示アイテム数のチェック
        let ret_type = SlctReturnType::from(&r#type);
        // 戻り値がデフォルトの場合アイテム数は31が限界
        let too_many_items = ret_type == SlctReturnType::Default && items.len() > 31;
        (! too_many_items)
            .then_some(())
            .ok_or(UWindowError::SlctboxGotTooManyItems)?;

        let hwnd = Self::create_window(title)?;
        let hfont = font.unwrap_or_default().create()?;

        let slctbox = Self {
            hwnd,
            message,
            hfont,
            ctl_type: (&r#type).into(),
            ret_type,
            progress,
            items,
            x, y,
        };

        slctbox.draw()?;
        slctbox.activate();
        Ok(slctbox)
    }
    fn set_procress_bar(&self, x: i32, y: i32) -> UWindowResult<ChildCtl<ProgressBar>> {
        let hwnd = WindowBuilder::new("", PROGRESS_CLASSW)
            .style(WS_CHILD|WS_VISIBLE)
            .menu(Self::ID_PROGRESS as isize)
            .size(Some(x), Some(y), Some(10), Some(10))
            .parent(self.hwnd)
            .build()?;
        let child = ChildCtl::new(hwnd, Some(Self::ID_PROGRESS as isize), self.hwnd, ProgressBar);
        Ok(child)
    }
    fn set_buttons(&self) -> UWindowResult<Vec<ChildCtl<Button>>> {
        self.items.iter().enumerate()
            .map(|(index, title)| {
                let id = (index + Self::CTL_ID_FIRST) as isize;
                self.set_button(title, 0, 0, id, false, Self::ITEM_MIN_WIDTH)
            })
            .collect()
    }
    fn set_checkbox(&self, title: &str, x: i32, y: i32, id: isize, min_width: i32) -> UWindowResult<ChildCtl<CheckBox>> {
        let btn_style = WINDOW_STYLE(wm::BS_AUTOCHECKBOX as u32);
        let hwnd = WindowBuilder::new(title, WC_BUTTONW)
            .style(WS_CHILD|WS_VISIBLE|WS_TABSTOP|btn_style)
            .parent(self.hwnd)
            .menu(id)
            .build()?;
        let size = self.set_font(hwnd, title);
        let mut child = ChildCtl::new(hwnd, Some(id), self.hwnd(), CheckBox);
        let nwidth = min_width.max(size.cx + 40);
        let nheight = size.cy + 4;
        child.move_to(x, y, Some(nwidth), Some(nheight));
        Ok(child)
    }
    fn set_checkboxes(&self) -> UWindowResult<Vec<ChildCtl<CheckBox>>> {
        self.items.iter().enumerate()
            .map(|(index, title)| {
                let id = (index + Self::CTL_ID_FIRST) as isize;
                self.set_checkbox(title, 0, 0, id, Self::ITEM_MIN_WIDTH)
            })
            .collect()
    }
    fn set_radiobutton(&self, title: &str, id: isize) -> UWindowResult<ChildCtl<RadioButton>> {
        let style = WINDOW_STYLE(wm::BS_AUTORADIOBUTTON as u32);
        let hwnd = WindowBuilder::new(title, WC_BUTTONW)
            .style(WS_CHILD|WS_VISIBLE|WS_TABSTOP|style)
            .parent(self.hwnd)
            .menu(id)
            .build()?;
        let size = self.set_font(hwnd, title);
        let mut child = ChildCtl::new(hwnd, Some(id), self.hwnd, RadioButton);
        let width = Self::ITEM_MIN_WIDTH.max(size.cx + 40);
        let height = size.cy + 4;
        child.move_to(0, 0, Some(width), Some(height));
        Ok(child)
    }
    fn set_radiobuttons(&self) -> UWindowResult<Vec<ChildCtl<RadioButton>>> {
        self.items.iter().enumerate()
            .map(|(index, title)| {
                let id = (index + Self::CTL_ID_FIRST) as isize;
                self.set_radiobutton(title, id)
            })
            .collect()
    }
    fn set_combobox(&self) -> UWindowResult<ChildCtl<ComboBox>> {
        let style = WINDOW_STYLE((wm::CBS_DROPDOWNLIST|wm::CBS_AUTOHSCROLL) as u32);
        let hwnd = WindowBuilder::new("", WC_COMBOBOXW)
            .style(WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_VSCROLL|style)
            .parent(self.hwnd)
            .menu(Self::ID_COMBOBOX as isize)
            .build()?;
        unsafe {
            self.items.iter().for_each(|item| {
                let hstring = HSTRING::from(item);
                let lparam = LPARAM(hstring.as_ptr() as isize);
                wm::SendMessageW(hwnd, wm::CB_ADDSTRING, None, lparam);
            });
            wm::SendMessageW(hwnd, wm::CB_SETCURSEL, WPARAM(0), None);
            let longest = self.items.iter()
                .reduce(|a,b| if a.len() > b.len() {a} else {b})
                .unwrap();
            let size = self.set_font(hwnd, longest);
            let mut child = ChildCtl::new(hwnd, Some(Self::ID_COMBOBOX as isize), self.hwnd, ComboBox);
            let width = Self::ITEM_MIN_WIDTH.max(size.cx + 40);
            let height = size.cy + 4;
            child.move_to(0, 0, Some(width), Some(height));
            Ok(child)
        }
    }
    fn set_listbox(&self) -> UWindowResult<ChildCtl<ListBox>> {
        let style = WINDOW_STYLE((wm::LBS_NOTIFY|wm::LBS_MULTIPLESEL) as u32);
        let hwnd = WindowBuilder::new("", WC_LISTBOXW)
            .style(WS_CHILD|WS_VISIBLE|wm::WS_BORDER|WS_VSCROLL|style)
            .parent(self.hwnd)
            .menu(Self::ID_LISTBOX as isize)
            .build()?;
        unsafe {
            self.items.iter().for_each(|item| {
                let hstring = HSTRING::from(item);
                let lparam = LPARAM(hstring.as_ptr() as isize);
                wm::SendMessageW(hwnd, wm::LB_ADDSTRING, None, lparam);
            });
            let longest = self.items.iter()
                .reduce(|a,b| if a.len() > b.len() {a} else {b})
                .unwrap();
            let size = self.set_font(hwnd, longest);
            let mut child = ChildCtl::new(hwnd, Some(Self::ID_LISTBOX as isize), self.hwnd, ListBox);
            let width = Self::ITEM_MIN_WIDTH.max(size.cx + 40);
            let height = (self.items.len() as i32 + 1) * size.cy;
            child.move_to(0, 0, Some(width), Some(height));
            Ok(child)
        }
    }

    fn index_as_return_value(&self, index: i32) -> UWindowResult<SlctReturnValue> {
        match self.ret_type {
            SlctReturnType::String => {
                self.items.get(index as usize)
                    .map(|s| SlctReturnValue::String(s.clone()))
                    .ok_or(UWindowError::SlctItemOutOfBounds)
            },
            SlctReturnType::Index => Ok(SlctReturnValue::Index(index)),
            SlctReturnType::Default => {
                let (c, failed) = 2i32.overflowing_pow(index as u32);
                (!failed).then_some(SlctReturnValue::Const(c))
                    .ok_or(UWindowError::SlctItemOutOfBounds)
            },
        }
    }
    fn index_vec_as_return_value(&self, indexes: Vec<i32>) -> UWindowResult<SlctReturnValue> {
        let multi = indexes.into_iter()
            .map(|index| self.index_as_return_value(index))
            .collect::<UWindowResult<Vec<_>>>()?;
        if self.ret_type == SlctReturnType::Default {
            let c = multi.into_iter()
                .map(|r| if let SlctReturnValue::Const(c) = r {c} else {0})
                .reduce(|a, b| a | b)
                .unwrap_or_default();
            Ok(SlctReturnValue::Const(c))
        } else {
            Ok(SlctReturnValue::Multi(multi))
        }
    }
}

impl UWindow<DialogResult<SlctReturnValue>> for Slctbox {
    const CLASS_NAME: PCWSTR = w!("UWSCR.Slctbox");

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

        // ラベル部分
        let x = margin;
        let y = margin;
        let label = match &self.message {
            Some(title) => Some(self.set_static(title, x, y)?),
            None => None,
        };
        let (y, label_width) = match &label {
            Some(label) => {
                let y = label.height() + y + Self::CTL_GAP;
                let width = label.width().max(Self::ITEM_MIN_WIDTH);
                (y, width)
            },
            None => (y, Self::ITEM_MIN_WIDTH),
        };

        let mut ctl_y = y;
        let (ctl_w, ctl_h) = match self.ctl_type {
            SlctCtlType::Button => {
                let mut btns = self.set_buttons()?;
                let max_width = btns.iter().map(|b| b.width())
                    .reduce(|a,b| a.max(b))
                    .unwrap_or(Self::ITEM_MIN_WIDTH)
                    .max(label_width);
                for btn in &mut btns {
                    btn.move_to(x, ctl_y, Some(max_width), None);
                    ctl_y += btn.height() + Self::BUTTON_GAP;
                }
                (max_width, ctl_y + Self::BUTTON_GAP)
            },
            SlctCtlType::CheckBox => {
                let mut chks = self.set_checkboxes()?;
                let max_width = chks.iter().map(|b| b.width())
                    .reduce(|a,b| a.max(b))
                    .unwrap_or(Self::ITEM_MIN_WIDTH)
                    .max(label_width);
                for chk in &mut chks {
                    chk.move_to(x, ctl_y, Some(max_width), None);
                    ctl_y += chk.height() + Self::BUTTON_GAP;
                }
                (max_width, ctl_y + Self::BUTTON_GAP)
            },
            SlctCtlType::Radio => {
                let mut radios = self.set_radiobuttons()?;
                let max_width = radios.iter().map(|r|r.width())
                    .reduce(|a,b| a.max(b))
                    .unwrap_or(Self::ITEM_MIN_WIDTH)
                    .max(label_width);
                for radio in &mut radios {
                    radio.move_to(x, ctl_y, Some(max_width), None);
                    ctl_y += radio.height() + Self::BUTTON_GAP;
                }
                (max_width, ctl_y + Self::BUTTON_GAP)
            },
            SlctCtlType::Combo => {
                let mut combo = self.set_combobox()?;
                let max_width = label_width.max(combo.width());
                combo.move_to(x, ctl_y, Some(max_width), None);
                (max_width, ctl_y + combo.height() + Self::BUTTON_GAP)
            },
            SlctCtlType::List => {
                let mut list = self.set_listbox()?;
                let max_width = label_width.max(list.width());
                list.move_to(x, ctl_y, Some(max_width), None);
                (max_width, ctl_y + list.height() + Self::BUTTON_GAP)
            },
        };

        let (mut btn_ok, ok_height) = if self.ctl_type.ok_required() {
            let btn = self.set_button("OK", 0, 0, Self::ID_OK as isize, true, Self::OK_WIDTH)?;
            let h = btn.height();
            (Some(btn), h)
        } else {
            (None, 0)
        };
        let (mut progress, progress_height) = match self.progress {
            Some(_) => {
                let progress = self.set_procress_bar(0, 0)?;
                let h = progress.height();
                (Some(progress), h)
            },
            None => (None, 0),
        };

        let width = ctl_w + margin * 2;
        let height = (self.title_bar_height()+ ctl_h + Self::BUTTON_GAP * 2 + ok_height + progress_height + margin)
            .min(Self::MAX_WINDOW_HEIGHT);

        let (x, y) = if self.x.is_none() || self.y.is_none() {
            let center = Self::get_center_pos(width, height);
            let x = self.x.unwrap_or(center.x);
            let y = self.y.unwrap_or(center.y);
            (x, y)
        } else {
            (self.x.unwrap(), self.y.unwrap())
        };
        self.move_to(x, y, width+ margin, height);
        let (cw, _) = self.get_client_wh();
        let btn_bottom = if let Some(btn_ok) = &mut btn_ok {
            let btn_x = cw/2 - btn_ok.width()/2;
            let btn_y = ctl_h + Self::BUTTON_GAP;
            btn_ok.move_to(btn_x, btn_y, None, None);
            btn_ok.focus();
            btn_y + btn_ok.height()
        } else {
            ctl_h + Self::BUTTON_GAP
        };
        if let Some(progress) = &mut progress {
            let prog_y = btn_bottom + Self::BUTTON_GAP;
            progress.move_to(0, prog_y, Some(cw), None);
        }
        self.show();
        Ok(())
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn font(&self) -> Gdi::HFONT {
        self.hfont
    }

    fn message_loop(&self) -> UWindowResult<DialogResult<SlctReturnValue>> {
        unsafe {
            let mut msg = wm::MSG::default();
            let hwnd = HWND::default();

            // プログレスバー
            let hprogress = wm::GetDlgItem(self.hwnd, Self::ID_PROGRESS);
            let progress = (hprogress.0 > 0).then(|| {
                wm::SendMessageW(hprogress, PBM_SETRANGE32, WPARAM(0), LPARAM(100));
                std::time::Instant::now()
            });
            let limit = self.progress.unwrap_or_default();
            let set_progress = move || {
                match progress {
                    Some(start) => {
                        let elapsed = start.elapsed().as_secs_f64();
                        if elapsed >= limit {
                            false
                        } else {
                            let pos = ((limit - elapsed) / limit * 100.0) as usize;
                            wm::SendMessageW(hprogress, PBM_SETPOS, WPARAM(pos), None);
                            true
                        }
                    },
                    None => true
                }
            };

            let point = self.get_pos().unwrap_or_default();
            let result = loop {
                let point = self.get_pos().unwrap_or(point);
                if ! set_progress() {
                    self.destroy();
                    break Ok(DialogResult::new(SlctReturnValue::Timeout, point));
                }
                match wm::GetMessageW(&mut msg, hwnd, 0, 0).0 {
                    -1 => {
                        break Err(UWindowError::Win32(core::Error::from_win32()));
                    },
                    0 => {
                        if msg.hwnd == self.hwnd {
                            break Ok(Default::default());
                        }
                    },
                    _ => if msg.message == wm::WM_COMMAND && msg.wParam.hi_word() as u32 == wm::BN_CLICKED {
                        let id = msg.wParam.lo_word() as i16 as i32;
                        if id == Self::ID_CANCEL {
                            self.destroy();
                            break Ok(Default::default());
                        } else if id == Self::ID_OK {
                            match self.ctl_type {
                                SlctCtlType::Button => {
                                    unreachable!()
                                },
                                SlctCtlType::CheckBox => {
                                    let indexes = self.items.iter().enumerate()
                                        .filter_map(|(i, _)| {
                                            let id = (i + Self::CTL_ID_FIRST) as i32;
                                            let chk = wm::GetDlgItem(self.hwnd, id);
                                            let r = wm::SendMessageW(chk, wm::BM_GETCHECK, None, None);

                                            (r.0 as u32 == BST_CHECKED.0).then_some(i as i32)
                                        })
                                        .collect::<Vec<_>>();
                                    let result = self.index_vec_as_return_value(indexes)?;
                                    self.destroy();
                                    break Ok(DialogResult::new(result, point));
                                },
                                SlctCtlType::Radio => {
                                    let index = self.items.iter().enumerate()
                                        .find_map(|(index, _)| {
                                            let id = (index + Self::CTL_ID_FIRST) as i32;
                                            let rdo = wm::GetDlgItem(self.hwnd, id);
                                            let r = wm::SendMessageW(rdo, wm::BM_GETCHECK, None, None);
                                            (r.0 as u32 == BST_CHECKED.0).then_some(index as i32)
                                        });
                                    let result = index.map(|i| self.index_as_return_value(i))
                                        .unwrap_or(Ok(SlctReturnValue::Cancel))
                                        .map(|r| DialogResult::new(r, point));
                                    self.destroy();
                                    break result;
                                },
                                SlctCtlType::Combo => {
                                    let cmb = wm::GetDlgItem(self.hwnd, Self::ID_COMBOBOX);
                                    let index = wm::SendMessageW(cmb, wm::CB_GETCURSEL, None, None).0 as i32;
                                    let result = self.index_as_return_value(index)
                                        .map(|r| DialogResult::new(r, point));
                                    self.destroy();
                                    break result;
                                },
                                SlctCtlType::List => {
                                    let lst = wm::GetDlgItem(self.hwnd, Self::ID_LISTBOX);
                                    let cnt = wm::SendMessageW(lst, wm::LB_GETSELCOUNT, None, None).0 as usize;
                                    let mut buf = vec![0i32; cnt];
                                    let wparam = WPARAM(cnt);
                                    let lparam = LPARAM(buf.as_mut_ptr() as isize);
                                    wm::SendMessageW(lst, wm::LB_GETSELITEMS, wparam, lparam);
                                    let result = self.index_vec_as_return_value(buf)
                                        .map(|r| DialogResult::new(r, point));
                                    self.destroy();
                                    break result;
                                },
                            }
                        } else if self.ctl_type == SlctCtlType::Button {
                            let index = id - Self::CTL_ID_FIRST as i32;
                            let result = self.index_as_return_value(index)?;
                            self.destroy();
                            break Ok(DialogResult::new(result, point));
                        }
                    }
                };
                if ! wm::IsDialogMessageW(self.hwnd(), &msg).as_bool() {
                    wm::TranslateMessage(&msg);
                    wm::DispatchMessageW(&msg);
                }
            };
            result
        }
    }
    unsafe extern "system"
    fn dlgproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                wm::WM_COMMAND => {
                    let _ = wm::PostMessageW(HWND(0), msg, wparam, lparam);
                    LRESULT(0)
                }
                wm::WM_CLOSE => {
                    let _ = wm::PostMessageW(HWND(0), wm::WM_COMMAND, WPARAM(Self::ID_CANCEL as usize), None);
                    LRESULT(0)
                },
                msg => wm::DefDlgProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}


#[derive(Debug, Default)]
pub enum SlctReturnValue {
    Const(i32),
    Index(i32),
    String(String),
    Multi(Vec<SlctReturnValue>),
    #[default]
    Cancel,
    Timeout,
}

#[derive(PartialEq)]
pub struct SlctType(i32);
pub const SLCT_BTN: SlctType = SlctType(1);
pub const SLCT_CHK: SlctType = SlctType(2);
pub const SLCT_RDO: SlctType = SlctType(4);
pub const SLCT_CMB: SlctType = SlctType(8);
pub const SLCT_LST: SlctType = SlctType(16);
pub const SLCT_STR: SlctType = SlctType(64);
pub const SLCT_NUM: SlctType = SlctType(128);

impl SlctType {
    pub fn new(t: i32) -> Self {
        Self(t)
    }
}

impl std::ops::BitOr for SlctType {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl SlctType {
    fn includes(&self, other: &SlctType) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, PartialEq)]
enum SlctCtlType {
    Button,
    CheckBox,
    Radio,
    Combo,
    List,
}
#[derive(Debug,PartialEq)]
enum SlctReturnType {
    String,
    Index,
    Default
}
impl From<&SlctType> for SlctCtlType {
    fn from(t: &SlctType) -> Self {
        if t.includes(&SLCT_BTN) {
            Self::Button
        } else if t.includes(&SLCT_CHK) {
            Self::CheckBox
        } else if t.includes(&SLCT_RDO) {
            Self::Radio
        } else if t.includes(&SLCT_CMB) {
            Self::Combo
        } else if t.includes(&SLCT_LST) {
            Self::List
        } else {
            Self::Button
        }
    }
}
impl From<&SlctType> for SlctReturnType {
    fn from(t: &SlctType) -> Self {
        if t.includes(&SLCT_STR) {
            Self::String
        } else if t.includes(&SLCT_NUM) {
            Self::Index
        } else {
            Self::Default
        }
    }
}
impl SlctCtlType {
    fn ok_required(&self) -> bool {
        self != &Self::Button
    }
}

struct ProgressBar;
impl ChildClass for ProgressBar {}
struct CheckBox;
impl ChildClass for CheckBox {}
struct RadioButton;
impl ChildClass for RadioButton {}
struct ComboBox;
impl ChildClass for ComboBox {}
struct ListBox;
impl ChildClass for ListBox {}