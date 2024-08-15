use crate::object::*;
use crate::builtins::*;
use crate::Evaluator;
use crate::error::UErrorMessage::BuiltinArgCastError;
use util::winapi::{
    get_ansi_length, from_ansi_bytes, to_ansi_bytes, contains_unicode_char,
    to_wide_string,
};

use regex::Regex;
use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use serde_json;
use kanaria::{
    string::UCSStr,
    utils::{ConvertTarget, CharExtend}
};
use windows::{
    core::PCSTR,
    Win32::Globalization::{
        CP_ACP, WC_COMPOSITECHECK, MB_PRECOMPOSED,
        WideCharToMultiByte, MultiByteToWideChar,
    },
};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("copy", copy, get_desc!(copy));
    sets.add("length", length, get_desc!(length));
    sets.add("lengthb", lengthb, get_desc!(lengthb));
    sets.add("lengthu", lengthu, get_desc!(lengthu));
    sets.add("lengths", lengths, get_desc!(lengths));
    sets.add("lengthw", lengthw, get_desc!(lengthw));
    sets.add("as_string", as_string, get_desc!(as_string));
    sets.add("newre", newre, get_desc!(newre));
    sets.add("regex", regex, get_desc!(regex));
    sets.add("testre", testre, get_desc!(testre));
    sets.add("match", regexmatch, get_desc!(regexmatch));
    sets.add("replace", replace, get_desc!(replace));
    sets.add("chgmoj", replace, get_desc!(replace));
    sets.add("tojson", tojson, get_desc!(tojson));
    sets.add("fromjson", fromjson, get_desc!(fromjson));
    sets.add("copy", copy, get_desc!(copy));
    sets.add("pos", pos, get_desc!(pos));
    sets.add("betweenstr", betweenstr, get_desc!(betweenstr));
    sets.add("chknum", chknum, get_desc!(chknum));
    sets.add("val", val, get_desc!(val));
    sets.add("trim", trim, get_desc!(trim));
    sets.add("chr", chr, get_desc!(chr));
    sets.add("asc", asc, get_desc!(asc));
    sets.add("chrb", chrb, get_desc!(chrb));
    sets.add("ascb", ascb, get_desc!(ascb));
    sets.add("isunicode", isunicode, get_desc!(isunicode));
    sets.add("strconv", strconv, get_desc!(strconv));
    sets.add("format", format, get_desc!(format));
    sets.add("token", token, get_desc!(token));
    sets.add("encode", encode, get_desc!(encode));
    sets.add("decode", decode, get_desc!(decode));
    sets
}

#[builtin_func_desc(
    desc="文字列等の長さを得る",
    rtype={desc="長さ",types="数値"}
    args=[
        {n="値",t="文字列/配列/連想配列/構造体/RemoteObject/バイト配列",d="長さを得たい値"},
    ],
)]
pub fn length(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => s.chars().count(),
        Object::Num(n) => n.to_string().len(),
        Object::Array(v) => v.len(),
        Object::Bool(b) => b.to_string().len(),
        Object::HashTbl(h) => h.lock().unwrap().len(),
        Object::StructDef(sdef) => sdef.size,
        Object::UStruct(ust) => ust.size(),
        Object::Empty => 0,
        Object::Null => 1,
        Object::ByteArray(ref arr) => arr.len(),
        Object::RemoteObject(ref remote) => {
            let len = remote.length()?;
            return Ok(Object::Num(len));
        },
        Object::UObject(u) => u.get_size()?,
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o)))
    };
    Ok(Object::Num(len as f64))
}

#[builtin_func_desc(
    desc="文字列のANSIバイト数を得る",
    rtype={desc="ANSIバイト数",types="数値"}
    args=[
        {n="文字列",t="文字列",d="長さを得たい文字列"},
    ],
)]
pub fn lengthb(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => get_ansi_length(&s),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o)))
    };
    Ok(Object::Num(len as f64))
}

#[builtin_func_desc(
    desc="文字列のUTF8バイト数を得る",
    rtype={desc="UTF8バイト数",types="数値"}
    args=[
        {n="文字列",t="文字列",d="長さを得たい文字列"},
    ],
)]
pub fn lengthu(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let len = match args.get_as_object(0, None)? {
        Object::String(s) => s.as_bytes().len(),
        Object::Num(n) => n.to_string().len(),
        Object::Bool(b) => b.to_string().len(),
        Object::Empty => 0,
        Object::Null => 1,
        o => return Err(builtin_func_error(UErrorMessage::InvalidArgument(o)))
    };
    Ok(Object::Num(len as f64))
}

#[builtin_func_desc(
    desc="サロゲートペアを二文字分としてカウントする",
    rtype={desc="文字数",types="数値"}
    args=[
        {n="文字列",t="文字列",d="長さを得たい文字列"},
    ],
)]
pub fn lengths(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let length = str.chars()
            .map(|char| char.len_utf16())
            .reduce(|a,b| a+b)
            .unwrap_or_default();
    Ok(Object::Num(length as f64))
}

#[builtin_func_desc(
    desc="NULL終端Unicode文字列としての長さを得る",
    rtype={desc="配列長",types="数値"}
    args=[
        {n="文字列",t="文字列",d="長さを得たい文字列"},
    ],
)]
pub fn lengthw(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let len = to_wide_string(&str).len();
    Ok(len.into())
}

#[builtin_func_desc(
    desc="値を文字列にする",
    rtype={desc="文字列",types="文字列"}
    args=[
        {n="値",t="値",d="文字列にしたい値"},
    ],
)]
pub fn as_string(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::String(format!("{}", args.get_as_object(0, None)?)))
}

// 正規表現

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum RegexEnum {
    #[strum[props(desc="正規表現のテストを行う")]]
    REGEX_TEST  = 0, // default
    #[strum[props(desc="正規表現にマッチする文字列を得る")]]
    REGEX_MATCH = 1,
}

