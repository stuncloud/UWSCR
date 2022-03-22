use super::*;

static SLCTBOX_CLASS: OnceCell<UWindowResult<String>> = OnceCell::new();
const MARGIN_X: i32 = 10;
const MARGIN_Y: i32 = 10;
const BUTTON_MARGIN: i32 = 4;
const MIN_BTN_WIDTH: i32 = 180;
const MIN_LIST_WIDTH: i32 = 240;
const PROGRESS_BAR_HEIGHT: i32 = 10;

const SLCT_BTN_OK: i32 = -1;
const SLCT_PANEL_ID: i32 = -11;
const SLCT_LST_ID: i32 = -12;
const SLCT_CMB_ID: i32 = -13;
const SLCT_PROGRESS_ID: i32 = -14;

#[derive(Debug)]
pub struct Slctbox {
    hwnd: HWND,
    r#type: SlctType,
    option: SlctOption,
    items: Vec<String>,
    wait: u32,
    font: FontFamily,
}

#[derive(Debug, PartialEq)]
pub enum SlctType {
    Button,
    CheckBox,
    Radio,
    Combo,
    List,
}
impl From<u32> for SlctType {
    fn from(n: u32) -> Self {
        match n {
            2 => Self::CheckBox,
            4 => Self::Radio,
            8 => Self::Combo,
            16 => Self::List,
            _ => Self::Button
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum SlctOption {
    String,
    Index,
    None
}
impl From<u32> for SlctOption {
    fn from(n: u32) -> Self {
        match n {
            64 => Self::String,
            128 => Self::Index,
            _ => Self::None
        }
    }
}

type PanelSize = (i32, i32);

impl Slctbox {
    pub fn new(title: &str, message: Option<String>, r#type: SlctType, option: SlctOption, items: Vec<String>, font: FontFamily, wait: u32, pos_x: Option<i32>, pos_y: Option<i32>) -> UWindowResult<Self> {
        if option == SlctOption::None && items.len() > 31 {
            return Err(UWindowError::SlctBoxIndexOverFlowed(items.len() as i32));
        }
        let hwnd = Self::create(title)?;
        let mut slctbox = Self {hwnd, r#type, items, option, wait, font };
        let (top, width) = if let Some(message) = message {
            let label = slctbox.set_message(&message)?;
            let top = label.y + label.size.cy + MARGIN_Y;
            let width = label.size.cx;
            (top, width.max(MIN_BTN_WIDTH))
        } else {
            (MARGIN_Y, MIN_BTN_WIDTH)
        };
        let (width, height) = match slctbox.r#type {
            SlctType::Button => slctbox.set_buttons(top, width, None)?,
            SlctType::List => slctbox.set_list(top, width)?,
            SlctType::CheckBox => slctbox.set_checkbox(top, width)?,
            SlctType::Radio => slctbox.set_radio(top, width)?,
            SlctType::Combo => slctbox.set_combo(top, width)?,
        };
        let (pad_x, pad_y) = Window::get_window_margin(hwnd);
        let width = width + MARGIN_X * 2 + pad_x;
        let mut height = top + height + MARGIN_Y + pad_y;
        if wait > 0 {
            slctbox.set_progress_bar(height-pad_y, width)?;
            height += PROGRESS_BAR_HEIGHT;
        }
        let (x, y) = if pos_x.is_none() | pos_y.is_none() {
            let (center_x, center_y) = Window::calculate_center_pos(width, height);
            (pos_x.unwrap_or(center_x), pos_y.unwrap_or(center_y))
        } else {
            (pos_x.unwrap(), pos_y.unwrap())
        };
        Window::move_window(hwnd, x, y, width, height);
        Ok(slctbox)
    }
    fn set_message(&self, message: &str) -> UWindowResult<Child> {
        let font = self.font.as_handle().ok();
        let label = Window::set_label(self.hwnd, message, MARGIN_X, MARGIN_Y, font, None)?;
        Ok(label)
    }
    fn set_buttons(&mut self, top: i32, width: i32, styles: Option<WINDOW_STYLE>) -> UWindowResult<PanelSize> {
        let font = self.font.as_handle().ok();
        let panel = Window::create_panel(self.hwnd, None, Some(Self::subclass), Some(SLCT_PANEL_ID))?;
        let mut index = 0_u32;
        let mut btop = 0;
        let mut height = 0;
        let mut width = MIN_BTN_WIDTH.max(width);
        let mut btns = vec![];
        for item in &self.items {
            let btn_id = index as i32;
            let btn = Window::set_button(panel, item, 0, btop, btn_id, font, styles)?;
            width = width.max(btn.size.cx);
            btop += btn.size.cy + BUTTON_MARGIN;
            height += btn.size.cy + BUTTON_MARGIN;
            index += 1;
            btns.push(btn);
        }
        btns.iter_mut().for_each(|btn| {
            btn.move_to(None, None, Some(width), None);
        });
        Window::move_window(panel, MARGIN_X, top, width, height);
        Ok((width, height))
    }
    fn set_list(&mut self, top: i32, width: i32) -> UWindowResult<PanelSize> {
        let font = self.font.as_handle().ok();
        let width = width.max(MIN_LIST_WIDTH);
        let mut height = 200;
        let dwstyle = WS_CHILD|WS_VISIBLE|WS_BORDER|WS_VSCROLL|WINDOW_STYLE(LBS_NOTIFY as u32)|WINDOW_STYLE(LBS_MULTIPLESEL as u32);
        let list = Window::create_window(
            Some(self.hwnd),
            "LISTBOX",
            None,
            WINDOW_EX_STYLE::default(),
            dwstyle,
            MARGIN_X, top,
            width, height,
            Some(SLCT_LST_ID)
        )?;
        for item in &self.items {
            let wide = item.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
            Window::send_message(list, LB_ADDSTRING, None, Some(wide.as_ptr() as isize));
        }
        if let Some(hfont) = font {
            Window::set_font(list, hfont);
        }
        let btop = top + height + BUTTON_MARGIN;
        height += self.set_ok_button(btop, width)?;
        Ok((width, height))
    }
    fn set_combo(&mut self, top: i32, width: i32) -> UWindowResult<PanelSize> {
        let font = self.font.as_handle().ok();
        let width = width.max(MIN_LIST_WIDTH);
        let combo_height = 100;
        let mut height = self.font.size + 2;
        let dwstyle = WS_CHILD|WS_VISIBLE|WS_VSCROLL
                |WINDOW_STYLE(CBS_DROPDOWNLIST as u32)
                |WINDOW_STYLE(CBS_AUTOHSCROLL as u32);
        let combo = Window::create_window(
            Some(self.hwnd), "COMBOBOX", None,
            WINDOW_EX_STYLE::default(), dwstyle,
            MARGIN_X, top, width, combo_height,
            Some(SLCT_CMB_ID)
        )?;
        for item in &self.items {
            let wide = item.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
            Window::send_message(combo, CB_ADDSTRING, None, Some(wide.as_ptr() as isize));
        }
        Window::send_message(combo, CB_SETCURSEL, Some(0), None);
        if let Some(hfont) = font {
            Window::set_font(combo, hfont);
        }
        let btop = top + height + MARGIN_X;
        height += self.set_ok_button(btop, width)?;
        Window::focus(combo);
        Ok((width, height))
    }
    fn set_ok_button(&mut self, btop: i32, width: i32) -> UWindowResult<i32> {
        let mut ok_btn = Window::set_button(self.hwnd, "OK", MARGIN_X, btop, SLCT_BTN_OK, None, Some(WS_BORDER))?;
        let left = Window::calculate_center(width, ok_btn.size.cx) + MARGIN_X;
        ok_btn.move_to(Some(left), None, None, None);
        let height = ok_btn.size.cy + BUTTON_MARGIN;
        Ok(height)
    }
    fn set_selectable_buttons(&mut self, top: i32, width: i32, styles: WINDOW_STYLE) -> UWindowResult<PanelSize> {
        let (width, mut height) = self.set_buttons(top, width, Some(styles))?;
        let btop = top + height;
        height += self.set_ok_button(btop, width)?;
        Ok((width, height))
    }
    fn set_checkbox(&mut self, top: i32, width: i32) -> UWindowResult<PanelSize> {
        let styles = WINDOW_STYLE(BS_AUTOCHECKBOX as u32);
        self.set_selectable_buttons(top, width, styles)
    }
    fn set_radio(&mut self, top: i32, width: i32) -> UWindowResult<PanelSize> {
        let styles = WINDOW_STYLE(BS_AUTORADIOBUTTON as u32);
        let result = self.set_selectable_buttons(top, width, styles);
        let hpanel = Window::get_dlg_item(self.hwnd, SLCT_PANEL_ID);
        let first = Window::get_dlg_item(hpanel, 0);
        let wparam = Some(BST_CHECKED.0 as usize);
        Window::send_message(first, BM_SETCHECK, wparam, None);
        result
    }
    fn set_progress_bar(&self, top: i32, width: i32) -> UWindowResult<HWND> {
        Window::create_window(
            Some(self.hwnd),
            "msctls_progress32",
            None,
            WINDOW_EX_STYLE::default(),
            WS_CHILD|WS_VISIBLE,
            0, top, width, PROGRESS_BAR_HEIGHT,
            Some(SLCT_PROGRESS_ID)
        )
    }

    fn create(title: &str) -> UWindowResult<HWND> {
        let class_name = Window::get_class_name("UWSCR.SlctBox", &SLCTBOX_CLASS, Some(Self::wndproc))?;
        Window::create_window(
            None,
            &class_name,
            Some(title),
            WS_EX_TOPMOST,
            WS_OVERLAPPED|WS_SYSMENU|WS_VISIBLE|WINDOW_STYLE(PBS_SMOOTH),
            0, 0, 100, 100,
            None
        )
    }

    fn get_return_value(&self, index: i32) -> UWindowResult<SlctReturnValue> {
        match self.option {
            SlctOption::String => self.get_string_value(index),
            SlctOption::Index => Ok(SlctReturnValue::Index(index)),
            SlctOption::None => self.get_const_value(index),
        }
    }
    fn get_multi_return_value(&self, indexes: Vec<i32>) -> UWindowResult<SlctReturnValue> {
        if self.option == SlctOption::None {
            let total = indexes.into_iter()
                .map(|i| Self::index_to_const(i))
                .collect::<UWindowResult<Vec<i32>>>()?
                .into_iter()
                .reduce(|a, b| a + b)
                .unwrap_or_default();
                Ok(SlctReturnValue::Const(total))
        } else {
            let vec = indexes.into_iter()
                .map(|i| self.get_return_value(i))
                .collect::<UWindowResult<Vec<SlctReturnValue>>>()?;
            Ok(SlctReturnValue::Multi(vec))
        }
    }
    fn index_to_const(index: i32) -> UWindowResult<i32> {
        let (slct_const, failed) = 2_i32.overflowing_pow(index as u32);
        if failed {
            return Err(UWindowError::SlctBoxIndexOverFlowed(index));
        }
        Ok(slct_const)
    }

    fn get_const_value(&self, index: i32) -> UWindowResult<SlctReturnValue> {
        let slct_const = Self::index_to_const(index)?;
        Ok(SlctReturnValue::Const(slct_const))
    }
    fn get_string_value(&self, index: i32) -> UWindowResult<SlctReturnValue> {
        match self.items.get(index as usize) {
            Some(item) => Ok(SlctReturnValue::String(item.to_string())),
            None => Err(UWindowError::SlctBoxInvalidIndex(index)),
        }
    }

    pub fn convert_to_type_and_option(n: u32) -> (SlctType, SlctOption) {
        let so = (n & 0xC0).into(); // SLCT_STR|SLCT_NUM
        let st = (n & 0x1F).into(); // SLCT_BTN|SLCT_CHK|SLCT_RDO|SLCT_CMB|SLCT_LST
        (st, so)
    }
}

#[derive(Debug)]
pub enum SlctReturnValue {
    Const(i32),
    Index(i32),
    String(String),
    Multi(Vec<SlctReturnValue>),
    Cancel
}
impl Default for SlctReturnValue {
    fn default() -> Self {
        Self::Cancel
    }
}

type SlctboxResult = (SlctReturnValue,i32,i32);

impl UWindow<SlctboxResult> for Slctbox {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn show(&self) {
        Window::show(self.hwnd());
    }

    fn message_loop(&self) -> UWindowResult<SlctboxResult> {
        unsafe {
            let mut msg = MSG::default();
            let mut rect = RECT::default();
            let hprogress = Window::get_dlg_item(self.hwnd, SLCT_PROGRESS_ID);
            let start = if ! hprogress.is_invalid() {
                Window::send_message(hprogress, PBM_SETRANGE32, Some(0), Some(100));
                Some(std::time::Instant::now())
            } else {None};
            let limit = (self.wait * 1000) as f64;
            let get_pos = |elapsed: f64, limit: f64| {
                ((limit - elapsed) / limit * 100_f64) as usize
            };

            let result = loop {
                if let Some(ins) = start {
                    let el = ins.elapsed().as_millis() as f64;
                    if el > limit {
                        break SlctReturnValue::Cancel;
                    } else {
                        let pos = get_pos(el, limit);
                        Window::send_message(hprogress, PBM_SETPOS, Some(pos), None);
                    }
                }
                if PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                    rect = Window::get_window_rect(self.hwnd);
                    match msg.message {
                        WM_COMMAND if msg.wParam.hi_word() as u32 == BN_CLICKED => {
                            let id = msg.wParam.lo_word() as i16 as i32;
                            if id == SLCT_BTN_OK {
                                break match self.r#type {
                                    SlctType::CheckBox => {
                                        let mut checked = vec![];
                                        let hpanel = Window::get_dlg_item(self.hwnd, SLCT_PANEL_ID);
                                        for index in 0..self.items.len() {
                                            let id = index as i32;
                                            let hwnd = Window::get_dlg_item(hpanel, id);
                                            if hwnd.is_invalid() {
                                                break;
                                            }
                                            let lresult = Window::send_message(hwnd, BM_GETCHECK, None, None);
                                            if lresult.0 as u32 == BST_CHECKED.0 {
                                                checked.push(id);
                                            }
                                        }
                                        self.get_multi_return_value(checked)?
                                    },
                                    SlctType::Radio => {
                                        let hpanel = Window::get_dlg_item(self.hwnd, SLCT_PANEL_ID);
                                        let id = (0..self.items.len()).find(|id| {
                                            let hwnd = Window::get_dlg_item(hpanel, *id as i32);
                                            let r = Window::send_message(hwnd, BM_GETCHECK, None, None);
                                            r.0 as u32 == BST_CHECKED.0
                                        });
                                        if let Some(index) = id {
                                            self.get_return_value(index as i32)?
                                        } else {
                                            SlctReturnValue::Cancel
                                        }
                                    },
                                    SlctType::Combo => {
                                        let hwnd = Window::get_dlg_item(self.hwnd, SLCT_CMB_ID);
                                        let r = Window::send_message(hwnd, CB_GETCURSEL, None, None);
                                        self.get_return_value(r.0 as i32)?
                                    },
                                    SlctType::List => {
                                        let hwnd = Window::get_dlg_item(self.hwnd, SLCT_LST_ID);
                                        let len = Window::send_message(hwnd, LB_GETSELCOUNT, None, None).0 as usize;
                                        let mut buf: Vec<i32> = vec![];
                                        buf.resize(len, 0);
                                        let wparam = Some(len);
                                        let lparam = Some(buf.as_mut_ptr() as isize);
                                        Window::send_message(hwnd, LB_GETSELITEMS, wparam, lparam);
                                        self.get_multi_return_value(buf)?
                                    },
                                    _ => SlctReturnValue::Cancel
                                };
                            } else {
                                if self.r#type == SlctType::Button {
                                    break self.get_return_value(id)?;
                                }
                            }
                        },
                        WM_QUIT => {
                            if self.hwnd.0 == msg.lParam.0 {
                                break SlctReturnValue::Cancel;
                            }
                        },
                        _ => {}
                    }
                } else {
                    if self.hwnd.0 == msg.lParam.0 {
                        break SlctReturnValue::Cancel;
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };
            DestroyWindow(self.hwnd);
            Ok((result, rect.left, rect.top))
        }
    }

    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(hwnd.0));
                LRESULT(0)
            },
            WM_COMMAND => {
                PostMessageW(HWND(0), umsg, wparam, lparam);
                LRESULT(0)
            },
            msg => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    unsafe extern "system"
    fn subclass(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        CallWindowProcW(Some(Self::wndproc), hwnd, umsg, wparam, lparam)
    }


}