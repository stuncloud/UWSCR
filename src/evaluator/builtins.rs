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
    text_control::set_builtin_constant(&mut builtins_consts);

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

// ビルトイン関数の引数を受け取るための関数群
// i: 引数のインデックス
// default: 省略可能な引数のデフォルト値、必須引数ならNoneを渡す
// 引数が省略されていた場合はdefaultの値を返す
// 引数が必須なのになかったらエラーを返す

pub fn get_non_float_argument_value<T>(args: &Vec<Object>, i: usize, default: Option<T>) -> Result<T, String>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        match args[i] {
            Object::Num(n) => T::cast(n).or(Err(
                format!("unable to cast {} to {}", n, std::any::type_name::<T>())
            )),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        default.ok_or(format!("argument {} required", i + 1))
    }
}

pub fn get_num_argument_value<T>(args: &Vec<Object>, i: usize, default: Option<T>) -> Result<T, String>
    where T: cast::From<f64, Output=T>,
{
    if args.len() >= i + 1 {
        match args[i] {
            Object::Num(n) => Ok(T::cast(n)),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        default.ok_or(format!("argument {} required", i + 1))
    }
}

pub fn get_string_argument_value(args: &Vec<Object>, i: usize, default: Option<String>) -> Result<String, String> {
    if args.len() >= i + 1 {
        match &args[i] {
            Object::String(s) => Ok(s.clone()),
            Object::RegEx(re) => Ok(re.clone()),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        default.ok_or(format!("argument {} required", i + 1))
    }
}

pub fn get_bool_argument_value(args: &Vec<Object>, i: usize, default: Option<bool>) -> Result<bool, String> {
    if args.len() >= i + 1 {
        match args[i] {
            Object::Bool(b) => Ok(b),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        default.ok_or(format!("argument {} required", i + 1))
    }
}

pub fn get_bool_or_int_argument_value<T>(args: &Vec<Object>, i: usize, default: Option<T>) -> Result<T, String>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        let err = "cast error".to_string();
        match args[i] {
            Object::Bool(b) => if b {
                T::cast(1.0).or(Err(err))
            } else {
                T::cast(0.0).or(Err(err))
            },
            Object::Num(n) => T::cast(n).or(Err(
                format!("unable to cast {} to {}", n, std::any::type_name::<T>())
            )),
            _ => Err(format!("bad argument: {}", args[i]))
        }
    } else {
        default.ok_or(format!("argument {} required", i + 1))
    }
}
