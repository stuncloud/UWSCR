use super::*;

use windows::{
    core::w,
    Win32::{
        Foundation::HWND,
        UI::{
            WindowsAndMessaging as wm,
            Controls::WC_EDITW,
            Shell::{DragAcceptFiles, DragQueryFileW, DragQueryPoint, DragFinish, HDROP},
        },
        Graphics::Gdi,
    }
};

use std::sync::OnceLock;
static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct InputField {
    label: Option<String>,
    default: Option<String>,
    mask: bool,
}
impl InputField {
    pub fn new(label: Option<String>, default: Option<String>, mask: bool) -> Self {
        Self { label, default, mask }
    }
}

pub struct InputBox {
    hwnd: HWND,
    hfont: Gdi::HFONT,
    caption: String,
    field: Vec<InputField>,
    x: Option<i32>,
    y: Option<i32>,
}

impl InputBox {
    const MARGIN: i32 = 16;
    const CAPTION_GAP: i32 = 10;
    const LABEL_GAP: i32 = 4;
    const FIELD_GAP: i32 = 5;
    const BUTTON_V_GAP: i32 = 10;
    const BUTTON_H_GAP: i32 = 16;
    const ID_EDIT_FIRST: i32 = 100;
    const ID_OK: i32 = 1;
    const ID_CANCEL: i32 = 2;
    const MIN_LABEL_WIDTH: i32 = 80;
    const MIN_EDIT_WIDTH: i32 = 350;
    const MIN_FIELD_WIDTH: i32 = 600;

    pub fn new(
        title: &str,
        font: Option<FontFamily>,
        caption: String,
        field: Vec<InputField>,
        x: Option<i32>,
        y: Option<i32>,
    ) -> UWindowResult<Self> {
        let hwnd = Self::create_window(title)?;
        let hfont = font.unwrap_or_default().create()?;
        let input = Self { hwnd, hfont, caption, field, x, y };

        input.draw()?;
        input.show();
        input.activate();

        Ok(input)
    }

    fn set_edit(&self, default: Option<&str>, mask: bool, id: i32) -> UWindowResult<ChildCtl<Edit>> {
        let style = mask.then_some(WINDOW_STYLE(wm::ES_PASSWORD as u32)).unwrap_or_default();
        let menu = (Self::ID_EDIT_FIRST + id) as isize;
        let title = default.unwrap_or("");
        let hwnd = WindowBuilder::new(title, WC_EDITW)
            .style(WS_CHILD|WS_VISIBLE|WS_TABSTOP|wm::WS_BORDER|style)
            .parent(self.hwnd)
            .menu(menu)
            .build()?;
        let size = self.set_font(hwnd, "Dummy");

        let mut child = ChildCtl::new(hwnd, Some(menu), self.hwnd, Edit);
        let height = Some(size.cy + 8);
        child.move_to(0, 0, Some(Self::MIN_EDIT_WIDTH), height);
        Ok(child)
    }

    fn enable_drag_and_drop(hwnd: HWND) {
        unsafe {
            DragAcceptFiles(hwnd, true);
        }
    }
    unsafe fn get_edit_text(&self, hwnd: HWND) -> String {
        let cnt = wm::GetWindowTextLengthW(hwnd) + 1;
        let mut buf = vec![0u16; cnt as usize];
        wm::GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string()
    }
    unsafe fn set_edit_text(hwnd: HWND, wide: &[u16]) {
        let lpstring = PCWSTR::from_raw(wide.as_ptr());
        let _ = wm::SetWindowTextW(hwnd, lpstring);
    }

}

