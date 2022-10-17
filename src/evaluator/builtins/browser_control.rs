
use crate::{
    evaluator::{
        builtins::*,
        devtools_protocol::{Browser, DevtoolsProtocolError},
    }
};

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("browsercontrol", 4, browser_control);
    sets
}

const DEFAULT_PORT: u16 = 9222;

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum BcEnum {
    BC_CHROME = 1,
    BC_MSEDGE = 2,
    BC_UNKNOWN = -1
}

// browsercontrol(種類, [フィルタ=EMPTY, ポート=9222, ヘッドレス=FALSE])
pub fn browser_control(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let typearg = args.get_as_int::<i32>(0, None)?;
    let filter = args.get_as_string_or_empty(1)?;
    let port = args.get_as_int::<u16>(2, Some(DEFAULT_PORT))?;
    let headless = args.get_as_bool(3, Some(false))?;
    let browser = match FromPrimitive::from_i32(typearg).unwrap_or(BcEnum::BC_UNKNOWN) {
        BcEnum::BC_CHROME => Browser::new_chrome(port, filter, headless)?,
        BcEnum::BC_MSEDGE => Browser::new_msedge(port, filter, headless)?,
        BcEnum::BC_UNKNOWN => return Err(builtin_func_error(
            UErrorMessage::InvalidArgument((port as f64).into()),
        ))
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Browser(browser)))
}

impl From<DevtoolsProtocolError> for BuiltinFuncError {
    fn from(e: DevtoolsProtocolError) -> Self {
        Self::UError(e.into())
    }
}