
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
    sets.add("parsehtml", 1, parse_html);
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