#[builtin_func_desc(
    desc="正規表現オブジェクトを返す",
    rtype={desc="正規表現オブジェクト",types="正規表現オブジェクト"}
    args=[
        {n="正規表現",t="文字列",d="正規表現パターンを示す文字列"},
        {o,n="大小文字",t="真偽値",d="TRUEなら大文字小文字を区別する"},
        {o,n="複数行",t="真偽値",d="TRUEなら複数行が対象となり `^` が行頭 `$` が行末にマッチ"},
        {o,n="改行",t="真偽値",d="TRUEなら `.` が改行(`\\n`)にマッチするようになる"},
    ],
)]
pub fn newre(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

fn test_regex(target: String, pattern: String) -> BuiltinFuncResult {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            Ok(Object::Bool(re.is_match(target.as_str())))
        },
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern)))
    }
}

fn match_regex(target: String, pattern: String) -> BuiltinFuncResult {
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
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern)))
    }
}

fn replace_regex(target: String, pattern: String, replace_to: String) -> BuiltinFuncResult {
    match Regex::new(pattern.as_str()) {
        Ok(re) => {
            Ok(Object::String(re.replace_all(target.as_str(), replace_to.as_str()).to_string()))
        },
        Err(_) => Err(builtin_func_error(UErrorMessage::InvalidRegexPattern(pattern)))
    }
}

#[builtin_func_desc(
    desc="正規表現をテストする",
    rtype={desc="正規表現がマッチすればTRUE",types="真偽値"}
    args=[
        {n="元文字",t="文字列",d="対象文字列"},
        {n="正規表現",t="文字列または正規表現",d="正規表現文字列またはオブジェクト"},
    ],
)]
pub fn testre(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    test_regex(target, pattern)
}

#[builtin_func_desc(
    desc="正規表現による文字列操作",
    rtype={desc="操作方法による",types="文字列/配列/真偽値"}
    args=[
        {n="元文字",t="文字列",d="対象文字列"},
        {n="正規表現",t="文字列または正規表現",d="正規表現パターンを示す文字列またはオブジェクト"},
        {o,n="操作方法",t="定数または文字列",d=r#"以下のいずれかを指定
        - REGEX_TEST: 元文字に正規表現がマッチするかを真偽値で返す
        - REGEX_MATCH: 正規表現にマッチした文字列の配列を返す
        - 文字列: 正規表現にマッチした文字列を置換する文字列"#},
    ],
)]
pub fn regex(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    match args.get_as_object(2, Some(Object::Empty))? {
        Object::Num(n) => {
            match FromPrimitive::from_f64(n).unwrap_or(RegexEnum::REGEX_TEST) {
                RegexEnum::REGEX_MATCH => match_regex(target, pattern),
                _ => test_regex(target, pattern)
            }
        },
        Object::String(s) |
        Object::RegEx(s) => replace_regex(target, pattern, s.clone()),
        Object::Empty => test_regex(target, pattern),
        o => Err(builtin_func_error(UErrorMessage::InvalidArgument(o)))
    }
}

#[builtin_func_desc(
    desc="正規表現にマッチした文字列を列挙",
    rtype={desc="マッチ文字の配列",types="配列"}
    args=[
        {n="元文字列",t="文字列",d="対象文字列"},
        {n="正規表現",t="文字列または正規表現",d="正規表現文字列またはオブジェクト"},
    ],
)]
pub fn regexmatch(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let pattern = args.get_as_string(1, None)?;
    match_regex(target, pattern)
}

#[builtin_func_desc(
    desc="文字列を置換する",
    rtype={desc="置換された文字列",types="文字列"}
    args=[
        {n="元文字",t="文字列",d="対象文字列"},
        {n="置換対象",t="文字列または正規表現",d="元文字内の置換される文字"},
        {n="置換後",t="文字列",d="置換対象文字列を置き換える文字列"},
        {o,n="正規表現モード",t="真偽値",d="TRUEなら置換対象を正規表現として扱う"},
    ],
)]
pub fn replace(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = args.get_as_string(0, None)?;
    let (pattern, is_regex) = match args.get_as_object(1, None)? {
        Object::String(s) => (s.clone(), args.get_as_bool(3, Some(false))?),
        Object::RegEx(re) => (re.clone(), true),
        o => (o.to_string(), false)
    };
    let replace_to = args.get_as_string(2, None)?;

    if is_regex {
        replace_regex(target, pattern, replace_to)
    } else {
        let lower = target.to_ascii_lowercase();
        let pat = pattern.to_ascii_lowercase();
        let len = pat.len();
        let mut out = target;

        for (pos, _) in lower.rmatch_indices(&pat) {
            out.replace_range(pos..pos+len, &replace_to);
        }
        Ok(Object::String(out))
    }
}

#[builtin_func_desc(
    desc="UObjectをjson文字列にする",
    rtype={desc="json文字列",types="文字列"}
    args=[
        {n="UObject",t="UObject",d="jsonに変換するUObject"},
        {o,n="整形",t="真偽値",d="TRUEならjsonに改行等を含めて見やすくする"},
    ],
)]
pub fn tojson(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prettify = args.get_as_bool(1, Some(false))?;
    let to_string = if prettify {serde_json::to_string_pretty} else {serde_json::to_string};
    let uo = args.get_as_uobject(0)?;
    let value = uo.value();
    to_string(&value).map_or_else(
        |e| Err(builtin_func_error(UErrorMessage::Any(e.to_string()))),
        |s| Ok(Object::String(s))
    )
}

#[builtin_func_desc(
    desc="json文字列をUObjectにする",
    rtype={desc="成功時UObject、失敗時はEMPTY",types="UObject"}
    args=[
        {n="json文字列",t="文字列",d="UObjectに変換するjson文字列"},
    ],
)]
pub fn fromjson(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let json = args.get_as_string(0, None)?;
    serde_json::from_str::<serde_json::Value>(json.as_str()).map_or_else(
        |_| Ok(Object::Empty),
        |v| Ok(Object::UObject(UObject::new(v)))
    )
}

