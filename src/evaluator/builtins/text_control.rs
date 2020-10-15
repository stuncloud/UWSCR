use crate::evaluator::object::*;
use crate::evaluator::builtins::builtin_func_error;

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