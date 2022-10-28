
use windows::{
    core::{PWSTR, PSTR},
    Win32::{
        Foundation::{
            HWND, WPARAM, LPARAM, BOOL, RECT, HANDLE, POINT,
            CloseHandle,
        },
        UI::{
            Controls::{
                WC_BUTTON, WC_LISTBOX, WC_COMBOBOX, WC_TABCONTROL, WC_TREEVIEW, WC_LISTVIEW, WC_LINK, //WC_HEADER,
                TOOLBARCLASSNAME,
                BST_CHECKED, BST_UNCHECKED, BST_INDETERMINATE,
                TCM_GETITEMW, TCM_GETITEMA, TCM_GETITEMCOUNT, TCM_GETCURSEL, TCM_GETUNICODEFORMAT, TCM_GETITEMRECT, TCM_SETCURFOCUS,
                TCIF_TEXT,
                TVIF_HANDLE, TVIF_TEXT,
                TVM_GETUNICODEFORMAT, TVM_GETITEMA, TVM_GETITEMW, TVM_GETNEXTITEM, TVM_GETITEMRECT, TVM_SELECTITEM,
                TVGN_ROOT, TVGN_CHILD, TVGN_NEXT, TVGN_CARET,
                LVM_GETUNICODEFORMAT, LVM_GETHEADER, LVM_GETITEMCOUNT, LVM_GETITEMTEXTW, LVM_GETITEMTEXTA, LVM_GETSUBITEMRECT,
                LVIF_TEXT,
                HDM_GETITEMCOUNT, HDM_GETITEMW, HDM_GETITEMA, HDM_GETITEMRECT,
                HDI_TEXT,
            },
            WindowsAndMessaging::{
                WM_COMMAND,
                BN_CLICKED,
                BS_CHECKBOX, BS_AUTOCHECKBOX, BS_3STATE, BS_AUTO3STATE, BS_RADIOBUTTON, BS_AUTORADIOBUTTON,
                BM_SETCHECK, BM_GETCHECK,
                CB_GETCOUNT, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_SETCURSEL, CB_GETCURSEL, CB_SETEDITSEL,
                CBN_SELCHANGE,
                LB_GETCOUNT, LB_GETTEXT, LB_GETTEXTLEN, LB_SETCURSEL, LB_GETCURSEL, LB_GETITEMRECT,
                LBN_SELCHANGE,
                WS_DISABLED,
                EnumChildWindows, PostMessageW, GetDlgCtrlID, SendMessageW,
                IsWindowUnicode,
                HMENU, GetMenu, GetMenuItemCount, GetSubMenu, GetMenuItemID, GetMenuItemRect,
                MIIM_TYPE, MIIM_STATE, MENUITEMINFOW, GetMenuItemInfoW, MENUITEMINFOA, GetMenuItemInfoA,
                MFS_CHECKED,
            }
        },
        Graphics::Gdi::ClientToScreen,
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

use crate::winapi::{
    get_class_name, get_window_title, make_wparam, get_window_style,
    from_ansi_bytes, from_wide_string, make_word,
};
use crate::evaluator::builtins::{
    ThreeState,
};
use super::clkitem::{ClkItem, MouseInput, match_title, ClkResult};
use super::get_process_id_from_hwnd;

use std::mem;
use std::ptr;
use std::ffi::c_void;

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
            EnumChildWindows(self.hwnd, Some(Self::enum_child_callback), lparam);
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
            // println!("\u{001b}[33m[debug] target: {:#?}\u{001b}[0m", &target);
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
                    // TargetClass::ListViewHeader => {
                    //     todo!()
                    // },
                    TargetClass::ToolBar => {
                        todo!()
                    },
                    TargetClass::Link => {
                        todo!()
                    },
                    _ => {
                        let title = get_window_title(hwnd);
                        if item.matches(&title) {
                            item.found = Some(ItemFound::new(hwnd, target, ItemInfo::None));
                            return false.into();
                        }
                    }
                }
            }
            // 子要素のサーチ
            EnumChildWindows(hwnd, Some(Self::enum_child_callback), lparam);
            if item.found.is_some() {
                return false.into();
            }
            true.into()
        }
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
                        _ => ClkResult::failed()
                    },
                    TargetClass::Link => match found.info {
                        _ => ClkResult::failed()
                    },
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
                        todo!()
                    },
                    TargetClass::Link => {
                        todo!()
                    },
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

    fn post_wm_command(&self, hwnd: HWND, id: Option<i32>, command: u32) -> bool {
        let id = id.unwrap_or(Self::get_window_id(hwnd));
        let msg = command as u16;
        let wparam = make_wparam(id as u16, msg).0;
        let lparam = hwnd.0;
        Self::post_message(self.hwnd, WM_COMMAND, wparam, lparam)
    }

    fn post_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> bool {
        unsafe {
            PostMessageW(hwnd, msg, WPARAM(wparam), LPARAM(lparam)).as_bool()
        }
    }
    fn send_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
        unsafe {
            SendMessageW(hwnd, msg, WPARAM(wparam), LPARAM(lparam)).0
        }
    }
    fn get_window_id(hwnd: HWND) -> i32 {
        unsafe {
            GetDlgCtrlID(hwnd)
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
        for i in 0..count as usize {
            if let Some(name) = tab_ctrl.get_name(i) {
                if item.matches(name.trim_end_matches("\0")) {
                    return Some(i);
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
    fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> (i32, i32) {
        let mut point = POINT { x, y };
        unsafe { ClientToScreen(hwnd, &mut point); }
        (point.x, point.y)
    }
    fn sleep(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
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
    Menu(usize, bool, i32, i32),
    HItem(isize, u32),
    ListView(i32, i32, ListView),
    ListViewHeader(i32, u32)
}

#[derive(Debug, PartialEq)]
enum TargetClass {
    Button,
    List,
    ComboBox,
    Tab,
    Menu,
    TreeView,
    ListView,
    // ListViewHeader,
    ToolBar,
    Link,
    Other(String),
}

impl std::fmt::Display for TargetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetClass::Button => write!(f, "{}", WC_BUTTON),
            TargetClass::List => write!(f, "{}", WC_LISTBOX),
            TargetClass::ComboBox => write!(f, "{}", WC_COMBOBOX),
            TargetClass::Tab => write!(f, "{}", WC_TABCONTROL),
            TargetClass::Menu => write!(f, "#32768"),
            TargetClass::TreeView => write!(f, "{}", WC_TREEVIEW),
            TargetClass::ListView => write!(f, "{}", WC_LISTVIEW),
            // TargetClass::ListViewHeader => write!(f, "{}", WC_HEADER),
            TargetClass::ToolBar => write!(f, "{}", TOOLBARCLASSNAME),
            TargetClass::Link => write!(f, "{}", WC_LINK),
            TargetClass::Other(s) => write!(f, "{s}"),
        }
    }
}

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
            // "sysheader32" => Self::ListViewHeader,
            "toolbarwindow32" => Self::ToolBar,
            "syslink" => Self::Link,
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

struct ProcessMemory {
    pub hprocess: HANDLE,
    pub pointer: *mut c_void,
}

impl ProcessMemory {
    fn new<T>(pid: u32, obj: Option<&T>) -> Option<Self> {
        let hprocess = Self::open_process(pid)?;
        let pointer = unsafe {
            VirtualAllocEx(hprocess, ptr::null(), mem::size_of::<T>(), MEM_COMMIT, PAGE_READWRITE)
        };
        if let Some(obj) = obj {
            let lpbuffer = obj as *const T as *const c_void;
            let nsize = mem::size_of::<T>();
            unsafe {
                WriteProcessMemory(hprocess, pointer, lpbuffer, nsize, ptr::null_mut());
            }
        }
        Some(Self { hprocess, pointer })
    }
    fn new2<T, U>(pid: u32, value: U) -> Option<Self> {
        let hprocess = Self::open_process(pid)?;
        let pointer = unsafe {
            VirtualAllocEx(hprocess, ptr::null(), mem::size_of::<T>(), MEM_COMMIT, PAGE_READWRITE)
        };
        let lpbuffer = &value as *const U as *const c_void;
        let nsize = mem::size_of::<T>();
        if nsize < mem::size_of::<U>() {
            // 書き込むデータのサイズが確保したメモリサイズを超えたらダメ
            None
        } else {
            unsafe {
                WriteProcessMemory(hprocess, pointer, lpbuffer, nsize, ptr::null_mut());
            }
            Some(Self { hprocess, pointer })
        }
    }
    fn read<T>(&self, buf: &mut T) {
        let lpbuffer = buf as *mut T as *mut c_void;
        let nsize = mem::size_of::<T>();
        unsafe {
            ReadProcessMemory(self.hprocess, self.pointer, lpbuffer, nsize, ptr::null_mut());
        }
    }
    fn is_process_x64(pid: u32) -> Option<bool> {
        let hprocess = Self::open_process(pid)?;
        let mut wow64process = true.into();
        unsafe {
            IsWow64Process(hprocess, &mut wow64process);
            CloseHandle(hprocess);
        }
        let is_x64 = ! wow64process.as_bool();
        Some(is_x64)
    }
    fn open_process(pid: u32) -> Option<HANDLE> {
        unsafe {
            let dwdesiredaccess = PROCESS_VM_READ|PROCESS_VM_WRITE|PROCESS_VM_OPERATION|PROCESS_QUERY_INFORMATION;
            OpenProcess(dwdesiredaccess, false, pid).ok()
        }
    }
    fn _as_ptr<T>(&self) -> *mut T {
        self.pointer as *mut T
    }
}

impl Drop for ProcessMemory {
    fn drop(&mut self) {
        unsafe {
            VirtualFreeEx(self.hprocess, self.pointer, 0, MEM_RELEASE);
            CloseHandle(self.hprocess);
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
        let target_arch = if ProcessMemory::is_process_x64(pid)? {
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
            let remote_buf = ProcessMemory::new::<[u16; 260]>(self.pid, None)?;
            // リモートバッファに名前を受ける
            self.get_remote_name(index, remote_buf.pointer, buf.len() as i32, TCM_GETITEMW)?;
            // 対象プロセスのバッファからローカルのバッファに読み出し
            remote_buf.read(&mut buf);
            Some(String::from_utf16_lossy(&buf))
        } else {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new::<[u16; 260]>(self.pid, None)?;
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
                    GetMenuItemRect(self.hwnd, hmenu, npos as u32, &mut rect);
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
        unsafe { GetMenuItemInfoW(hmenu, npos, true, &mut info) };
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
        unsafe { GetMenuItemInfoA(hmenu, npos, true, &mut info) };
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
        let target_arch = if ProcessMemory::is_process_x64(pid)? {
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
                let child = self.get_child(hitem);
                if child != 0 {
                    self.search(item, Some(child), Some(new_path));
                    if item.found.is_some() {
                        // 子から見つかれば終了
                        return;
                    }
                }
                // 次の要素を探す
                let next = self.get_next(hitem);
                if next != 0 {
                    self.search(item, Some(next), path);
                }
            }
        }
    }
    fn get_root(&self) -> isize {
        Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_ROOT as usize, 0)
    }
    fn get_child(&self, hitem: isize) -> isize {
        Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_CHILD as usize, hitem)
    }
    fn get_next(&self, hitem: isize) -> isize {
        Win32::send_message(self.hwnd, TVM_GETNEXTITEM, TVGN_NEXT as usize, hitem)
    }
    fn get_point(hwnd: HWND, pid: u32, hitem: isize) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        let remote = ProcessMemory::new2::<RECT, _>(pid, hitem)?;
        let lparam = remote.pointer as isize;
        remote.read(&mut rect);
        Win32::send_message(hwnd, TVM_GETITEMRECT, 1, lparam);
        remote.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let (x, y) = Win32::client_to_screen(hwnd, x, y);
        Some((x, y))
    }
    fn get_name(&self, hitem: isize) -> Option<String> {
        if self.is_unicode {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new::<[u16; 260]>(self.pid, None)?;

            self.get_remote_name(hitem, remote_buf.pointer, buf.len() as i32, TVM_GETITEMW)?;

            remote_buf.read(&mut buf);
            let name = from_wide_string(&buf);
            Some(name)
        } else {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new::<[u8; 260]>(self.pid, None)?;

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
        let target_arch = if ProcessMemory::is_process_x64(pid)? {TargetArch::X64} else {TargetArch::X86};
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
    fn get_name(&self, row: isize, column: isize) -> Option<String> {
        if self.is_unicode {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new::<[u16; 260]>(self.pid, None)?;
            self.get_remote_name(row, column, remote.pointer, buf.len() as i32, LVM_GETITEMTEXTW);
            remote.read(&mut buf);
            let text = from_wide_string(&buf);
            Some(text)
        } else {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new::<[u8; 260]>(self.pid, None)?;
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
                lvitem.mask = LVIF_TEXT;
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
                lvitem.mask = LVIF_TEXT;
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
            let remote = ProcessMemory::new::<[u16; 260]>(self.pid, None)?;
            self.get_remote_header_name(hwnd, index, remote.pointer, buf.len() as i32, HDM_GETITEMW);
            remote.read(&mut buf);
            let text = from_wide_string(&buf);
            Some(text)
        } else {
            let mut buf = [0; 260];
            let remote = ProcessMemory::new::<[u8; 260]>(self.pid, None)?;
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
        let remote = ProcessMemory::new::<RECT>(pid, None)?;
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