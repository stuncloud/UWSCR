
use crate::{
    evaluator::{
        Evaluator,
        builtins::*,
        object::{
            browser::{BrowserBuilder, BrowserType},
            WebRequest,
            HtmlNode,
        },
    },
};

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

use std::sync::{Arc, Mutex};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("browsercontrol", 2, browser_control);
    sets.add("browserbuilder", 1, browser_builder);
    sets.add("remoteobjecttype", 1, remote_object_type);
    sets.add("webrequest", 1, webrequest);
    sets.add("webrequestbuilder", 0, webrequest_builder);
    sets.add("brgetdata", 5, browser_getdata);
    sets.add("brsetdata", 5, browser_setdata);
    sets.add("brgetsrc", 5, browser_getsource);
    sets.add("brlink", 4, browser_link);
    sets
}

const DEFAULT_PORT: u16 = 9222;

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum BcEnum {
    BC_CHROME  = 1,
    BC_MSEDGE  = 2,
    BC_VIVALDI = 11,
}

pub fn browser_control(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = args.get_as_int(0, None)?;
    let Some(browser_type) = FromPrimitive::from_i32(t) else {
        return Err(builtin_func_error(UErrorMessage::InvalidBrowserType(t)));
    };
    let r#type = match browser_type {
        BcEnum::BC_CHROME => BrowserType::Chrome,
        BcEnum::BC_MSEDGE => BrowserType::MSEdge,
        BcEnum::BC_VIVALDI => BrowserType::Vivaldi,
    };
    let port = args.get_as_int(1, Some(DEFAULT_PORT))?;
    let mut builder = BrowserBuilder::new(r#type, DEFAULT_PORT);
    builder.port(port);
    let browser = builder.start()?;
    Ok(Object::Browser(browser))
}
pub fn browser_builder(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = args.get_as_int(0, None)?;
    let Some(browser_type) = FromPrimitive::from_i32(t) else {
        return Err(builtin_func_error(UErrorMessage::InvalidBrowserType(t)));
    };
    let r#type = match browser_type {
        BcEnum::BC_CHROME => BrowserType::Chrome,
        BcEnum::BC_MSEDGE => BrowserType::MSEdge,
        BcEnum::BC_VIVALDI => BrowserType::Vivaldi,
    };
    let builder = BrowserBuilder::new(r#type, DEFAULT_PORT);
    Ok(Object::BrowserBuilder(Arc::new(Mutex::new(builder))))
}

pub fn remote_object_type(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let remote = args.get_as_remoteobject(0)?;
    #[cfg(debug_assertions)]
    println!("\u{001b}[90m{:?}\u{001b}[0m", remote);
    let t = remote.get_type();
    Ok(t.into())
}

pub fn webrequest(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let uri = args.get_as_string(0, None)?;
    let req = WebRequest::new();
    let res = req.get(&uri)?;
    Ok(Object::WebResponse(res))
}

pub fn webrequest_builder(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let req = WebRequest::new();
    Ok(Object::WebRequest(Arc::new(Mutex::new(req))))
}

pub fn parse_html(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let html = args.get_as_string(0, None)?;
    let node = HtmlNode::new(&html);
    Ok(Object::HtmlNode(node))
}

pub fn browser_getdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let name = args.get_as_string(1, None)?;
    let obj = if let Some((left, right)) = name.split_once('=') {
        if left.to_ascii_lowercase() == "tag" {
            // タグ指定
            if right.to_ascii_lowercase() == "table" {
                // テーブル
                let nth = args.get_as_nth(2)? as usize;
                let row = args.get_as_nth(3)? as usize;
                let col = args.get_as_nth(4)? as usize;
                tab.get_data_by_table_point(nth, row, col)?
            } else {
                // テーブル以外のタグ
                match args.get_as_num_or_string(2).unwrap_or(TwoTypeArg::U(1_usize)) {
                    TwoTypeArg::T(prop) => {
                        // プロパティ指定
                        if let Some((prop_name, prop_value)) = prop.split_once('=') {
                            let nth = args.get_as_nth(3)?;
                            tab.get_data_by_tagname_and_property(right.into(), prop_name, prop_value, nth as usize)?
                        } else {
                            // プロパティ指定がない
                            Object::Empty
                        }
                    },
                    TwoTypeArg::U(nth) => {
                        // 順番指定
                        tab.get_data_by_tagname(right.into(), nth)?
                    },
                }
            }
        } else {
            // タグ指定じゃない場合はnameとみなす
            let value = args.get_as_string_or_empty(2)?;
            let nth = args.get_as_nth(3)?;
            tab.get_data_by_name_value(name, value, nth as usize)?
        }
    } else {
        // name-value指定
        let value = args.get_as_string_or_empty(2)?;
        let nth = args.get_as_nth(3)?;
        tab.get_data_by_name_value(name, value, nth as usize)?
    };
    Ok(obj)
}

pub fn browser_setdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let obj = match args.get_as_string_or_bool(1, None)? {
        TwoTypeArg::T(new_value) => {
            // value書き換え
            let name = args.get_as_string(2, None)?;
            let value = args.get_as_string_or_empty(3)?;
            let nth = args.get_as_nth(4)? as usize;
            let obj = tab.set_data_by_name_value(new_value, name, value, nth)?;
            obj
        },
        TwoTypeArg::U(b) => if b {
            // クリック
            let name = args.get_as_string(2, None)?;
            if let Some((left, tag)) = name.split_once('=') {
                if left.to_ascii_lowercase() == "tag" {
                    if tag.to_ascii_lowercase() == "img" {
                        // imgタグ
                        let src = args.get_as_string_or_empty(3)?;
                        let nth = args.get_as_nth(4)? as usize;
                        let obj = tab.click_img(src, nth)?;
                        obj
                    } else {
                        // img以外
                        match args.get_as_num_or_string(3).unwrap_or(TwoTypeArg::U(1_usize)) {
                            TwoTypeArg::T(prop) => {
                                if let Some((prop_name, prop_value)) = prop.split_once('=') {
                                    let nth = args.get_as_nth(4)? as usize;
                                    let obj = tab.click_by_tag_and_property(tag.into(), prop_name, prop_value, nth)?;
                                    obj
                                } else {
                                    Object::Bool(false)
                                }
                            },
                            TwoTypeArg::U(nth) => {
                                let obj = tab.click_by_nth_tag(tag.into(), nth)?;
                                obj
                            },
                        }
                    }
                } else {
                    // タグ指定じゃないのでname-valueとしてクリック
                    let value = args.get_as_string_or_empty(3)?;
                    let nth = args.get_as_nth(4)? as usize;
                    let obj = tab.click_by_name_value(name, value, nth)?;
                    obj
                }
            } else {
                // name-valueでクリック
                let value = args.get_as_string_or_empty(3)?;
                let nth = args.get_as_nth(4)? as usize;
                let obj = tab.click_by_name_value(name, value, nth)?;
                obj
            }
        } else {
            Object::Bool(false)
        },
    };
    Ok(obj)
}

pub fn browser_getsource(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let tag = args.get_as_string(1, None)?;
    let nth = args.get_as_nth(2)? as usize;
    let obj = tab.get_source(tag, nth)?;
    Ok(obj)
}

pub fn browser_link(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let text = args.get_as_string(1, None)?;
    let nth = args.get_as_nth(2)? as usize;
    let exact_match = args.get_as_bool(3, Some(false))?;
    let obj = tab.click_link(text, nth, exact_match)?;
    Ok(obj)
}