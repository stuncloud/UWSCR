use crate::evaluator::object::*;
use crate::evaluator::builtins::builtin_func_error;

use std::collections::HashMap;
use std::{thread, time};

pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("sleep", 1, sleep),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
    }
}

pub fn sleep(args: Vec<Object>) -> Object {
    match args[0] {
        Object::Num(n) => {
            if n > 0.0 {
                thread::sleep(time::Duration::from_secs_f64(n));
            }
        },
        _ => return builtin_func_error("sleep", format!("bad argument: {}", args[0]).as_str())
    }
    Object::Empty
}