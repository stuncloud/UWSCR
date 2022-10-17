use super::ClkConst;
use super::acc;
use super::win32;
use crate::evaluator::builtins::window_low::move_mouse_to;
use crate::evaluator::object::Object;
use crate::evaluator::builtins::ThreeState;

use windows::{
    Win32::{
        Foundation::{
            HWND, RECT, WPARAM, LPARAM,
        },
        UI::{
            WindowsAndMessaging::{
                WM_LBUTTONDOWN, WM_LBUTTONUP,
                WM_RBUTTONDOWN, WM_RBUTTONUP,
                WM_LBUTTONDBLCLK,
                GetWindowRect, PostMessageW,
                SetForegroundWindow,
            },
            Input::{
                KeyboardAndMouse::{
                    IsWindowEnabled,
                }
            },
        },
    }
};

pub struct ClkItem {
    pub name: String,
    pub target: ClkTarget,
    back_ground: bool,
    pub move_mouse: bool,
    pub short: bool,
    backwards: bool,
    button: ClkButton,
    api: ClkApi,
    pub order: u32,
    as_hwnd: bool,
}

pub struct ClkTarget {
    pub button: bool,
    pub list: bool,
    pub tab: bool,
    pub menu: bool,
    pub treeview: bool,
    pub listview: bool,
    pub toolbar: bool,
    pub link: bool,
}

pub enum ClkButton {
    Left {double: bool},
    Right,
    Default
}

struct ClkApi {
    win32: bool,
    uia: bool,
    acc: bool,
}

pub struct ClkResult {
    clicked: bool,
    hwnd: HWND,
    point: Option<(i32, i32)>
}
impl Default for ClkResult {
    fn default() -> Self {
        Self {
            clicked: false,
            hwnd: HWND(0),
            point: None,
        }
    }
}
impl ClkResult {
    pub fn new(clicked: bool, hwnd: HWND) -> Self {
        Self { clicked, hwnd, point: None }
    }
    pub fn new_with_point(clicked: bool, hwnd: HWND, x: i32, y: i32) -> Self {
        Self { clicked, hwnd, point: Some((x, y)) }
    }
    fn to_object(&self, as_hwnd: bool) -> Object {
        if as_hwnd {
            let n = self.hwnd.0 as f64;
            Object::Num(n)
        } else {
            Object::Bool(self.clicked)
        }
    }
    pub fn failed() -> Self {
        Self::default()
    }
    pub fn _succeed(hwnd: HWND, point: Option<(i32, i32)>) -> Self {
        Self { clicked: true, hwnd, point }
    }
}

impl ClkItem {
    pub fn new(name: String, n: usize, order: u32) -> Self {
        Self {
            name,
            target: ClkTarget::new(n),
            back_ground: n.is_available(ClkConst::CLK_BACK),
            move_mouse: n.is_available(ClkConst::CLK_MOUSEMOVE),
            short: n.is_available(ClkConst::CLK_SHORT),
            backwards: n.is_available(ClkConst::CLK_FROMLAST),
            button: ClkButton::new(n),
            api: ClkApi::new(n),
            order,
            as_hwnd: n.is_available(ClkConst::CLK_HWND),
        }
    }

    pub fn click(&self, hwnd: HWND, check: ThreeState) -> Object {
        if ! self.back_ground {
            activate_window(hwnd);
        }
        let result = if self.api.win32 {
            self.click_win32(hwnd, &check)
        } else {ClkResult::default()};
        let result = if ! result.clicked && self.api.uia {
            self.click_uia(hwnd, &check)
        } else {result};
        let result = if ! result.clicked && self.api.acc {
            self.click_acc(hwnd, check.as_bool())
        } else {result};
        if self.move_mouse && result.clicked {
            let (x, y) = match result.point {
                Some(p) => p,
                None => MouseInput::point_from_hwnd(result.hwnd),
            };
            move_mouse_to(x, y);
        }
        result.to_object(self.as_hwnd)
    }
    fn click_win32(&self, hwnd: HWND, check: &ThreeState) -> ClkResult {
        let win32 = win32::Win32::new(hwnd);
        win32.click(self, check)
    }
    fn click_uia(&self, _hwnd: HWND, _check: &ThreeState) -> ClkResult {
        ClkResult::default()
    }
    fn click_acc(&self, hwnd: HWND, check: bool) -> ClkResult {
        if let Some(window) = acc::Acc::from_hwnd(hwnd) {
            let item = acc::SearchItem::from_clkitem(self);
            let mut order = self.order;

            match window.search(&item, &mut order, self.backwards) {
                Some(target) => {
                    let result = match self.button {
                        ClkButton::Left { double } => if let Some(hwnd) = target.get_hwnd() {
                            if double {
                                MouseInput::left_dblclick(hwnd)
                            } else {
                                MouseInput::left_click(hwnd)
                            }
                        } else {
                            false
                        },
                        ClkButton::Right => if let Some(hwnd) = target.get_hwnd() {
                            MouseInput::right_click(hwnd)
                        } else {
                            false
                        },
                        ClkButton::Default => target.invoke_default_action(check),
                    };
                    let (x, y) = MouseInput::point_from_hwnd(hwnd);
                    ClkResult::new_with_point(result, target.get_hwnd().unwrap_or_default(), x, y)
                },
                None => ClkResult::default(),
            }
        } else {
            ClkResult::default()
        }
    }
}

