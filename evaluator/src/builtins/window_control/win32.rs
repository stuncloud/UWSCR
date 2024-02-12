
use windows::{
    core::{PWSTR, PSTR, HSTRING},
    Win32::{
        Foundation::{
            HWND, WPARAM, LPARAM, BOOL, RECT, HANDLE, POINT,
            CloseHandle,
        },
        UI::{
            Controls::{
                BST_CHECKED, BST_UNCHECKED, BST_INDETERMINATE,
                TCM_GETITEMW, TCM_GETITEMA, TCM_GETITEMCOUNT, TCM_GETCURSEL, TCM_GETUNICODEFORMAT, TCM_GETITEMRECT, TCM_SETCURFOCUS,
                TCIF_TEXT,
                // treeview
                TVIF_HANDLE, TVIF_TEXT,
                TVM_GETUNICODEFORMAT, TVM_GETITEMA, TVM_GETITEMW, TVM_GETNEXTITEM, TVM_GETITEMRECT, TVM_SELECTITEM,
                TVGN_ROOT, TVGN_CHILD, TVGN_NEXT, TVGN_CARET, TVGN_NEXTSELECTED,
                // listview
                LVM_GETUNICODEFORMAT, LVM_GETHEADER, LVM_GETITEMCOUNT, LVM_GETITEMTEXTW, LVM_GETITEMTEXTA, LVM_GETSUBITEMRECT,
                LVM_GETNEXTITEMINDEX, LVITEMINDEX, LVNI_SELECTED,
                LVIF_TEXT,
                HDM_GETITEMCOUNT, HDM_GETITEMW, HDM_GETITEMA, HDM_GETITEMRECT,
                HDI_TEXT,
                TB_GETUNICODEFORMAT, TB_BUTTONCOUNT, TB_GETBUTTONTEXTW, TB_GETBUTTONTEXTA, TB_GETITEMRECT, TB_GETBUTTON,
                // slider
                TBM_GETRANGEMIN, TBM_GETRANGEMAX, TBM_GETPAGESIZE, TBS_VERT, TBM_SETPOS, TBM_GETTHUMBRECT,
                SB_GETPARTS, SB_GETRECT, SB_GETTEXTLENGTHW, SB_GETTEXTW,
                // sendstr
                EM_REPLACESEL,
                // syslink
                // LITEM,LM_GETITEM, LIF_ITEMID, LIF_ITEMINDEX, LIF_URL,
            },
            WindowsAndMessaging::{
                WM_COMMAND,
                BN_CLICKED,
                BS_CHECKBOX, BS_AUTOCHECKBOX, BS_3STATE, BS_AUTO3STATE, BS_RADIOBUTTON, BS_AUTORADIOBUTTON,
                BM_SETCHECK, BM_GETCHECK,
                CB_GETCOUNT, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_SETCURSEL, CB_GETCURSEL, CB_SETEDITSEL,
                CBN_SELCHANGE,
                LB_GETCOUNT, LB_GETTEXT, LB_GETTEXTLEN, LB_SETCURSEL, LB_GETCURSEL, LB_GETITEMRECT,
                LB_GETSELCOUNT, LB_GETSELITEMS,
                LBN_SELCHANGE,
                WS_DISABLED,
                EnumChildWindows, PostMessageW, GetDlgCtrlID, SendMessageW,
                IsWindowUnicode,
                HMENU, GetMenu, GetMenuItemCount, GetSubMenu, GetMenuItemID, GetMenuItemRect,
                MIIM_TYPE, MIIM_STATE, MENUITEMINFOW, GetMenuItemInfoW, MENUITEMINFOA, GetMenuItemInfoA,
                MFS_CHECKED,
                GetWindowThreadProcessId, GetParent,
                // slider
                IsWindowVisible, GetWindowRect,
                GetScrollInfo, SCROLLBAR_CONSTANTS, SB_HORZ, SB_VERT, SCROLLINFO, SIF_ALL,
                GetScrollBarInfo, SCROLLBARINFO, OBJECT_IDENTIFIER, OBJID_HSCROLL, OBJID_VSCROLL,
                WM_HSCROLL, WM_VSCROLL, SB_THUMBTRACK,
                // get/sendstr
                WM_GETTEXT, WM_GETTEXTLENGTH, WM_CHAR,
                GetGUIThreadInfo, GUITHREADINFO, WM_SETTEXT,
                // GetWindowTextW, GetWindowTextLengthW,
            },
        },
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
        System::{
            Threading::{
                PROCESS_VM_READ, PROCESS_VM_WRITE, PROCESS_VM_OPERATION, PROCESS_QUERY_INFORMATION,
                OpenProcess, IsWow64Process,
            },
            Memory::{
                MEM_COMMIT, PAGE_READWRITE, MEM_RELEASE,
                VirtualAllocEx, VirtualFreeEx,
            },
            Diagnostics::Debug::{
                ReadProcessMemory, WriteProcessMemory,
            }
        }
    }
};

use util::winapi::{
    get_class_name, get_window_title, make_wparam, get_window_style,
    from_ansi_bytes, from_wide_string, make_word,
};
use crate::builtins::{
    ThreeState,
    window_low::move_mouse_to,
};
use super::clkitem::{ClkItem, MouseInput, match_title, ClkResult};
use super::{get_process_id_from_hwnd, get_window_rect};
use super::acc::U32Ext;

use std::mem;
use std::ffi::c_void;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Win32 {
    hwnd: HWND,
}

