pub mod window_control;
pub mod window_low;
pub mod text_control;
pub mod system_controls;
pub mod key_codes;

use crate::evaluator::object::*;
use std::collections::HashMap;

use cast;

pub fn init_builtins() -> (HashMap<String, Object>, HashMap<String, Object>) {
    // Builtin function
    let mut builtins_funcs = HashMap::new();
    set_builtin_functions(&mut builtins_funcs);
    window_control::set_builtin_functions(&mut builtins_funcs);
    window_low::set_builtin_functions(&mut builtins_funcs);
    text_control::set_builtin_functions(&mut builtins_funcs);
    system_controls::set_builtin_functions(&mut builtins_funcs);

    // Builtin Constant
    let mut builtins_consts = HashMap::new();
    set_builtin_constant(&mut builtins_consts);
    key_codes::set_builtin_constant(&mut builtins_consts);
    window_control::set_builtin_constant(&mut builtins_consts);
    window_low::set_builtin_constant(&mut builtins_consts);

    (builtins_funcs, builtins_consts)
}

fn set_builtin_constant(map: &mut HashMap<String, Object>) {
    let num_constant = vec![
        ("HASH_CASECARE", 0x00001000),
        ("HASH_SORT", 0x00002000),
    ];
    for (name, value) in num_constant {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinConst(Box::new(Object::Num(value as f64))));
    }
    map.insert("GET_UWSC_PRO".to_string(), Object::BuiltinConst(Box::new(Object::Bool(false))));
}

pub fn set_builtin_functions(map: &mut HashMap<String, Object>) {
    let funcs: Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("eval", 1, builtin_eval),
        ("print_env", 1, print_env),
    ];
    for (name, arg_len, func) in funcs {
        map.insert(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func));
    }
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

pub fn builtin_func_error(name: &str,msg: &str)-> Object {
    Object::Error(format!("builtin function error [{}]: {}", name, msg))
}

pub fn get_non_float_argument_value<T>(args: &Vec<Object>, i: usize, default: T) -> Result<T, String>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        match args[i] {
            Object::Num(n) => match T::cast(n) {
                Ok(t) => Ok(t),
                Err(_) => Err("cast error".to_string())
            },
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        Ok(default)
    }
}

pub fn get_num_argument_value<T>(args: &Vec<Object>, i: usize, default: T) -> Result<T, String>
    where T: cast::From<f64, Output=T>,
{
    if args.len() >= i + 1 {
        match args[i] {
            Object::Num(n) => Ok(T::cast(n)),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        Ok(default)
    }
}

pub fn get_string_argument_value(args: &Vec<Object>, i: usize, default: String) -> Result<String, String> {
    if args.len() >= i + 1 {
        match &args[i] {
            Object::String(s) => Ok(s.clone()),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        Ok(default)
    }
}