#[builtin_func_desc(
    desc="文字列をコピー",
    rtype={desc="コピーした文字列",types="文字列"}
    args=[
        {n="元文字",t="文字列",d="コピー元"},
        {n="開始",t="数値",d="コピー開始位置"},
        {o,n="長さ",t="数値",d="開始位置からコピーする文字数"},
    ],
)]
pub fn copy(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

/// target文字列からpattern文字列がヒットした位置をVecで返す
fn find_all(target: &str, pattern: &str) -> Vec<usize> {
    let target_lower = target.to_ascii_lowercase();
    let t_bytes = target_lower.as_bytes();
    let t_len = t_bytes.len();
    let pattern_lower = pattern.to_ascii_lowercase();
    let p_bytes = pattern_lower.as_bytes();
    let Some(p_first) = p_bytes.first() else {
        return Vec::new();
    };
    let p_len = p_bytes.len();
    t_bytes.iter().enumerate()
        .filter_map(|(i, b)| {
            if b == p_first {
                if (t_len - i) >= p_len {
                    let found = &t_bytes[i..(i+p_len)];
                    (found == p_bytes).then_some(i)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}
fn find_pos(target: &str, pattern: &str, nth: i32) -> Option<usize> {
    let mut found = find_all(&target, &pattern);
    let index = if nth > 0 {
        nth - 1
    } else if nth < 0 {
        found.reverse();
        nth.abs() - 1
    } else {
        nth
    } as usize;
    found.get(index).copied()
}
fn find_nth_pos(target: &str, pattern: &str, nth: i32) -> usize {
    match find_pos(target, pattern, nth) {
        Some(pos) => {
            let next = next_pos(target, pos);
            let mut tmp = target.to_string();
            tmp.truncate(next);
            tmp.chars().count()
        },
        None => 0,
    }
}
fn next_pos(str: &str, mut p: usize) -> usize {
    loop {
        p += 1;
        if str.is_char_boundary(p) {
            break p;
        }
    }
}

#[builtin_func_desc(
    desc="文字の位置を返す",
    rtype={desc="見つかった位置、なければ0",types="数値"}
    args=[
        {n="探す文字",t="文字列",d="位置を得たい文字列"},
        {n="元文字",t="文字列",d="検索対象"},
        {o,n="n番目",t="数値",d="探す文字が複数ある場合その順番"},
    ],
)]
pub fn pos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let substr = args.get_as_string(0, None)?;
    let str = args.get_as_string(1, None)?;
    let nth = args.get_as_int(2, Some(1_i32))?;

    let pos = find_nth_pos(&str, &substr, nth);
    Ok(pos.into())
}

fn drain_to_nth_pattern(target: &str, pattern: &str, nth: i32) -> Option<String> {
    let pos = find_pos(target, pattern, nth)? + pattern.len();
    let found = target[pos..].to_string();
    Some(found)
}
fn truncate_to_nth_pattern(target: &str, pattern: &str, nth: i32) -> Option<String> {
    let pos = find_pos(target, pattern, nth)?;
    let found = target[..pos].to_string();
    Some(found)
}
fn find_all_between(target: &str, from: &str, to: &str, flag: bool) -> Vec<String> {
    let from_pos = find_all(&target, &from);
    let to_pos = find_all(&target, &to);
    let from_len = from.len();
    let to_len = to.len();

    let pairs: Vec<(usize, usize)> = if flag {
        from_pos.into_iter()
            .filter_map(|_f| {
                let t = *to_pos.iter().find(|n| **n > _f)?;
                let f = _f + from_len;
                Some((f, t))
            })
            .collect()
    } else {
        let mut tail_of_pair = 0;
        from_pos.into_iter()
            .filter_map(|_f| {
                // from発見位置が前ペア移行なら続行
                (_f >= tail_of_pair).then_some(())?;

                let t = *to_pos.iter().find(|n| **n > _f)?;
                tail_of_pair = t + to_len;
                let f = _f + from_len;
                Some((f, t))
            })
            .collect()
    };
    let words = pairs.into_iter()
        .map(|(f, t)| target[f..t].to_string())
        .collect();
    words
}
fn find_all_between_backward(target: &str, from: &str, to: &str, flag: bool) -> Vec<String> {

    let mut from_pos = find_all(&target, &from);
    from_pos.reverse();
    let mut to_pos = find_all(&target, &to);
    to_pos.reverse();
    let from_len = from.len();

    let pairs: Vec<(usize, usize)> = if flag {
        to_pos.into_iter()
            .filter_map(|_t| {
                let f = *from_pos.iter().find(|n| **n < _t)? + from_len;
                Some((f, _t))
            })
            .collect()
    } else {
        let mut head_of_pair = target.len();
        to_pos.into_iter()
            .filter_map(|_t| {
                // to位置が前のペアより前なら続行
                (head_of_pair > _t).then_some(())?;

                let f = *from_pos.iter().find(|n| **n < _t)?;
                head_of_pair = f;
                Some((f + from_len, _t))
            })
            .collect()
    };
    let words = pairs.into_iter()
        .map(|(f, t)| target[f..t].to_string())
        .collect();
    words
}

fn find_nth_between(target: &str, from: &str, to: &str, nth: i32, flag: bool) -> Option<String> {
    if nth < 0 {
        // 逆順
        let found = find_all_between_backward(target, from, to, flag);
        let index = nth.abs() as usize - 1;
        found.get(index).map(|s| s.to_string())
    } else {
        let found = find_all_between(target, from, to, flag);
        let index = nth as usize - 1;
        found.get(index).map(|s| s.to_string())
    }
}

#[builtin_func_desc(
    desc="指定した文字の間の文字列を得る",
    rtype={desc="該当する文字列",types="文字列または配列"}
    args=[
        {n="元文字",t="文字列",d="コピー元文字列"},
        {o,n="前文字",t="文字列",d="得たい文字の前に来る文字、省略時は先頭から"},
        {o,n="後文字",t="文字列",d="得たい文字の後に来る文字、省略時は末尾まで"},
        {o,n="n番目",t="数値",d="該当する文字が複数ある場合その順番、マイナスなら後ろから、前後文字指定時に0で該当文字列すべてを配列で返す"},
        {o,n="数え方",t="真偽値",d="FALSEなら前後文字をペアとしてその順番、TRUEならn番目の前(後)文字の対の間を得る"},
    ],
)]
pub fn betweenstr(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let from = args.get_as_string_or_empty(1)?
        .filter(|s| s.len() > 0);
    let to = args.get_as_string_or_empty(2)?
        .filter(|s| s.len() > 0);
    let nth = args.get_as_int(3, Some(1_i32))?;
    let flag = args.get_as_bool(4, Some(false))?;

    let between = match (from, to) {
        (None, None) => Some(str),
        (None, Some(to)) => truncate_to_nth_pattern(&str, &to, nth),
        (Some(from), None) => drain_to_nth_pattern(&str, &from, nth),
        (Some(from), Some(to)) => {
            if nth == 0 {
                let all = find_all_between(&str, &from, &to, flag)
                    .into_iter()
                    .map(|s| s.into())
                    .collect();
                return Ok(Object::Array(all));
            } else {
                find_nth_between(&str, &from, &to, nth, flag)
            }
        },
    };
    Ok(between.map(|s|Object::String(s)).unwrap_or_default())
}