impl Win32 {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }
    fn search(&self, item: &mut SearchItem) {
        if item.target.contains(&TargetClass::Menu) {
            Self::search_menu(self.hwnd, item);
            if item.found.is_some() {
                return;
            }
        }
        let p = item as *mut SearchItem;
        let lparam = LPARAM(p as isize);
        unsafe {
            if item.target.contains(&TargetClass::ScrollBar) {
                EnumChildWindows(self.hwnd, Some(Self::slider_callback), lparam);
            } else {
                EnumChildWindows(self.hwnd, Some(Self::enum_child_callback), lparam);

            }
        }
    }
    unsafe extern "system"
    fn enum_child_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if let HWND(0) = hwnd {
            false.into()
        } else {
            let item = &mut *(lparam.0 as *mut SearchItem);
            let class = get_class_name(hwnd);
            let target = TargetClass::from(class);
            if item.target.contains(&target) {
                match target {
                    TargetClass::ComboBox => {
                        if let ListIndex::Index(i) = Self::search_combo_box(hwnd, item) {
                            item.found = Some(ItemFound::new(hwnd, target, ItemInfo::Index(i)));
                            return false.into();
                        }
                    },
                    TargetClass::List => {
                        match Self::search_list_box(hwnd, item) {
                            ListIndex::Index(i) => {
                                item.found = Some(ItemFound::new(hwnd, target, ItemInfo::Index(i)));
                                return false.into();
                            },
                            ListIndex::Multi(v) => {
                                item.found = Some(ItemFound::new(hwnd, target, ItemInfo::Indexes(v)));
                                return false.into();
                            },
                            ListIndex::None => {},
                        }
                    },
                    TargetClass::Tab => if let Some(i) = Self::search_tab(hwnd, item) {
                        item.found = Some(ItemFound::new(hwnd, target, ItemInfo::Index(i)));
                        return false.into();
                    },
                    TargetClass::Menu => {
                        // ここには来ない
                    },
                    TargetClass::TreeView => {
                        Self::search_treeview(hwnd, item);
                        if item.found.is_some() {
                            return false.into();
                        }
                    },
                    TargetClass::ListView => {
                        Self::search_listview(hwnd, item);
                        if item.found.is_some() {
                            return false.into();
                        }
                    },
                    TargetClass::ToolBar => {
                        Self::search_toolbar(hwnd, item);
                        if item.found.is_some() {
                            return false.into();
                        }
                    },
                    TargetClass::Link => {
                        todo!()
                    },
                    TargetClass::ScrollBar |
                    TargetClass::TrackBar => {
                        // ここには来ない
                    },
                    TargetClass::Button => {
                        let title = get_window_title(hwnd);
                        if item.matches(&title) {
                            item.found = Some(ItemFound::new(hwnd, target, ItemInfo::None));
                            return false.into();
                        }
                    },
                    TargetClass::Edit |
                    TargetClass::Static => {
                        if item.is_in_exact_order() {
                            item.found = Some(ItemFound::new(hwnd, target, ItemInfo::None));
                            return false.into();
                        }
                    },
                    TargetClass::StatusBar => {
                        let count = StatusBar::new(hwnd).count();
                        for i in 0..count {
                            if item.is_in_exact_order() {
                                item.found = Some(ItemFound::new(hwnd, target, ItemInfo::Index(i)));
                                return false.into();
                            }
                        }
                    }
                    TargetClass::Other(_) => {},
                }
            }
            true.into()
        }
    }

    unsafe extern "system"
    fn slider_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if let HWND(0) = hwnd {
            return false.into()
        };

        let item = &mut *(lparam.0 as *mut SearchItem);
        let class = get_class_name(hwnd);
        let target = TargetClass::from(class);
        match target {
            TargetClass::TrackBar => if item.is_in_exact_order() {
                item.found = Some(ItemFound::new(hwnd, target, ItemInfo::TrackBar));
                return false.into();
            },
            _ => if IsWindowVisible(hwnd).as_bool() {
                // スクロールバーを探す
                if let Some(scrollinfo) = Slider::get_scrollbar_info(hwnd, SB_HORZ) {
                    if item.is_in_exact_order() {
                        let point = Slider::get_scrollbar_point(hwnd, OBJID_HSCROLL);
                        let info = ItemInfo::ScrollBar(scrollinfo, SB_HORZ, point);
                        item.found = Some(ItemFound::new(hwnd, target, info));
                        return false.into()
                    }
                } else if let Some(scrollinfo) = Slider::get_scrollbar_info(hwnd, SB_VERT) {
                    if item.is_in_exact_order() {
                        let point = Slider::get_scrollbar_point(hwnd, OBJID_VSCROLL);
                        let info = ItemInfo::ScrollBar(scrollinfo, SB_VERT, point);
                        item.found = Some(ItemFound::new(hwnd, target, info));
                        return false.into()
                    }
                }
            }
        }
        true.into()
    }
    pub fn get_point(&self, clk_item: &ClkItem) -> ClkResult {
        let mut item = SearchItem::from_clkitem(clk_item);
        self.search(&mut item);
        if let Some(found) = item.found {
            if Self::is_disabled(found.hwnd) {
                ClkResult::failed()
            } else {
                match found.target {
                    TargetClass::Button => ClkResult::new(true, found.hwnd, None),
                    TargetClass::List => match found.info {
                        ItemInfo::Index(i) => {
                            let point = Self::get_list_item_point(found.hwnd, i);
                            ClkResult::new(true, found.hwnd, Some(point))
                        },
                        _ => ClkResult::failed()
                    },
                    TargetClass::ComboBox => ClkResult::new(true, found.hwnd, None),
                    TargetClass::Tab => match found.info {
                        ItemInfo::Index(i) => {
                            let point = Self::get_tab_point(found.hwnd, i);
                            ClkResult::new(true, found.hwnd, point)
                        },
                        _ => ClkResult::failed()
                    },
                    TargetClass::Menu => match found.info {
                        ItemInfo::Menu(_, _, x, y) => {
                            ClkResult::new(true, found.hwnd, Some((x, y)))
                        }
                        _ => ClkResult::failed()
                    },
                    TargetClass::TreeView => match found.info {
                        ItemInfo::HItem(hitem, pid) => {
                            let point = TreeView::get_point(found.hwnd, pid, hitem);
                            ClkResult::new(true, found.hwnd, point)
                        }
                        _ => ClkResult::failed()
                    },
                    TargetClass::ListView => match found.info {
                        ItemInfo::ListView(row, column, lv) => {
                            let point = lv.get_point(row, column);
                            ClkResult::new(true, found.hwnd, point)
                        },
                        ItemInfo::ListViewHeader(index, pid) => {
                            let point = ListView::get_header_point(found.hwnd, index, pid);
                            ClkResult::new(true, found.hwnd, point)
                        }
                        _ => ClkResult::failed()
                    },
                    TargetClass::ToolBar => match found.info {
                        ItemInfo::ToolBar(index, _, pid) => {
                            let point = ToolBar::get_point(found.hwnd, index, pid);
                            ClkResult::new(true, found.hwnd, point)
                        }
                        _ => ClkResult::failed()
                    },
                    TargetClass::Link => match found.info {
                        _ => ClkResult::failed()
                    },
                    TargetClass::ScrollBar |
                    TargetClass::TrackBar |
                    TargetClass::Edit |
                    TargetClass::Static |
                    TargetClass::StatusBar |
                    TargetClass::Other(_) => ClkResult::failed(),
                }
            }
        } else {
            ClkResult::failed()
        }
    }
    pub fn click(&self, clk_item: &ClkItem, check: &ThreeState) -> ClkResult {
        let mut item = SearchItem::from_clkitem(clk_item);
        self.search(&mut item);
        // println!("\u{001b}[35m[debug] item: {:#?}\u{001b}[0m", &item);
        if let Some(found) = item.found {
            if Self::is_disabled(found.hwnd) {
                ClkResult::failed()
            } else {
                match found.target {
                    TargetClass::Button => {
                        match ButtonType::from(found.hwnd) {
                            ButtonType::Button => if check.as_bool() {
                                let clicked = self.post_wm_command(found.hwnd, None, BN_CLICKED);
                                ClkResult::new(clicked, found.hwnd, None)
                            } else {
                                ClkResult::new(true, found.hwnd, None)
                            },
                            ButtonType::Check => {
                                let wparam = if check.as_bool() {
                                    BST_CHECKED.0 as usize
                                } else {
                                    BST_UNCHECKED.0 as usize
                                };
                                Self::post_message(found.hwnd, BM_SETCHECK, wparam, 0);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let is_checked = Self::send_message(found.hwnd, BM_GETCHECK, 0, 0);
                                let clicked = (is_checked > 0) == check.as_bool();
                                ClkResult::new(clicked, found.hwnd, None)
                            },
                            ButtonType::ThreeState => {
                                let wparam = match check {
                                    ThreeState::True => BST_CHECKED.0 as usize,
                                    ThreeState::False => BST_UNCHECKED.0 as usize,
                                    ThreeState::Other => BST_INDETERMINATE.0 as usize,
                                };
                                Self::post_message(found.hwnd, BM_SETCHECK, wparam, 0);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let checked = Self::send_message(found.hwnd, BM_GETCHECK, 0, 0);
                                let clicked = checked == wparam as isize;
                                ClkResult::new(clicked, found.hwnd, None)
                            },
                            ButtonType::Radio => if check.as_bool() {
                                MouseInput::left_click(found.hwnd, None);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let is_checked = Self::send_message(found.hwnd, BM_GETCHECK, 0, 0);
                                let clicked = (is_checked > 0) == check.as_bool();
                                ClkResult::new(clicked, found.hwnd, None)
                            } else {
                                ClkResult::new(true, found.hwnd, None)
                            },
                        }
                    },
                    TargetClass::List => match found.info {
                        ItemInfo::Indexes(v) => {
                            let mut list_result = true;
                            let mut point = (0, 0);
                            for i in v {
                                Self::post_message(found.hwnd, LB_SETCURSEL, i, 0);
                                Self::sleep(30);
                                let index = Self::send_message(found.hwnd, LB_GETCURSEL, 0, 0);
                                if i as isize != index {
                                    list_result = false;
                                    point = Self::get_list_item_point(found.hwnd, index as usize);
                                    break;
                                }
                            }
                            ClkResult::new(list_result, found.hwnd, Some(point))
                        },
                        ItemInfo::Index(i) => {
                            let clicked = if check.as_bool() {
                                Self::post_message(found.hwnd, LB_SETCURSEL, i, 0);
                                Self::sleep(30);
                                let index = Self::send_message(found.hwnd, LB_GETCURSEL, 0, 0);
                                Self::post_wm_command(&self, found.hwnd, None, LBN_SELCHANGE);
                                i as isize == index
                            } else {
                                true
                            };
                            let point = Self::get_list_item_point(found.hwnd, i);
                            ClkResult::new(clicked, found.hwnd, Some(point))

                        },
                        _ => ClkResult::failed(),
                    },
                    TargetClass::ComboBox => if let ItemInfo::Index(i) = found.info {
                        let clicked = if check.as_bool() {
                            Self::post_message(found.hwnd, CB_SETCURSEL, i, 0);
                            let lparam = make_word(0, -1);
                            Self::post_message(found.hwnd, CB_SETEDITSEL, i, lparam);
                            // let id = Self::send_message(found.hwnd, CB_GETITEMDATA, i, 0) as i32;
                            // println!("\u{001b}[31m[debug] id: {:#?}\u{001b}[0m", &id);
                            Self::post_wm_command(&self, found.hwnd, None, CBN_SELCHANGE);
                            Self::sleep(30);
                            let index = Self::send_message(found.hwnd, CB_GETCURSEL, 0, 0);
                            i as isize == index
                        } else {
                            true
                        };
                        // let (x, y) = Self::get_list_item_point(found.hwnd, i);
                        ClkResult::new(clicked, found.hwnd, None)
                    } else {
                        ClkResult::failed()
                    },
                    TargetClass::Tab => if let ItemInfo::Index(i) = found.info {
                        let clicked = if check.as_bool() {
                            Self::post_message(found.hwnd, TCM_SETCURFOCUS, i, 0);
                            Self::sleep(30);
                            let index = Self::send_message(found.hwnd, TCM_GETCURSEL, 0, 0);
                            i as isize == index
                        } else {
                            true
                        };
                        let point = Self::get_tab_point(found.hwnd, i);
                        ClkResult::new(clicked, found.hwnd, point)
                    } else {
                        ClkResult::failed()
                    },
                    TargetClass::Menu => {
                        if let ItemInfo::Menu(id, checked, x, y) = found.info {
                            match (check.as_bool(), checked) {
                                // checkがtrueかつメニューにチェックがなければクリック
                                (true, false) |
                                // checkがfalseかつメニューにチェックがあればクリック
                                (false, true) => {
                                    let clicked = Menu::click(found.hwnd, id);
                                    ClkResult::new(clicked, found.hwnd, Some((x, y)))
                                },
                                // それ以外は項目があればTRUEを返す
                                _ => ClkResult::new(true, found.hwnd, Some((x, y))),
                            }
                        } else {
                            ClkResult::failed()
                        }
                    },
                    TargetClass::TreeView => {
                        if let ItemInfo::HItem(hitem, pid) = found.info {
                            let clicked = if check.as_bool() {
                                TreeView::click(found.hwnd, hitem)
                            } else {
                                true
                            };
                            let point = TreeView::get_point(found.hwnd, pid, hitem);
                            ClkResult::new(clicked, found.hwnd, point)
                        } else {
                            ClkResult::failed()
                        }
                    },
                    TargetClass::ListView => {
                        match found.info {
                            ItemInfo::ListView(row, column, lv) => {
                                let point = lv.get_point(row, column);
                                let clicked = if check.as_bool() {
                                    // lv.click(row, column)
                                    MouseInput::left_click(found.hwnd, point)
                                } else {
                                    true
                                };
                                ClkResult::new(clicked, found.hwnd, point)
                            },
                            ItemInfo::ListViewHeader(index, pid) => {
                                let point = ListView::get_header_point(found.hwnd, index, pid);
                                let clicked = if check.as_bool() {
                                    MouseInput::left_click(found.hwnd, point)
                                } else {
                                    true
                                };
                                ClkResult::new(clicked, found.hwnd, point)
                            }
                            _ => ClkResult::failed()
                        }
                    },
                    // TargetClass::ListViewHeader => {
                    //     todo!()
                    // },
                    TargetClass::ToolBar => {
                        match found.info {
                            ItemInfo::ToolBar(index, id, pid) => {
                                let clicked = if check.as_bool() {
                                    ToolBar::click(found.hwnd, id)
                                } else {
                                    true
                                };
                                let point = ToolBar::get_point(found.hwnd, index, pid);
                                ClkResult::new(clicked, found.hwnd, point)
                            }
                            _ => ClkResult::failed(),
                        }
                    },
                    TargetClass::Link => {
                        todo!()
                    },
                    TargetClass::ScrollBar |
                    TargetClass::TrackBar => ClkResult::failed(),
                    TargetClass::Edit |
                    TargetClass::Static |
                    TargetClass::StatusBar => ClkResult::failed(),
                    TargetClass::Other(_) => ClkResult::failed(),
                }
            }
        } else {
            ClkResult::failed()
        }
    }

    fn is_disabled(hwnd: HWND) -> bool {
        get_window_style(hwnd) as u32 & WS_DISABLED.0 > 0
    }
    fn is_visible(hwnd: HWND) -> bool {
        unsafe {
            IsWindowVisible(hwnd).as_bool()
        }
    }

    fn post_wm_command(&self, hwnd: HWND, id: Option<i32>, command: u32) -> bool {
        let id = id.unwrap_or(Self::get_window_id(hwnd));
        let msg = command as u16;
        let wparam = make_wparam(id as u16, msg).0;
        let lparam = hwnd.0;
        Self::post_message(self.hwnd, WM_COMMAND, wparam, lparam)
    }

    fn post_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> bool {
        unsafe {
            PostMessageW(hwnd, msg, WPARAM(wparam), LPARAM(lparam)).is_ok()
        }
    }
    fn send_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
        unsafe {
            SendMessageW(hwnd, msg, WPARAM(wparam), LPARAM(lparam)).0
        }
    }
    // fn _post_thread_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> bool {
    //     unsafe {
    //         let idthread = GetWindowThreadProcessId(hwnd, None);
    //         PostThreadMessageW(idthread, msg, WPARAM(wparam), LPARAM(lparam)).as_bool()
    //     }
    // }
    fn get_parent(hwnd: HWND) -> HWND {
        unsafe {
            GetParent(hwnd)
        }
    }
    fn get_window_id(hwnd: HWND) -> i32 {
        unsafe {
            GetDlgCtrlID(hwnd)
        }
    }
    fn get_text(hwnd: HWND) -> Option<String> {
        let len = Win32::send_message(hwnd, WM_GETTEXTLENGTH, 0, 0);
        if len > 0 {
            let mut buf = vec![0; len as usize + 1];
            let size = Win32::send_message(hwnd, WM_GETTEXT, buf.len(), buf.as_mut_ptr() as isize);
            let text = String::from_utf16_lossy(&buf[..size as usize]);
            Some(text)
        } else {
            None
        }
    }
    fn search_combo_box(hwnd: HWND, item: &mut SearchItem) -> ListIndex {
        Self::search_list(hwnd, item, CB_GETCOUNT, CB_GETLBTEXTLEN, CB_GETLBTEXT)
    }
    fn search_list_box(hwnd: HWND, item: &mut SearchItem) -> ListIndex {
        if item.name.contains("\t") {
            Self::search_list_multi(hwnd, item, LB_GETCOUNT, LB_GETTEXTLEN, LB_GETTEXT)
        } else {
            Self::search_list(hwnd, item, LB_GETCOUNT, LB_GETTEXTLEN, LB_GETTEXT)
        }
    }
    fn search_list(hwnd: HWND, item: &mut SearchItem, msg_cnt: u32, msg_get_len: u32, msg_get_txt: u32) -> ListIndex {
        let count = Self::send_message(hwnd, msg_cnt, 0, 0);
        let size = if count < 0 {
            return ListIndex::None;
        } else {
            count as usize
        };
        for i in 0..size {
            let len = Self::send_message(hwnd, msg_get_len, i, 0);
            if len > 0 {
                let mut buf = Vec::new();
                buf.resize(len as usize, 0);
                let lparam = buf.as_mut_ptr() as isize;
                Self::send_message(hwnd, msg_get_txt, i, lparam) as usize;
                let text = String::from_utf16_lossy(&buf);
                let trimmed = text.trim_end_matches('\0');
                if item.matches(trimmed) {
                    return ListIndex::Index(i);
                }
            }
        }
        ListIndex::None
    }
    fn search_list_multi(hwnd: HWND, item: &mut SearchItem, msg_cnt: u32, msg_get_len: u32, msg_get_txt: u32) -> ListIndex {
        let count = Self::send_message(hwnd, msg_cnt, 0, 0);
        let size = if count < 0 {
            return ListIndex::None;
        } else {
            count as usize
        };
        let mut index = vec![];
        for i in 0..size {
            let len = Self::send_message(hwnd, msg_get_len, i, 0);
            if len > 0 {
                let mut buf = Vec::new();
                buf.resize(len as usize, 0);
                let lparam = buf.as_mut_ptr() as isize;
                Self::send_message(hwnd, msg_get_txt, i, lparam) as usize;
                let text = String::from_utf16_lossy(&buf);
                let trimmed = text.trim_end_matches('\0');
                let found = item.name.split("\t")
                    .map(|s| s.to_string())
                    .find(|pat| {
                        match_title(trimmed, pat, item.short)
                    })
                    .is_some();
                if found {
                    index.push(i);
                }
            }
        }
        ListIndex::Multi(index)
    }
    fn get_list_item_point(hwnd: HWND, index: usize) -> (i32, i32) {
        let mut rect = RECT::default();
        let lparam = &mut rect as *mut RECT as isize;
        Self::send_message(hwnd, LB_GETITEMRECT, index, lparam);
        let (x, y) = Win32::get_middle_left(rect);
        Win32::client_to_screen(hwnd, x, y)
    }

    fn search_tab(hwnd: HWND, item: &mut SearchItem) -> Option<usize> {
        let count = Self::send_message(hwnd, TCM_GETITEMCOUNT, 0, 0);
        let tab_ctrl = TabControl::new(hwnd)?;
        for i in 0..count {
            if let Some(name) = tab_ctrl.get_name(i as usize) {
                if item.matches(name.trim_end_matches("\0")) {
                    return Some(i as usize);
                }
            }
        }
        None
    }
    fn get_tab_point(hwnd: HWND, index: usize) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        let pid = get_process_id_from_hwnd(hwnd);
        let remote_rect = ProcessMemory::new(pid, Some(&rect))?;
        let lparam = remote_rect.pointer as isize;
        Win32::send_message(hwnd, TCM_GETITEMRECT, index, lparam);
        remote_rect.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let point = Win32::client_to_screen(hwnd, x, y);
        Some(point)
    }

    fn search_menu(hwnd: HWND, item: &mut SearchItem) {
        let menu = Menu::new(hwnd);
        menu.search(item, None, None);
    }

    fn search_treeview(hwnd: HWND, item: &mut SearchItem) {
        if let Some(tv) = TreeView::new(hwnd) {
            tv.search(item, None, None);
        }
    }

    fn search_listview(hwnd: HWND, item: &mut SearchItem) {
        if let Some(lv) = ListView::new(hwnd) {
            lv.search(item);
        }
    }

    fn search_toolbar(hwnd: HWND, item: &mut SearchItem) {
        if let Some(tb) = ToolBar::new(hwnd) {
            tb.search(item);
        }
    }

    fn is_window_unicode(hwnd: HWND) -> bool {
        unsafe { IsWindowUnicode(hwnd).as_bool() }
    }
    fn get_center(rect: RECT) -> (i32, i32) {
        let x = rect.left + (rect.right - rect.left) / 2;
        let y = rect.top + (rect.bottom - rect.top) / 2;
        (x, y)
    }
    fn get_middle_left(rect: RECT) -> (i32, i32) {
        let x = rect.left + 5.min(rect.right - rect.left); // ちょっとだけずらす
        let y = rect.top + (rect.bottom - rect.top) / 2;
        (x, y)
    }
    pub fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> (i32, i32) {
        let mut point = POINT { x, y };
        unsafe { ClientToScreen(hwnd, &mut point); }
        (point.x, point.y)
    }
    fn sleep(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    fn is_process_x64(pid: u32) -> Option<bool> {
        unsafe {
            let dwdesiredaccess = PROCESS_VM_READ|PROCESS_VM_WRITE|PROCESS_VM_OPERATION|PROCESS_QUERY_INFORMATION;
            let hprocess = OpenProcess(dwdesiredaccess, false, pid).ok()?;
            let mut wow64process = true.into();
            let _ = IsWow64Process(hprocess, &mut wow64process);
            let _ = CloseHandle(hprocess);
            let is_x64 = ! wow64process.as_bool();
            Some(is_x64)
        }
    }

    pub fn get_slider(hwnd: HWND, nth: u32) -> Option<Slider> {
        let mut item = SearchItem::new_slider(nth);
        let win32 = Self::new(hwnd);
        win32.search(&mut item);
        if let Some(found) = item.found {
            match found.info {
                ItemInfo::TrackBar => {
                    let slider = Slider::TrackBar(found.hwnd, hwnd);
                    Some(slider)
                },
                ItemInfo::ScrollBar(info, dir, point) => {
                    let slider = Slider::ScrollBar(found.hwnd, info, dir.0, point);
                    Some(slider)
                }
                _ => None
            }
        } else {
            None
        }
    }

    pub fn get_check_state(hwnd: HWND, name: String, nth: u32) -> i32 {
        let mut item = SearchItem {
            name,
            short: true,
            target: vec![TargetClass::Button, TargetClass::Menu],
            order: nth,
            found: None,
        };
        Self::new(hwnd).search(&mut item);
        if let Some(found) = item.found {
            match found.info {
                ItemInfo::None => Self::send_message(found.hwnd, BM_GETCHECK, 0, 0) as i32,
                ItemInfo::Menu(_, c, _, _) => c as i32,
                _ => -1,
            }
        } else {
            -1
        }
    }
    pub fn get_edit_str(hwnd: HWND, nth: u32, mouse: bool) -> Option<String> {
        Self::get_str(hwnd, TargetClass::Edit, nth, mouse)
    }
    pub fn get_static_str(hwnd: HWND, nth: u32, mouse: bool) -> Option<String> {
        Self::get_str(hwnd, TargetClass::Static, nth, mouse)
    }
    pub fn get_status_str(hwnd: HWND, nth: u32, mouse: bool) -> Option<String> {
        Self::get_str(hwnd, TargetClass::StatusBar, nth, mouse)
    }
    fn get_str(hwnd: HWND, target: TargetClass, nth: u32, mouse: bool) -> Option<String> {
        let mut item = SearchItem {
            name: String::new(),
            short: false,
            target: vec![target],
            order: nth,
            found: None,
        };
        Self::new(hwnd).search(&mut item);
        match item.found {
            Some(found) => {
                let (str, x, y) = match found.target {
                    TargetClass::Edit => {
                        let rect = get_window_rect(found.hwnd);
                        let len = Win32::send_message(found.hwnd, WM_GETTEXTLENGTH, 0, 0) as usize;
                        let mut buf = vec![0; len + 1];
                        let str = if Win32::send_message(found.hwnd, WM_GETTEXT, len + 1, buf.as_mut_ptr() as isize) > 0 {
                            Some(from_wide_string(&buf))
                        } else {
                            None
                        };
                        (str, rect.x(), rect.y())
                    },
                    TargetClass::Static => {
                        let title = get_window_title(found.hwnd);
                        let rect = get_window_rect(found.hwnd);
                        (Some(title), rect.x(), rect.y())
                    },
                    TargetClass::StatusBar => {
                        if let ItemInfo::Index(i) = found.info {
                            let sb = StatusBar::new(found.hwnd);
                            let str = sb.get_str(i);
                            let (x, y) = match sb.get_rect(i) {
                                Some(rect) => Self::client_to_screen(found.hwnd, rect.left, rect.top),
                                None => (0, 0),
                            };
                            (str, x, y)
                        } else {
                            (None, 0, 0)
                        }
                    },
                    _ => (None, 0, 0),
                };
                if mouse && str.is_some() {
                    move_mouse_to(x+5, y+5);
                }
                str
            },
            None => None,
        }
    }
    pub fn sendstr(hwnd: HWND, nth: u32, str: &str, mode: super::SendStrMode) -> Option<()>{
        let edit = if nth == 0 {
            let focused = Self::get_focused_control(hwnd);
            let class_name = get_class_name(focused).to_ascii_lowercase();
            if class_name == "edit".to_string() {
                focused
            } else {
                return None;
            }
        } else {
            let mut item = SearchItem {
                name: String::new(),
                short: false,
                target: vec![TargetClass::Edit],
                order: nth,
                found: None,
            };
            Self::new(hwnd).search(&mut item);
            if let Some(found) = item.found {
                found.hwnd
            } else {
                return None;
            }
        };
        let hstring = HSTRING::from(str);
        let lparam = hstring.as_ptr() as isize;
        match mode {
            super::SendStrMode::Append => {
                Self::send_message(edit, EM_REPLACESEL, 0, lparam);
            },
            super::SendStrMode::Replace => {
                Self::send_message(edit, WM_SETTEXT, 0, lparam);
            },
            super::SendStrMode::OneByOne => {
                let vec = hstring.as_wide()
                    .into_iter()
                    .map(|n| *n as usize);
                for char in vec {
                    Win32::send_message(edit, WM_CHAR, char, 1);
                }
            },
        }
        Some(())
    }
    pub fn get_focused_control(hwnd: HWND) -> HWND {
        unsafe {
            let idthread = GetWindowThreadProcessId(hwnd, None);
            let mut pgui = GUITHREADINFO::default();
            pgui.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
            let _ = GetGUIThreadInfo(idthread, &mut pgui);
            pgui.hwndFocus
        }
    }

    // getitem

    pub fn getitem(hwnd: HWND, target: u32, nth: i32, column: i32, ignore_disabled: bool) -> Vec<String> {
        let mut gi = GetItem::new(target, nth, column, ignore_disabled);
        if gi.target.contains(&TargetClass::Menu) {
            Menu::new(hwnd).get_names(None, &mut gi);
        }
        let lparam = LPARAM(&mut gi as *mut GetItem as isize);
        unsafe {
            EnumChildWindows(hwnd, Some(Self::getitem_callback), lparam);
        }
        gi.found
    }
    unsafe extern "system"
    fn getitem_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if let HWND(0) = hwnd {
            false.into()
        } else {
            let gi = &mut *(lparam.0 as *mut GetItem);
            if Self::is_disabled(hwnd) && gi.ignore_disabled {
                // ディセーブル無視で対象がdisabledなら次へ
                return true.into();
            }
            let class = TargetClass::from(get_class_name(hwnd));
            if gi.target.contains(&class) {
                match class {
                    TargetClass::Button => {
                        gi.add(get_window_title(hwnd));
                    },
                    TargetClass::List => {
                        if gi.is_nth_list() {
                            for i in 0..Self::send_message(hwnd, LB_GETCOUNT, 0, 0) {
                                let len = Self::send_message(hwnd, LB_GETTEXTLEN, i as usize, 0);
                                if len > 0 {
                                    let mut buf = vec![0; len as usize];
                                    let len = Self::send_message(hwnd, LB_GETTEXT, i as usize, buf.as_mut_ptr() as isize);
                                    let name = String::from_utf16_lossy(&buf[0..len as usize]);
                                    gi.add(name);
                                }
                            }
                        }
                    },
                    TargetClass::ComboBox => {
                        if gi.is_nth_list() {
                            for i in 0..Self::send_message(hwnd, CB_GETCOUNT, 0, 0) {
                                let len = Self::send_message(hwnd, CB_GETLBTEXTLEN, i as usize, 0);
                                if len > 0 {
                                    let mut buf = vec![0; len as usize];
                                    let len = Self::send_message(hwnd, CB_GETLBTEXT, i as usize, buf.as_mut_ptr() as isize);
                                    let name = String::from_utf16_lossy(&buf[0..len as usize]);
                                    gi.add(name);
                                }
                            }
                        }
                    },
                    TargetClass::Tab => {
                        if let Some(tab) = TabControl::new(hwnd) {
                            for i in 0..Self::send_message(hwnd, TCM_GETITEMCOUNT, 0, 0) {
                                if let Some(name) = tab.get_name(i as usize) {
                                    gi.add(name);
                                }
                            }
                        }
                    },
                    TargetClass::TreeView => {
                        if gi.is_nth_treeview() {
                            if let Some(tv) = TreeView::new(hwnd) {
                                tv.get_names(None, gi);
                            }
                        }
                    },
                    TargetClass::ListView => {
                        if gi.is_nth_listview() {
                            if let Some(lv) = ListView::new(hwnd) {
                                lv.get_names(gi);
                            }
                        }
                    },
                    TargetClass::ToolBar => {
                        if let Some(tb) = ToolBar::new(hwnd) {
                            tb.get_names(gi);
                        }
                    },
                    TargetClass::Link => {
                        SysLink::new(hwnd).get_names(gi);
                    },
                    TargetClass::Edit |
                    TargetClass::Static => {
                        if let Some(text) = Self::get_text(hwnd) {
                            gi.add(text);
                        }
                    },
                    TargetClass::StatusBar => {
                        StatusBar::new(hwnd).get_names(gi);
                    },
                    // なにもしない
                    TargetClass::Menu |
                    TargetClass::ScrollBar |
                    TargetClass::TrackBar |
                    TargetClass::Other(_) => {},
                }
            }
            true.into()
        }
    }

    pub fn getslctlst(hwnd: HWND, nth: u32, column: isize) -> Vec<String> {
        let mut gsl = GetSlctLst::new(nth, column);
        let lparam = LPARAM(&mut gsl as *mut GetSlctLst as isize);
        unsafe {
            EnumChildWindows(hwnd, Some(Self::getslctlst_callback), lparam);
        }
        gsl.found
    }
    unsafe extern "system"
    fn getslctlst_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if let HWND(0) = hwnd {
            false.into()
        } else {
            if ! Self::is_visible(hwnd) {
                return true.into();
            }
            let gsl = &mut *(lparam.0 as *mut GetSlctLst);
            let target_class = TargetClass::from(get_class_name(hwnd));
            match target_class {
                TargetClass::ComboBox => if gsl.is_nth_control() {
                    let index = Self::send_message(hwnd, CB_GETCURSEL, 0, 0);
                    if index > -1 {
                        let len = Self::send_message(hwnd, CB_GETLBTEXTLEN, index as usize, 0);
                        if len > 0 {
                            let mut buf = vec![0; len as usize];
                            let len = Self::send_message(hwnd, CB_GETLBTEXT, index as usize, buf.as_mut_ptr() as isize);
                            let name = String::from_utf16_lossy(&buf[0..len as usize]);
                            gsl.add(name);
                            return false.into();
                        }
                    }
                },
                TargetClass::List => if gsl.is_nth_control() {
                    let count = Self::send_message(hwnd, LB_GETSELCOUNT, 0, 0);
                    let indexes = if count < 0 {
                        // 単一選択
                        let index = Self::send_message(hwnd, LB_GETCURSEL, 0, 0);
                        if index > -1 {
                            vec![index]
                        } else {
                            vec![]
                        }
                    } else {
                        // 複数選択
                        let mut buf = vec![0i32; count as usize];
                        if Self::send_message(hwnd, LB_GETSELITEMS, count as usize, buf.as_mut_ptr() as isize) > -1 {
                            buf.into_iter().map(|n| n as isize).collect()
                        } else {
                            vec![]
                        }
                    };
                    for index in indexes {
                        let len = Self::send_message(hwnd, LB_GETTEXTLEN, index as usize, 0);
                        if len > 0 {
                            let mut buf = vec![0; len as usize];
                            let len = Self::send_message(hwnd, LB_GETTEXT, index as usize, buf.as_mut_ptr() as isize);
                            let name = String::from_utf16_lossy(&buf[0..len as usize]);
                            gsl.add(name);
                        }
                    }
                    return false.into();
                },
                TargetClass::ListView => if gsl.is_nth_control() {
                    if let Some(lv) = ListView::new(hwnd) {
                        let selected = lv.get_selected(gsl.column);
                        if selected.len() > 0 {
                            gsl.extend(selected);
                        }
                    }
                },
                TargetClass::TreeView => if gsl.is_nth_control() {
                    if let Some(tv) = TreeView::new(hwnd) {
                        if let Some(hitem) = tv.get_selected() {
                            if let Some(value) = tv.get_name(hitem) {
                                gsl.add(value);
                            }
                        }
                    }
                },
                _ => {},
            }
            true.into()
        }
    }
}

