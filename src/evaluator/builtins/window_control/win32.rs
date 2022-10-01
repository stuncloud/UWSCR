
use windows::{
    Win32::{
        Foundation::{
            HWND, WPARAM, LPARAM, BOOL, RECT, HANDLE, POINT,
            CloseHandle,
        },
        UI::{
            Controls::{
                WC_BUTTON, WC_LISTBOX, WC_COMBOBOX, WC_TABCONTROL, WC_TREEVIEW, WC_LISTVIEW, WC_HEADER, WC_LINK,
                TOOLBARCLASSNAME,
                BST_CHECKED, BST_UNCHECKED, BST_INDETERMINATE,
                TCM_GETITEMW, TCM_GETITEMA, TCM_GETITEMCOUNT, TCM_GETCURSEL, TCM_GETUNICODEFORMAT, TCM_GETITEMRECT, TCM_SETCURFOCUS,
                TCIF_TEXT,
            },
            WindowsAndMessaging::{
                WM_COMMAND,
                BN_CLICKED,
                BS_CHECKBOX, BS_AUTOCHECKBOX, BS_3STATE, BS_AUTO3STATE, BS_RADIOBUTTON, BS_AUTORADIOBUTTON,
                BM_SETCHECK, BM_GETCHECK,
                CB_GETCOUNT, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_SETCURSEL, CB_GETCURSEL,
                LB_GETCOUNT, LB_GETTEXT, LB_GETTEXTLEN, LB_SETCURSEL, LB_GETCURSEL, LB_GETITEMRECT,
                WS_DISABLED,
                EnumChildWindows, PostMessageW, GetDlgCtrlID, SendMessageW,
                IsWindowUnicode,
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
    from_ansi_bytes,
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
            if item.target.contains(&target) {
                match target {
                    TargetClass::ComboBox => {
                        if let ListIndex::Index(index) = Self::search_combo_box(hwnd, item) {
                            item.found = Some((hwnd, target, vec![index]));
                            return false.into();
                        }
                    },
                    TargetClass::List => {
                        match Self::search_list_box(hwnd, item) {
                            ListIndex::Index(i) => {
                                item.found = Some((hwnd, target, vec![i]));
                                return false.into();
                            },
                            ListIndex::Multi(v) => {
                                item.found = Some((hwnd, target, v));
                                return false.into();
                            },
                            ListIndex::None => {},
                        }
                    },
                    TargetClass::Tab => if let Some(index) = Self::search_tab(hwnd, item) {
                        item.found = Some((hwnd, target, vec![index]));
                        return false.into();
                    },
                    TargetClass::Menu => {
                        // foo\bar\baz 形式
                        todo!()
                    },
                    TargetClass::TreeView => {
                        todo!()
                    },
                    TargetClass::ListView => {
                        todo!()
                    },
                    TargetClass::ListViewHeader => {
                        todo!()
                    },
                    TargetClass::ToolBar => {
                        todo!()
                    },
                    TargetClass::Link => {
                        todo!()
                    },
                    _ => {
                        let title = get_window_title(hwnd);
                        if item.matches(&title) {
                            item.found = Some((hwnd, target, vec![]));
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
    pub fn click(&self, clk_item: &ClkItem, check: &ThreeState) -> ClkResult {
        let mut item = SearchItem::from_clkitem(clk_item);
        self.search(&mut item);
        // println!("\u{001b}[35m[debug] item: {:#?}\u{001b}[0m", &item);
        if let Some((hwnd, class, index)) = item.found {
            if Self::is_disabled(hwnd) {
                ClkResult::failed()
            } else {
                match class {
                    TargetClass::Button => {
                        match ButtonType::from(hwnd) {
                            ButtonType::Button => if check.as_bool() {
                                let clicked = self.post_wm_command(hwnd, BN_CLICKED);
                                ClkResult::new(clicked, hwnd)
                            } else {
                                ClkResult::new(true, hwnd)
                            },
                            ButtonType::Check => {
                                let wparam = if check.as_bool() {
                                    BST_CHECKED.0 as usize
                                } else {
                                    BST_UNCHECKED.0 as usize
                                };
                                Self::post_message(hwnd, BM_SETCHECK, wparam, 0);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let is_checked = Self::send_message(hwnd, BM_GETCHECK, 0, 0);
                                let clicked = (is_checked > 0) == check.as_bool();
                                ClkResult::new(clicked, hwnd)
                            },
                            ButtonType::ThreeState => {
                                let wparam = match check {
                                    ThreeState::True => BST_CHECKED.0 as usize,
                                    ThreeState::False => BST_UNCHECKED.0 as usize,
                                    ThreeState::Other => BST_INDETERMINATE.0 as usize,
                                };
                                Self::post_message(hwnd, BM_SETCHECK, wparam, 0);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let checked = Self::send_message(hwnd, BM_GETCHECK, 0, 0);
                                let clicked = checked == wparam as isize;
                                ClkResult::new(clicked, hwnd)
                            },
                            ButtonType::Radio => if check.as_bool() {
                                MouseInput::left_click(hwnd);
                                // チェック状態を確認してチェック指示と比較
                                Self::sleep(30);
                                let is_checked = Self::send_message(hwnd, BM_GETCHECK, 0, 0);
                                let clicked = (is_checked > 0) == check.as_bool();
                                ClkResult::new(clicked, hwnd)
                            } else {
                                ClkResult::new(true, hwnd)
                            },
                        }
                    },
                    TargetClass::List => if index.len() > 1 {
                        let mut list_result = true;
                        let mut point = (0, 0);
                        for i in index {
                            Self::post_message(hwnd, LB_SETCURSEL, i, 0);
                            Self::sleep(30);
                            let index = Self::send_message(hwnd, LB_GETCURSEL, 0, 0);
                            if i as isize != index {
                                list_result = false;
                                point = Self::get_list_item_point(hwnd, index as usize);
                                break;
                            }
                        }
                        let (x, y) = point;
                        ClkResult::new_with_point(list_result, hwnd, x, y)
                    } else if index.len() == 1 {
                        let i = index[0];
                        Self::post_message(hwnd, LB_SETCURSEL, i, 0);
                        Self::sleep(30);
                        let index = Self::send_message(hwnd, LB_GETCURSEL, 0, 0);
                        let clicked = i as isize == index;
                        let (x, y) = Self::get_list_item_point(hwnd, index as usize);
                        ClkResult::new_with_point(clicked, hwnd, x, y)
                    } else {
                        ClkResult::failed()
                    },
                    TargetClass::ComboBox => if index.len() > 0 {
                        let i = index[0];
                        Self::post_message(hwnd, CB_SETCURSEL, i, 0);
                        Self::sleep(30);
                        let index = Self::send_message(hwnd, CB_GETCURSEL, 0, 0);
                        let clicked = i as isize == index;
                        let (x, y) = Self::get_list_item_point(hwnd, index as usize);
                        ClkResult::new_with_point(clicked, hwnd, x, y)
                    } else {
                        ClkResult::failed()
                    },
                    TargetClass::Tab => {
                        if index.len() > 0 {
                            if check.as_bool() {
                                let i = index[0];
                                Self::post_message(hwnd, TCM_SETCURFOCUS, i, 0);
                                Self::sleep(30);
                                let index = Self::send_message(hwnd, TCM_GETCURSEL, 0, 0);
                                let clicked = i as isize == index;
                                if let Some((x, y)) = Self::get_tab_point(hwnd, index as usize) {
                                    ClkResult::new_with_point(clicked, hwnd, x, y)
                                } else {
                                    ClkResult::new(clicked, hwnd)
                                }
                            } else {
                                ClkResult::failed()
                            }
                        } else {
                            ClkResult::failed()
                        }
                    },
                    TargetClass::Menu => {
                        todo!()
                    },
                    TargetClass::TreeView => {
                        todo!()
                    },
                    TargetClass::ListView => {
                        todo!()
                    },
                    TargetClass::ListViewHeader => {
                        todo!()
                    },
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

    fn post_wm_command(&self, hwnd: HWND, command: u32) -> bool {
        let id = Self::get_window_id(hwnd);
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
        Self::get_center(rect)
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
        let remote_rect = ProcessMemory::new(pid, &rect)?;
        let lparam = remote_rect.pointer as isize;
        Win32::send_message(hwnd, TCM_GETITEMRECT, index, lparam);
        remote_rect.read(&mut rect);
        let (x, y) = Win32::get_center(rect);
        let point = Win32::client_to_screen(hwnd, x, y);
        Some(point)
    }

    fn _is_window_unicode(hwnd: HWND) -> bool {
        unsafe { IsWindowUnicode(hwnd).as_bool() }
    }
    fn get_center(rect: RECT) -> (i32, i32) {
        let x = rect.left + (rect.right - rect.left) / 2;
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
    found: Option<(HWND, TargetClass, Vec<usize>)>,
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
            si.target.push(TargetClass::ListViewHeader);
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

#[derive(Debug, PartialEq)]
enum TargetClass {
    Button,
    List,
    ComboBox,
    Tab,
    Menu,
    TreeView,
    ListView,
    ListViewHeader,
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
            TargetClass::ListViewHeader => write!(f, "{}", WC_HEADER),
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
            "sysheader32" => Self::ListViewHeader,
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
    fn new<T>(pid: u32, obj: &T) -> Option<Self> {
        let hprocess = Self::open_process(pid)?;
        let pointer = unsafe {
            VirtualAllocEx(hprocess, ptr::null(), mem::size_of::<T>(), MEM_COMMIT, PAGE_READWRITE)
        };
        let lpbuffer = obj as *const T as *const c_void;
        let nsize = mem::size_of::<T>();
        unsafe {
            WriteProcessMemory(hprocess, pointer, lpbuffer, nsize, ptr::null_mut());
        }
        Some(Self { hprocess, pointer })
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
}

impl Drop for ProcessMemory {
    fn drop(&mut self) {
        unsafe {
            VirtualFreeEx(self.hprocess, self.pointer, 0, MEM_RELEASE);
            CloseHandle(self.hprocess);
        }
    }
}

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
        // let is_x64 = cfg!(target_arch="x86_64");
        // let is_target_x64 = ProcessMemory::is_process_x64(pid)?;
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
            let remote_buf = ProcessMemory::new(self.pid, &buf)?;
            // リモートバッファに名前を受ける
            self.get_remote_name(index, remote_buf.pointer, buf.len() as i32, TCM_GETITEMW)?;
            // 対象プロセスのバッファからローカルのバッファに読み出し
            remote_buf.read(&mut buf);
            Some(String::from_utf16_lossy(&buf))
        } else {
            let mut buf = [0; 260];
            let remote_buf = ProcessMemory::new(self.pid, &buf)?;
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
                let remote_item = ProcessMemory::new(self.pid, &tcitem)?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, index, lparam);
            },
            TargetArch::X64 => {
                let mut tcitem = TCITEM64::default();
                tcitem.mask = TCIF_TEXT.0;
                tcitem.cchTextMax = len;
                tcitem.pszText = p_buffer as i64;
                let remote_item = ProcessMemory::new(self.pid, &tcitem)?;
                let lparam = remote_item.pointer as isize;
                Win32::send_message(self.hwnd, msg, index, lparam);
            },
        }
        Some(())
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct TCITEM64 {
    pub mask: u32,
    pub dwState: u32,
    pub dwStateMask: u32,
    pub pszText: i64,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: i64,
}
impl Default for TCITEM64 {
    fn default() -> Self {
        Self { mask: 0, dwState: 0, dwStateMask: 0, pszText: 0, cchTextMax: 0, iImage: 0, lParam: 0 }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct TCITEM86 {
    pub mask: u32,
    pub dwState: u32,
    pub dwStateMask: u32,
    pub pszText: i32,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: i32,
}
impl Default for TCITEM86 {
    fn default() -> Self {
        Self { mask: 0, dwState: 0, dwStateMask: 0, pszText: 0, cchTextMax: 0, iImage: 0, lParam: 0 }
    }
}