#[builtin_func_desc(
    desc="数値変換可能かどうかを調べる",
    rtype={desc="TRUEなら数値変換可能",types="真偽値"}
    args=[
        {n="値",t="値",d="対象の値"},
    ],
)]
pub fn chknum(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let result = args.get_as_int(0, None::<f64>).is_ok();
    Ok(Object::Bool(result))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum ErrConst {
    #[strum[props(desc="一部関数におけるエラー値")]]
    ERR_VALUE = -999999,
}

#[builtin_func_desc(
    desc="文字列を数値に変換",
    rtype={desc="変換された数値、失敗時はエラー値",types="数値"}
    args=[
        {n="数値文字列",t="文字列",d="数値に変換する文字列"},
        {o,n="エラー値",t="数値",d="数値変換できない場合に返す値、デフォルトはERR_VALUE"},
    ],
)]
pub fn val(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let result = args.get_as_num(0, None::<f64>);
    let err = args.get_as_num(1, Some(ErrConst::ERR_VALUE as i32 as f64))?;
    let val = result.unwrap_or(err);
    Ok(val.into())
}

#[builtin_func_desc(
    desc="文字列の両端からホワイトスペースや制御文字、または指定した文字を除去する",
    rtype={desc="トリムされた文字列",types="文字列"}
    sets=[
        "ホワイトスペース除去",
        [
            {n="元文字列",t="文字列",d="両端のホワイトスペース及び制御文字を除去したい文字列"},
            {n="全角空白",t="真偽値",d="TRUEなら全角スペースも除去する"},
        ],
        "指定文字除去",
        [
            {n="元文字列",t="文字列",d="トリムされる文字列"},
            {n="除去文字",t="文字列",d="指定した文字が連続する限りそれを除去する、文字列の場合各文字のいずれかが連続すれば除去される"},
        ],
    ],
)]
pub fn trim(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

#[builtin_func_desc(
    desc="Unicodeコードポイントから文字を得る",
    rtype={desc="該当する文字、なければ空文字",types="文字列"}
    args=[
        {n="コードポイント",t="数値",d="Unicodeコードポイント"},
    ],
)]
pub fn chr(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let code = args.get_as_int(0, None::<u32>)?;
    let char = match char::from_u32(code) {
        Some(c) => c.to_string(),
        None => String::new(),
    };
    Ok(char.into())
}
#[builtin_func_desc(
    desc="文字からUnicodeコードポイントを得る",
    rtype={desc="該当するUnicodeコードポイント、なければ0",types="数値"}
    args=[
        {n="文字",t="文字列",d="コードポイントを得たい文字、文字列の場合先頭の文字のみ"},
    ],
)]
pub fn asc(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let code = match str.chars().next() {
        Some(first) => {
            first as u32
        },
        None => 0,
    };
    Ok(Object::Num(code as f64))
}

#[builtin_func_desc(
    desc="バイトコードからASCII文字を得る",
    rtype={desc="該当するASCII文字、なければ空文字",types="文字列"}
    args=[
        {n="バイトコード",t="数値",d="0-255で指定"},
    ],
)]
pub fn chrb(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let code = match args.get_as_int(0, None::<u8>) {
        Ok(n) => n,
        Err(e) => match e.message() {
            // キャスト失敗の場合は0を返す
            BuiltinArgCastError(_, _) => 0,
            _ => return Err(e)
        }
    };
    let ansi = from_ansi_bytes(&[code]);
    Ok(ansi.into())
}
#[builtin_func_desc(
    desc="ASCII文字からバイトコードを得る",
    rtype={desc="該当するASCIIバイトコード、なければ0",types="数値"}
    args=[
        {n="文字",t="文字列",d="バイトコードを得たいASCII文字、文字列の場合先頭の文字のみ"},
    ],
)]
pub fn ascb(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let bytes = to_ansi_bytes(&str);
    let code = bytes.get(0).unwrap_or(&0);
    Ok(Object::Num(*code as f64))
}

