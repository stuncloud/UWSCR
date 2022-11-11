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
                KEYEVENTF_SCANCODE, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
                keybd_event, MapVirtualKeyW
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

fn send_win_key(vk: u8, action: KeyActionEnum, wait: u64) -> BuiltinFuncResult {
    thread::sleep(time::Duration::from_millis(wait));
    unsafe {
        let dw_flags = KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY;
        let scancode = MapVirtualKeyW(vk as u32, 0) as u8;
        match action {
            KeyActionEnum::CLICK => {
                keybd_event(
                    0,
                    scancode,
                    dw_flags,
                    0
                );
                // enigoと同様に20ms待つ
                thread::sleep(time::Duration::from_millis(20));
                keybd_event(
                    0,
                    scancode,
                    KEYEVENTF_KEYUP | dw_flags,
                    0
                );
            },
            KeyActionEnum::DOWN => {
                keybd_event(
                    0,
                    scancode,
                    dw_flags,
                    0
                );
            },
            KeyActionEnum::UP => {
                keybd_event(
                    0,
                    scancode,
                    KEYEVENTF_KEYUP | dw_flags,
                    0
                );
            },
        }
    }
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}

pub fn kbd(args: BuiltinFuncArgs) -> BuiltinFuncResult {

    let mut enigo = Enigo::new();
    let obj = args.get_as_object(0, None)?;
    let key_action = args.get_as_const::<KeyActionEnum>(1, false)?
        .unwrap_or(KeyActionEnum::CLICK);
    let ms= args.get_as_int::<u64>(2, Some(0))?;

    let vk_win = key_codes::VirtualKeyCode::VK_WIN as isize as f64;
    let vk_rwin = key_codes::VirtualKeyCode::VK_START as isize as f64;
    let key = match obj {
        Object::Num(n) => if n == vk_win || n == vk_rwin {
            return send_win_key(n as u8, key_action, ms)
        } else {
            Key::Raw(n as u16)
        },
        Object::String(s) => {
            thread::sleep(time::Duration::from_millis(ms));
            enigo.key_sequence(s.as_str());
            return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
        }
        _ => return Err(builtin_func_error(UErrorMessage::InvalidArgument(obj)))
    };
    thread::sleep(time::Duration::from_millis(ms));
    match key_action {
        KeyActionEnum::CLICK => enigo.key_click(key),
        KeyActionEnum::DOWN => enigo.key_down(key),
        KeyActionEnum::UP => enigo.key_up(key),
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Empty))
}