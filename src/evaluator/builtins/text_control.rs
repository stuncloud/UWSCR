use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::environment::NamedObject;

use regex::Regex;

pub fn set_builtins(vec: &mut Vec<NamedObject>) {
    let funcs : Vec<(&str, i32, fn(Vec<Object>)->Object)> = vec![
        ("copy", 5, copy),
        ("length", 1, length),
        ("lengthb", 1, lengthb),
        ("as_string", 1, as_string),
        ("newre", 4, newregex),
        ("regex", 3, regex),
        ("testre", 2, regextest),
        ("match", 2, regexmatch),
        ("replace", 4, replace),
        ("chgmoj", 4, replace),
    ];
    for (name, arg_len, func) in funcs {
        vec.push(NamedObject::new_builtin_func(name.to_ascii_uppercase(), Object::BuiltinFunction(arg_len, func)));
    }
    let num_constant = vec![
        ("REGEX_TEST" , REGEX_TEST),
        ("REGEX_MATCH", REGEX_MATCH),
    ];
    for (key, value) in num_constant {
        vec.push(NamedObject::new_builtin_const(key.to_ascii_uppercase(), Object::Num(value.into())));
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

// 正規表現

const REGEX_TEST : u8  = 0; // default
const REGEX_MATCH: u8  = 1;

pub fn newregex(args: Vec<Object>) -> Object {
    let mut pattern = match get_string_argument_value(&args, 0, None) {
        Ok(p) => p,
        Err(e) => return builtin_func_error("regex", e.as_str())
    };
    let mut opt = String::new();
    match get_bool_argument_value(&args, 1, Some(true)) {
        Ok(b) => if ! b {
            opt = format!("{}{}", opt, "i");
        },
        Err(e) => return builtin_func_error("regex", e.as_str())
    };

    match get_bool_argument_value(&args, 2, Some(false)) {
        Ok(b) => if b {
            opt = format!("{}{}", opt, "m");
        },
        Err(e) => return builtin_func_error("regex", e.as_str())
    };
    match get_bool_argument_value(&args, 3, Some(false)) {
        Ok(b) => if b {
            opt = format!("{}{}", opt, "a");
        },
        Err(e) => return builtin_func_error("regex", e.as_str())
    };
    if opt.len() > 0 {
        pattern = format!("(?{}){}", opt, pattern);
    }
    Object::RegEx(pattern)
}

fn test_regex(target: String, pattern: String) -> Result<Object, String> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => Ok(Object::Bool(
            re.is_match(target.as_str())
        )),
        Err(_) => Err("bad regex".to_string())
    }
}

fn match_regex(target: String, pattern: String) -> Result<Object, String> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            let mut matches = vec![];
            for cap in re.captures_iter(target.as_str()) {
                if cap.len() > 1 {
                    let mut sub = vec![];
                    for m in cap.iter() {
                        sub.push(Object::String(
                            m.unwrap().as_str().to_string()
                        ));
                    }
                    matches.push(Object::Array(sub));
                } else {
                    matches.push(Object::String(
                        cap.get(0).unwrap().as_str().to_string()
                    ))
                }
            }
            Ok(Object::Array(matches))
        },
        Err(_) => Err("bad regex".to_string())
    }
}

fn replace_regex(target: String, pattern: String, replace_to: String) -> Result<Object, String> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            Ok(Object::String(
                re.replace_all(target.as_str(), replace_to.as_str()).to_string()
            ))
        },
        Err(_) => Err("bad regex".to_string())
    }
}

pub fn regextest(args: Vec<Object>) -> Object {
    let target = match get_string_argument_value(&args, 0, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("testre", e.as_str())
    };
    let pattern = match get_string_argument_value(&args, 1, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("testre", e.as_str())
    };
    test_regex(target, pattern).unwrap_or_else(|e| builtin_func_error("testre", e.as_str()))
}

pub fn regex(args: Vec<Object>) -> Object {
    let target = match get_string_argument_value(&args, 0, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("regex", e.as_str())
    };
    let pattern = match get_string_argument_value(&args, 1, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("regex", e.as_str())
    };
    if args.len() >= 3 {
        match &args[2] {
            Object::Num(n) => {
                match *n as u8 {
                    REGEX_MATCH => match_regex(target, pattern).unwrap_or_else(|e| builtin_func_error("regex", e.as_str())),
                    _ => test_regex(target, pattern).unwrap_or_else(|e| builtin_func_error("regex", e.as_str())),
                }
            },
            Object::String(s) |
            Object::RegEx(s) => {
                replace_regex(target, pattern, s.clone()).unwrap_or_else(|e| builtin_func_error("regex", e.as_str()))
            },
            Object::Error(e) => Object::Error(e.clone()),
            _ => Object::Error(format!("bad argument: {}", args[2]))
        }
    } else {
        test_regex(target, pattern).unwrap_or_else(|e| builtin_func_error("regex", e.as_str()))
    }
}

pub fn regexmatch(args: Vec<Object>) -> Object {
    let target = match get_string_argument_value(&args, 0, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("match", e.as_str())
    };
    let pattern = match get_string_argument_value(&args, 1, None) {
        Ok(t) => t,
        Err(e) => return builtin_func_error("match", e.as_str())
    };
    match_regex(target, pattern).unwrap_or_else(|e| builtin_func_error("match", e.as_str()))
}

pub fn replace(args: Vec<Object>) -> Object {
    let target = match get_string_argument_value(&args, 0, None) {
        Ok(s) => s,
        Err(e) => return builtin_func_error("replace", e.as_str())
    };
    let pattern = match get_string_argument_value(&args, 1, None) {
        Ok(s) => s,
        Err(e) => return builtin_func_error("replace", e.as_str())
    };
    let replace_to = match get_string_argument_value(&args, 2, None) {
        Ok(s) => s,
        Err(e) => return builtin_func_error("replace", e.as_str())
    };
    let is_regex = match get_bool_argument_value(&args, 3, Some(false)) {
        Ok(b) => match args[1] {
            Object::RegEx(_) => true,
            _ => b
        },
        Err(e) => return builtin_func_error("replace", e.as_str())
    };

    if is_regex {
        replace_regex(target, pattern, replace_to).unwrap_or_else(|e| builtin_func_error("replace", e.as_str()))
    } else {
        Object::String(
            target.replace(&pattern, replace_to.as_str())
        )
    }
}
