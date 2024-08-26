use crate::object::*;
use crate::builtins::*;
use crate::{Evaluator, MouseOrg, MorgTarget};
use util::winapi::make_lparam;

use std::{thread, time};
use std::mem::size_of;
use std::sync::{Arc, Mutex, OnceLock, LazyLock};

use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{POINT, HWND, RECT, WPARAM, LPARAM, HANDLE},
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT,
                KEYBDINPUT, INPUT_KEYBOARD, VIRTUAL_KEY,
                KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
                MOUSEINPUT, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE,
                MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
                MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
                MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
                MOUSEEVENTF_WHEEL, MOUSEEVENTF_HWHEEL,
            },
            Input::Pointer::{
                InitializeTouchInjection, InjectTouchInput,
                TOUCH_FEEDBACK_NONE,
                POINTER_TOUCH_INFO, POINTER_INFO,
                POINTER_FLAGS, POINTER_FLAG_DOWN, POINTER_FLAG_UP, POINTER_FLAG_UPDATE, POINTER_FLAG_INRANGE, POINTER_FLAG_INCONTACT,
                POINTER_BUTTON_CHANGE_TYPE,
            },
            WindowsAndMessaging::{
                GetWindowRect, GetClientRect,
                GetCursorPos, SetCursorPos,
                PostMessageW, WM_MOUSEMOVE,
                WM_LBUTTONUP, WM_LBUTTONDOWN,
                WM_RBUTTONUP, WM_RBUTTONDOWN,
                WM_MBUTTONUP, WM_MBUTTONDOWN,
                WM_KEYUP, WM_KEYDOWN, WM_CHAR,
                WM_MOUSEWHEEL, WM_MOUSEHWHEEL, WHEEL_DELTA,
                PT_TOUCH, TOUCH_MASK_CONTACTAREA, TOUCH_MASK_ORIENTATION, TOUCH_MASK_PRESSURE,
            },
        },
        Graphics::Gdi::ClientToScreen,
    },
};

