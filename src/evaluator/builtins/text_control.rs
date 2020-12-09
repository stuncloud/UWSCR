use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use regex::Regex;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("copy", 5, copy);
    sets.add("length", 1, length);
    sets.add("lengthb", 1, lengthb);
    sets.add("as_string", 1, as_string);
    sets.add("newre", 4, newre);
    sets.add("regex", 3, regex);
    sets.add("testre", 2, testre);
    sets.add("match", 2, regexmatch);
    sets.add("replace", 4, replace);
    sets.add("chgmoj", 4, replace);
    sets
}

pub fn copy(_args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::String("copy is not working for now, sorry!".into()))
}

pub fn length(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match get_any_argument_value(&args, 0, None)? {
        Object::String(s) => s.chars().count(),
        Object::Num(n) => n.to_string().len(),
        Object::Array(v) => v.len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Hash(h, _) => h.len(),
        Object::SortedHash(t, _) => t.len(),
        Object::Empty => 0,
        Object::Null => 1,
        _ => return Err(builtin_func_error("length", "given value is not countable"))
    };
    Ok(Object::Num(len as f64))
}

pub fn lengthb(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match get_any_argument_value(&args, 0, None)? {
        Object::String(s) => s.as_bytes().len(),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        _ => return Err(builtin_func_error("length", "given value is not countable"))
    };
    Ok(Object::Num(len as f64))
}

pub fn as_string(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::String(format!("{}", get_any_argument_value(&args, 0, None)?)))
}

// 正規表現

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum RegexEnum {
    REGEX_TEST  = 0, // default
    REGEX_MATCH  = 1,
}

pub fn newre(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut pattern = get_string_argument_value(&args, 0, None)?;
    let mut opt = String::new();
    if ! get_bool_argument_value(&args, 1, Some(true))? {
        opt = format!("{}{}", opt, "i");
    };

    if get_bool_argument_value(&args, 2, Some(false))? {
        opt = format!("{}{}", opt, "m");
    };
    if get_bool_argument_value(&args, 3, Some(false))? {
        opt = format!("{}{}", opt, "a");
    };
    if opt.len() > 0 {
        pattern = format!("(?{}){}", opt, pattern);
    }
    Ok(Object::RegEx(pattern))
}

fn test_regex(target: String, pattern: String, f_name: &str) -> Result<Object, UError> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => Ok(Object::Bool(
            re.is_match(target.as_str())
        )),
        Err(_) => Err(builtin_func_error(f_name, "bad regex".to_string()))
    }
}

fn match_regex(target: String, pattern: String, f_name: &str) -> Result<Object, UError> {
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
        Err(_) => Err(builtin_func_error(f_name, "bad regex".to_string()))
    }
}

fn replace_regex(target: String, pattern: String, replace_to: String, f_name: &str) -> Result<Object, UError> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            Ok(Object::String(
                re.replace_all(target.as_str(), replace_to.as_str()).to_string()
            ))
        },
        Err(_) => Err(builtin_func_error(f_name, "bad regex".to_string()))
    }
}

pub fn testre(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = get_string_argument_value(&args, 0, None)?;
    let pattern = get_string_argument_value(&args, 1, None)?;
    test_regex(target, pattern, args.name())
}

pub fn regex(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = get_string_argument_value(&args, 0, None)?;
    let pattern = get_string_argument_value(&args, 1, None)?;
    match get_any_argument_value(&args, 2, Some(Object::Empty))? {
        Object::Num(n) => {
            match FromPrimitive::from_f64(n).unwrap_or(RegexEnum::REGEX_TEST) {
                RegexEnum::REGEX_MATCH => match_regex(target, pattern, args.name()),
                _ => test_regex(target, pattern, args.name()),
            }
        },
        Object::String(s) |
        Object::RegEx(s) => replace_regex(target, pattern, s.clone(), args.name()),
        Object::Error(e) => Ok(Object::Error(e.clone())),
        Object::Empty => test_regex(target, pattern, args.name()),
        _ => Err(builtin_func_error(args.name(), format!("bad argument: {}", args.item(2).unwrap())))
    }
}

pub fn regexmatch(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = get_string_argument_value(&args, 0, None)?;
    let pattern = get_string_argument_value(&args, 1, None)?;
    match_regex(target, pattern, args.name())
}

pub fn replace(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = get_string_argument_value(&args, 0, None)?;
    let (pattern, is_regex) = match get_any_argument_value(&args, 1, None)? {
        Object::String(s) => (s.clone(), get_bool_argument_value(&args, 3, Some(false))?),
        Object::RegEx(re) => (re.clone(), true),
        _ => return Err(builtin_func_error(args.name(), format!("bad argument: {}", args.item(1).unwrap())))
    };
    let replace_to = get_string_argument_value(&args, 2, None)?;

    if is_regex {
        replace_regex(target, pattern, replace_to, args.name())
    } else {
        Ok(Object::String(
            target.replace(&pattern, replace_to.as_str())
        ))
    }
}