impl ClkTarget {
    pub fn new(n: usize) -> Self {
        let clk_target_all = ClkConst::CLK_BTN as usize | ClkConst::CLK_LIST as usize | ClkConst::CLK_TAB as usize | ClkConst::CLK_MENU as usize | ClkConst::CLK_TREEVIEW as usize | ClkConst::CLK_LISTVIEW as usize | ClkConst::CLK_TOOLBAR as usize | ClkConst::CLK_LINK as usize;
        if (n & clk_target_all) == 0 {
            Self { button: true, list: true, tab: true, menu: true, treeview: true, listview: true, toolbar: true, link: true }
        } else {
            Self {
                button: n.is_available(ClkConst::CLK_BTN),
                list: n.is_available(ClkConst::CLK_LIST),
                tab: n.is_available(ClkConst::CLK_TAB),
                menu: n.is_available(ClkConst::CLK_MENU),
                treeview: n.is_available(ClkConst::CLK_TREEVIEW),
                listview: n.is_available(ClkConst::CLK_LISTVIEW),
                toolbar: n.is_available(ClkConst::CLK_TOOLBAR),
                link: n.is_available(ClkConst::CLK_LINK),
            }
        }
    }
}

impl ClkButton {
    pub fn new(n: usize) -> Self {
        if n.is_available(ClkConst::CLK_LEFTCLK) {
            Self::Left { double: n.is_available(ClkConst::CLK_DBLCLK) }
        } else if n.is_available(ClkConst::CLK_RIGHTCLK) {
            Self::Right
        } else {
            Self::Default
        }
    }
}

impl ClkApi {
    pub fn new(n: usize) -> Self {
        let clk_api_all = ClkConst::CLK_ACC as usize | ClkConst::CLK_API as usize | ClkConst::CLK_UIA as usize;
        if (n & clk_api_all) == 0 {
            Self { win32: true,uia: true,acc: true }
        } else {
            Self {
                win32: n.is_available(ClkConst::CLK_API),
                uia: n.is_available(ClkConst::CLK_UIA),
                acc: n.is_available(ClkConst::CLK_ACC),
            }
        }
    }
}

trait UsizeExt {
    fn is_available(&self, c: ClkConst) -> bool;
}
impl UsizeExt for usize {
    fn is_available(&self, c: ClkConst) -> bool {
        (*self & c as usize) > 0
    }
}


pub struct MouseInput {}

impl MouseInput {
    pub fn left_click(hwnd: HWND) -> bool {
        Self::click(hwnd, vec![WM_LBUTTONDOWN, WM_LBUTTONUP])
    }
    pub fn right_click(hwnd: HWND) -> bool {
        Self::click(hwnd, vec![WM_RBUTTONDOWN, WM_RBUTTONUP])
    }
    pub fn left_dblclick(hwnd: HWND) -> bool {
        Self::click(hwnd, vec![WM_LBUTTONDBLCLK])
    }
    fn click(hwnd: HWND, msgs: Vec<u32>) -> bool {
        unsafe {
            if IsWindowEnabled(hwnd).as_bool() {
                let lparam = Self::point_lparam_from_hwnd(hwnd);
                let mut result = true;
                for msg in msgs {
                    let r = PostMessageW(hwnd, msg, WPARAM(0), LPARAM(lparam));
                    result = result && r.as_bool()
                }
                result
            } else {
                false
            }
        }
    }
    pub fn point_from_hwnd(hwnd: HWND) -> (i32, i32) {
        unsafe {
            let mut lprect = RECT::default();
            GetWindowRect(hwnd, &mut lprect);
            // だいたい真ん中あたりを狙う
            let x = lprect.left + (lprect.right - lprect.left) / 2;
            let y = lprect.top + (lprect.bottom - lprect.top) / 2;
            (x, y)
        }
    }
    fn point_lparam_from_hwnd(hwnd: HWND) -> isize {
        unsafe {
            let mut lprect = RECT::default();
            GetWindowRect(hwnd, &mut lprect);
            // だいたい真ん中あたりを狙う
            let x = (lprect.right - lprect.left) / 2;
            let y = (lprect.bottom - lprect.top) / 2;
            let lparam = (x as u32 & 0xFFFF) | (y as u32 & 0xFFFF) << 16;
            lparam as isize
        }
    }
}

fn fix_title(title: &str) -> Vec<String> {
    let mut titles = vec![title.to_string()];
    if title.contains("&") {
        let replaced = title.replace("&", "");
        titles.push(replaced);
    }
    if let Some((head, _)) = title.split_once("(&") {
        titles.push(head.trim_end().to_string())
    }
    titles
}
pub fn match_title(title: &str, pat: &str, partial: bool) -> bool {
    let lower_title = title.to_ascii_lowercase();
    let lower_pat = pat.to_ascii_lowercase();
    if partial {
        lower_title.find(&lower_pat).is_some()
    } else {
        let titles = fix_title(&lower_title);
        titles.contains(&lower_pat)
    }
}

fn activate_window(hwnd: HWND) -> bool {
    unsafe {
        SetForegroundWindow(hwnd).as_bool()
    }
}