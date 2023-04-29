
use crate::{
    evaluator::{
        Evaluator,
        builtins::*,
        // devtools_protocol::{Browser, DevtoolsProtocolError},
        object::Browser
    },
};

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("browsercontrol", 4, browser_control);
    sets.add("ConvertFromRemoteObject", 1, convert_from_remote_object);
    sets
}

const DEFAULT_PORT: u16 = 9222;

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum BcEnum {
    BC_CHROME = 1,
    BC_MSEDGE = 2,
}

/// browsercontrol(種類, [プロファイルフォルダ=EMPTY, ポート=9222, ヘッドレス=FALSE])
pub fn browser_control(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = args.get_as_int(0, None)?;
    let Some(browser_type) = FromPrimitive::from_i32(t) else {
        return Err(builtin_func_error(UErrorMessage::InvalidBrowserType(t)));
    };
    let profile = args.get_as_string_or_empty(1)?;
    let port = args.get_as_int::<u16>(2, Some(DEFAULT_PORT))?;
    let headless = args.get_as_bool(3, Some(false))?;
    let browser = match browser_type {
        BcEnum::BC_CHROME => Browser::new_chrome(port, headless, profile)?,
        BcEnum::BC_MSEDGE => Browser::new_msedge(port, headless, profile)?,
    };
    Ok(Object::Browser(browser))
}

pub fn convert_from_remote_object(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let remote = args.get_as_remoteobject(0)?;
    let obj = if remote.is_object() {
        Object::RemoteObject(remote)
    } else {
        match remote.get_value() {
            Some(value) => value.into(),
            None => Object::Null,
        }
    };
    Ok(obj)
}