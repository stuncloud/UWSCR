use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::Evaluator;
use crate::settings::USETTINGS;

use std::ops::BitOr;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("createoleobj", 1, createoleobj);
    sets.add("getactiveoleobj", 2, getactiveoleobj);
    sets.add("getoleitem", 1, getoleitem);
    // sets.add("oleevent", 4, oleevent);
    sets.add("vartype", 2, vartype);
    sets.add("safearray", 4, safearray);
    sets.add("xlopen", 36, xlopen);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum VarType {
    VAR_EMPTY    = 0,   // VT_EMPTY
    VAR_NULL     = 1,   // VT_NULL
    VAR_SMALLINT = 2,   // VT_I2
    VAR_INTEGER  = 3,   // VT_I4
    VAR_SINGLE   = 4,   // VT_R4
    VAR_DOUBLE   = 5,   // VT_R8
    VAR_CURRENCY = 6,   // VT_CY
    VAR_DATE     = 7,   // VT_DATE
    VAR_BSTR     = 8,   // VT_BSTR
    VAR_DISPATCH = 9,   // VT_DISPATCH
    VAR_ERROR    = 10,  // VT_ERROR
    VAR_BOOLEAN  = 11,  // VT_BOOL
    VAR_VARIANT  = 12,  // VT_VARIANT
    VAR_UNKNOWN  = 13,  // VT_UNKNOWN
    VAR_SBYTE    = 16,  // VT_I1
    VAR_BYTE     = 17,  // VT_UI1
    VAR_WORD     = 18,  // VT_UI2
    VAR_DWORD    = 19,  // VT_UI4
    VAR_INT64    = 20,  // VT_I8
    VAR_ASTR     = 256, // VT_LPSTR
    VAR_USTR     = 258, // VT_LPWSTR
    VAR_UWSCR    = 512, // UWSCRデータ型
    VAR_ARRAY    = 0x2000,
}
impl PartialEq<VarType> for u16 {
    fn eq(&self, other: &VarType) -> bool {
        match ToPrimitive::to_u16(other) {
            Some(n) => *self == n,
            None => false,
        }
    }
}
impl BitOr<VarType> for u16 {
    type Output = u16;

    fn bitor(self, rhs: VarType) -> Self::Output {
        match ToPrimitive::to_u16(&rhs) {
            Some(n) => n | self,
            None => 0,
        }
    }
}

fn ignore_ie(prog_id: &str) -> BuiltInResult<()> {
    if ComObject::is_ie(prog_id)? {
        let usettings = USETTINGS.lock().unwrap();
        if ! usettings.options.allow_ie_object {
            return Err(BuiltinFuncError::new_with_kind(
                UErrorKind::ProgIdError,
                UErrorMessage::InternetExplorerNotAllowed
            ));
        }
    }
    Ok(())
}

fn createoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_string(0, None)?;
    // ignore IE
    ignore_ie(&id)?;
    let obj = ComObject::new(id)?;
    Ok(Object::ComObject(obj))
}

fn getactiveoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_string(0, None)?;
    // ignore IE
    ignore_ie(&id)?;
    match ComObject::get_instance(id)? {
        Some(obj) => Ok(Object::ComObject(obj)),
        None => Err(builtin_func_error(UErrorMessage::FailedToGetObject))
    }
}

pub fn getoleitem(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let vec = com.to_object_vec()?;
    Ok(Object::Array(vec))
}

// pub fn oleevent(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
//     let mut com = args.get_as_comobject(0)?;
//     if args.len() > 1 {
//         // セット
//         let interface = args.get_as_string(1, None)?;
//         let event = args.get_as_string(2, None)?;
//         let func = match args.get_as_function_or_string(3, true)? {
//             Some(two) => match two {
//                 TwoTypeArg::T(name) => {
//                     match evaluator.env.get_function(&name) {
//                         Some(obj) => match obj {
//                             Object::Function(func) => Ok(func),
//                             _ => Err(builtin_func_error(UErrorMessage::IsNotUserFunction(name))),
//                         },
//                         None => Err(builtin_func_error(UErrorMessage::FunctionNotFound(name))),
//                     }
//                 },
//                 TwoTypeArg::U(func) => Ok(func),
//             },
//             None => Err(builtin_func_error(UErrorMessage::FunctionRequired)),
//         }?;
//         com.set_event_handler(&interface, &event, func, evaluator.clone())?;
//     } else {
//         // 解除
//         com.remove_event_handler()?;
//     }
//     Ok(Object::Empty)
// }

fn vartype(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok((-1).into())
}

fn safearray(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::Empty)
}

impl From<ComError> for BuiltinFuncError {
    fn from(e: ComError) -> Self {
        BuiltinFuncError::UError(e.into())
    }
}

// Excel

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum ExcelConst {
    XL_DEFAULT = 0,
    XL_NEW     = 1,
    XL_BOOK    = 2,
    XL_OOOC    = 3,
}

pub fn xlopen(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let file = args.get_as_string_or_empty(0)?;
    let flg = args.get_as_const::<ExcelOpenFlag>(1, false)?.unwrap_or_default();
    let params = args.get_rest_as_string_array(2, 0)?;
    let com = Excel::open(file, flg, params)?;
    Ok(Object::ComObject(com))
}