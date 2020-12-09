use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use std::{thread, time};

use enigo::*;
use winapi::{
    um::{
        winuser,
    },
    shared::{
        windef::{POINT},
        minwindef::{FALSE},
    },
};
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("mmv", 3, mmv);
    sets.add("btn", 5, btn);
    sets.add("kbd", 3, kbd);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum MouseButtonEnum {
    LEFT = 0,
    RIGHT = 1,
    MIDDLE = 2,
    WHEEL = 5,
    WHEEL2 = 6,
    TOUCH = 7,
    UNKNOWN_MOUSE_BUTTON = -1,
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum KeyActionEnum {
    CLICK = 0,
    DOWN = 1,
    UP = 2,
    UNKNOWN_ACTION = -1,
}

pub fn mmv(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let x = get_non_float_argument_value(&args, 0, Some(0))?;
    let y = get_non_float_argument_value(&args, 1, Some(0))?;
    let ms = get_non_float_argument_value::<u64>(&args, 2, Some(0))?;
    let mut enigo = Enigo::new();
    thread::sleep(time::Duration::from_millis(ms));
    enigo.mouse_move_to(x, y);
    Ok(Object::Empty)
}

pub fn btn(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut enigo = Enigo::new();
    let arg1 = get_non_float_argument_value::<i32>(&args, 1, Some(KeyActionEnum::CLICK as i32))?;
    let p = get_current_pos(args.name())?;
    let (cur_x, cur_y) = (p.x, p.y);
    let x = get_non_float_argument_value(&args, 2, Some(cur_x))?;
    let y = get_non_float_argument_value(&args, 3, Some(cur_y))?;
    let ms= get_non_float_argument_value::<u64>(&args, 4, Some(0))?;
    let btn = get_non_float_argument_value::<i32>(&args, 0, None)?;
    let button = match FromPrimitive::from_i32(btn).unwrap_or(MouseButtonEnum::UNKNOWN_MOUSE_BUTTON) {
        MouseButtonEnum::LEFT => MouseButton::Left,
        MouseButtonEnum::RIGHT => MouseButton::Right,
        MouseButtonEnum::MIDDLE => MouseButton::Middle,
        MouseButtonEnum::WHEEL => {
            thread::sleep(time::Duration::from_millis(ms));
            enigo.mouse_move_to(x, y);
            enigo.mouse_scroll_y(arg1);
            return Ok(Object::Empty);
        },
        MouseButtonEnum::WHEEL2 => {
            thread::sleep(time::Duration::from_millis(ms));
            enigo.mouse_move_to(x, y);
            enigo.mouse_scroll_x(arg1);
            return Ok(Object::Empty);
        },
        MouseButtonEnum::TOUCH => {
            return Err(builtin_func_error(args.name(), "TOUCH is not yet supported."));
        },
        _ => return Ok(Object::Empty)
    };

    thread::sleep(time::Duration::from_millis(ms));
    enigo.mouse_move_to(x, y);
    match FromPrimitive::from_i32(arg1).unwrap_or(KeyActionEnum::CLICK) {
        KeyActionEnum::CLICK => enigo.mouse_click(button),
        KeyActionEnum::DOWN => enigo.mouse_down(button),
        KeyActionEnum::UP => enigo.mouse_up(button),
        _ => return Err(builtin_func_error(args.name(), format!("bad argument: {}", arg1)))
    }
    Ok(Object::Empty)
}

pub fn get_current_pos(name: &str) -> Result<POINT, UError>{
    let mut point = POINT {x: 0, y: 0};
    unsafe {
        if winuser::GetCursorPos(&mut point) == FALSE {
            return Err(builtin_func_error("failed to get cursor position".into(), name));
        };
    }
    Ok(point)
}

pub fn kbd(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut enigo = Enigo::new();
    let ms= get_non_float_argument_value::<u64>(&args, 2, Some(0))?;
    let obj = get_any_argument_value(&args, 0, None)?;
    let key = match obj {
        Object::Num(n) => Key::Raw(n as u16),
        Object::String(s) => {
            thread::sleep(time::Duration::from_millis(ms));
            enigo.key_sequence(s.as_str());
            return Ok(Object::Empty);
        }
        _ => return Err(builtin_func_error(args.name(), format!("bad argument: {}", obj)))
    };
    if args.len() >= 2 {
        thread::sleep(time::Duration::from_millis(ms));
        let action = get_non_float_argument_value::<i32>(&args, 1, Some(0))?;
        match FromPrimitive::from_i32(action).unwrap_or(KeyActionEnum::UNKNOWN_ACTION) {
            KeyActionEnum::CLICK => enigo.key_click(key),
            KeyActionEnum::DOWN => enigo.key_down(key),
            KeyActionEnum::UP => enigo.key_up(key),
            _ => (),
        };
    } else {
        thread::sleep(time::Duration::from_millis(ms));
        enigo.key_click(key);
    }
    Ok(Object::Empty)
}