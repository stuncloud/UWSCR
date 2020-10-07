pub mod window_control;
pub mod text_control;

use crate::evaluator::object::*;
use std::collections::HashMap;

pub fn init_builtins() -> HashMap<String, Object> {
    let builtin_functin_list : Vec<(&str, i32, fn(Vec<Object>)->Object)>= vec![
        ("getid", 4, window_control::getid),
        ("clkitem", 5, window_control::clkitem),
        ("copy", 5, text_control::copy),
    ];
    let mut builtins = HashMap::new();
    for (name, args_len, func) in builtin_functin_list {
        builtins.insert(name.to_string(), Object::BuiltinFunction(args_len, func));
    }

    let builtin_constants = vec![
        ("HASH_CASECARE", 0x00001000),
        ("HASH_SORT", 0x00002000),
    ];
    for (name, value) in builtin_constants {
        builtins.insert(name.to_string(), Object::Num(value as f64));
    }
    builtins
}
