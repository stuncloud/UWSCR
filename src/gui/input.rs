use super::*;

static INPUTBOX_CLASS: OnceCell<Result<String, UWindowError>> = OnceCell::new();

#[derive(Debug)]
pub struct InputBox {
    hwnd: HWND,
    font: FontFamily,
    caption: String,
    fields: Vec<InputField>,
    edit: Vec<HWND>,
    btn_ok: HWND,
    btn_cancel: HWND,
}

const WIDTH: i32          = 192;
const HEIGHT: i32         = 108;
const MAX_WIDTH: i32      = 1980;
const _MAX_HEIGHT: i32     = 1080;
const MARGIN_X: i32       = 16;
const MARGIN_Y: i32       = 8;
const MIN_EDIT_WIDTH: i32 = 250;
const EDIT_MARGIN: i32    = 4;

impl InputBox {
    pub fn new(title: &str, font: Option<FontFamily>, caption: &str, fields: Vec<InputField>, x: Option<i32>, y: Option<i32>) -> UWindowResult<Self> {
        let class_name = Window::get_class_name("UWSCR.Input", &INPUTBOX_CLASS, Some(Self::wndproc))?;
        let hwnd = Window::create_window(
            None,
            &class_name,
            title,
            WS_EX_TOPMOST,
            WS_OVERLAPPED|WS_SYSMENU|WS_VISIBLE,
            0,
            0,
            WIDTH,
            HEIGHT,
            None
        )?;
        let font = font.unwrap_or_default();
        let mut ib = Self { hwnd, font, edit: vec![], btn_ok: HWND(0), btn_cancel: HWND(0), caption: caption.into(), fields };
        ib.set_input(x, y)?;
        Window::focus(ib.edit[0]);
        Ok(ib)
    }
    fn set_buttons(&mut self) -> UWindowResult<Child> {
        let hpanel = Window::create_panel(self.hwnd, None, Some(Self::subclass), None)?;
        let ok = Window::set_button(hpanel, "&OK", 0, 0, BTN_OK.into(), None, None)?;
        let cancel = Window::set_button(hpanel, "&Cancel", MARGIN_X + ok.size.cx, 0, BTN_CANCEL.into(), None, None)?;
        self.btn_ok = ok.hwnd;
        self.btn_cancel = cancel.hwnd;
        let width = ok.size.cx + MARGIN_X + cancel.size.cx;
        let mut panel = Child::from(hpanel);
        panel.move_to(None, None, Some(width), Some(ok.size.cy));
        Ok(panel)
    }
    pub fn set_input(&mut self, x: Option<i32>, y: Option<i32>) -> UWindowResult<()> {
        let mut bpanel = self.set_buttons()?;
        let hfont = self.font.as_handle()?;
        // メッセージ表示部分
        let mut top = MARGIN_Y;
        let label = Window::set_label(self.hwnd, &self.caption, MARGIN_X, top, Some(hfont), None)?;
        top += label.size.cy + MARGIN_Y;
        let mut id = 100;
        let mut ipanels = self.fields.iter()
            .map(|f| {
                id += 1;
                f.set(self.hwnd, top, hfont, id)
            })
            .collect::<Result<Vec<InputPanel>, UWindowError>>()?;
        self.edit = ipanels.iter().map(|p| p.edit.hwnd).collect();

        /* 位置の調整 */
        // 幅の算出
        let (w_margin_x, w_margin_y) = Window::get_window_margin(self.hwnd);
        let tx = MARGIN_X * 2 + w_margin_x;
        let width_list = vec![
            WIDTH,
            label.size.cx + tx,
            ipanels.max_width() + tx,
            bpanel.size.cx + tx
        ];
        let width = Window::calculate_width(width_list, MAX_WIDTH);
        // 入力欄
        let panel_width = width - tx;
        top = Self::reset_input_panel(&mut ipanels, panel_width, top);
        // ボタン
        let bx = Window::calculate_center(width, bpanel.size.cx);
        bpanel.move_to(Some(bx), Some(top), None, None);
        let height = top + bpanel.size.cy + MARGIN_Y + w_margin_y;

        let (pos_x, pos_y) = if x.is_none() | y.is_none() {
            let (center_x, center_y) = Window::calculate_center_pos(width, height);
            (x.unwrap_or(center_x), y.unwrap_or(center_y))
        } else {
            (x.unwrap(), y.unwrap())
        };
        // let (x, y) = Window::calculate_center_pos(width, height);
        Window::move_window(self.hwnd, pos_x, pos_y, width, height);

        Ok(())
    }
    fn reset_input_panel(panels: &mut Vec<InputPanel>, panel_width: i32, mut y: i32) -> i32 {
        let label_width = panels.max_label_width();
        let edit_width = panels.max_edit_width();
        for panel in panels {
            panel.reset(label_width, edit_width, panel_width, y);
            y += panel.panel.size.cy + MARGIN_Y;
            // println!("[debug] panel: {:#?}", &panel);
        }
        y
    }
    fn move_focus(&self, hwnd: HWND, forward: bool) {
        let h = if let Some(pos) = self.edit.iter().position(|h| *h == hwnd) {
            let next = if forward {pos as i32 + 1} else {pos as i32 - 1};
            if next < 0 {
                self.btn_cancel
            } else if next >= self.edit.len() as i32 {
                self.btn_ok
            } else {
                self.edit[next as usize]
            }
        } else if hwnd == self.btn_ok {
            if forward {
                self.btn_cancel
            } else {
                self.edit[self.edit.len()-1]
            }
        } else if hwnd == self.btn_cancel {
            if forward {
                self.edit[0]
            } else {
                self.btn_ok
            }
        } else {
            self.edit[0]
        };
        Window::focus(h);
    }
    fn move_btn_focus(&self, hwnd: HWND) {
        if hwnd == self.btn_ok {
            Window::focus(self.btn_cancel);
        } else if hwnd == self.btn_cancel {
            Window::focus(self.btn_ok);
        }
    }
}

