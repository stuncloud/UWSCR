use crate::evaluator::object::*;
use crate::evaluator::builtins::builtin_func_error;

use std::collections::HashMap;

pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs : Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("copy", 5, copy),
        ("length", 1, length),
        ("lengthb", 1, lengthb),
        ("as_string", 1, as_string),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
    }
}

pub fn copy(args: Vec<Object>) -> Object {
    Object::String(format!("{}", args.len()))
}

pub fn length(args: Vec<Object>) -> Object {
    let len = match &args[0] {
        Object::String(s) => s.chars().count(),
        Object::Num(n) => n.to_string().len(),
        Object::Array(v) => v.len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Hash(h, _) => h.len(),
        Object::SortedHash(t, _) => t.len(),
        Object::Empty => 0,
        Object::Null => 1,
        _ => return builtin_func_error("length", "given value is not countable")
    };
    Object::Num(len as f64)
}

pub fn lengthb(args: Vec<Object>) -> Object {
    let len = match &args[0] {
        Object::String(s) => s.as_bytes().len(),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        _ => return builtin_func_error("length", "given value is not countable")
    };
    Object::Num(len as f64)
}

pub fn as_string(args: Vec<Object>) -> Object {
    Object::String(format!("{}", &args[0]))
}