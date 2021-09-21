use crate::evaluator::com_object::ComArg;
use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::UError;
use crate::settings::usettings_singleton;
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
                IDispatch, SAFEARRAY,
                GetActiveObject,
            }
        }
    },
    to_wide_string,
};

use std::{ptr};
use libc::c_void;
// use std::sync::{Arc, Mutex};
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
// use num_traits::FromPrimitive;
use windows::Interface;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("createoleobj", 1, createoleobj);
    sets.add("getactiveoleobj", 1, getactiveoleobj);
    sets.add("vartype", 2, vartype);
    sets.add("safearray", 4, safearray);
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

fn ignore_ie(prog_id: &str) -> Result<(), UError> {
    if prog_id.to_ascii_lowercase().contains("internetexplorer.application") {
        let singleton = usettings_singleton(None);
        let usettings = singleton.0.lock().unwrap();
        if ! usettings.options.allow_ie_object {
            return Err(UError::new(
                "CreateOleObj Error",
                "Internet Explorer is not supported",
                None
            ));
        }
    }
    Ok(())
}

fn createoleobj(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prog_id = get_string_argument_value(&args, 0, None)?;
    // ignore IE
    ignore_ie(&prog_id)?;
    let idispatch = create_instance(&prog_id)?;
    Ok(Object::ComObject(idispatch))
}

fn create_instance(prog_id: &str) -> Result<IDispatch, windows::Error> {
    let mut wide = to_wide_string(prog_id);
    let obj: IDispatch = unsafe {
        let rclsid = CLSIDFromProgID(PWSTR(wide.as_mut_ptr()))?;
        CoCreateInstance(&rclsid, None, CLSCTX_ALL)?
    };
    Ok(obj)
}

fn getactiveoleobj(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prog_id = get_string_argument_value(&args, 0, None)?;
    // ignore IE
    ignore_ie(&prog_id)?;
    let disp = get_active_object(&prog_id)?;
    Ok(Object::ComObject(disp))
}

fn get_active_object(prog_id: &str) -> Result<IDispatch, windows::Error> {
    let mut wide = to_wide_string(prog_id);
    let obj = unsafe {
        let rclsid = CLSIDFromProgID(PWSTR(wide.as_mut_ptr()))?;
        println!("[debug] wide: {:?}", &wide);
        println!("[debug] rclsid: {:?}", &rclsid);

        let pvreserved = ptr::null_mut() as *mut c_void;
        let mut ppunk = None;
        GetActiveObject(&rclsid, pvreserved, &mut ppunk)?;
        println!("[debug] ppunk: {:?}", &ppunk);
        match ppunk {
            Some(u) => u.cast::<IDispatch>()?,
            None => return Err(windows::Error::new(
                windows::HRESULT(0)
                , "Unknown error on GetActiveObject"
            ))
        }
    };
    Ok(obj)
}

fn vartype(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let vt = get_non_float_argument_value::<i32>(&args, 1, Some(-1))?;
    let o = get_any_argument_value(&args, 0, None)?;
    if vt < 0 {
        let n = match o {
            Object::Variant(ref v) => v.vt() as f64,
            _ => VarType::VAR_UWSCR as u32 as f64
        };
        Ok(Object::Num(n))
    } else {
        let _is_array = (vt as u16 | VarType::VAR_ARRAY as u16) > 0;
        // VARIANT型への変換 VAR_UWSCRの場合は通常のObjectに戻す
        if vt == VarType::VAR_UWSCR as i32 {
            match o {
                Object::Variant(v) => Ok(Object::from_variant(v)?),
                o => Ok(o)
            }
        } else {
            let variant = match o {
                Object::Variant(ref v) => v.change_type(vt as u16)?,
                o => {
                    let ca = ComArg::from_object(o)?;
                    let v = ca.to_variant();
                    v.change_type(vt as u16)?
                }
            };
            Ok(Object::Variant(variant))
        }
    }
}

fn safearray(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let lbound = match get_num_or_array_argument_value(&args, 0, Some(Object::Num(0.0)))? {
        Object::Num(n) => n as i32,
        Object::Array(arr) => {
            let mut sa = SAFEARRAY::new(0, (arr.len() - 1) as i32);
            let mut i = 0;
            for obj in arr {
                let com_arg = ComArg::from_object(obj)?;
                let mut variant = com_arg.to_variant();
                sa.set(i, &mut variant)?;
                i += 1;
            }
            return Ok(Object::SafeArray(sa))
        },
        _ => 0,
    };
    let ubound = get_non_float_argument_value::<i32>(&args, 1, Some(-1))?;
    let min = i32::min_value();
    let lbound2 = get_non_float_argument_value::<i32>(&args, 2, Some(min))?;
    let mut ubound2 = get_non_float_argument_value::<i32>(&args, 3, Some(min))?;

    let safe_array = if lbound2 > min {
        // 二次元
        if ubound2 == min {
            ubound2 = lbound2 - 1;
        }
        SAFEARRAY::new2(lbound, ubound, lbound2, ubound2)
    } else {
        // 一次元
        SAFEARRAY::new(lbound, ubound)
    };
    Ok(Object::SafeArray(safe_array))
}