enum ListIndex {
    Index(usize),
    Multi(Vec<usize>),
    None
}

#[derive(Debug)]
pub struct SearchItem {
    name: String,
    short: bool,
    target: Vec<TargetClass>,
    order: u32,
    found: Option<ItemFound>,
}
impl SearchItem {
    pub fn from_clkitem(item: &ClkItem) -> Self {
        let mut si = Self {
            name: item.name.to_string(),
            short: item.short,
            target: vec![],
            order: item.order,
            found: None,
        };
        if item.target.button {
            si.target.push(TargetClass::Button);
        }
        if item.target.list {
            si.target.push(TargetClass::List);
            si.target.push(TargetClass::ComboBox);
        }
        if item.target.tab {
            si.target.push(TargetClass::Tab);
        }
        if item.target.menu {
            si.target.push(TargetClass::Menu);
        }
        if item.target.treeview {
            si.target.push(TargetClass::TreeView);
        }
        if item.target.listview {
            si.target.push(TargetClass::ListView);
            // si.target.push(TargetClass::ListViewHeader);
        }
        if item.target.toolbar {
            si.target.push(TargetClass::ToolBar);
        }
        if item.target.link {
            si.target.push(TargetClass::Link);
        }
        si
    }
    fn matches(&mut self, other: &str) -> bool {
        if match_title(other, &self.name, self.short) {
            self.is_in_exact_order()
        } else {
            false
        }
    }
    fn is_in_exact_order(&mut self) -> bool {
        self.order -= 1;
        self.order < 1
    }
    pub fn new_slider(order: u32) -> Self {
        Self {
            name: String::new(),
            short: false,
            target: vec![TargetClass::ScrollBar, TargetClass::TrackBar],
            order,
            found: None,
        }
    }
}