static INIT_TOUCH_INJECTION: OnceLock<()> = OnceLock::new();
static TOUCH_POINT: LazyLock<Arc<Mutex<TouchPoint>>> = LazyLock::new(|| Arc::new(Mutex::new(TouchPoint(None))));
pub static INPUT_EXTRA_INFO: LazyLock<usize> = LazyLock::new(|| std::process::id() as usize);

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("mmv", mmv, get_desc!(mmv) );
    sets.add("btn", btn, get_desc!(btn));
    sets.add("kbd", kbd, get_desc!(kbd));
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum MouseButtonEnum {
    #[strum[props(desc="左ボタン")]]
    LEFT = 0,
    #[strum[props(desc="右ボタン")]]
    RIGHT = 1,
    #[strum[props(desc="中央ボタン")]]
    MIDDLE = 2,
    #[strum[props(desc="ホイル上下回転")]]
    WHEEL = 5,
    #[strum[props(desc="ホイル左右回転")]]
    WHEEL2 = 6,
    #[strum[props(desc="タッチ操作")]]
    TOUCH = 7,
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum KeyActionEnum {
    #[default]
    #[strum[props(desc="クリック")]]
    CLICK = 0,
    #[strum[props(desc="ボタン押し下げ")]]
    DOWN = 1,
    #[strum[props(desc="ボタン開放")]]
    UP = 2,
}

pub fn move_mouse_to(x: i32, y: i32) -> bool {
    unsafe {
        SetCursorPos(x, y).is_ok() &&
        SetCursorPos(x, y).is_ok()
    }
}

#[builtin_func_desc(
    desc="マウスカーソルを移動させる",
    args=[
        {n="x", t="数値", d="移動先X座標"},
        {n="y", t="数値", d="移動先Y座標"},
        {n="ms", t="数値", d="移動を行うまでの待機時間、デフォルトは0", o},
    ],
)]
pub fn mmv(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let x = args.get_as_int(0, Some(0))?;
    let y = args.get_as_int(1, Some(0))?;
    let ms = args.get_as_int::<u64>(2, Some(0))?;

    sleep(ms);
    Input::from(&evaluator.mouseorg).move_mouse(x, y);

    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="指定座標にマウスボタン操作を送信",
    args=[
        {
            n="ボタン定数", t="定数", o,
            d=r#"以下の定数のいずれかを指定
- LEFT: 左クリック
- RIGHT: 右クリック
- MIDDLE: 中央クリック
- WHEEL: 上下ホイル回転
- WHEEL2: 左右ホイル回転
- TOUCH: タッチ操作
"#
        },
        {
            n="状態", t="定数または数値", o,
            d=r#"マウス操作を以下から指定
- LEFT, RIGHT, MIDDLE, TOUCH
    - CLICK: クリック (下げて離す)
    - DOWN: ボタン押し下げ
    - UP: ボタン開放
- WHEEL
    - 数値: 正なら下方向、負なら上方向
- WHEEL2
    - 数値: 正なら右方向、負なら左方向
"#
        },
        {n="x", t="数値", d="X座標、EMPTYならマウス位置", o},
        {n="y", t="数値", d="Y座標、EMPTYならマウス位置", o},
        {n="ms", t="数値", d="操作を行うまでの待機時間、デフォルトは0", o},
    ],
)]
pub fn btn(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let Some(btn) = args.get_as_const::<MouseButtonEnum>(0, true)? else {
        // 不正な定数の場合何もしない
        return Ok(Object::Empty);
    };

    let action = args.get_as_int(1, Some(0))?;
    let p = get_current_pos()?;
    let (cur_x, cur_y) = (p.x, p.y);
    let x = args.get_as_int( 2, Some(cur_x))?;
    let y = args.get_as_int( 3, Some(cur_y))?;
    let ms= args.get_as_int::<u64>(4, Some(0))?;

    sleep(ms);
    let input = Input::from(&evaluator.mouseorg);
    match btn {
        MouseButtonEnum::LEFT => {
            let action = FromPrimitive::from_i32(action).unwrap_or_default();
            input.mouse_button(x, y, &MouseButton::Left, action);
        },
        MouseButtonEnum::RIGHT => {
            let action = FromPrimitive::from_i32(action).unwrap_or_default();
            input.mouse_button(x, y, &MouseButton::Right, action);
        },
        MouseButtonEnum::MIDDLE => {
            let action = FromPrimitive::from_i32(action).unwrap_or_default();
            input.mouse_button(x, y, &MouseButton::Middle, action);
        },
        MouseButtonEnum::WHEEL => {
            input.mouse_wheel(x, y, action, false);
        },
        MouseButtonEnum::WHEEL2 => {
            input.mouse_wheel(x, y, action, true);
        },
        MouseButtonEnum::TOUCH => {
            let action = FromPrimitive::from_i32(action).unwrap_or_default();
            input.touch(x, y, action, ms);
        },
    }

    Ok(Object::Empty)
}

pub fn get_current_pos() -> BuiltInResult<POINT>{
    let mut point = POINT {x: 0, y: 0};
    unsafe {
        if GetCursorPos(&mut point).is_ok() == false {
            return Err(builtin_func_error(UErrorMessage::UnableToGetCursorPosition));
        };
    }
    Ok(point)
}

