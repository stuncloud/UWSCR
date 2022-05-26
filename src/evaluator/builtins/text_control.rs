use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::com_object::{SAFEARRAYHelper};
use crate::error::evaluator::UErrorMessage::BuiltinArgCastError;
use crate::winapi::{
    get_ansi_length, from_ansi_bytes, to_ansi_bytes, contains_unicode_char,
};

use regex::Regex;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use serde_json;
use kanaria::{
    string::UCSStr,
    utils::{ConvertTarget, CharExtend}
};

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
    sets.add("betweenstr", 5, betweenstr);
    sets.add("chknum", 1, chknum);
    sets.add("val", 2, val);
    sets.add("trim", 2, trim);
    sets.add("chr", 1, chr);
    sets.add("asc", 1, asc);
    sets.add("chrb", 1, chrb);
    sets.add("ascb", 1, ascb);
    sets.add("isunicode", 1, isunicode);
    sets.add("strconv", 2, strconv);
    sets.add("format", 4, format);
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

fn find_pos(str: &str, substr: &str) -> Option<usize>{
    match str.find(substr) {
        Some(mut p) => {
            loop {
                p+=1;
                if str.is_char_boundary(p) {
                    break;
                }
            }
            Some(p)
        },
        None => None,
    }
}
fn rfind_pos(str: &str, substr: &str) -> Option<usize>{
    match str.rfind(substr) {
        Some(mut p) => {
            loop {
                p+=1;
                if str.is_char_boundary(p) {
                    break;
                }
            }
            Some(p)
        },
        None => None,
    }
}

fn find_nth(str: &str, substr: &str, nth: i32) -> Option<usize> {
    let mut str = str.to_ascii_lowercase();
    let substr = substr.to_ascii_lowercase();
    let n = if nth == 0 {1} else {nth.abs() as usize};
    if str.contains(&substr) {
        let mut pos = Some(0_usize);
        for _ in 0..n {
            if nth < 0 {
                match rfind_pos(&str, &substr) {
                    Some(p) => {
                        if let Some(pos) = pos.as_mut() {
                            *pos = p;
                        }
                        str.drain(p..);
                    },
                    None => return None,
                }
            } else {
                match find_pos(&str, &substr) {
                    Some(p) => {
                        if let Some(pos) = pos.as_mut() {
                            *pos += p;
                        }
                        str.drain(..p);
                    },
                    None => return None,
                }
            }
        }
        pos
    } else {
        None
    }
}

pub fn pos(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let substr = args.get_as_string(0, None)?;
    let mut str = args.get_as_string(1, None)?;
    let nth = args.get_as_int(2, Some(1_i32))?;

    let pos = if let Some(p) = find_nth(&str, &substr, nth) {
        str.truncate(p);
        str.chars().count()
    } else {
        0
    };
    Ok(pos.into())
}

fn truncate_str(str: &mut String, mut p: usize) {
    loop {
        p-=1;
        if str.is_char_boundary(p) {
            break;
        }
    }
    str.truncate(p);
}
fn drain_str(str: &mut String, mut p: usize) {
    loop {
        p-=1;
        if str.is_char_boundary(p) {
            break;
        }
    }
    str.drain(..p);
}
fn next_pos(str: &str, mut p: usize) -> usize {
    loop {
        p += 1;
        if str.is_char_boundary(p) {
            break p;
        }
    }
}

fn find_nth_between(str: &str, from: &str, to: &str, nth: i32, flag: bool) -> Option<(usize, usize)> {
    let mut lower = str.to_ascii_lowercase();
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    let n = if nth == 0 {1} else {nth.abs() as usize};

    let mut pos = Some((0_usize, 0_usize));
    if lower.contains(&from) && lower.contains(&to) {
        if nth < 0 {
            for _ in 0..n {
                let to_found_at = match lower.rfind(&to) {
                    Some(p) => p,
                    None => {
                        pos = None;
                        break;
                    },
                };
                let mut temp = lower.clone();
                temp.drain(to_found_at..);
                let from_found_at = match temp.rfind(&from) {
                    Some(p) => p,
                    None => {
                        pos = None;
                        break;
                    },
                };
                let from_pos = from_found_at + from.len();
                if let Some((p, len)) = pos.as_mut() {
                    *p = from_pos;
                    *len = to_found_at - from_pos;
                }
                let drain_from = if flag {
                    to_found_at
                } else {
                    from_found_at
                };
                lower.drain(drain_from..);
            }
        } else {
            let mut drained = 0_usize;
            for _i in 0..n {
                let from_found_at = match lower.find(&from) {
                    Some(p) => p,
                    None => {
                        pos = None;
                        break;
                    },
                };
                let from_pos = from_found_at + from.len();
                let mut temp = lower.clone();
                temp.drain(..from_pos);
                let to_found_at = match temp.find(&to) {
                    Some(p) => p,
                    None => {
                        pos = None;
                        break;
                    },
                };
                let drain_to = if flag {
                    next_pos(&lower, from_found_at)
                } else {
                    next_pos(&lower, to_found_at + from_pos)
                };
                if let Some((p, len)) = pos.as_mut() {
                    *p = drained + from_pos;
                    *len = to_found_at;
                }
                drained += drain_to;
                lower.drain(..drain_to);
            }
        }
        pos
    } else {
        None
    }
}

