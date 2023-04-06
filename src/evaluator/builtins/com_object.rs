use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::Evaluator;
use crate::evaluator::com_object::{VARIANTHelper, SAFEARRAYHelper};
use crate::settings::USETTINGS;
use crate::winapi::WString;

use windows::{
    core::{PCWSTR, GUID, ComInterface},
    Win32::{
        System::{
            Com::{
                CLSCTX_ALL,
                SAFEARRAY, IDispatch,
                CLSIDFromProgID, CoCreateInstance,
                VARENUM,
            },
            Ole::{
                GetActiveObject,
            }
        }
    }
};

use std::{ptr};
use std::ops::BitOr;
use libc::c_void;
// use std::sync::{Arc, Mutex};
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    // sets.add("createoleobj", 1, createoleobj);
    // sets.add("getactiveoleobj", 1, getactiveoleobj);
    sets.add("&ダミー&", 1, createoleobj);
    sets.add("%ダミー%", 1, getactiveoleobj);
    sets.add("vartype", 2, vartype);
    sets.add("safearray", 4, safearray);
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
    if prog_id.to_ascii_lowercase().contains("internetexplorer.application") {
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
    let prog_id = args.get_as_string(0, None)?;
    // ignore IE
    ignore_ie(&prog_id)?;
    let idispatch = create_instance(&prog_id)?;
    Ok(Object::ComObject(idispatch))
}

fn get_clsid_from_progid(prog_id: &str) -> BuiltInResult<GUID> {
    unsafe {
        let wide = prog_id.to_wide_null_terminated();
        let rclsid = CLSIDFromProgID(PCWSTR::from_raw(wide.as_ptr()))?;
        Ok(rclsid)
    }
}

fn create_instance(prog_id: &str) -> BuiltInResult<IDispatch> {
    let obj: IDispatch = unsafe {
        let rclsid = get_clsid_from_progid(prog_id)?;
        CoCreateInstance(&rclsid, None, CLSCTX_ALL)?
    };
    Ok(obj)
}

fn getactiveoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let prog_id = args.get_as_string(0, None)?;
    // ignore IE
    ignore_ie(&prog_id)?;
    let disp = match get_active_object(&prog_id)? {
        Some(d) => d,
        None => return Err(builtin_func_error(UErrorMessage::FailedToGetObject))
    };
    Ok(Object::ComObject(disp))
}

fn get_active_object(prog_id: &str) -> BuiltInResult<Option<IDispatch>> {
    let obj = unsafe {
        let rclsid = get_clsid_from_progid(prog_id)?;
        let pvreserved = ptr::null_mut() as *mut c_void;
        let mut ppunk = None;
        GetActiveObject(&rclsid, pvreserved, &mut ppunk)?;
        match ppunk {
            Some(unk) => {
                let disp = unk.cast::<IDispatch>()?;
                Some(disp)
            },
            None => None
        }
    };
    Ok(obj)
}

fn vartype(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let vt = args.get_as_int::<i32>(1, Some(-1))?;
    let o = args.get_as_object(0, None)?;
    if vt < 0 {
        let n = match o {
            Object::Variant(ref v) => v.0.vt().0 as f64,
            _ => VarType::VAR_UWSCR as u32 as f64
        };
        Ok(Object::Num(n))
    } else {
        let vt = vt as u16;
        let _is_array = (vt | VarType::VAR_ARRAY) > 0;
        // VARIANT型への変換 VAR_UWSCRの場合は通常のObjectに戻す
        if vt == VarType::VAR_UWSCR {
            match o {
                Object::Variant(v) => Ok(Object::from_variant(&v.0)?),
                o => Ok(o)
            }
        } else {
            let variant = match o {
                Object::Variant(ref v) => v.0.change_type(VARENUM(vt))?,
                o => {
                    let v = o.to_variant()?;
                    v.change_type(VARENUM(vt))?
                }
            };
            Ok(Object::Variant(Variant(variant)))
        }
    }
}

fn safearray(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let lbound = match args.get_as_int_or_array(0, Some(Object::Num(0.0)))? {
        Object::Num(n) => n as i32,
        Object::Array(arr) => {
            let mut sa = SAFEARRAY::new(0, (arr.len() - 1) as i32);
            let mut i = 0;
            for obj in arr {
                let mut variant = obj.to_variant()?;
                sa.set(i, &mut variant)?;
                i += 1;
            }
            return Ok(Object::SafeArray(sa))
        },
        _ => 0,
    };
    let ubound = args.get_as_int::<i32>(1, Some(-1))?;
    let min = i32::min_value();
    let lbound2 = args.get_as_int::<i32>(2, Some(min))?;
    let mut ubound2 = args.get_as_int::<i32>(3, Some(min))?;

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