#[derive(Debug)]
struct ItemFound {
    hwnd: HWND,
    target: TargetClass,
    info: ItemInfo,
}
impl ItemFound {
    fn new(hwnd: HWND, target: TargetClass, info: ItemInfo) -> Self {
        Self { hwnd, target, info }
    }
}
#[derive(Debug)]
enum ItemInfo {
    None,
    Index(usize),
    Indexes(Vec<usize>),
    /// id, チェック状態, x, y
    Menu(usize, bool, i32, i32),
    HItem(isize, u32),
    ListView(i32, i32, ListView),
    ListViewHeader(i32, u32),
    /// index, id, pid
    ToolBar(usize, usize, u32),
    TrackBar,
    ScrollBar(SCROLLINFO, SCROLLBAR_CONSTANTS, (i32, i32)),
}

struct GetItem {
    /// 取得対象クラス
    target: Vec<TargetClass>,
    /// リスト、リストビュー、ツリービューでn番目を取得、-1なら全部
    nth_list: i32,
    nth_listview: i32,
    nth_treevie: i32,
    /// リストビューのカラム、0ならカラム名
    column: i32,
    /// ディセーブルは無視
    ignore_disabled: bool,
    /// 見つかった文字列
    found: Vec<String>,
}
impl GetItem {
    fn new(t: u32, nth: i32, column: i32, ignore_disabled: bool) -> Self {
        use super::GetItemConst;
        let mut target = vec![];
        if t.includes(GetItemConst::ITM_BTN) {target.push(TargetClass::Button);}
        if t.includes(GetItemConst::ITM_LIST) {
            target.push(TargetClass::List);
            target.push(TargetClass::ComboBox);
        }
        if t.includes(GetItemConst::ITM_TAB) {target.push(TargetClass::Tab);}
        if t.includes(GetItemConst::ITM_MENU) {target.push(TargetClass::Menu);}
        if t.includes(GetItemConst::ITM_TREEVIEW) {target.push(TargetClass::TreeView);}
        if t.includes(GetItemConst::ITM_LISTVIEW) {target.push(TargetClass::ListView);}
        if t.includes(GetItemConst::ITM_EDIT) {target.push(TargetClass::Edit);}
        if t.includes(GetItemConst::ITM_STATIC) {target.push(TargetClass::Static);}
        if t.includes(GetItemConst::ITM_STATUSBAR) {target.push(TargetClass::StatusBar);}
        if t.includes(GetItemConst::ITM_TOOLBAR) {target.push(TargetClass::ToolBar);}
        if t.includes(GetItemConst::ITM_LINK) {target.push(TargetClass::Link);}
        let nth_list = nth;
        let nth_listview = nth;
        let nth_treevie = nth;
        Self { target, column, ignore_disabled, found: vec![], nth_list, nth_listview, nth_treevie }
    }

