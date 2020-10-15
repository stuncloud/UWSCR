pub mod window_control;
pub mod text_control;

use crate::evaluator::object::*;
use std::collections::HashMap;

pub fn init_builtins() -> HashMap<String, Object> {
    let builtin_functin_list : Vec<(&str, i32, fn(Vec<Object>)->Object)>= vec![
        ("getid", 4, window_control::getid),
        ("clkitem", 5, window_control::clkitem),
        ("copy", 5, text_control::copy),
        ("length", 1, text_control::length),
        ("lengthb", 1, text_control::lengthb),
        ("asstring", 1, text_control::as_string),
        ("eval", 1, builtin_eval),
    ];
    let mut builtins = HashMap::new();
    for (name, args_len, func) in builtin_functin_list {
        builtins.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(args_len, func));
    }

    let builtin_constants = vec![
        ("HASH_CASECARE", 0x00001000),
        ("HASH_SORT", 0x00002000),
    ];
    for (name, value) in builtin_constants {
        builtins.insert(name.to_ascii_uppercase(), Object::Num(value as f64));
    }
    builtins
}

pub fn builtin_func_error(name: &str,msg: &str)-> Object {
    Object::Error(format!("builtin function error [{}]: {}", name, msg))
}

pub fn builtin_eval(args: Vec<Object>) -> Object {
    match &args[0] {
        Object::String(s) => Object::Eval(s.to_string()),
        _ => builtin_func_error("eval", "given value is not string")
    }
}