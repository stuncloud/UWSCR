use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use std::collections::HashMap;
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


pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("mmv", 3, mmv),
        ("btn", 5, btn),
        ("kbd", 3, kbd),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
    }
}

pub fn set_builtin_constant(map: &mut HashMap<String, Object>) {
    let num_constant = vec![
        ("LEFT"   , LEFT),
        ("RIGHT"  , RIGHT),
        ("MIDDLE" , MIDDLE),
        ("WHEEL"  , WHEEL),
        ("WHEEL2" , WHEEL2),
        ("TOUCH"  , TOUCH),
        ("CLICK"  , CLICK),
        ("DOWN"   , DOWN),
        ("UP"     , UP),
    ];
    for (key, value) in num_constant {
        map.insert(
            key.to_ascii_uppercase(),
            Object::BuiltinConst(Box::new(Object::Num(value.into())))
        );
    }
}

const LEFT: i32 = 0;
const RIGHT: i32 = 1;
const MIDDLE: i32 = 2;
const WHEEL: i32 = 5;
const WHEEL2: i32 = 6;
const TOUCH: i32 = 7;
const CLICK: i32 = 0;
const DOWN: i32 = 1;
const UP: i32 = 2;

pub fn mmv(args: Vec<Object>) -> Object {
    let x = match get_num_argument_value(&args, 0, 0.0) {
        Ok(n) => n as i32,
        Err(e) => return builtin_func_error("mmv", e.as_str())
    };
    let y = match get_num_argument_value(&args, 1, 0.0) {
        Ok(n) => n as i32,
        Err(e) => return builtin_func_error("mmv", e.as_str())
    };
    let ms = match get_num_argument_value(&args, 2, 0.0) {
        Ok(n) => n as u64,
        Err(_) => 0
    };
    let mut enigo = Enigo::new();
    thread::sleep(time::Duration::from_millis(ms));
    enigo.mouse_move_to(x, y);
    Object::Empty
}

pub fn btn(args: Vec<Object>) -> Object {
    let mut enigo = Enigo::new();
    let arg1 = match get_non_float_argument_value::<i32>(&args, 1, CLICK) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("btn", e.as_str())
    };
    let (cur_x, cur_y) = match get_current_pos() {
        Ok(p) => (p.x, p.y),
        Err(err) => return err
    };
    let x = match get_non_float_argument_value(&args, 2, cur_x) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("btn", e.as_str())
    };
    let y = match get_non_float_argument_value(&args, 3, cur_y) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("btn", e.as_str())

    };
    let ms= match get_non_float_argument_value::<u64>(&args, 4, 0) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("btn", e.as_str())
    };
    let button = match args[0] {
        Object::Num(n) => {
            match n as i32 {
                LEFT => MouseButton::Left,
                RIGHT => MouseButton::Right,
                MIDDLE => MouseButton::Middle,
                WHEEL => {
                    thread::sleep(time::Duration::from_millis(ms));
                    enigo.mouse_move_to(x, y);
                    enigo.mouse_scroll_y(arg1);
                    return Object::Empty;
                },
                WHEEL2 => {
                    thread::sleep(time::Duration::from_millis(ms));
                    enigo.mouse_move_to(x, y);
                    enigo.mouse_scroll_x(arg1);
                    return Object::Empty;
                },
                TOUCH => {
                    return builtin_func_error("btn", "TOUCH is not yet supported.")
                },
                _ => return builtin_func_error("btn", format!("bad argument: {}", n).as_str())
            }
        },
        _ => return builtin_func_error("btn", format!("bad argument: {}", args[0]).as_str())
    };

    thread::sleep(time::Duration::from_millis(ms));
    enigo.mouse_move_to(x, y);
    match arg1 {
        CLICK => enigo.mouse_click(button),
        DOWN => enigo.mouse_down(button),
        UP => enigo.mouse_up(button),
        _ => return builtin_func_error("btn", format!("bad argument: {}", arg1).as_str())
    }
    Object::Empty
}

pub fn get_current_pos() -> Result<POINT, Object>{
    let mut point = POINT {x: 0, y: 0};
    unsafe {
        if winuser::GetCursorPos(&mut point) == FALSE {
            return Err(Object::Error("failed to get cursor position".to_string()))
        };
    }
    Ok(point)
}

pub fn kbd(args: Vec<Object>) -> Object {
    let mut enigo = Enigo::new();
    let ms= match get_non_float_argument_value::<u64>(&args, 2, 0) {
        Ok(n) => n,
        Err(e) => return builtin_func_error("btn", e.as_str())
    };
    let key = match &args[0] {
        Object::Num(n) => Key::Raw(*n as u16),
        Object::String(s) => {
            thread::sleep(time::Duration::from_millis(ms));
            enigo.key_sequence(s.as_str());
            return Object::Empty;
        }
        _ => return builtin_func_error("kbd", format!("bad argument: {}", args[0]).as_str())
    };
    if args.len() >= 2 {
        thread::sleep(time::Duration::from_millis(ms));
        match args[1] {
            Object::Num(n) => match n as i32 {
                CLICK => enigo.key_click(key),
                DOWN => enigo.key_down(key),
                UP => enigo.key_up(key),
                _ => return builtin_func_error("kbd", format!("bad argument: {}", args[1]).as_str())
            },
            _ => return builtin_func_error("kbd", format!("bad argument: {}", args[1]).as_str())
        };
    } else {
        thread::sleep(time::Duration::from_millis(ms));
        enigo.key_click(key);
    }
    Object::Empty
}