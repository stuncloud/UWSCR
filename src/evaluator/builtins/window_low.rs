use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use std::{thread, time};

use enigo::*;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use windows::{
    Win32::{
        Foundation::POINT,
        UI::{
            Input::KeyboardAndMouse::{
                KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
                SendInput, INPUT, KEYBDINPUT, INPUT_KEYBOARD, VIRTUAL_KEY,
            },
            WindowsAndMessaging::{GetCursorPos, SetCursorPos},
        },
    },
};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("mmv", 3, mmv);
    sets.add("btn", 5, btn);
    sets.add("kbd", 3, kbd);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum MouseButtonEnum {
    LEFT = 0,
    RIGHT = 1,
    MIDDLE = 2,
    WHEEL = 5,
    WHEEL2 = 6,
    TOUCH = 7,
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum KeyActionEnum {
    CLICK = 0,
    DOWN = 1,
    UP = 2,
}

pub fn move_mouse_to(x: i32, y: i32) -> bool {
    unsafe {
        SetCursorPos(x, y);
        SetCursorPos(x, y).as_bool()
    }
}

pub fn mmv(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let x = args.get_as_int(0, Some(0))?;
    let y = args.get_as_int(1, Some(0))?;
    let ms = args.get_as_int::<u64>(2, Some(0))?;
    thread::sleep(time::Duration::from_millis(ms));
    // move_mouse_to_scaled(x, y);
    move_mouse_to(x, y);
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}

pub fn btn(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let Some(btn) = args.get_as_const::<MouseButtonEnum>(0, true)? else {
        // 不正な定数の場合何もしない
        return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
    };
    let mut enigo = Enigo::new();
    let action = args.get_as_int::<i32>(1, Some(KeyActionEnum::CLICK as i32))?;
    let p = get_current_pos()?;
    let (cur_x, cur_y) = (p.x, p.y);
    let x = args.get_as_int( 2, Some(cur_x))?;
    let y = args.get_as_int( 3, Some(cur_y))?;
    let ms= args.get_as_int::<u64>(4, Some(0))?;
    let button = match btn {
        MouseButtonEnum::LEFT => MouseButton::Left,
        MouseButtonEnum::RIGHT => MouseButton::Right,
        MouseButtonEnum::MIDDLE => MouseButton::Middle,
        MouseButtonEnum::WHEEL => {
            thread::sleep(time::Duration::from_millis(ms));
            move_mouse_to(x, y);
            enigo.mouse_scroll_y(action);
            return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
        },
        MouseButtonEnum::WHEEL2 => {
            thread::sleep(time::Duration::from_millis(ms));
            move_mouse_to(x, y);
            enigo.mouse_scroll_x(action);
            return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
        },
        MouseButtonEnum::TOUCH => {
            return Err(builtin_func_error(UErrorMessage::NotYetSupported("TOUCH".into())));
        },
    };

    thread::sleep(time::Duration::from_millis(ms));
    move_mouse_to(x, y);
    match FromPrimitive::from_i32(action).unwrap_or(KeyActionEnum::CLICK) {
        KeyActionEnum::CLICK => enigo.mouse_click(button),
        KeyActionEnum::DOWN => enigo.mouse_down(button),
        KeyActionEnum::UP => enigo.mouse_up(button),
    }
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}

pub fn get_current_pos() -> BuiltInResult<POINT>{
    let mut point = POINT {x: 0, y: 0};
    unsafe {
        if GetCursorPos(&mut point).as_bool() == false {
            return Err(builtin_func_error(UErrorMessage::UnableToGetCursorPosition));
        };
    }
    Ok(point)
}

pub fn kbd(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let key = args.get_as_num_or_string(0)?;
    let action = args.get_as_const::<KeyActionEnum>(1, false)?
        .unwrap_or(KeyActionEnum::CLICK);
    let wait= args.get_as_int::<u64>(2, Some(0))?;

    let vk_win = key_codes::VirtualKeyCode::VK_WIN as u8;
    let vk_rwin = key_codes::VirtualKeyCode::VK_START as u8;
    match key {
        TwoTypeArg::U(vk) => {
            let extend = vk == vk_win || vk == vk_rwin;
            Input::send_key(vk, action, wait, extend);
        },
        TwoTypeArg::T(s) => {
            Input::send_str(&s, wait);
        }
    };
    Ok(BuiltinFuncReturnValue::Empty)
}


struct Input {}

impl Input {
    fn send_key(vk: u8, action: KeyActionEnum, wait: u64, extend: bool) {
        thread::sleep(time::Duration::from_millis(wait));
        match action {
            KeyActionEnum::CLICK => {
                Self::key_down(vk, extend);
                // 20ms待って離す
                thread::sleep(time::Duration::from_millis(20));
                Self::key_up(vk, extend)
            },
            KeyActionEnum::DOWN => Self::key_down(vk, extend),
            KeyActionEnum::UP => Self::key_up(vk, extend),
        }
    }
    fn send_str(str: &str, wait: u64) {
        thread::sleep(time::Duration::from_millis(wait));
        unsafe {
            let pinputs = str.encode_utf16()
                .map(|scan| {
                    let mut input = INPUT::default();
                    input.r#type = INPUT_KEYBOARD;
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: scan,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    input
                })
                .collect::<Vec<_>>();
            SendInput(&pinputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
    fn key_down(vk: u8, extend: bool) {
        unsafe {
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
                dwExtraInfo: 0,
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
    fn key_up(vk: u8, extend: bool) {
        unsafe {
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
                dwExtraInfo: 0,
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
}