    fn add(&mut self, name: String) {
        if name.len() > 0 {
            self.found.push(name)
        }
    }
    fn is_nth_list(&mut self) -> bool {
        if self.nth_list < 0 {
            // -1以下なら必ずtrue
            true
        } else if self.nth_list > 0 {
            // 1以上なら1引いて0になれば該当のもの
            self.nth_list -= 1;
            self.nth_list == 0
        } else {
            // 0の場合
            false
        }
    }
    fn is_nth_listview(&mut self) -> bool {
        if self.nth_listview < 0 {
            // -1以下なら必ずtrue
            true
        } else if self.nth_listview > 0 {
            // 1以上なら1引いて0になれば該当のもの
            self.nth_listview -= 1;
            self.nth_listview == 0
        } else {
            // 0の場合
            false
        }
    }
    fn is_nth_treeview(&mut self) -> bool {
        if self.nth_treevie < 0 {
            // -1以下なら必ずtrue
            true
        } else if self.nth_treevie > 0 {
            // 1以上なら1引いて0になれば該当のもの
            self.nth_treevie -= 1;
            self.nth_treevie == 0
        } else {
            // 0の場合
            false
        }
    }
}

struct GetSlctLst {
    nth: u32,
    column: isize,
    found: Vec<String>,
}
impl GetSlctLst {
    fn new(nth: u32, column: isize) -> Self {
        Self { nth, column, found: vec![] }
    }
    fn add(&mut self, value: String) {
        self.found.push(value);
    }
    fn extend(&mut self, values: Vec<String>) {
        self.found.extend(values)
    }
    fn is_nth_control(&mut self) -> bool {
        if self.nth > 0 {
            self.nth -= 1;
        }
        self.nth == 0
    }
}

#[derive(Debug, PartialEq)]
enum TargetClass {
    /// button
    Button,
    /// listbox
    List,
    /// combobox
    ComboBox,
    /// systabcontrol32
    Tab,
    Menu,
    /// systreeview32
    TreeView,
    /// syslistview32
    ListView,
    /// toolbarwindow32
    ToolBar,
    /// syslink
    Link,
    /// scrollbar
    ScrollBar,
    /// msctls_trackbar32
    TrackBar,
    /// edit
    Edit,
    /// static
    Static,
    /// msctls_statusbar32
    StatusBar,
    Other(String),
}

// impl std::fmt::Display for TargetClass {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             TargetClass::Button => write!(f, "{}", WC_BUTTON),
//             TargetClass::List => write!(f, "{}", WC_LISTBOX),
//             TargetClass::ComboBox => write!(f, "{}", WC_COMBOBOX),
//             TargetClass::Tab => write!(f, "{}", WC_TABCONTROL),
//             TargetClass::Menu => write!(f, "#32768"),
//             TargetClass::TreeView => write!(f, "{}", WC_TREEVIEW),
//             TargetClass::ListView => write!(f, "{}", WC_LISTVIEW),
//             // TargetClass::ListViewHeader => write!(f, "{}", WC_HEADER),
//             TargetClass::ToolBar => write!(f, "{}", TOOLBARCLASSNAME),
//             TargetClass::Link => write!(f, "{}", WC_LINK),
//             TargetClass::Other(s) => write!(f, "{s}"),
//         }
//     }
// }

impl From<String> for TargetClass {
    fn from(s: String) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "button" => Self::Button,
            "listbox" => Self::List,
            "combobox" => Self::ComboBox,
            "systabcontrol32" => Self::Tab,
            "#32768" => Self::Menu,
            "systreeview32" => Self::TreeView,
            "syslistview32" => Self::ListView,
            "toolbarwindow32" => Self::ToolBar,
            "syslink" => Self::Link,
            "scrollbar" => Self::ScrollBar,
            "msctls_trackbar32" => Self::TrackBar,
            "edit" => Self::Edit,
            "static" => Self::Static,
            "msctls_statusbar32" => Self::StatusBar,
            _ => Self::Other(s),
        }
    }
}

enum ButtonType {
    Button,
    Check,
    Radio,
    ThreeState,
}

impl From<HWND> for ButtonType {
    fn from(h: HWND) -> Self {
        let style = get_window_style(h);
        match style & (BS_CHECKBOX|BS_AUTOCHECKBOX|BS_RADIOBUTTON|BS_AUTORADIOBUTTON|BS_3STATE|BS_AUTO3STATE) {
            BS_3STATE |
            BS_AUTO3STATE => Self::ThreeState,
            BS_CHECKBOX |
            BS_AUTOCHECKBOX => Self::Check,
            BS_RADIOBUTTON |
            BS_AUTORADIOBUTTON  => Self::Radio,
            _ => Self::Button
        }
    }
}

struct ProcessMemory<T> {
    pub hprocess: HANDLE,
    pub pointer: *mut c_void,
    phantom: PhantomData<T>
}

impl<T> ProcessMemory<T> {
    fn new(pid: u32, value: Option<&T>) -> Option<Self> {
        let maybe_pm = Self::new2(pid, mem::size_of::<T>());
        if let Some(value) = value {
            if let Some(pm) = &maybe_pm {
                pm.write(value);
            }
        }
        maybe_pm
    }
    fn new2(pid: u32, dwsize: usize) -> Option<Self> {
        unsafe {
            let dwdesiredaccess = PROCESS_VM_READ|PROCESS_VM_WRITE|PROCESS_VM_OPERATION|PROCESS_QUERY_INFORMATION;
            let hprocess = OpenProcess(dwdesiredaccess, false, pid).ok()?;
            let pointer = VirtualAllocEx(hprocess, None, dwsize, MEM_COMMIT, PAGE_READWRITE);
            let pm = Self { hprocess, pointer, phantom: PhantomData };
            Some(pm)
        }
    }
    fn write(&self, value: &T) -> bool {
        unsafe {
            let lpbuffer = value as *const T as *const c_void;
            let nsize = mem::size_of::<T>();
            WriteProcessMemory(self.hprocess, self.pointer, lpbuffer, nsize, None).is_ok()
        }
    }
    fn write2<U>(&self, value: U) -> bool {
        unsafe {
            let lpbuffer = &value as *const U as *const c_void;
            let nsize = mem::size_of::<T>();
            if nsize < mem::size_of::<U>() {
                // 書き込むデータのサイズが確保したメモリサイズを超えたらダメ
                false
            } else {
                WriteProcessMemory(self.hprocess, self.pointer, lpbuffer, nsize, None).is_ok()
            }
        }
    }
    fn read(&self, buf: &mut T) {
        let lpbuffer = buf as *mut T as *mut c_void;
        let nsize = mem::size_of::<T>();
        unsafe {
            let _ = ReadProcessMemory(self.hprocess, self.pointer, lpbuffer, nsize, None);
        }
    }
    fn _as_ptr(&self) -> *mut T {
        self.pointer as *mut T
    }
}

