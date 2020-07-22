pub mod window;
pub mod textcontrol;

use crate::evaluator::object::*;
use std::collections::HashMap;
// use crate::evaluator::builtins::*;

pub fn init() -> HashMap<String, Object> {
    let builtin_functin_list : Vec<(&str, i32, fn(Vec<Object>)->Object)>= vec![
        ("getid", 4, window::getid),
        ("clkitem", 5, window::clkitem),
        ("copy", 5, textcontrol::copy),
    ];
    let mut builtins = HashMap::new();
    for (name, args_len, func) in builtin_functin_list {
        builtins.insert(String::from(name), Object::Builtin(args_len, func));
    }
    builtins
}
