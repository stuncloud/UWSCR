use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::winapi::{
    bindings::Windows::Win32::{
        Foundation::{
            PWSTR
        },
        System::{
            Com::{
                CLSCTX_ALL,
                CLSIDFromProgID, CoCreateInstance,
            },
            OleAutomation::{
                IDispatch
            }
        }
    },
    to_wide_string,
};

// use std::sync::{Arc, Mutex};
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("createoleobj", 1, createoleobj);
    sets.add("vartype", 2, vartype);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
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

fn createoleobj(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prog_id = get_string_argument_value(&args, 0, None)?;
    let idispatch = create_instance(&prog_id)?;
    Ok(Object::ComObject(idispatch))
    // Ok(Object::ComObject(Arc::new(Mutex::new(idispatch))))
}

fn create_instance(prog_id: &str) -> Result<IDispatch, windows::Error> {
    let mut wide = to_wide_string(prog_id);
    let obj: IDispatch = unsafe {
        let clsid = CLSIDFromProgID(PWSTR(wide.as_mut_ptr()))?;
        CoCreateInstance(&clsid, None, CLSCTX_ALL)?
    };
    Ok(obj)
}

fn vartype(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = get_non_float_argument_value::<i32>(&args, 1, Some(-1))?;
    let o = get_any_argument_value(&args, 0, None)?;
    if t < 0 {
        let vt = match o {
            Object::Variant(ref v) => v.vt() as f64,
            _ => VarType::VAR_UWSCR as u32 as f64
        };
        Ok(Object::Num(vt))
    } else {
        // VARIANT型への変換 VAR_UWSCRの場合は通常のObjectに戻す
        let _is_array = (t as u16 | VarType::VAR_ARRAY as u16) > 0;
        let vt = FromPrimitive::from_i32(t).unwrap_or(VarType::VAR_UWSCR);
        match vt {
            VarType::VAR_EMPTY => {}
            VarType::VAR_NULL => {}
            VarType::VAR_SMALLINT => {}
            VarType::VAR_INTEGER => {}
            VarType::VAR_SINGLE => {}
            VarType::VAR_DOUBLE => {}
            VarType::VAR_CURRENCY => {}
            VarType::VAR_DATE => {}
            VarType::VAR_BSTR => {}
            VarType::VAR_DISPATCH => {}
            VarType::VAR_ERROR => {}
            VarType::VAR_BOOLEAN => {}
            VarType::VAR_VARIANT => {}
            VarType::VAR_UNKNOWN => {}
            VarType::VAR_SBYTE => {}
            VarType::VAR_BYTE => {}
            VarType::VAR_WORD => {}
            VarType::VAR_DWORD => {}
            VarType::VAR_INT64 => {}
            VarType::VAR_ASTR => {}
            VarType::VAR_USTR => {}
            VarType::VAR_UWSCR => {}
            VarType::VAR_ARRAY => {}
        }
        Ok(o)
    }
}