use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::com_object::{SAFEARRAYHelper};
use crate::winapi::{
    get_ansi_length,
};

use regex::Regex;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use serde_json;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("copy", 5, copy);
    sets.add("length", 2, length);
    sets.add("lengthb", 1, lengthb);
    sets.add("lengthu", 1, lengthu);
    sets.add("lengths", 1, lengths);
    sets.add("as_string", 1, as_string);
    sets.add("newre", 4, newre);
    sets.add("regex", 3, regex);
    sets.add("testre", 2, testre);
    sets.add("match", 2, regexmatch);
    sets.add("replace", 4, replace);
    sets.add("chgmoj", 4, replace);
    sets.add("tojson", 2, tojson);
    sets.add("fromjson", 1, fromjson);
    sets.add("copy", 3, copy);
    sets.add("pos", 3, pos);
    sets
}

pub fn length(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => s.chars().count(),
        Object::Num(n) => n.to_string().len(),
        Object::Array(v) => v.len(),
        Object::Bool(b) => b.to_string().len(),
        Object::HashTbl(h) => h.lock().unwrap().len(),
        Object::Struct(_, n, _) |
        Object::UStruct(_, n, _) => n,
        Object::Empty => 0,
        Object::Null => 1,
        Object::SafeArray(ref s) => {
            let get_dim = args.get_as_bool(1, Some(false))?;
            s.len(get_dim)?
        },
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o), args.name()))
    };
    Ok(Object::Num(len as f64))
}

pub fn lengthb(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => get_ansi_length(&s),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o), args.name()))
    };
    Ok(Object::Num(len as f64))
}

pub fn lengthu(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => s.as_bytes().len(),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o), args.name()))
    };
    Ok(Object::Num(len as f64))
}

pub fn lengths(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let length = str.chars()
            .map(|char| char.len_utf16())
            .reduce(|a,b| a+b)
            .unwrap_or_default();
    Ok(Object::Num(length as f64))
}

pub fn as_string(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::String(format!("{}", args.get_as_object(0, None)?)))
}

// 正規表現

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum RegexEnum {
    REGEX_TEST  = 0, // default
    REGEX_MATCH  = 1,
}

pub fn newre(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut pattern = args.get_as_string(0, None)?;
    let mut opt = String::new();
    if ! args.get_as_bool(1, Some(false))? {
        opt = format!("{}{}", opt, "i");
    };

    if args.get_as_bool(2, Some(false))? {
        opt = format!("{}{}", opt, "m");
    };
    if args.get_as_bool(3, Some(false))? {
        opt = format!("{}{}", opt, "a");
    };
    if opt.len() > 0 {
        pattern = format!("(?{}){}", opt, pattern);
    }
    Ok(Object::RegEx(pattern))
}

fn test_regex(target: String, pattern: String, f_name: String) -> Result<Object, UError> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => Ok(Object::Bool(
            re.is_match(target.as_str())
        )),
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern), f_name))
    }
}

fn match_regex(target: String, pattern: String, f_name: String) -> Result<Object, UError> {
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
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern), f_name))
    }
}

fn replace_regex(target: String, pattern: String, replace_to: String, f_name: String) -> Result<Object, UError> {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            Ok(Object::String(
                re.replace_all(target.as_str(), replace_to.as_str()).to_string()
            ))
        },
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern), f_name))
    }
}

pub fn testre(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    test_regex(target, pattern, args.name())
}

pub fn regex(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    match args.get_as_object(2, Some(Object::Empty))? {
        Object::Num(n) => {
            match FromPrimitive::from_f64(n).unwrap_or(RegexEnum::REGEX_TEST) {
                RegexEnum::REGEX_MATCH => match_regex(target, pattern, args.name()),
                _ => test_regex(target, pattern, args.name()),
            }
        },
        Object::String(s) |
        Object::RegEx(s) => replace_regex(target, pattern, s.clone(), args.name()),
        Object::Empty => test_regex(target, pattern, args.name()),
        o => Err(builtin_func_error(UErrorMessage::InvalidArgument(o), args.name()))
    }
}

pub fn regexmatch(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    match_regex(target, pattern, args.name())
}

pub fn replace(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let (pattern, is_regex) = match args.get_as_object(1, None)? {
        Object::String(s) => (s.clone(), args.get_as_bool(3, Some(false))?),
        Object::RegEx(re) => (re.clone(), true),
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o), args.name()))
    };
    let replace_to = args.get_as_string(2, None)?;

    if is_regex {
        replace_regex(target, pattern, replace_to, args.name())
    } else {
        let mut out = target.clone();
        let mut lower = target.to_ascii_lowercase();
        let pat_lower = pattern.to_ascii_lowercase();
        let len = pat_lower.len();
        let r = replace_to.as_str();
        loop {
            let pos = match lower.find(pat_lower.as_str()) {
                Some(n) => n,
                None => break,
            };
            lower.replace_range(pos..(pos+len), r);
            out.replace_range(pos..(pos+len), r);
        }
        Ok(Object::String(out))
    }
}

pub fn tojson(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prettify = args.get_as_bool(1, Some(false))?;
    let to_string = if prettify {serde_json::to_string_pretty} else {serde_json::to_string};
    let uo = args.get_as_uobject(0)?;
    let value = uo.value();
    to_string(&value).map_or_else(
        |e| Err(builtin_func_error(UErrorMessage::Any(e.to_string()), args.name())),
        |s| Ok(Object::String(s))
    )
}

pub fn fromjson(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let json = args.get_as_string(0, None)?;
    serde_json::from_str::<serde_json::Value>(json.as_str()).map_or_else(
        |_| Ok(Object::Empty),
        |v| Ok(Object::UObject(UObject::new(v)))
    )
}

pub fn copy(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let start = args.get_as_int(1, None::<usize>)?;
    let length = args.get_as_int_or_empty::<usize>(2)?;

    let chars = str.chars();
    let index = start.saturating_sub(1);
    let skipped = chars.skip(index);
    let copied: String = if let Some(l) = length {
        let took = skipped.take(l);
        took.collect()
    } else {
        skipped.collect()
    };
    Ok(copied.into())
}

pub fn pos(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let search = args.get_as_string(0, None)?;
    let mut str = args.get_as_string(1, None)?;
    let nth = args.get_as_int(2, Some(1_i32))?;

    let mut target = str.to_ascii_lowercase();
    let search = search.to_ascii_lowercase();

    if target.contains(&search) {
        let n = if nth == 0 {
            1
        } else {
            nth.abs() as usize
        };

        let mut pos = Some(0_usize);
        if nth < 0 {
            // 後ろから
            for _ in 0..n {
                match target.rfind(&search) {
                    Some(mut p) => {
                        loop {
                            p += 1;
                            if target.is_char_boundary(p) {
                                if let Some(pos) = pos.as_mut() {
                                    *pos = p;
                                }
                                break;
                            }
                        }
                        target.drain(p..);
                    },
                    None => {
                        pos = None;
                        break;
                    },
                };
            }
        } else {
            for _ in 0..n {
                match target.find(&search) {
                    Some(mut p) => {
                        loop {
                            p +=1;
                            if target.is_char_boundary(p) {
                                if let Some(pos) = pos.as_mut() {
                                    *pos += p;
                                }
                                break;
                            }
                        }
                        target.drain(..p);
                    },
                    None => {
                        pos = None;
                        break;
                    },
                };
            }
        };
        let pos = if let Some(p) = pos {
            str.truncate(p);
            str.chars().count()
        } else {
            0
        };
        Ok(pos.into())
    } else {
        Ok(0_usize.into())
    }
}