pub mod window_control;
pub mod text_control;

use crate::evaluator::object::*;
use std::collections::HashMap;

pub fn init_builtins() -> (HashMap<String, Object>, HashMap<String, Object>) {
    let mut builtins_funcs = HashMap::new();
    let builtin_function_list : Vec<(&str, i32, fn(Vec<Object>)->Object)>= vec![
        ("getid", 4, window_control::getid),
        ("clkitem", 5, window_control::clkitem),
        ("copy", 5, text_control::copy),
        ("length", 1, text_control::length),
        ("lengthb", 1, text_control::lengthb),
        ("as_string", 1, text_control::as_string),
        ("eval", 1, builtin_eval),
        ("print_env", 1, print_env),
    ];
    for (name, args_len, func) in builtin_function_list {
        builtins_funcs.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(args_len, func));
    }

    let mut builtins_consts = HashMap::new();
    let builtin_constant_list = vec![
        ("HASH_CASECARE", 0x00001000),
        ("HASH_SORT", 0x00002000),
    ];
    for (name, value) in builtin_constant_list {
        builtins_consts.insert(name.to_ascii_uppercase(), Object::Num(value as f64));
    }
    (builtins_funcs, builtins_consts)
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

pub fn print_env(args: Vec<Object>) -> Object {
    match &args[0] {
        Object::String(s) => Object::Debug(DebugType::PrintEnv(s.to_string())),
        _ => Object::Empty
    }
}