#[builtin_func_desc(
    desc="指定座標にマウスボタン操作を送信",
    args=[
        {n="入力値", t="定数または文字列", d="仮想キーコード(VK定数)または入力したい文字列"},
        {
            n="状態", t="定数", o,
            d=r#"以下から指定、デフォルトはCLICK
- CLICK: クリック (下げて離す)
- DOWN: ボタン押し下げ
- UP: ボタン開放
"#
        },
        {n="ms", t="数値", d="操作を行うまでの待機時間、デフォルトは0", o},
    ],
)]
pub fn kbd(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let key = args.get_as_num_or_string(0)?;
    let action = args.get_as_const::<KeyActionEnum>(1, false)?
        .unwrap_or(KeyActionEnum::CLICK);
    let wait= args.get_as_int::<u64>(2, Some(0))?;

    let vk_win = key_codes::VirtualKeyCode::VK_WIN as u8;
    let vk_rwin = key_codes::VirtualKeyCode::VK_START as u8;
    let input = Input::from(&evaluator.mouseorg);
    match key {
        TwoTypeArg::U(vk) => {
            let extend = vk == vk_win || vk == vk_rwin;
            input.send_key(vk, action, wait, extend);
        },
        TwoTypeArg::T(s) => {
            input.send_str(&s, wait);
        }
    };
    Ok(Object::Empty)
}

pub fn get_morg_point(morg: &Option<MouseOrg>) -> Option<(i32, i32)> {
    Input::from(morg).get_offset()
}