#[builtin_func_desc(
    desc="文字列にUnicode専用(ANSIにない)文字が含まれるかどうかを調べる",
    rtype={desc="専用文字が含まれていればTRUE",types="真偽値"}
    args=[
        {n="元文字列",t="文字列",d="対象文字列"},
    ],
)]
pub fn isunicode(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let is_unicode = contains_unicode_char(&str);
    Ok(Object::Bool(is_unicode))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum StrconvConst {
    #[strum[props(desc="小文字に変換")]]
    SC_LOWERCASE = 0x100,
    #[strum[props(desc="大文字に変換")]]
    SC_UPPERCASE = 0x200,
    #[strum[props(desc="ひらがなに変換")]]
    SC_HIRAGANA = 0x100000,
    #[strum[props(desc="カタカナに変換")]]
    SC_KATAKANA = 0x200000,
    #[strum[props(desc="半角に変換")]]
    SC_HALFWIDTH = 0x400000,
    #[strum[props(desc="全角に変換")]]
    SC_FULLWIDTH = 0x800000,
}

#[builtin_func_desc(
    desc="文字列を変換する",
    rtype={desc="変換された文字列",types="文字列"}
    args=[
        {n="元文字列",t="文字列",d="対象文字列"},
        {n="変換方法",t="定数",d=r#"以下から指定、全角・半角変換は他の変換とOR連結可
- SC_LOWERCASE: 小文字に変換
- SC_UPPERCASE: 大文字に変換
- SC_HIRAGANA: ひらがなに変換
- SC_KATAKANA: カタカナに変換
- SC_HALFWIDTH: 半角文字に変換
- SC_FULLWIDTH: 全角文字に変換"#},
    ],
)]
pub fn strconv(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum FormatConst {
    #[strum[props(desc="半角スペースで左埋め")]]
    FMT_DEFAULT = 0,
    #[strum[props(desc="0で左埋め")]]
    FMT_ZERO = 1,
    #[strum[props(desc="半角スペースで右埋め")]]
    FMT_RIGHT = 2,
    #[strum[props(desc="0で右埋め")]]
    FMT_ZEROR = 3,
}
impl Default for FormatConst {
    fn default() -> Self {
        Self::FMT_DEFAULT
    }
}

#[builtin_func_desc(
    desc="文字列フォーマット",
    rtype={desc="フォーマットされた文字列",types="文字列"}
    sets=[
        "数値フォーマット",
        [
            {n="数値",t="数値",d="フォーマットする数値"},
            {n="幅",t="数値",d="フォーマット後の文字列幅"},
            {o,n="桁数",t="数値",d=r#"小数点以下の桁数、または変換方法を指定
- 1以上: 小数点以下の桁数を指定値で丸める
- 0: 変換しない
- -1: 16進数に変換 (大文字)
- -2: 16進数に変換 (小文字)
- -3: 2進数に変換"#},
            {o,n="埋め方法",t="定数",d=r#"幅が数値桁数を越えた場合に余白を埋める方法を以下のいずれかで指定
- FMT_DEFAULT: 半角スペースで左埋め (デフォルト)
- FMT_ZERO: 0で左埋め
- FMT_RIGHT: 半角スペースで右埋め
- FMT_ZEROR: 0で右埋め"#},
        ],
        "文字列フォーマット",
        [
            {n="元文字列",t="文字列",d="フォーマットする文字列"},
            {n="幅",t="数値",d="フォーマット後の文字列幅、元文字列幅を超える場合は指定幅まで元文字列を繰り返す"},
        ],
        "日時フォーマット",
        [
            {n="秒数",t="数値",d="2020/01/01からの秒数"},
            {n="日時フォーマット文字列",t="文字列",d="指示子についての詳細 [リンク](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)"},
            {n="ミリ秒",t="真偽値",d="TRUEの場合秒数をミリ秒として扱う"},
        ],
    ],
)]
pub fn format(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let val = args.get_as_f64_or_string(0)?;
    let fmt = args.get_as_f64_or_string(1)?;

    let fixed = match val {
        TwoTypeArg::T(s) => {
            let len = match fmt {
                TwoTypeArg::T(s) => {
                    match s.parse() {
                        Ok(u) => u,
                        Err(_) => Err(builtin_func_error(UErrorMessage::NotANumber(s.into())))?,
                    }
                },
                TwoTypeArg::U(num) => if num < 0.0 {0} else {num as usize},
            };
            let cnt = s.chars().count();
            if cnt == 0 || len == 0 || cnt >= len {
                s.to_string()
            } else {
                let t = (len / cnt) + 1;
                let new = s.repeat(t);
                new.to_char_vec()[0..len].into_iter().collect()
            }
        },
        TwoTypeArg::U(n) => {
            match fmt {
                TwoTypeArg::T(fmt) => {
                    let milli = args.get_as_bool(2, Some(false))?;
                    let secs = n as i64;
                    let s = system_controls::gettime::format(&fmt, secs, milli)
                        .map_err(|e| builtin_func_error(UErrorMessage::FormatTimeError(e.to_string())))?;
                    s.into()
                },
                TwoTypeArg::U(num) => {
                    let digit = args.get_as_int(2, Some(0_i32))?;
                    let fill = args.get_as_const(3, false)?.unwrap_or_default();
                    let len = if num < 0.0 {0} else {num as usize};
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
            }
        },
    };

    // let len = args.get_as_int(1, None::<i32>)?;
    // let len = if len < 0 {0_usize} else {len as usize};

    // let fixed = match val {
    //     TwoTypeArg::T(ref s) => {
    //         let cnt = s.chars().count();
    //         if cnt == 0 || len == 0 || cnt >= len {
    //             s.to_string()
    //         } else {
    //             let t = (len / cnt) + 1;
    //             let new = s.repeat(t);
    //             new.to_char_vec()[0..len].into_iter().collect()
    //         }
    //     },
    //     TwoTypeArg::U(n) => {
    //         let s = match digit {
    //             1.. => format!("{:.1$}", n, digit as usize),
    //             -1 => format!("{:X}", n as i64),
    //             -2 => format!("{:x}", n as i64),
    //             -3 => format!("{:b}", n as i64),
    //             _ => {n.to_string()}
    //         };
    //         match fill {
    //             FormatConst::FMT_DEFAULT => format!("{:>1$}", s, len),
    //             FormatConst::FMT_ZERO => format!("{:0>1$}", s, len),
    //             FormatConst::FMT_RIGHT => format!("{:<1$}", s, len),
    //             FormatConst::FMT_ZEROR => format!("{:0<1$}", s, len),
    //         }
    //     },
    // };
    Ok(fixed.into())
}

#[builtin_func_desc(
    desc="区切り文字から前の文字列を切り出す",
    rtype={desc="切り出した文字列",types="文字列"}
    args=[
        {n="区切り文字",t="文字列",d="区切り文字、文字列の場合文字毎に区切り扱い"},
        {n="var 元文字",t="文字列",d="切り出される文字列、切り出し後は残った文字が返る"},
        {o,n="区切り方法",t="真偽値",d="TRUEなら連続した区切り文字を一つとして扱う、FALSEならそれぞれで区切る"},
        {o,n="ダブルクォート",t="真偽値",d="TRUEならダブルクォートで括られた箇所にある区切り文字を無視する"},
    ],
)]
pub fn token(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let delimiter = args.get_as_string(0, None)?;
    let base = args.get_as_string(1, None)?;
    let expression = args.get_expr(1);
    let delimiter_flg = args.get_as_bool(2, Some(false))?;
    let dblquote_flg = args.get_as_bool(3, Some(false))?;

    let delimiter_chars = delimiter.chars().collect::<Vec<_>>();
    let delimiter = delimiter_chars.as_slice();

    if base.contains(delimiter) {
        if dblquote_flg {
            let mut is_in_dbl_quote = false;
            let mut pos = 0_usize;
            let mut deli_pos = None::<usize>;
            let mut rem_pos = 0_usize;
            let mut base_chars = base.chars();
            loop {
                match base_chars.next() {
                    Some(char) => {
                        if char == '"' {
                            is_in_dbl_quote = !is_in_dbl_quote;
                        }
                        if ! is_in_dbl_quote {
                            if deli_pos.is_some() {
                                if ! delimiter_chars.contains(&char) {
                                    rem_pos = pos;
                                    break;
                                }
                            } else {
                                if delimiter_chars.contains(&char) {
                                    deli_pos = Some(pos);
                                    if ! delimiter_flg {
                                        rem_pos = pos + 1;
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    None => break,
                }
                pos +=  1;
            }
            let sfrt = if let Some(p) = deli_pos {
                let chars = base.chars().collect::<Vec<_>>();
                let token = chars[..p].iter().collect::<String>();
                let remained = chars[rem_pos..].iter().collect();
                evaluator.update_tokened_variable(expression, remained)?;
                token.into()
            } else {
                evaluator.update_tokened_variable(expression, "".into())?;
                base.into()
            };
            Ok(sfrt)


        } else {
            let (token, mut remained) = base.split_once(delimiter)
                .map(|(t, r)| (t.to_string(), r.to_string()))
                .unwrap_or_default();
            if delimiter_flg {
                loop {
                    if let Some((t,r)) = remained.split_once(delimiter) {
                        if t.is_empty() {
                            remained = r.to_string();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            evaluator.update_tokened_variable(expression, remained)?;
            Ok(token.into())
        }
    } else {
        evaluator.update_tokened_variable(expression, "".into())?;
        Ok(base.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum CodeConst {
    CODE_ANSI = 1,
    #[strum[props(desc="URLエンコード")]]
    CODE_URL = 2,
    CODE_UTF8 = 3,
    #[strum[props(desc="実体参照変換")]]
    CODE_HTML = 4,
    #[strum[props(desc="ANSIバイト配列")]]
    CODE_BYTEARRAY = 5,
    #[strum[props(desc="Unicodeバイト配列")]]
    CODE_BYTEARRAYW = 6,
    #[strum[props(desc="UTF8バイト配列")]]
    CODE_BYTEARRAYU = 7,
}

impl From<f64> for CodeConst {
    fn from(n: f64) -> Self {
        FromPrimitive::from_f64(n).unwrap_or(Self::CODE_UTF8)
    }
}

struct ByteArray {

}

impl ByteArray {
    fn as_byte(str: &str) -> Vec<u8> {
        str.to_string().into_bytes()
    }
    fn from_byte(byte: &[u8]) -> String {
        String::from_utf8_lossy(byte).into_owned()
    }
    fn as_ansi(str: &str) -> Vec<u8> {
        unsafe {
            let wide = str.encode_utf16().collect::<Vec<_>>();
            let len = WideCharToMultiByte(
                CP_ACP,
                WC_COMPOSITECHECK,
                &wide,
                None,
                PCSTR::null(),
                None,
            );
            if len > 0 {
                let mut result = vec![0; len as usize];
                WideCharToMultiByte(
                    CP_ACP,
                    WC_COMPOSITECHECK,
                    &wide,
                    Some(&mut result),
                    PCSTR::null(),
                    None,
                );
                result
            } else {
                vec![]
            }
        }
    }
    fn from_ansi(byte: &[u8]) -> String {
        unsafe {
            let len = MultiByteToWideChar(
                CP_ACP,
                MB_PRECOMPOSED,
                byte,
                None
            );
            if len > 0 {
                let mut wide = vec![0; len as usize];
                MultiByteToWideChar(
                    CP_ACP,
                    MB_PRECOMPOSED,
                    byte,
                    Some(&mut wide)
                );
                String::from_utf16_lossy(&wide)
            } else {
                String::new()
            }
        }
    }
    fn as_wide(str: &str) -> Vec<u8> {
        let bytes = str.encode_utf16()
            .map(|n| n.to_le_bytes())
            .flatten()
            .collect();
        bytes
    }
    fn from_wide(byte: &[u8]) -> String {
        let (_, wide, _) = unsafe {
            byte.align_to::<u16>()
        };
        String::from_utf16_lossy(wide)
    }
}

#[builtin_func_desc(
    desc="文字列をエンコードする",
    rtype={desc="変換方式による",types=""}
    args=[
        {n="元文字列",t="文字列",d="エンコードする文字列"},
        {n="変換方式",t="定数",d=r#"以下のいずれかを指定
- CODE_URL: URLエンコード
- CODE_HTML: 一部の記号等を文字実態参照にする (`<` → `&lt;`)
- CODE_BYTEARRAY: バイト配列(ANSI)にする
- CODE_BYTEARRAYW: バイト配列(Unicode)にする
- CODE_BYTEARRAYU: バイト配列(UTF8)にする"#},
    ],
)]
pub fn encode(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let Some(code) = args.get_as_const::<CodeConst>(1, true)? else {
        return Ok(Object::String(str));
    };

    let result = match code {
        CodeConst::CODE_ANSI => str.into(),
        CodeConst::CODE_URL => {
            let encoded = urlencoding::encode(&str);
            encoded.into_owned().into()
        },
        CodeConst::CODE_UTF8 => str.into(),
        CodeConst::CODE_HTML => {
            let content = str.as_bytes();
            let encoded = htmlentity::entity::encode(
                content,
                &htmlentity::entity::EncodeType::Named,
                &htmlentity::entity::CharacterSet::SpecialChars,
            );
            let string_result: htmlentity::types::StringResult = encoded.into();
            match string_result {
                Ok(enc) => enc.into(),
                Err(_) => str.into(),
            }
        },
        CodeConst::CODE_BYTEARRAY => {
            let bytes = ByteArray::as_ansi(&str);
            bytes.into()
        },
        CodeConst::CODE_BYTEARRAYW => {
            let bytes = ByteArray::as_wide(&str);
            bytes.into()
        },
        CodeConst::CODE_BYTEARRAYU => {
            let bytes = ByteArray::as_byte(&str);
            bytes.into()
        },
    };
    Ok(result)
}

#[builtin_func_desc(
    desc="文字列にデコードする",
    rtype={desc="デコードされた文字列",types="文字列"}
    args=[
        {n="デコード対象",t="文字列またはバイト配列",d="デコードする文字列またはバイト配列"},
        {n="変換方式",t="定数",d=r#"以下のいずれかを指定
- CODE_URL: URLエンコードされた文字列を戻す
- CODE_HTML: 文字実態参照を記号にする (`&lt;` → `<`)
- CODE_BYTEARRAY: バイト配列(ANSI)を文字列にする
- CODE_BYTEARRAYW: バイト配列(Unicode)を文字列にする
- CODE_BYTEARRAYU: バイト配列(UTF8)を文字列にする"#},
    ],
)]
pub fn decode(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let value = args.get_as_string_or_bytearray(0)?;
    let Some(code) = args.get_as_const::<CodeConst>(1, true)? else {
        return Ok(Object::Empty);
    };

    let result = match value {
        TwoTypeArg::T(s) => match code {
            CodeConst::CODE_URL => match urlencoding::decode(&s) {
                Ok(cow) => cow.into_owned().into(),
                Err(_) => Object::Empty,
            },
            CodeConst::CODE_HTML => {
                let content = s.as_bytes();
                let decoded = htmlentity::entity::decode(content);
                let string_result: htmlentity::types::StringResult = decoded.into();
                match string_result {
                    Ok(dec) => dec.into(),
                    Err(_) => s.into(),
                }
            },
            _ => s.into()
        },
        TwoTypeArg::U(byte) => match code {
            CodeConst::CODE_BYTEARRAY => ByteArray::from_ansi(&byte).into(),
            CodeConst::CODE_BYTEARRAYW => ByteArray::from_wide(&byte).into(),
            CodeConst::CODE_BYTEARRAYU => ByteArray::from_byte(&byte).into(),
            _ => Object::Empty
        },
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::*;
    use parser::lexer::Lexer;
    use parser::Parser;

    fn new_evaluator(input: Option<&str>) -> Evaluator {
        let mut e = Evaluator::new(Environment::new(vec![]));
        if let Some(input) = input {
            match Parser::new(Lexer::new(input), None, None).parse() {
                Ok(program) => {
                    if let Err(err) = e.eval(program, false) {
                        panic!("\nError:\n{:#?}\ninput:\n{}\n", err, input);
                    }
                },
                Err(err) => panic!("{err:#?}"),
            }
        }
        e
    }

    fn builtin_test(e: &mut Evaluator, input: &str, expected: EvalResult<Option<Object>>) {
        match Parser::new(Lexer::new(input), None, None).parse() {
            Ok(program) => {
                let result = e.eval(program, false);
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
            },
            Err(err) => panic!("{err:#?}"),
        };
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
            // gh-109
            (r#"pos("あ", "あああ123", 1)"#, Ok(Some(1.into()))),
            (r#"pos("あ", "あああ123", 2)"#, Ok(Some(2.into()))),
            (r#"pos("あ", "あああ123", 3)"#, Ok(Some(3.into()))),
            (r#"pos("あ", "あああ123", -1)"#, Ok(Some(3.into()))),
            (r#"pos("あ", "あああ123", -2)"#, Ok(Some(2.into()))),
            (r#"pos("あ", "あああ123", -3)"#, Ok(Some(1.into()))),
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
        moji6 = "あいう123あいうあいう456あいうあいう789あいう"
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
            (r#"betweenstr(moji1, "from", "to", 3)"#, Ok(Some(Object::Empty))),
            (r#"betweenstr(moji1, "from", "foo")"#, Ok(Some(Object::Empty))),
            (r#"betweenstr(moji1, "foo", "to")"#, Ok(Some(Object::Empty))),
            (r#"betweenstr(moji1, "foo", "bar")"#, Ok(Some(Object::Empty))),
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
            (r#"betweenstr(moji5, "ab", "fab", 1, FALSE)"#, Ok(Some("あ".into()))),
            (r#"betweenstr(moji5, "ab", "fab", 2, FALSE)"#, Ok(Some(Object::Empty))),
            (r#"betweenstr(moji5, "ab", "fab", 1, TRUE)"#, Ok(Some("あ".into()))),
            (r#"betweenstr(moji5, "ab", "fab", 2, TRUE)"#, Ok(Some("い".into()))),
            // gh-109
            (r#"betweenstr("あ123", "あ")"#, Ok(Some("123".into()))),
            (r#"betweenstr("あ123", "あ",,-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("あいう123", "あいう")"#, Ok(Some("123".into()))),
            (r#"betweenstr("あいう123", "あいう",,-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("abc123", "abc")"#, Ok(Some("123".into()))),
            (r#"betweenstr("abc123", "abc",,-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("あああ123", "あ")"#, Ok(Some("ああ123".into()))),
            (r#"betweenstr("あああ123", "あ",,2)"#, Ok(Some("あ123".into()))),
            (r#"betweenstr("あああ123", "あ",,3)"#, Ok(Some("123".into()))),
            (r#"betweenstr("あああ123", "あ",,-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("あああ123", "あ",,-2)"#, Ok(Some("あ123".into()))),
            (r#"betweenstr("あああ123", "あ",,-3)"#, Ok(Some("ああ123".into()))),
            (r#"betweenstr("ああああ123", "ああ")"#, Ok(Some("ああ123".into()))),
            (r#"betweenstr("ああああ123", "ああ",,2)"#, Ok(Some("あ123".into()))),
            (r#"betweenstr("ああああ123", "ああ",,3)"#, Ok(Some("123".into()))),
            (r#"betweenstr("ああああ123", "ああ",,-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("ああああ123", "ああ",,-2)"#, Ok(Some("あ123".into()))),
            (r#"betweenstr("ああああ123", "ああ",,-3)"#, Ok(Some("ああ123".into()))),
            (r#"betweenstr("123あ",, "あ")"#, Ok(Some("123".into()))),
            (r#"betweenstr("123あ",, "あ",-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("123あいう",, "あいう")"#, Ok(Some("123".into()))),
            (r#"betweenstr("123あいう",, "あいう",-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("123abc",, "abc")"#, Ok(Some("123".into()))),
            (r#"betweenstr("123abc",, "abc",-1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("あいう123",,"123")"#, Ok(Some("あいう".into()))),
            (r#"betweenstr("あいう123",,"123",-1)"#, Ok(Some("あいう".into()))),
            (r#"betweenstr("123ああ",, "あ",1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("123ああ",, "あ",2)"#, Ok(Some("123あ".into()))),
            (r#"betweenstr("123ああ",, "あ",-1)"#, Ok(Some("123あ".into()))),
            (r#"betweenstr("123ああ",, "あ",-2)"#, Ok(Some("123".into()))),
            (r#"betweenstr("123あああ",, "ああ",1)"#, Ok(Some("123".into()))),
            (r#"betweenstr("123あああ",, "ああ",2)"#, Ok(Some("123あ".into()))),
            (r#"betweenstr("123あああ",, "ああ",-1)"#, Ok(Some("123あ".into()))),
            (r#"betweenstr("123あああ",, "ああ",-2)"#, Ok(Some("123".into()))),
            // gh-118
            (r#"betweenstr(moji6, "あいう", "あいう", 1)"#, Ok(Some("123".into()))),
            (r#"betweenstr(moji6, "あいう", "あいう", 2)"#, Ok(Some("456".into()))),
            (r#"betweenstr(moji6, "あいう", "あいう", 3)"#, Ok(Some("789".into()))),
            // index 0
            (
                r#"betweenstr(moji6, "あいう", "あいう", 0, FALSE)"#,
                Ok(Some(Object::Array(vec![
                    "123".into(),
                    "456".into(),
                    "789".into(),
                ])))
            ),
            (
                r#"betweenstr(moji6, "あいう", "あいう", 0, TRUE)"#,
                Ok(Some(Object::Array(vec![
                    "123".into(),
                    "".into(),
                    "456".into(),
                    "".into(),
                    "789".into(),
                ])))
            ),
            (
                r#"betweenstr("あいう123かきくあいう456かきくあいう789かきく", "あいう", "かきく", 0)"#,
                Ok(Some(Object::Array(vec![
                    "123".into(),
                    "456".into(),
                    "789".into(),
                ])))
            ),
            // gh-180
            (r#"betweenstr('abcabc', "a", "bc", 1, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", 2, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", -1, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", -2, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", 1, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", 2, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", -1, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('abcabc', "a", "bc", -2, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", 1, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", 2, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", -1, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", -2, FALSE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", 1, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", 2, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", -1, TRUE)"#, Ok(Some("".into()))),
            (r#"betweenstr('aaaa', "a", "a", -2, TRUE)"#, Ok(Some("".into()))),

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

    impl From<Vec<&str>> for Object {
        fn from(vec: Vec<&str>) -> Self {
        let arr = vec.into_iter()
            .map(|s| s.into())
            .collect();
        Object::Array(arr)
    }
    }

    #[test]
    fn test_token() {
        let script = r#"
        moji1 = "あ-い-う-え-お"
        moji2 = "あいうabcえお"
        moji3 = "あいうabcえお"
        moji4 = "あいうえお"
        "#;
        let mut e = new_evaluator(Some(script));
        let test_cases = [
            (
                r#"[token("-", moji1), moji1]"#,
                Ok(Some(vec!["あ", "い-う-え-お"].into()))
            ),
            (
                r#"[token("-", moji1), moji1]"#,
                Ok(Some(vec!["い", "う-え-お"].into()))
            ),
            (
                r#"[token("abc", moji2), moji2]"#,
                Ok(Some(vec!["あいう", "bcえお"].into()))
            ),
            (
                r#"[token("abc", moji2, FALSE), moji2]"#,
                Ok(Some(vec!["", "cえお"].into()))
            ),
            (
                r#"[token("abc", moji3, TRUE), moji3]"#,
                Ok(Some(vec!["あいう", "えお"].into()))
            ),
            (
                r#"[token("abc", moji4), moji4]"#,
                Ok(Some(vec!["あいうえお", ""].into()))
            ),
            (
                r#"
                moji = "<#DBL>あabcか<#DBL>abcさ"
                [token("abc", moji, FALSE, FALSE), moji]
                "#,
                Ok(Some(vec![r#""あ"#, r#"bcか"abcさ"#].into()))
            ),
            (
                r#"
                moji = "<#DBL>あabcか<#DBL>abcさ"
                [token("abc", moji, FALSE, TRUE), moji]
                "#,
                Ok(Some(vec![r#""あabcか""#, "bcさ"].into()))
            ),
            (
                r#"
                moji = "<#DBL>あabcか<#DBL>abcさ"
                [token("abc", moji, TRUE, FALSE), moji]
                "#,
                Ok(Some(vec![r#""あ"#, r#"か"abcさ"#].into()))
            ),
            (
                r#"
                moji = "<#DBL>あabcか<#DBL>abcさ"
                [token("abc", moji, TRUE, TRUE), moji]
                "#,
                Ok(Some(vec![r#""あabcか""#, "さ"].into()))
            ),
        ];
        for (input, expected) in test_cases {
            builtin_test(&mut e, input, expected);
        }
    }

}