impl UWindow<DialogResult<InputResult>> for InputBox {
    const CLASS_NAME: PCWSTR = w!("UWSCR.Input");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_dlg_class(&REGISTER_CLASS)?;
        WindowBuilder::new(title, Self::CLASS_NAME)
            .style(wm::WS_SYSMENU)
            .ex_style(wm::WS_EX_TOPMOST)
            .build()
    }

    fn draw(&self) -> UWindowResult<()> {
        let x = Self::MARGIN;
        let y = Self::MARGIN;
        let caption = self.set_static(&self.caption, x, y)?;

        let caption_width = caption.width();

        let mut fields = self.field.iter().enumerate()
            .map(|(i, f)| {
                let label = match &f.label {
                    Some(label) => {
                        Some(self.set_static(&label, x, y)?)
                    },
                    None => None,
                };
                let default = f.default.as_deref();
                let edit = self.set_edit(default, f.mask, i as i32)?;
                Ok((label, edit))
            })
            .collect::<UWindowResult<Vec<_>>>()?;
        let (max_label_width, max_field_width) = fields.iter()
            .map(|(l,e)| {
                let label_w = l.as_ref().map(|l| l.width().max(Self::MIN_LABEL_WIDTH))
                    .unwrap_or(Self::MIN_LABEL_WIDTH);
                let field_w = label_w + e.width() + Self::LABEL_GAP;
                (label_w, field_w)
            })
            .reduce(|(lw1, fw1),(lw2, fw2)| {
                let lw = lw1.max(lw2);
                let fw = (fw1).max(fw2);
                (lw, fw)
            })
            .map(|(lw, fw)| (lw, fw.max(caption_width).max(Self::MIN_FIELD_WIDTH)))
            .unwrap_or_default();

        let mut field_y = Self::MARGIN + caption.height() + Self::CAPTION_GAP;

        for (label, edit) in &mut fields {
            match label {
                Some(label) => {
                    label.move_to(x, field_y, Some(max_label_width), Some(edit.height()));
                    let edit_x = x + max_label_width + Self::LABEL_GAP;
                    let edit_w = max_field_width - (max_label_width + Self::LABEL_GAP);
                    edit.move_to(edit_x, field_y, Some(edit_w), None);
                },
                None => {
                    edit.move_to(x, field_y, Some(max_field_width), None);
                },
            }
            field_y += edit.height() + Self::FIELD_GAP;
        }

        let mut btn_ok = self.set_button("&OK", 0, 0, Self::ID_OK as isize, true, 100)?;
        let mut btn_cancel = self.set_button("&Cancel", 0, 0, Self::ID_CANCEL as isize, false, 100)?;

        let btn_y = field_y + Self::BUTTON_V_GAP;
        let height = self.title_bar_height() + btn_y + btn_ok.height() + Self::MARGIN + Self::BUTTON_V_GAP;
        let width = max_field_width + Self::MARGIN * 3;

        let (x, y) = if self.x.is_none() || self.y.is_none() {
            let center = Self::get_center_pos(width, height);
            let x = self.x.unwrap_or(center.x);
            let y = self.y.unwrap_or(center.y);
            (x, y)
        } else {
            (self.x.unwrap(), self.y.unwrap())
        };
        self.move_to(x, y, width, height);

        let (cw, _) = self.get_client_wh();
        let btn_width = btn_ok.width() + Self::BUTTON_H_GAP + btn_cancel.width();
        let btn_ok_x = cw/2 - btn_width/2;
        let btn_cancel_x = btn_ok_x + btn_ok.width() + Self::BUTTON_H_GAP;
        btn_ok.move_to(btn_ok_x, btn_y, None, None);
        btn_cancel.move_to(btn_cancel_x, btn_y, None, None);

        Self::enable_drag_and_drop(self.hwnd);

        Ok(())
    }

    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn font(&self) -> Gdi::HFONT {
        self.hfont
    }

    fn message_loop(&self) -> UWindowResult<DialogResult<InputResult>> {
        unsafe {
            let mut msg = wm::MSG::default();
            let hwnd = HWND::default();
            let point = self.get_pos().unwrap_or_default();
            let result = loop {
                let point = self.get_pos().unwrap_or(point);
                match wm::GetMessageW(&mut msg, hwnd, 0, 0).0 {
                    -1 => {
                        break Err(UWindowError::Win32(core::Error::from_win32()));
                    },
                    0 => {
                        if msg.hwnd == self.hwnd {
                            break Ok(DialogResult::new(None, point));
                        }
                    },
                    _ => {
                        match msg.message {
                            wm::WM_DROPFILES => {
                                let hdrop = HDROP(msg.wParam.0 as isize);
                                let cnt = DragQueryFileW(hdrop, u32::MAX, None);
                                let files = (0..cnt)
                                    .map(|_| {
                                        let len = DragQueryFileW(hdrop, 0, None) as usize + 1;
                                        let mut buf = vec![0u16; len];
                                        DragQueryFileW(hdrop, 0, Some(buf.as_mut()));
                                        buf
                                    })
                                    .reduce(|mut b1, mut b2| {
                                        if let Some(last) = b1.last_mut() {
                                            if *last == 0 {
                                                *last = '\t' as u16;
                                            } else {
                                                b1.push('\t' as u16);
                                            }
                                        }
                                        b1.append(&mut b2);
                                        b1
                                    })
                                    .unwrap_or_default();
                                let mut pt = POINT::default();
                                if DragQueryPoint(hdrop, &mut pt).as_bool() {
                                    let child = wm::ChildWindowFromPoint(self.hwnd, pt);
                                    if wm::GetDlgCtrlID(child) >= Self::ID_EDIT_FIRST {
                                        Self::set_edit_text(child, &files);
                                    } else {
                                        let first = wm::GetDlgItem(self.hwnd, Self::ID_EDIT_FIRST);
                                        Self::set_edit_text(first, &files);
                                    }
                                }
                                DragFinish(hdrop);
                            }
                            wm::WM_COMMAND => {
                                if msg.wParam.hi_word() as u32 == wm::BN_CLICKED {
                                    let id = msg.wParam.lo_word() as i16 as i32;
                                    match id {
                                        Self::ID_OK => {
                                            let input = self.field.iter().enumerate()
                                            .map(|(index, _)| {
                                                    let id = index as i32 + Self::ID_EDIT_FIRST;
                                                    let edit = wm::GetDlgItem(self.hwnd, id);
                                                    self.get_edit_text(edit)
                                                })
                                                .collect();
                                            self.destroy();
                                            break Ok(DialogResult::new(Some(input), point));
                                        },
                                        _ => {
                                            self.destroy();
                                            break Ok(DialogResult::new(None, point));
                                        }
                                    }
                                }
                            },
                            _ => {}
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
        match msg {
            wm::WM_DROPFILES |
            wm::WM_COMMAND => {
                let _ = wm::PostMessageW(HWND(0), msg, wparam, lparam);
                LRESULT(0)
            },
            wm::WM_CLOSE => {
                // let _ = wm::DestroyWindow(hwnd);
                let _ = wm::PostMessageW(HWND(0), wm::WM_COMMAND, WPARAM(Self::ID_CANCEL as usize), None);
                LRESULT(0)
            },
            msg => wm::DefDlgProcW(hwnd, msg, wparam, lparam)
        }
    }
}

type InputResult = Option<Vec<String>>;

struct Edit;
impl ChildClass for Edit {}