pub struct Input {
    hwnd: Option<HWND>,
    /// 起点がクライアント領域ならtrue, ウィンドウ領域ならfalse
    client: bool,
    /// 直接送信ならtrue
    direct: bool,
}
impl From<&Option<MouseOrg>> for Input {
    fn from(morg: &Option<MouseOrg>) -> Self {
        match morg {
            Some(morg) => {
                let hwnd = Some(morg.hwnd);
                let (client, direct) = match morg.target {
                    MorgTarget::Window => (false, false),
                    MorgTarget::Client => (true, false),
                    MorgTarget::Direct => (true, true),
                };
                Self { hwnd, client, direct }
            },
            None => Self { hwnd: None, client: false, direct: false },
        }
    }
}
impl Input {
    pub fn is_client(&self) -> bool {
        self.client
    }
    fn get_offset(&self) -> Option<(i32, i32)> {
        unsafe {
            let hwnd = self.hwnd?;
            let mut rect = RECT::default();
            if self.client {
                let _ = GetClientRect(hwnd, &mut rect);
                let mut point = POINT { x: rect.left, y: rect.top };
                ClientToScreen(hwnd, &mut point);
                Some((point.x, point.y))
            } else {
                let _ = GetWindowRect(hwnd, &mut rect);
                Some((rect.left, rect.top))
            }
        }
    }
    pub fn fix_point(&self, x: i32, y: i32) -> (i32, i32) {
        if let Some((dx, dy)) = self.get_offset() {
            (x + dx, y + dy)
        } else {
            (x, y)
        }
    }
    fn send_key(&self, vk: u8, action: KeyActionEnum, wait: u64, extend: bool) {
        sleep(wait);
        match action {
            KeyActionEnum::CLICK => {
                self.key_down(vk, extend);
                // 20ms待って離す
                sleep(20);
                self.key_up(vk, extend)
            },
            KeyActionEnum::DOWN => self.key_down(vk, extend),
            KeyActionEnum::UP => self.key_up(vk, extend),
        }
    }
    fn send_str(&self, str: &str, wait: u64) {
        sleep(wait);
        unsafe {
            if self.direct {
                let hstring = HSTRING::from(str);
                hstring.as_wide()
                    .into_iter()
                    .map(|n| *n as usize)
                    .for_each(|char| {let _ = PostMessageW(self.hwnd.as_ref(), WM_CHAR, WPARAM(char), LPARAM(1));});
            } else {
                let pinputs = str.encode_utf16()
                    .map(|scan| {
                        let mut input = INPUT::default();
                        input.r#type = INPUT_KEYBOARD;
                        input.Anonymous.ki = KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: scan,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: *INPUT_EXTRA_INFO,
                        };
                        input
                    })
                    .collect::<Vec<_>>();
                SendInput(&pinputs, size_of::<INPUT>() as i32);
            }
        }
    }
    fn key_down(&self, vk: u8, extend: bool) {
        unsafe {
            if self.direct {
                let _ = PostMessageW(self.hwnd.as_ref(), WM_KEYDOWN, WPARAM(vk as usize), LPARAM(0));
            } else {
                let mut input = INPUT::default();
                let dwflags = if extend {
                    KEYEVENTF_EXTENDEDKEY
                } else {
                    KEYBD_EVENT_FLAGS(0)
                };
                // let scan = MapVirtualKeyW(vk as u32, 0) as u16;
                let wvk = VIRTUAL_KEY(vk as u16);
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: wvk,
                    wScan: 0,
                    dwFlags: dwflags,
                    time: 0,
                    dwExtraInfo: *INPUT_EXTRA_INFO,
                };
                SendInput(&[input], size_of::<INPUT>() as i32);
            }
        }
    }
    fn key_up(&self, vk: u8, extend: bool) {
        unsafe {
            if self.direct {
                let _ = PostMessageW(self.hwnd.as_ref(), WM_KEYUP, WPARAM(vk as usize), LPARAM(0));
            } else {
                let mut input = INPUT::default();
                let dwflags = if extend {
                    KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY
                } else {
                    KEYEVENTF_KEYUP
                };
                // let scan = MapVirtualKeyW(vk as u32, 0) as u16;
                let wvk = VIRTUAL_KEY(vk as u16);
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: wvk,
                    wScan: 0,
                    dwFlags: dwflags,
                    time: 0,
                    dwExtraInfo: *INPUT_EXTRA_INFO,
                };
                SendInput(&[input], size_of::<INPUT>() as i32);
            }
        }
    }
    fn move_mouse(&self, x: i32, y: i32) -> bool {
        unsafe {
            if self.direct {
                let lparam = make_lparam(x, y);
                PostMessageW(self.hwnd.as_ref(), WM_MOUSEMOVE, None, lparam).is_ok()
            } else {
                let (x, y) = self.fix_point(x, y);
                move_mouse_to(x, y)
            }
        }
    }
    fn mouse_down(&self, x: i32, y: i32, btn: &MouseButton) {
        unsafe {
            if self.direct {
                let msg = match btn {
                    MouseButton::Left => WM_LBUTTONDOWN,
                    MouseButton::Right => WM_RBUTTONDOWN,
                    MouseButton::Middle => WM_MBUTTONDOWN,
                };
                let lparam = make_lparam(x, y);
                let _ = PostMessageW(self.hwnd.as_ref(), msg, None, lparam);
            } else {
                let (x, y) = self.fix_point(x, y);
                let dwflags = match btn {
                    MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                    MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
                } | MOUSEEVENTF_ABSOLUTE;
                let mut input = INPUT::default();
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: dwflags,
                    time: 0,
                    dwExtraInfo: *INPUT_EXTRA_INFO,
                };
                SendInput(&[input], size_of::<INPUT>() as i32);
            }
        }
    }
    fn mouse_up(&self, x: i32, y: i32, btn: &MouseButton) {
        unsafe {
            if self.direct {
                let msg = match btn {
                    MouseButton::Left => WM_LBUTTONUP,
                    MouseButton::Right => WM_RBUTTONUP,
                    MouseButton::Middle => WM_MBUTTONUP,
                };
                let lparam = make_lparam(x, y);
                let _ = PostMessageW(self.hwnd.as_ref(), msg, None, lparam);
            } else {
                let (x, y) = self.fix_point(x, y);
                let dwflags = match btn {
                    MouseButton::Left => MOUSEEVENTF_LEFTUP,
                    MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
                } | MOUSEEVENTF_ABSOLUTE;
                let mut input = INPUT::default();
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: dwflags,
                    time: 0,
                    dwExtraInfo: *INPUT_EXTRA_INFO,
                };
                SendInput(&[input], size_of::<INPUT>() as i32);
            }
        }
    }
    fn mouse_click(&self, x: i32, y: i32, btn: &MouseButton) {
        self.mouse_down(x, y, btn);
        self.mouse_up(x, y, btn);
    }
    fn mouse_button(&self, x: i32, y: i32, btn: &MouseButton, action: KeyActionEnum) {
        self.move_mouse(x, y);
        match action {
            KeyActionEnum::CLICK => self.mouse_click(x, y, btn),
            KeyActionEnum::DOWN => self.mouse_down(x, y, btn),
            KeyActionEnum::UP => self.mouse_up(x, y, btn),
        }
    }
    fn mouse_wheel(&self, x: i32, y: i32, amount: i32, horizontal: bool) {
        self.move_mouse(x, y);
        unsafe {
            if self.direct {
                let msg = if horizontal {WM_MOUSEHWHEEL} else {WM_MOUSEWHEEL};
                let amount = amount * WHEEL_DELTA as i32;
                let wparam = ((amount & 0xFFFF) << 16) as usize;
                let (x, y) = self.fix_point(x, y);
                let lparam = ((x & 0xFFFF) | (y & 0xFFFF) << 16) as isize;
                let _ = PostMessageW(self.hwnd.as_ref(), msg, WPARAM(wparam), LPARAM(lparam));
            } else {
                let dwflags = if horizontal {MOUSEEVENTF_HWHEEL} else {MOUSEEVENTF_WHEEL};
                let mut input = INPUT::default();
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: amount as u32,
                    dwFlags: dwflags,
                    time: 0,
                    dwExtraInfo: *INPUT_EXTRA_INFO,
                };
                SendInput(&[input], size_of::<INPUT>() as i32);
            }
        }
    }
    fn touch(&self, x: i32, y: i32, action: KeyActionEnum, ms: u64) {
        unsafe {
            // 初回のみ初期化を行う
            INIT_TOUCH_INJECTION.get_or_init(|| {
                let _ = InitializeTouchInjection(1, TOUCH_FEEDBACK_NONE);
            });
            match action {
                KeyActionEnum::CLICK => {
                    self.touch_click(x, y);
                },
                KeyActionEnum::DOWN => {
                    self.touch_down(x, y);
                },
                KeyActionEnum::UP => {
                    self.touch_up(x, y, ms);
                },
            }
        }
    }
    fn touch_click(&self, x: i32, y: i32) -> bool {
        unsafe {
            let (x, y) = self.fix_point(x, y);
            let mut info = Self::new_pointer_touch_info(x, y, POINTER_FLAG_DOWN|POINTER_FLAG_INRANGE|POINTER_FLAG_INCONTACT);
            let down = InjectTouchInput(&[info]).is_ok();
            info.pointerInfo.pointerFlags = POINTER_FLAG_UP;
            let up = InjectTouchInput(&[info]).is_ok();
            down && up
        }
    }
    fn touch_down(&self, x: i32, y: i32) -> bool {
        unsafe {
            let (x, y) = self.fix_point(x, y);
            let info = Self::new_pointer_touch_info(x, y, POINTER_FLAG_DOWN|POINTER_FLAG_INRANGE|POINTER_FLAG_INCONTACT);
            let r = InjectTouchInput(&[info]).is_ok();
            if r {
                // DOWNした座標を登録
                let mut tp = TOUCH_POINT.lock().unwrap();
                *tp = TouchPoint(Some((x, y)));
            }
            r
        }
    }
    fn touch_up(&self, x: i32, y: i32, ms: u64) -> bool {
        unsafe {
            let (x, y) = self.fix_point(x, y);
            let maybe_moved = {
                let tp = TOUCH_POINT.lock().unwrap();
                tp.moved(x, y)
            };
            if let Some((moved, p1)) = maybe_moved {
                if moved {
                    let wait = ms.max(10);
                    // タッチを維持しつつ動かす
                    let points = Self::get_move_points(p1, (x, y));
                    let mut info = Self::new_pointer_touch_info(p1.0, p1.1, POINTER_FLAG_UPDATE|POINTER_FLAG_INRANGE|POINTER_FLAG_INCONTACT);
                    let _ = InjectTouchInput(&[info]);
                    for point in points {
                        info.set_point(point);
                        sleep(wait);
                        let _ = InjectTouchInput(&[info]);
                    }
                    info.set_point((x, y));
                    let _ = InjectTouchInput(&[info]);
                    info.pointerInfo.pointerFlags = POINTER_FLAG_UP;
                    let r = InjectTouchInput(&[info]).is_ok();
                    if r {
                        // UPしたら座標をリセット
                        let mut tp = TOUCH_POINT.lock().unwrap();
                        *tp = TouchPoint(None);
                    }
                    r
                } else {
                    // 座標が動いていなかったら即UPする
                    let info = Self::new_pointer_touch_info(x, y, POINTER_FLAG_UP);
                    let r = InjectTouchInput(&[info]).is_ok();
                    if r {
                        // UPしたら座標をリセット
                        let mut tp = TOUCH_POINT.lock().unwrap();
                        *tp = TouchPoint(None);
                    }
                    r
                }
            } else {
                // downしてないので何もしない
                false
            }
        }
    }
    fn get_move_points(p1: (i32, i32), p2: (i32, i32)) -> Vec<(i32, i32)> {
        let count = (p1.0 - p2.0).abs().min((p1.1 - p2.1).abs());
        let x1 = p1.0 as f64;
        let y1 = p1.1 as f64;
        let x2 = p2.0 as f64;
        let y2 = p2.1 as f64;

        let m = (y2 - y1) / (x2 - x1);
        let b = y1 - m * x1;

        let step = (x2 - x1) / (count as f64 - 1.0);

        (0..count).map(|i| {
            let x = x1 + i as f64 * step;
            let y = m * x + b;
            (x as i32, y as i32)
        }).collect()
    }
    fn new_pointer_touch_info(x: i32, y: i32, flags: POINTER_FLAGS) -> POINTER_TOUCH_INFO {
        let margin = 2;
        let mut touch_info = POINTER_TOUCH_INFO::default();
        touch_info.touchMask = TOUCH_MASK_CONTACTAREA|TOUCH_MASK_ORIENTATION|TOUCH_MASK_PRESSURE;
        touch_info.rcContact = RECT { left: x-margin, top: y-margin, right: x+margin, bottom: y+margin };
        touch_info.orientation = 90;
        touch_info.pressure = 1000;
        touch_info.pointerInfo = POINTER_INFO {
            pointerType: PT_TOUCH,
            pointerId: 0,
            frameId: 0,
            pointerFlags: flags,
            sourceDevice: HANDLE::default(),
            hwndTarget: HWND::default(),
            ptPixelLocation: POINT { x, y },
            ptHimetricLocation: POINT::default(),
            ptPixelLocationRaw: POINT::default(),
            ptHimetricLocationRaw: POINT::default(),
            dwTime: 0,
            historyCount: 0,
            InputData: 0,
            dwKeyStates: 0,
            PerformanceCount: 0,
            ButtonChangeType: POINTER_BUTTON_CHANGE_TYPE::default(),
        };
        touch_info
    }
}
enum MouseButton {
    Left,
    Right,
    Middle,
}
struct TouchPoint(Option<(i32, i32)>);
impl TouchPoint {
    fn moved(&self, x: i32, y: i32) -> Option<(bool, (i32, i32))> {
        match self.0 {
            Some(p) => {
                let moved = p.0 != x && p.1 != y;
                Some((moved, p))
            },
            None => None,
        }
    }
}
trait PointerTouchInfoExt {
    fn set_point(&mut self, point: (i32, i32));
}
impl PointerTouchInfoExt for POINTER_TOUCH_INFO {
    fn set_point(&mut self, point: (i32, i32)) {
        self.pointerInfo.ptPixelLocation.x = point.0;
        self.pointerInfo.ptPixelLocation.y = point.1;
    }
}

fn sleep(ms: u64) {
    thread::sleep(time::Duration::from_millis(ms))
}