type InputBoxResult = (Option<Vec<String>>, i32, i32);
impl UWindow<InputBoxResult> for InputBox {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn message_loop(&self) -> UWindowResult<InputBoxResult> {
        unsafe {
            let mut msg = MSG::default();
            let mut forward = true;
            let mut rect = RECT::default();
            let result = loop {
                if GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    rect = Window::get_window_rect(self.hwnd);
                    let repeat = (msg.lParam.0 as u32 & KF_REPEAT * 0x10000) > 0;
                    match msg.message {
                        WM_COMMAND => if msg.wParam.hi_word() as u32 == BN_CLICKED {
                            let id = msg.wParam.lo_word() as i32;
                            if MsgBoxButton::from(id) == BTN_OK {
                                let result = self.edit.iter()
                                        .map(|h| Window::get_edit_text(*h))
                                        .collect::<Vec<_>>();
                                break Some(result);
                            } else {
                                break None;
                            }
                        },
                        WM_KEYDOWN => {
                            let key = VIRTUAL_KEY(msg.wParam.0 as u16);
                            match key {
                                VK_ESCAPE => break None,
                                VK_RETURN => if msg.hwnd == self.btn_ok || msg.hwnd == self.btn_cancel {
                                    SendMessageW(msg.hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
                                } else {
                                    SendMessageW(self.btn_ok, BM_CLICK, WPARAM(0), LPARAM(0));
                                },
                                VK_SHIFT => forward = false,
                                VK_TAB => if repeat {
                                    self.move_focus(msg.hwnd, forward);
                                },
                                VK_DOWN => if repeat {
                                    self.move_focus(msg.hwnd, true);
                                },
                                VK_UP => if repeat {
                                    self.move_focus(msg.hwnd, false);
                                },
                                VK_LEFT | VK_RIGHT => if repeat {
                                    self.move_btn_focus(msg.hwnd);
                                }
                                _ => {}
                            }
                        },
                        WM_KEYUP => {
                            let key = VIRTUAL_KEY(msg.wParam.0 as u16);
                            match key {
                                VK_SHIFT => forward = true,
                                VK_TAB => self.move_focus(msg.hwnd, forward),
                                VK_DOWN => self.move_focus(msg.hwnd, true),
                                VK_UP => self.move_focus(msg.hwnd, false),
                                VK_LEFT|VK_RIGHT => self.move_btn_focus(msg.hwnd),
                                _ => {}
                            }
                        }
                        WM_GETDLGCODE => {
                            // println!("[debug] msg: MSG {{hwnd: {}, message: {}, wParam: {}, lParam: {}}}", &msg.hwnd, &msg.message, &msg.wParam, &msg.lParam);
                        },
                        _ => {
                            // println!("[debug] msg: {:?}", &msg);
                        },
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                } else {
                    // WM_QUITを送ってきたのが自身ならループを抜ける
                    if self.hwnd.0 == msg.lParam.0 {
                        break None;
                    }
                }
            };
            DestroyWindow(self.hwnd);
            Ok((result, rect.left, rect.top))
        }
    }