fn drain_between(str: &mut String, pos: usize, len: usize) {
    str.drain(..pos);
    str.truncate(len);
}

pub fn betweenstr(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut str = args.get_as_string(0, None)?;
    let from = args.get_as_string_or_empty(1)?;
    let to = args.get_as_string_or_empty(2)?;
    let nth = args.get_as_int(3, Some(1_i32))?;
    let flag = args.get_as_bool(4, Some(false))?;

    let between = match (from, to) {
        (None, None) => Some(str),
        (None, Some(to)) => match find_nth(&str, &to, nth) {
            Some(p) => {
                truncate_str(&mut str, p);
                Some(str)
            },
            None => None,
        },
        (Some(from), None) => match find_nth(&str, &from, nth) {
            Some(mut p) => {
                p += from.len();
                drain_str(&mut str, p);
                Some(str)
            },
            None => None,
        },
        (Some(from), Some(to)) => match find_nth_between(&str, &from, &to, nth, flag) {
            Some((pos, len)) => {
                drain_between(&mut str, pos, len);
                Some(str)
            },
            None => None,
        },
    };

    Ok(between.unwrap_or_default().into())
}

pub fn chknum(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let result = args.get_as_int(0, None::<i32>).is_ok();
    Ok(Object::Bool(result))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum ErrConst {
    ERR_VALUE = -999999,
}

pub fn val(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let result = args.get_as_num(0, None::<f64>);
    let err = args.get_as_num(1, Some(ErrConst::ERR_VALUE as i32 as f64))?;
    let val = result.unwrap_or(err);
    Ok(val.into())
}

pub fn trim(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let trim_option = args.get_as_string_or_bool(1, Some(TwoTypeArg::U(false)))?;
    let trimed = match trim_option {
        TwoTypeArg::T(s) => {
            // トリム文字指定
            let chars = s.chars().collect::<Vec<_>>();
            target.trim_matches(chars.as_slice())
        },
        TwoTypeArg::U(b) => if b {
            target.trim_matches(|c: char| {c.is_ascii_whitespace() || c.is_ascii_control() || c == '　'})
        } else {
            target.trim_matches(|c: char| {c.is_ascii_whitespace() || c.is_ascii_control()})
        },
    };
    Ok(trimed.into())
}

pub fn chr(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let code = args.get_as_int(0, None::<u32>)?;
    let char = match char::from_u32(code) {
        Some(c) => c.to_string(),
        None => String::new(),
    };
    Ok(char.into())
}
pub fn asc(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let code = match str.chars().next() {
        Some(first) => {
            first as u32
        },
        None => 0,
    };
    Ok(Object::Num(code as f64))
}

pub fn chrb(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let code = match args.get_as_int(0, None::<u8>) {
        Ok(n) => n,
        Err(e) => if let BuiltinArgCastError(_, _) = e.message {
            0
        } else {
            return Err(e);
        },
    };
    let ansi = from_ansi_bytes(&[code]);
    Ok(ansi.into())
}
pub fn ascb(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let bytes = to_ansi_bytes(&str);
    let code = bytes.get(0).unwrap_or(&0);
    Ok(Object::Num(*code as f64))
}

pub fn isunicode(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let is_unicode = contains_unicode_char(&str);
    Ok(Object::Bool(is_unicode))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum StrconvConst {
    SC_LOWERCASE = 0x100,
    SC_UPPERCASE = 0x200,
    SC_HIRAGANA = 0x100000,
    SC_KATAKANA = 0x200000,
    SC_HALFWIDTH = 0x400000,
    SC_FULLWIDTH = 0x800000,
}

pub fn strconv(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let base = args.get_as_string(0, None)?;
    let opt = args.get_as_int(1, None::<u32>)?;

    let mut strconv = StrConv::new(&base);
    let conv = strconv.convert(opt);
    Ok(conv.into())
}
struct StrConvType {
    case: StrConvCase,
    kana: StrConvKana,
    width: StrConvWidth,
}
enum StrConvCase {Upper,Lower,None}
enum StrConvKana {Hiragana,Katakana,None}
enum StrConvWidth {Full,Half,None}
impl From<u32> for StrConvType {
    fn from(n: u32) -> Self {
        let case = match (
            n & StrconvConst::SC_LOWERCASE as u32 > 0,
            n & StrconvConst::SC_UPPERCASE as u32 > 0
        ) {
            (true, false) => StrConvCase::Lower,
            (false, true) => StrConvCase::Upper,
            _ => StrConvCase::None,
        };
        let kana = match (
            n & StrconvConst::SC_HIRAGANA as u32 > 0,
            n & StrconvConst::SC_KATAKANA as u32 > 0
        ) {
            (true, false) => StrConvKana::Hiragana,
            (false, true) => StrConvKana::Katakana,
            _ => StrConvKana::None,
        };
        let width = match (
            n & StrconvConst::SC_HALFWIDTH as u32 > 0,
            n & StrconvConst::SC_FULLWIDTH as u32 > 0
        ) {
            (true, false) => StrConvWidth::Half,
            (false, true) => StrConvWidth::Full,
            _ => StrConvWidth::None,
        };
        Self { case, kana, width }
    }
}
struct StrConv {
    ucsstr: UCSStr<u32>,
}
impl StrConv {
    fn new(base: &str) -> Self {
        let ucsstr = UCSStr::from_str(base);
        Self { ucsstr }
    }
    fn convert(&mut self, opt: u32) -> String {
        let ctype = StrConvType::from(opt);
        // かな変換後は幅変換する場合がある
        self.kana(&ctype.kana)
        .width(&ctype.width);
        // 大小変換後は幅変換する場合がある
        self.case(&ctype.case)
        .width(&ctype.width);
        // 幅変換後はかな・大小変換する場合がある
        self.width(&ctype.width)
            .kana(&ctype.kana)
            .case(&ctype.case);

        self.ucsstr.to_string()
    }
    fn kana(&mut self, kana: &StrConvKana) -> &mut Self {
        match kana {
            StrConvKana::Hiragana => {self.ucsstr.hiragana();},
            StrConvKana::Katakana => {self.ucsstr.katakana();},
            StrConvKana::None => {},
        };
        self
    }
    fn case(&mut self, case: &StrConvCase) -> &mut Self {
        match case {
            StrConvCase::Upper => {self.ucsstr.upper_case();},
            StrConvCase::Lower => {self.ucsstr.lower_case();},
            StrConvCase::None => {},
        };
        self
    }
    fn width(&mut self, width: &StrConvWidth) -> &mut Self {
        match width {
            StrConvWidth::Full => {self.ucsstr.wide(ConvertTarget::ALL);},
            StrConvWidth::Half => {self.ucsstr.narrow(ConvertTarget::ALL);},
            StrConvWidth::None => {},
        };
        self
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum FormatConst {
    FMT_DEFAULT = 0,
    FMT_ZERO = 1,
    FMT_RIGHT = 2,
    FMT_ZEROR = 3,
}
impl From<f64> for FormatConst {
    fn from(n: f64) -> Self {
        FromPrimitive::from_f64(n).unwrap_or(Self::FMT_DEFAULT)
    }
}

pub fn format(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let val = args.get_as_num_or_string(0)?;
    let len = args.get_as_int(1, None::<usize>)?;
    let digit = args.get_as_int(2, Some(0_i32))?;
    let fill = args.get_as_const(3, Some(FormatConst::FMT_DEFAULT))?;

    let fixed = match val {
        TwoTypeArg::T(ref s) => {
            let cnt = s.chars().count();
            if cnt >= len {
                s.to_string()
            } else {
                let t = (len / cnt) + 1;
                let new = s.repeat(t);
                new.to_char_vec()[0..len].into_iter().collect()
            }
        },
        TwoTypeArg::U(n) => {
            let s = match digit {
                1.. => format!("{:.1$}", n, digit as usize),
                -1 => format!("{:X}", n as i64),
                -2 => format!("{:x}", n as i64),
                -3 => format!("{:b}", n as i64),
                _ => {n.to_string()}
            };
            match fill {
                FormatConst::FMT_DEFAULT => format!("{:>1$}", s, len),
                FormatConst::FMT_ZERO => format!("{:0>1$}", s, len),
                FormatConst::FMT_RIGHT => format!("{:<1$}", s, len),
                FormatConst::FMT_ZEROR => format!("{:0<1$}", s, len),
            }
        },
    };
    Ok(fixed.into())
}

#[cfg(test)]
mod tests {
    use crate::evaluator::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn new_evaluator(input: Option<&str>) -> Evaluator {
        let mut e = Evaluator::new(Environment::new(vec![]));
        if let Some(input) = input {
            let program = Parser::new(Lexer::new(input)).parse();
            if let Err(err) = e.eval(program) {
                panic!("\nError:\n{:#?}\ninput:\n{}\n", err, input);
            }
        }
        e
    }

    fn builtin_test(e: &mut Evaluator, input: &str, expected: EvalResult<Option<Object>>) {
        let program = Parser::new(Lexer::new(input)).parse();
        let result = e.eval(program);
        match expected {
            Ok(expected_obj) => match result {
                Ok(result_obj) => if result_obj.is_some() && expected_obj.is_some() {
                    let left = result_obj.unwrap();
                    let right = expected_obj.unwrap();
                    if ! left.is_equal(&right) {
                        panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", left, right, input);
                    }
                } else if result_obj.is_some() || expected_obj.is_some() {
                    // どちらかがNone
                    panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", result_obj, expected_obj, input);
                },
                Err(e) => panic!("this test should be ok: {}\n error: {}\n", input, e),
            },
            Err(expected_err) => match result {
                Ok(_) => panic!("this test should occure error:\n{}", input),
                Err(result_err) => if result_err != expected_err {
                    panic!("\nresult: {:?}\nexpected: {:?}\n\ninput: {}\n", result_err, expected_err, input);
                },
            },
        }
    }

    #[test]
    fn test_copy() {
        let script = r#"
        文字列 = "あいうえおかきくけこ"
        "#;
        let mut e = new_evaluator(Some(script));
        let test_cases = [
            (r#"copy(文字列, 6)"#, Ok(Some("かきくけこ".into()))),
            (r#"copy(文字列, 3, 4)"#, Ok(Some("うえおか".into()))),
            (r#"copy(文字列, 11)"#, Ok(Some("".into()))),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

    #[test]
    fn test_pos() {
        let script = r#"
        moji1 = "あいabうえおかきくうえABけこ"
        moji2 = "あいあいあ"
        "#;
        let mut e = new_evaluator(Some(script));
        let test_cases = [
            (r#"pos("うえ", moji1)"#, Ok(Some(5.into()))),
            (r#"pos("うえ", moji1, 0)"#, Ok(Some(5.into()))),
            (r#"pos("うえ", moji1, 1)"#, Ok(Some(5.into()))),
            (r#"pos("うえ", moji1, 2)"#, Ok(Some(11.into()))),
            (r#"pos("うえ", moji1, 3)"#, Ok(Some(0.into()))),
            (r#"pos("うえ", moji1, -1)"#, Ok(Some(11.into()))),
            (r#"pos("うえ", moji1, -2)"#, Ok(Some(5.into()))),
            (r#"pos("うえ", moji1, -3)"#, Ok(Some(0.into()))),
            (r#"pos("ab", moji1, 2)"#, Ok(Some(13.into()))),
            (r#"pos("ab", moji1, -1)"#, Ok(Some(13.into()))),
            (r#"pos("いぬ", "🐕いぬ")"#, Ok(Some(2.into()))),
            (r#"pos("あいあ", moji2, 1)"#, Ok(Some(1.into()))),
            (r#"pos("あいあ", moji2, 2)"#, Ok(Some(3.into()))),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

    #[test]
    fn test_betweenstr() {
        let script = r#"
        moji1 = "あfromいtoうfromえtoお"
        moji2 = "あfromいfromうtoえfromおtoかtoき"
        moji3 = "ababaあいうccc"
        moji4 = "aabaab"
        moji5 = "abあfabいfab"
        "#;
        let test_cases = [
            (r#"betweenstr(moji1)"#, Ok(Some("あfromいtoうfromえtoお".into()))),
            (r#"betweenstr(moji1,,)"#, Ok(Some("あfromいtoうfromえtoお".into()))),
            (r#"betweenstr(moji1,,,2)"#, Ok(Some("あfromいtoうfromえtoお".into()))),
            (r#"betweenstr(moji1,,,-1)"#, Ok(Some("あfromいtoうfromえtoお".into()))),
            (r#"betweenstr(moji1, "from")"#, Ok(Some("いtoうfromえtoお".into()))),
            (r#"betweenstr(moji1, "from",,2)"#, Ok(Some("えtoお".into()))),
            (r#"betweenstr(moji1, , "to")"#, Ok(Some("あfromい".into()))),
            (r#"betweenstr(moji1, , "to", 2)"#, Ok(Some("あfromいtoうfromえ".into()))),
            (r#"betweenstr(moji1, , "to", 2, TRUE)"#, Ok(Some("あfromいtoうfromえ".into()))),
            (r#"betweenstr(moji1, , "to", -1)"#, Ok(Some("あfromいtoうfromえ".into()))),
            (r#"betweenstr(moji1, "from", "to")"#, Ok(Some("い".into()))),
            (r#"betweenstr(moji1, "from", "to", 1)"#, Ok(Some("い".into()))),
            (r#"betweenstr(moji1, "from", "to", 2)"#, Ok(Some("え".into()))),
            (r#"betweenstr(moji1, "from", "to", 3)"#, Ok(Some("".into()))),
            (r#"betweenstr(moji1, "from", "foo")"#, Ok(Some("".into()))),
            (r#"betweenstr(moji1, "foo", "to")"#, Ok(Some("".into()))),
            (r#"betweenstr(moji1, "foo", "bar")"#, Ok(Some("".into()))),
            // moji2 = "あfromいfromうtoえfromおtoかtoき"
            (r#"betweenstr(moji2, "from", "to", 1)"#, Ok(Some("いfromう".into()))),
            (r#"betweenstr(moji2, "from", "to", 2, FALSE)"#, Ok(Some("お".into()))),
            (r#"betweenstr(moji2, "from", "to", 2, TRUE)"#, Ok(Some("う".into()))),
            (r#"betweenstr(moji2, "from", "to", -1)"#, Ok(Some("おtoか".into()))),
            (r#"betweenstr(moji2, "from", "to", -2, FALSE)"#, Ok(Some("う".into()))),
            (r#"betweenstr(moji2, "from", "to", -2, TRUE)"#, Ok(Some("お".into()))),
            // moji3 = "ababaあいうccc"
            (r#"betweenstr(moji3, "aba", "ccc", 1, TRUE)"#, Ok(Some("baあいう".into()))),
            (r#"betweenstr(moji3, "aba", "ccc", 2, TRUE)"#, Ok(Some("あいう".into()))),
            // moji4 = "aabaab"
            (r#"betweenstr(moji4, , "b", 2)"#, Ok(Some("aabaa".into()))),
            (r#"betweenstr(moji4, , "b", -1)"#, Ok(Some("aabaa".into()))),
            // moji5 = "abあfabいfab"
            (r#"betweenstr(moji5, "ab", "fab", 1)"#, Ok(Some("あ".into()))),
            (r#"betweenstr(moji5, "ab", "fab", 2)"#, Ok(Some("い".into()))),
        ];
        let mut e = new_evaluator(Some(script));
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

    #[test]
    fn test_chknum() {
        let mut e = new_evaluator(None);
        let test_cases = [
            (r#"chknum(1)"#, Ok(Some(true.into()))),
            (r#"chknum("2")"#, Ok(Some(true.into()))),
            (r#"chknum("３")"#, Ok(Some(false.into()))),
            (r#"chknum(TRUE)"#, Ok(Some(true.into()))),
            (r#"chknum("FALSE")"#, Ok(Some(false.into()))),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

    #[test]
    fn test_val() {
        let mut e = new_evaluator(None);
        let test_cases = [
            (r#"val(1)"#, Ok(Some(1.into()))),
            (r#"val("2")"#, Ok(Some(2.into()))),
            (r#"val("３")"#, Ok(Some((-999999).into()))),
            (r#"val(TRUE)"#, Ok(Some(1.into()))),
            (r#"val(FALSE)"#, Ok(Some(0.into()))),
            (r#"val("ほげ", 0)"#, Ok(Some(0.into()))),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

    #[test]
    fn test_trim() {
        let mut e = new_evaluator(None);
        let test_cases = [
            // スペース
            (r#"trim("  abc   ")"#, Ok(Some("abc".into()))),
            // 改行、タブ
            (r#"trim(" <#CR>  abc <#TAB>  ")"#, Ok(Some("abc".into()))),
            // 制御文字
            ("trim(' \u{001b}abc\u{001b} ')", Ok(Some("abc".into()))),
            // 全角空白
            (r#"trim(" 　abc　 ", TRUE)"#, Ok(Some("abc".into()))),
            (r#"trim(" 　abc　 ", FALSE)"#, Ok(Some("　abc　".into()))),
            // 指定文字
            (r#"trim("edeffededdabcedfffedeeddedf", "edf")"#, Ok(Some("abc".into()))),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

}