impl<T> Drop for ProcessMemory<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.hprocess, self.pointer, 0, MEM_RELEASE);
            let _ = CloseHandle(self.hprocess);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TargetArch {
    X86,
    X64,
}
struct TabControl {
    hwnd: HWND,
    pid: u32,
    target_arch: TargetArch,
    is_unicode: bool,
}
impl TabControl {
    fn new(hwnd: HWND) -> Option<Self> {
        let is_unicode = Win32::send_message(hwnd, TCM_GETUNICODEFORMAT, 0, 0) != 0;
        let pid = get_process_id_from_hwnd(hwnd);
        // // 対象プロセスと自身のアーキテクチャを確認
        let target_arch = if Win32::is_process_x64(pid)? {
            TargetArch::X64
        } else {
            TargetArch::X86
        };
        Some(Self { hwnd, pid, target_arch, is_unicode })
    }
    fn get_name(&self, index: usize) -> Option<String> {
        if self.is_unicode {
            // タブ名を受けるバッファ
            let mut buf = [0; 260];
            // 対象プロセス上に同様のバッファを作成
            let remote_buf = ProcessMemory::new(self.pid, None)?;
            // リモートバッファに名前を受ける
            self.get_remote_name(index, remote_buf.pointer, buf.len() as i32, TCM_GETITEMW)?;
            // 対象プロセスのバッファからローカルのバッファに読み出し
            remote_buf.read(&mut buf);
            Some(String::from_utf16_lossy(&buf))
        } else {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new(self.pid, None)?;
            self.get_remote_name(index, remote_buf.pointer, buf.len() as i32, TCM_GETITEMA)?;
            remote_buf.read(&mut buf);
            Some(from_ansi_bytes(&buf))
        }
    }
    fn get_remote_name(&self, index: usize, p_buffer: *mut c_void, len: i32, msg: u32) -> Option<()> {
        match self.target_arch {
            TargetArch::X86 => {
                let mut tcitem = TCITEM86::default();
                tcitem.mask = TCIF_TEXT.0;
                tcitem.cchTextMax = len;
                tcitem.pszText = p_buffer as i32;
                let remote_item = ProcessMemory::new(self.pid, Some(&tcitem))?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, index, lparam);
            },
            TargetArch::X64 => {
                let mut tcitem = TCITEM64::default();
                tcitem.mask = TCIF_TEXT.0;
                tcitem.cchTextMax = len;
                tcitem.pszText = p_buffer as i64;
                let remote_item = ProcessMemory::new(self.pid, Some(&tcitem))?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, index, lparam);
            },
        }
        Some(())
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TCITEM64 {
    pub mask: u32,
    pub dwState: u32,
    pub dwStateMask: u32,
    pub pszText: i64,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: i64,
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TCITEM86 {
    pub mask: u32,
    pub dwState: u32,
    pub dwStateMask: u32,
    pub pszText: i32,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: i32,
}

#[derive(Debug)]
struct Menu {
    hwnd: HWND,
    // pid: u32,
    is_unicode: bool,
}
impl Menu {
    fn new(hwnd: HWND) -> Self {
        // let pid = get_process_id_from_hwnd(hwnd);
        let is_unicode = Win32::is_window_unicode(hwnd);
        Self { hwnd, is_unicode }
    }
    fn click(hwnd: HWND, id: usize) -> bool {
        if id > 0 {
            Win32::send_message(hwnd, WM_COMMAND, id, 0);
            true
        } else {
            false
        }
    }
    fn search(&self, item: &mut SearchItem, hmenu: Option<HMENU>, path: Option<String>) {
        let hmenu = hmenu.unwrap_or(unsafe { GetMenu(self.hwnd) });
        unsafe {
            let cnt = GetMenuItemCount(hmenu);
            for npos in 0..cnt {
                let (name, checked) = self.get_name_and_check(hmenu, npos as u32);
                let path = if let Some(path) = path.as_ref() {
                    format!("{path}\\{name}")
                } else {
                    name.clone()
                };
                let found = if item.name.contains('\\') {
                    item.matches(&path)
                } else {
                    item.matches(&name)
                };
                if found {
                    let id = GetMenuItemID(hmenu, npos) as usize;
                    let mut rect = RECT::default();
                    let _ = GetMenuItemRect(self.hwnd, hmenu, npos as u32, &mut rect);
                    let (x, y) = Win32::get_center(rect);
                    let (x, y) = Win32::client_to_screen(self.hwnd, x, y);
                    item.found = Some(ItemFound::new(self.hwnd, TargetClass::Menu, ItemInfo::Menu(id, checked, x, y)));
                    return;
                }
                let sub = GetSubMenu(hmenu, npos);
                if ! sub.is_invalid() {
                    self.search(item, Some(sub), Some(path));
                }
                if item.found.is_some() {
                    return;
                }
            }
        }
    }
    fn get_names(&self, hmenu: Option<HMENU>, gi: &mut GetItem) {
        unsafe {
            let hmenu = hmenu.unwrap_or(GetMenu(self.hwnd));
            for npos in 0..GetMenuItemCount(hmenu) {
                let (name, _) = self.get_name_and_check(hmenu, npos as u32);
                gi.add(name);
                let sub = GetSubMenu(hmenu, npos);
                if ! sub.is_invalid() {
                    self.get_names(Some(sub), gi);
                }
            }
        }
    }
    fn get_name_and_check(&self, hmenu: HMENU, npos: u32) -> (String, bool) {
        if self.is_unicode {
            Self::get_name_w(hmenu, npos)
        } else {
            Self::get_name_a(hmenu, npos)
        }
    }
    fn get_name_w(hmenu: HMENU, npos: u32) -> (String, bool) {
        let mut buf = [0; 260];
        let mut info = MENUITEMINFOW::default();
        info.cbSize = mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_TYPE|MIIM_STATE;
        info.cch = buf.len() as u32;
        info.dwTypeData = PWSTR::from_raw(buf.as_mut_ptr());
        let _ = unsafe { GetMenuItemInfoW(hmenu, npos, true, &mut info) };
        let checked = (info.fState & MFS_CHECKED) == MFS_CHECKED;
        let name = from_wide_string(&buf);
        (name, checked)
    }
    fn get_name_a(hmenu: HMENU, npos: u32) -> (String, bool) {
        let mut buf = [0; 260];
        let mut info = MENUITEMINFOA::default();
        info.cbSize = mem::size_of::<MENUITEMINFOA>() as u32;
        info.fMask = MIIM_TYPE|MIIM_STATE;
        info.cch = buf.len() as u32;
        info.dwTypeData = PSTR::from_raw(buf.as_mut_ptr());
        let _ = unsafe { GetMenuItemInfoA(hmenu, npos, true, &mut info) };
        let checked = (info.fState & MFS_CHECKED) == MFS_CHECKED;
        let name = from_ansi_bytes(&buf);
        (name, checked)
    }
}

struct TreeView {
    hwnd: HWND,
    pid: u32,
    target_arch: TargetArch,
    is_unicode: bool,
}

impl TreeView {
    fn new(hwnd: HWND) -> Option<Self> {
        let is_unicode = Win32::send_message(hwnd, TVM_GETUNICODEFORMAT, 0, 0) != 0;
        let pid = get_process_id_from_hwnd(hwnd);
        let target_arch = if Win32::is_process_x64(pid)? {
            TargetArch::X64
        } else {
            TargetArch::X86
        };
        let tv = Self { hwnd, is_unicode, pid, target_arch };
        Some(tv)
    }
    fn search(&self, item: &mut SearchItem, hitem: Option<isize>, path: Option<String>) {
        let hitem = hitem.unwrap_or(self.get_root());
        if let Some(name) = self.get_name(hitem) {
            let new_path = match path.as_ref() {
                Some(p) => format!("{p}\\{name}"),
                None => name.clone(),
            };
            let found = if item.name.contains('\\') {
                item.matches(&new_path)
            } else {
                item.matches(&name)
            };
            if found {
                item.found = Some(ItemFound::new(self.hwnd, TargetClass::TreeView, ItemInfo::HItem(hitem, self.pid)));
            } else {
                // 子を探す
                if let Some(child) = self.get_child(hitem) {
                    self.search(item, Some(child), Some(new_path));
                    if item.found.is_some() {
                        // 子から見つかれば終了
                        return;
                    }
                }
                // 次の要素を探す
                if let Some(next) = self.get_next(hitem) {
                    self.search(item, Some(next), path);
                }
            }
        }
    }
    fn get_names(&self, hitem: Option<isize>, gi: &mut GetItem) {
        let hitem = hitem.unwrap_or(self.get_root());
        if let Some(name) = self.get_name(hitem) {
            gi.add(name);
            if let Some(child) = self.get_child(hitem) {
                self.get_names(Some(child), gi);
            }
            if let Some(next) = self.get_next(hitem) {
                self.get_names(Some(next), gi);
            }
        }
    }
    fn get_root(&self) -> isize {
        Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_ROOT as usize, 0)
    }
    fn get_child(&self, hitem: isize) -> Option<isize> {
        let h = Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_CHILD as usize, hitem);
        if h != 0 { Some(h) } else { None }
    }
    fn get_next(&self, hitem: isize) -> Option<isize> {
        let h = Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_NEXT as usize, hitem);
        if h != 0 { Some(h) } else { None }
    }
    fn get_selected(&self) -> Option<isize> {
        let h = Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_NEXTSELECTED as usize, 0);
        if h != 0 { Some(h) } else { None }
    }
    fn get_point(hwnd: HWND, pid: u32, hitem: isize) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        let remote = ProcessMemory::new(pid, None)?;
        // RECTの領域にアイテム識別番号を書き込む
        remote.write2(hitem);
        let lparam = remote.pointer as isize;
        Win32::send_message(hwnd, TVM_GETITEMRECT, 1, lparam);
        remote.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let (x, y) = Win32::client_to_screen(hwnd, x, y);
        Some((x, y))
    }
    fn get_name(&self, hitem: isize) -> Option<String> {
        if self.is_unicode {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new(self.pid, None)?;

            self.get_remote_name(hitem, remote_buf.pointer, buf.len() as i32, TVM_GETITEMW)?;

            remote_buf.read(&mut buf);
            let name = from_wide_string(&buf);
            Some(name)
        } else {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new(self.pid, None)?;

            self.get_remote_name(hitem, remote_buf.pointer, buf.len() as i32, TVM_GETITEMA)?;

            remote_buf.read(&mut buf);
            let name = from_ansi_bytes(&buf);
            Some(name)
        }
    }
    fn get_remote_name(&self, hitem: isize, pbuf: *mut c_void, len: i32, msg: u32) -> Option<()> {
        let res = match self.target_arch {
            TargetArch::X86 => {
                let mut tvitem = TVITEM86::default();
                tvitem.mask = (TVIF_HANDLE|TVIF_TEXT).0;
                tvitem.hItem = hitem as i32;
                tvitem.cchTextMax = len;
                tvitem.pszText = pbuf as i32;
                let remote_item = ProcessMemory::new(self.pid, Some(&tvitem))?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, 0, lparam) != 0
            },
            TargetArch::X64 => {
                let mut tvitem = TVITEM64::default();
                tvitem.mask = (TVIF_HANDLE|TVIF_TEXT).0;
                tvitem.hItem = hitem as i64;
                tvitem.cchTextMax = len;
                tvitem.pszText = pbuf as i64;
                let remote_item = ProcessMemory::new(self.pid, Some(&tvitem))?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, 0, lparam) != 0
            },
        };
        if res { Some(()) } else { None }
    }
    fn click(hwnd: HWND, hitem: isize) -> bool {
        Win32::send_message(hwnd, TVM_SELECTITEM, TVGN_CARET as usize, hitem) != 0
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TVITEM86 {
    mask: u32,
    hItem: i32,
    state: u32,
    stateMask: u32,
    pszText: i32,
    cchTextMax: i32,
    iImage: i32,
    iSelectedImage: i32,
    cChildren: i32,
    lParam: i32,
}
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TVITEM64 {
    mask: u32,
    hItem: i64,
    state: u32,
    stateMask: u32,
    pszText: i64,
    cchTextMax: i32,
    iImage: i32,
    iSelectedImage: i32,
    cChildren: i32,
    lParam: i64,
}

#[derive(Debug, Clone)]
struct ListView {
    hwnd: HWND,
    pid: u32,
    target_arch: TargetArch,
    is_unicode: bool,
}
impl ListView {
    fn new(hwnd: HWND) -> Option<Self> {
        let is_unicode = Win32::send_message(hwnd, LVM_GETUNICODEFORMAT, 0, 0) != 0;
        let pid = get_process_id_from_hwnd(hwnd);
        let target_arch = if Win32::is_process_x64(pid)? {TargetArch::X64} else {TargetArch::X86};
        Some(Self { hwnd, is_unicode, pid, target_arch })
    }
    fn search(&self, item: &mut SearchItem) {
        let header  = Win32::send_message(self.hwnd, LVM_GETHEADER, 0, 0);
        let h_header = HWND(header);
        let columns = Win32::send_message(h_header, HDM_GETITEMCOUNT, 0, 0);
        // ヘッダの検索
        for column in 0..columns as i32 {
            if let Some(text) = self.get_header_name(h_header, column) {
                if item.matches(&text) {
                    let info = ItemInfo::ListViewHeader(column, self.pid);
                    item.found = Some(ItemFound::new(h_header, TargetClass::ListView, info));
                    return;
                }
            }
        }
        // 行ごとのカラムを検索
        let rows = Win32::send_message(self.hwnd, LVM_GETITEMCOUNT, 0, 0);
        'row: for row in 0..rows {
            for column in 0..columns {
                if let Some(text) = self.get_name(row, column) {
                    if item.matches(&text) {
                        let info = ItemInfo::ListView(row as i32, column as i32, self.clone());
                        item.found = Some(ItemFound::new(self.hwnd, TargetClass::ListView, info));
                        break 'row;
                    }
                }
            }
        }
    }
    fn get_names(&self, gi: &mut GetItem) {
        let column = gi.column;
        let header  = Win32::send_message(self.hwnd, LVM_GETHEADER, 0, 0);
        let hwnd = HWND(header);
        let columns = Win32::send_message(hwnd, HDM_GETITEMCOUNT, 0, 0);
        if column < 0 {
            // ヘッダ名
            for index in 0..columns {
                if let Some(name) = self.get_header_name(hwnd, index as i32) {
                    gi.add(name);
                }
            }
        } else {
            let rows = Win32::send_message(self.hwnd, LVM_GETITEMCOUNT, 0, 0);
            for row in 0..rows {
                if column == 0 {
                    for index in 0..columns {
                        if let Some(name) = self.get_name(row, index) {
                            gi.add(name);
                        }
                    }
                } else {
                    let index = column as isize - 1;
                    if let Some(name) = self.get_name(row, index) {
                        gi.add(name);
                    }
                }
            }
        }
    }
    fn get_selected(&self, column: isize) -> Vec<String> {
        let mut selected = vec![];
        let mut index = LVITEMINDEX::default();
        index.iItem = -1;
        if let Some(remote) = ProcessMemory::new(self.pid, Some(&index)) {
            loop {
                if Win32::send_message(self.hwnd, LVM_GETNEXTITEMINDEX, remote.pointer as usize, LVNI_SELECTED as isize) != 0 {
                    remote.read(&mut index);
                    if let Some(value) = self.get_name(index.iItem as isize, column) {
                        selected.push(value);
                    }
                } else {
                    break
                }
            }
        }
        selected
    }
    fn get_name(&self, row: isize, column: isize) -> Option<String> {
        if self.is_unicode {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new(self.pid, None)?;
            self.get_remote_name(row, column, remote.pointer, buf.len() as i32, LVM_GETITEMTEXTW);
            remote.read(&mut buf);
            let text = from_wide_string(&buf);
            Some(text)
        } else {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new(self.pid, None)?;
            self.get_remote_name(row, column, remote.pointer, buf.len() as i32, LVM_GETITEMTEXTA);
            remote.read(&mut buf);
            let text = from_ansi_bytes(&buf);
            Some(text)
        }
    }
    fn get_remote_name(&self, row: isize, column: isize, pbuf: *mut c_void, len: i32, msg: u32) -> Option<()> {
        let n = match self.target_arch {
            TargetArch::X64 => {
                let mut lvitem = LVITEM64::default();
                lvitem.mask = LVIF_TEXT.0;
                lvitem.iItem = row as i32;
                lvitem.iSubItem = column as i32;
                lvitem.pszText = pbuf as i64;
                lvitem.cchTextMax = len;
                let remote = ProcessMemory::new(self.pid, Some(&lvitem))?;
                let lparam = remote.pointer as isize;
                Win32::send_message(self.hwnd, msg, row as usize, lparam)
            },
            TargetArch::X86 => {
                let mut lvitem = LVITEM86::default();
                lvitem.mask = LVIF_TEXT.0;
                lvitem.iItem = row as i32;
                lvitem.iSubItem = column as i32;
                lvitem.pszText = pbuf as i32;
                lvitem.cchTextMax = len;
                let remote = ProcessMemory::new(self.pid, Some(&lvitem))?;
                let lparam = remote.pointer as isize;
                Win32::send_message(self.hwnd, msg, row as usize, lparam)
            },
        };
        if n > 0 {Some(())} else {None}
    }
    fn get_point(&self, row: i32, column: i32) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        rect.top = column;
        let remote = ProcessMemory::new(self.pid, Some(&rect))?;
        let lparam = remote.pointer as isize;
        Win32::send_message(self.hwnd, LVM_GETSUBITEMRECT, row as usize, lparam);
        remote.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let (x, y) = Win32::client_to_screen(self.hwnd, x, y);
        Some((x, y))
    }
    fn get_header_name(&self, hwnd: HWND, index: i32) -> Option<String> {
        if self.is_unicode {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new(self.pid, None)?;
            self.get_remote_header_name(hwnd, index, remote.pointer, buf.len() as i32, HDM_GETITEMW);
            remote.read(&mut buf);
            let text = from_wide_string(&buf);
            Some(text)
        } else {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new(self.pid, None)?;
            self.get_remote_header_name(hwnd, index, remote.pointer, buf.len() as i32, HDM_GETITEMA);
            remote.read(&mut buf);
            let text = from_ansi_bytes(&buf);
            Some(text)
        }
    }
    fn get_remote_header_name(&self, hwnd: HWND, index: i32, pbuf: *mut c_void, len: i32, msg: u32) -> Option<()> {
        let result = match self.target_arch {
            TargetArch::X64 => {
                let mut hditem = HDITEM64::default();
                hditem.mask = HDI_TEXT.0;
                hditem.cchTextMax = len;
                hditem.pszText = pbuf as i64;
                let remote = ProcessMemory::new(self.pid, Some(&hditem))?;
                let wparam = index as usize;
                let lparam = remote.pointer as isize;
                Win32::send_message(hwnd, msg, wparam, lparam)
            },
            TargetArch::X86 => {
                let mut hditem = HDITEM86::default();
                hditem.mask = HDI_TEXT.0;
                hditem.cchTextMax = len;
                hditem.pszText = pbuf as i32;
                let remote = ProcessMemory::new(self.pid, Some(&hditem))?;
                let wparam = index as usize;
                let lparam = remote.pointer as isize;
                Win32::send_message(hwnd, msg, wparam, lparam)
            },
        };
        if result != 0 {
            Some(())
        } else {
            None
        }
    }
    fn get_header_point(hwnd: HWND, index: i32, pid: u32) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        let remote = ProcessMemory::new(pid, None)?;
        let wparam = index as usize;
        let lparam = remote.pointer as isize;
        if Win32::send_message(hwnd, HDM_GETITEMRECT, wparam, lparam) != 0 {
            remote.read(&mut rect);
            let (x, y) = Win32::get_center(rect);
            let point = Win32::client_to_screen(hwnd, x, y);
            Some(point)
        } else {
            None
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct LVITEM64 {
    mask: u32,
    iItem: i32,
    iSubItem: i32,
    state: u32,
    stateMask: u32,
    pszText: i64,
    cchTextMax: i32,
    iImage: i32,
    lParam: i64,
    iIndent: i32,
    iGroupId: i32,
    cColumns: u32,
    puColumns: i32,
    piColFmt: i64,
    iGroup: i32,
}
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct LVITEM86 {
    mask: u32,
    iItem: i32,
    iSubItem: i32,
    state: u32,
    stateMask: u32,
    pszText: i32,
    cchTextMax: i32,
    iImage: i32,
    lParam: i32,
    iIndent: i32,
    iGroupId: i32,
    cColumns: u32,
    puColumns: i32,
    piColFmt: i32,
    iGroup: i32,
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct HDITEM64 {
    mask: u32,
    cxy: i32,
    pszText: i64,
    hbm: i64,
    cchTextMax: i32,
    fmt: i32,
    lParam: i64,
    iImage: i32,
    iOrder: i32,
    r#type: u32,
    pvFilter: i64,
    state: u32,
}
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct HDITEM86 {
    mask: u32,
    cxy: i32,
    pszText: i32,
    hbm: i32,
    cchTextMax: i32,
    fmt: i32,
    lParam: i32,
    iImage: i32,
    iOrder: i32,
    r#type: u32,
    pvFilter: i32,
    state: u32,
}

struct ToolBar {
    hwnd: HWND,
    pid: u32,
    target_arch: TargetArch,
    is_unicode: bool,
}

impl ToolBar {
    fn new(hwnd: HWND) -> Option<Self> {
        let is_unicode = Win32::send_message(hwnd, TB_GETUNICODEFORMAT, 0, 0) != 0;
        let pid = get_process_id_from_hwnd(hwnd);
        let target_arch = if Win32::is_process_x64(pid)? {TargetArch::X64} else {TargetArch::X86};
        Some(Self { hwnd, pid, is_unicode, target_arch })
    }
    fn search(&self, item: &mut SearchItem) {
        let cnt = Win32::send_message(self.hwnd, TB_BUTTONCOUNT, 0, 0);
        for i in 0..cnt as usize {
            let id = self.get_id(i);
            if let Some(name) = self.get_name(id) {
                if item.matches(&name) {
                    item.found = Some(ItemFound::new(self.hwnd, TargetClass::ToolBar, ItemInfo::ToolBar(i, id, self.pid)));
                    break;
                }
            }
        }
    }
    fn get_names(&self, gi: &mut GetItem) {
        let cnt = Win32::send_message(self.hwnd, TB_BUTTONCOUNT, 0, 0);
        for i in 0..cnt as usize {
            let id = self.get_id(i);
            if let Some(name) = self.get_name(id) {
                gi.add(name);
            }
        }
    }
    fn get_id(&self, index: usize) -> usize {
        // id取得を試みる、失敗したらインデックスをそのまま返す
        match self.target_arch {
            TargetArch::X64 => {
                if let Some(remote) = ProcessMemory::new(self.pid, None) {
                    let p = remote.pointer as isize;
                    if Win32::send_message(self.hwnd, TB_GETBUTTON, index, p) != 0 {
                        let mut tbbutton = TBBUTTON64::default();
                        remote.read(&mut tbbutton);
                        tbbutton.idCommand as usize
                    } else {
                        index
                    }
                } else {
                    index
                }
            },
            TargetArch::X86 => {
                if let Some(remote) = ProcessMemory::new(self.pid, None) {
                    let p = remote.pointer as isize;
                    if Win32::send_message(self.hwnd, TB_GETBUTTON, index, p) != 0 {
                        let mut tbbutton = TBBUTTON86::default();
                        remote.read(&mut tbbutton);
                        tbbutton.idCommand as usize
                    } else {
                        index
                    }
                } else {
                    index
                }
            },
        }
    }
    fn get_name(&self, id: usize) -> Option<String> {
        if self.is_unicode {
            let remote = ProcessMemory::new(self.pid, None)?;
            let lparam = remote.pointer as isize;
            if Win32::send_message(self.hwnd, TB_GETBUTTONTEXTW, id, lparam) > -1 {
                let mut buf = [0; 260];
                remote.read(&mut buf);
                let name = from_wide_string(&buf);
                Some(name)
            } else {
                None
            }
        } else {
            let remote = ProcessMemory::new(self.pid, None)?;
            let lparam = remote.pointer as isize;
            if Win32::send_message(self.hwnd, TB_GETBUTTONTEXTA, id, lparam) > -1 {
                let mut buf = [0; 260];
                remote.read(&mut buf);
                let name = from_ansi_bytes(&buf);
                Some(name)
            } else {
                None
            }
        }
    }
    fn get_point(hwnd: HWND, index: usize, pid: u32) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        let remote = ProcessMemory::new(pid, None)?;
        let lparam = remote.pointer as isize;
        Win32::send_message(hwnd, TB_GETITEMRECT, index, lparam);
        remote.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let point = Win32::client_to_screen(hwnd, x, y);
        Some(point)
    }
    fn click(hwnd: HWND, id: usize) -> bool {
        let parent = Win32::get_parent(hwnd);
        Win32::post_message(parent, WM_COMMAND, id, hwnd.0)
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TBBUTTON64 {
    iBitmap: i32,
    idCommand: i32,
    fsState: u8,
    fsStyle: u8,
    bReserved: [u8; 6],
    dwData: u64,
    iString: i64,
}
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
struct TBBUTTON86 {
    iBitmap: i32,
    idCommand: i32,
    fsState: u8,
    fsStyle: u8,
    bReserved: [u8; 2],
    dwData: u32,
    iString: i32,
}

pub enum Slider {
    /// parent, SCROLLINFO, 縦横, (X, Y)
    ScrollBar(HWND, SCROLLINFO, i32, (i32, i32)),
    /// trackbar, parent
    TrackBar(HWND, HWND)
}

impl Slider {
    pub fn set_pos(&self, pos: i32, smooth: bool) -> bool {
        match self {
            Slider::ScrollBar(hwnd, info, dir, _) => {
                let pos = pos.min(info.nMax).max(info.nMin);
                let msg = if *dir == 0 {WM_HSCROLL} else {WM_VSCROLL};
                if smooth {
                    let mut next = info.nPos;
                    let back = info.nPos > pos;
                    loop {
                        if back {next -= 1;} else {next += 1;}
                        let wparam = (SB_THUMBTRACK.0 | (next & 0xFFFF) << 16) as usize;
                        Win32::send_message(*hwnd, msg, wparam, 0);
                        if next == pos {
                            break;
                        }
                    }
                } else {
                    let wparam = (SB_THUMBTRACK.0 | (pos & 0xFFFF) << 16) as usize;
                    Win32::send_message(*hwnd, msg, wparam, 0);
                }
                true
            },
            Slider::TrackBar(hwnd, _) => {
                Win32::send_message(*hwnd, TBM_SETPOS, 1, pos as isize);
                let pid = get_process_id_from_hwnd(*hwnd);
                let Some(remote) = ProcessMemory::new(pid, None) else {
                    return false;
                };
                let lparam = remote.pointer as isize;
                Win32::send_message(*hwnd, TBM_GETTHUMBRECT, 0, lparam);
                let mut rect = RECT::default();
                remote.read(&mut rect);
                let (x, y) = Win32::get_center(rect);
                let point = Win32::client_to_screen(*hwnd, x, y);
                MouseInput::left_click(*hwnd, Some(point));
                true
            },
        }
    }
    pub fn get_pos(&self) -> i32 {
        match self {
            Self::ScrollBar(_, info, _, _) => info.nPos,
            Self::TrackBar(hwnd, _) => {
                let tbm_getpos = 1024;
                Win32::send_message(*hwnd, tbm_getpos, 0, 0) as i32
            },
        }
    }
    pub fn get_min(&self) -> i32 {
        match self {
            Self::ScrollBar(_, info, _, _) => info.nMin,
            Self::TrackBar(hwnd, _) => Win32::send_message(*hwnd, TBM_GETRANGEMIN, 0, 0) as i32,
        }
    }
    pub fn get_max(&self) -> i32 {
        match self {
            Self::ScrollBar(_, info, _, _) => info.nMax,
            Self::TrackBar(hwnd, _) => Win32::send_message(*hwnd, TBM_GETRANGEMAX, 0, 0) as i32,
        }
    }
    pub fn get_page(&self) -> i32 {
        match self {
            Self::ScrollBar(_, info, _, _) => info.nPage as i32,
            Self::TrackBar(hwnd, _) => Win32::send_message(*hwnd, TBM_GETPAGESIZE, 0, 0) as i32,
        }
    }
    pub fn get_bar(&self) -> i32 {
        match self {
            Self::ScrollBar(_, _, dir, _) => *dir as i32,
            Self::TrackBar(hwnd, _) => {
                let style = TBS_VERT as i32;
                if get_window_style(*hwnd) & style > 0 {1} else {0}
            },
        }
    }
    pub fn get_point(&self) -> (i32, i32) {
        match self {
            Self::ScrollBar(_, _, _, point) => *point,
            Self::TrackBar(hwnd, parent) => {
                unsafe {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(*hwnd, &mut rect);
                    let mut point = POINT { x: rect.left, y: rect.top };
                    ScreenToClient(*parent, &mut point);
                    (point.x, point.y)
                }
            },
        }
    }
    unsafe fn get_scrollbar_info(hwnd: HWND, nbar: SCROLLBAR_CONSTANTS) -> Option<SCROLLINFO> {
        let mut info = SCROLLINFO::default();
        info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
        info.fMask = SIF_ALL;
        GetScrollInfo(hwnd, nbar, &mut info).ok()?;
        if info.nPage > 0 {
            Some(info)
        } else {
            None
        }
    }
    unsafe fn get_scrollbar_point(hwnd: HWND, idobject: OBJECT_IDENTIFIER) -> (i32, i32) {
        let mut info = SCROLLBARINFO::default();
        info.cbSize = std::mem::size_of::<SCROLLBARINFO>() as u32;
        let _ = GetScrollBarInfo(hwnd, idobject, &mut info);
        (info.rcScrollBar.left, info.rcScrollBar.top)
    }
}

struct SysLink {
    hwnd: HWND,
    // pid: u32,
}

impl SysLink {
    fn new(hwnd: HWND) -> Self {
        // let pid = get_process_id_from_hwnd(hwnd);
        // Self { hwnd, pid }
        Self { hwnd }
    }
    fn get_names(&self, gi: &mut GetItem) {
        if let Some(text) = Win32::get_text(self.hwnd) {
            let fragment = scraper::Html::parse_fragment(&text);
            if let Ok(selector) = scraper::Selector::parse("a") {
                for element in fragment.select(&selector) {
                    let name = element.text().collect();
                    gi.add(name);
                }
            }
        }
    }
}

struct StatusBar {
    hwnd: HWND,
    pid: u32,
}
impl StatusBar {
    fn new(hwnd: HWND) -> Self {
        let pid = get_process_id_from_hwnd(hwnd);
        Self { hwnd, pid }
    }
    fn count(&self) -> usize {
        Win32::send_message(self.hwnd, SB_GETPARTS, 0, 0) as usize
    }
    fn get_str(&self, index: usize) -> Option<String> {
        let len = Win32::send_message(self.hwnd, SB_GETTEXTLENGTHW, index, 0);
        let len = len as usize & 0xFFFF; // 下位WORDがサイズ
        if len > 0 {
            let mut buf = [0u16; 260];
            let remote = ProcessMemory::new(self.pid, None)?;
            let len = Win32::send_message(self.hwnd, SB_GETTEXTW, index, remote.pointer as isize);
            let len = len as usize & 0xFFFF; // 下位WORDがサイズ
            remote.read(&mut buf);
            let str = String::from_utf16_lossy(&buf[..len]).trim().to_string();
            Some(str)
        } else {
            None
        }
    }
    fn get_rect(&self, index: usize) -> Option<RECT> {
        let mut rect = RECT::default();
        let remote = ProcessMemory::new(self.pid, None)?;
        Win32::send_message(self.hwnd, SB_GETRECT, index, remote.pointer as isize);
        remote.read(&mut rect);
        Some(rect)
    }
    fn get_names(&self, gi: &mut GetItem) {
        for index in 0..self.count() {
            if let Some(name) = self.get_str(index) {
                gi.add(name);
            }
        }
    }
}