    unsafe extern "system"
    fn wndproc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match umsg {
            WM_DESTROY => {
                // LPARAMでhwndも伝える
                PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(hwnd.0));
                LRESULT(0)
            },
            WM_COMMAND => {
                PostMessageW(HWND::default(), umsg, wparam, lparam);
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

#[derive(Debug)]
pub struct InputField {
    pub label: Option<String>,
    pub default: Option<String>,
    pub mask: bool
}

impl InputField {
    pub fn new(label: Option<String>, default: Option<String>, mask: bool) -> Self {
        Self {
            label,
            default,
            mask
        }
    }
    pub fn set(&self, parent: HWND, y: i32, hfont: HFONT, id: i32) -> UWindowResult<InputPanel> {
        let rect = RECT { left: MARGIN_X, top: y, right: 0, bottom: 0 };
        let hpanel = Window::create_panel(parent, Some(rect), Some(InputBox::subclass), None)?;
        let label = match &self.label {
            Some(title) => Window::set_label(hpanel, title, 0, 0, Some(hfont), None)?,
            None => Child::default()
        };
        let title = match &self.default {
            Some(d) => d,
            None => "",
        };
        let mut styles = WS_BORDER;
        if self.mask {
            styles |= WINDOW_STYLE(ES_PASSWORD as u32);
        };
        // let size_opt = Some(SizeOption { margin_x: 4, margin_y: 3, min_width: 300, min_height: 20 });
        let mut edit = Window::set_child(hpanel, "edit", title, 0, 0, None, Some(hfont), Some(styles), Some(id))?;
        // 余白を入れる
        // Window::set_margin(edit.hwnd, 20, 2, 4, 2);
        // Window::set_margin2(edit.hwnd, 4);
        edit.move_to(Some(label.size.cx), Some(0), Some(MIN_EDIT_WIDTH), Some(24));
        let width = label.size.cx + EDIT_MARGIN + edit.size.cx;
        let height = label.size.cy.max(edit.size.cy);
        let mut panel = Child::from(hpanel);
        panel.move_to( Some(MARGIN_X), Some(y), Some(width), Some(height));
        // panel.move_to( None, None, Some(width), Some(height));

        Ok(InputPanel {panel, label, edit})
    }
}

#[derive(Debug)]
pub struct InputPanel {
    pub panel: Child,
    pub label: Child,
    pub edit: Child
}
impl InputPanel {
    fn reset(&mut self, label_width: i32, edit_width: i32, panel_width: i32, y: i32) {
        self.panel.move_to(Some(MARGIN_X), Some(y), Some(panel_width), None);
        let label_y = (self.edit.size.cy - self.label.size.cy - 2).max(0);
        self.label.move_to(Some(0), Some(label_y), Some(label_width), None);
        let (x, mut width) = if self.label.size.cx > 0 {
            let x = self.label.size.cx + EDIT_MARGIN;
            let w = panel_width - x;
            (x, w)
        } else {
            (0, panel_width)
        };
        width = width.max(edit_width);
        self.edit.move_to(Some(x), Some(0), Some(width), None);
    }
}
trait VecExt {
    fn max_label_width(&self) -> i32;
    fn max_edit_width(&self) -> i32;
    fn max_width(&self) -> i32;
}
impl VecExt for Vec<InputPanel> {
    fn max_label_width(&self) -> i32 {
        let max = self.iter().map(|p|p.label.size.cx).reduce(|a, b| a.max(b));
        max.unwrap_or_default()
    }
    fn max_edit_width(&self) -> i32 {
        let max = self.iter().map(|p|p.edit.size.cx).reduce(|a, b| a.max(b));
        max.unwrap_or_default()
    }
    fn max_width(&self) -> i32 {
        let max = self.iter().map(|p|p.panel.size.cx).reduce(|a, b| a.max(b));
        max.unwrap_or_default()
    }
}