use crate::winapi::{
    to_wide_string,
};
use windows::Win32::{
    Foundation::{PWSTR, BSTR, DISP_E_MEMBERNOTFOUND},
    System::{
        Com::{
        //     COINIT_APARTMENTTHREADED, CLSCTX_ALL,
        //     CLSIDFromProgID, CoInitializeEx, CoCreateInstance,
            DISPPARAMS, EXCEPINFO,
            IDispatch,
        },
        Ole::{
            DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPATCH_METHOD,
            DISPID_PROPERTYPUT,
            VT_ARRAY,
            // VT_BLOB,
            // VT_BLOB_OBJECT,
            VT_BOOL,
            VT_BSTR,
            // VT_BSTR_BLOB,
            // VT_BYREF,
            // VT_CARRAY,
            // VT_CF,
            // VT_CLSID,
            VT_CY,
            VT_DATE,
            // VT_DECIMAL,
            VT_DISPATCH,
            VT_EMPTY,
            VT_ERROR,
            // VT_FILETIME,
            // VT_HRESULT,
            VT_I1,
            VT_I2,
            VT_I4,
            VT_I8,
            // VT_ILLEGAL,
            // VT_ILLEGALMASKED,
            // VT_INT,
            // VT_INT_PTR,
            VT_LPSTR,
            VT_LPWSTR,
            VT_NULL,
            // VT_PTR,
            VT_R4,
            VT_R8,
            // VT_RECORD,
            // VT_RESERVED,
            // VT_SAFEARRAY,
            // VT_STORAGE,
            // VT_STORED_OBJECT,
            // VT_STREAM,
            // VT_STREAMED_OBJECT,
            VT_TYPEMASK,
            VT_UI1,
            VT_UI2,
            VT_UI4,
            // VT_UI8,
            // VT_UINT,
            // VT_UINT_PTR,
            // VT_UNKNOWN,
            // VT_USERDEFINED,
            VT_VARIANT,
            // VT_VECTOR,
            // VT_VERSIONED_STREAM,
            // VT_VOID,
            VARENUM,
            VariantChangeType, VariantCopy,
            SafeArrayCreate, SafeArrayGetElement, SafeArrayPutElement,
            SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayGetDim,
        },
        Com::{
            VARIANT, VARIANT_0_0,
            SAFEARRAY, SAFEARRAYBOUND,
        }
    },
};
use crate::evaluator::{
    EvalResult,
    object::{Object, Variant},
};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use std::{ffi::c_void, mem::ManuallyDrop};

/* COMエラー */

pub struct ComError {
    pub message: String,
    pub code: i32,
    pub description: Option<String>
}

impl ComError {
    pub fn new(e: &windows::core::Error, description: Option<String>) -> Self {
        Self {
            message: e.message().to_string(),
            code: e.code().0,
            description
        }
    }
}

impl From<windows::core::Error> for ComError {
    fn from(e: windows::core::Error) -> Self {
        Self::new(&e, None)
    }
}

impl From<ComError> for UError {
    fn from(e: ComError) -> Self {
        Self::new_com_error(
            UErrorKind::ComError(e.code),
            UErrorMessage::ComError(e.message, e.description)
        )
    }
}

/* Objectの拡張 */
impl Object {
    pub fn from_variant(variant: &VARIANT) -> EvalResult<Self> {
        // VT_ARRAYの場合
        let is_array = (variant.vt() & VT_ARRAY.as_u16()) > 0;
        if is_array {
            let mut sa = unsafe {
                let p = variant.Anonymous.Anonymous.Anonymous.parray;
                *p
            };
            let lb = sa.rgsabound[0].lLbound;
            let ub = sa.rgsabound[0].lLbound + sa.rgsabound[0].cElements as i32;
            let mut arr = vec![];
            for index in lb..ub {
                let v = sa.get(index)?;
                arr.push(Object::from_variant(&v)?);
            }
            return Ok(Object::Array(arr))
        }

        // let is_ref = (variant.vt() & VT_BYREF.0 as u16) > 0;
        let vt = variant.vt() & VT_TYPEMASK.as_u16();
        let obj = match vt as i32 {
            VT_EMPTY => Object::Empty,
            VT_NULL => Object::Null,
            VT_I2 |
            VT_I4 |
            VT_R4 |
            VT_R8 |
            VT_I1 |
            VT_UI1 |
            VT_UI2 |
            VT_UI4 |
            VT_I8 |
            VT_ERROR |
            VT_CY => {
                let v = variant.change_type(VT_R8)?;
                Object::Num(v.get_double())
            },
            VT_BSTR => {
                let bstr = variant.get_bstr()?;
                Object::String(bstr.to_string())
            },
            VT_DATE |
            VT_LPSTR |
            VT_LPWSTR => {
                let v = variant.change_type(VT_BSTR)?;
                let bstr = v.get_bstr()?;
                Object::String(bstr.to_string())
            },
            VT_DISPATCH => {
                let disp = variant.get_idispatch()?;
                Object::ComObject(disp)
            },
            VT_BOOL => Object::Bool(variant.get_bool()),
            _ => {
                let v = variant.copy()?;
                Object::Variant(Variant(v))
            },
        };
        Ok(obj)
    }

    pub fn to_variant(&self) -> EvalResult<VARIANT> {
        let mut variant = VARIANT::default();
        match self {
            Object::Num(n) => variant.set_double(*n),
            Object::String(s) => {
                let wide = to_wide_string(s);
                let bstr = BSTR::from_wide(&wide);
                variant.set_bstr(bstr);
            },
            Object::Bool(b) => variant.set_bool(b),
            Object::ComObject(d) => variant.set_idispatch(d),
            Object::Empty |
            Object::EmptyParam => {} ,//そのまま
            Object::Null => variant.set_vt(VT_NULL),
            Object::Array(a) => {
                let mut sa = SAFEARRAY::new(0, (a.len() - 1) as i32);
                for i in 0..a.len() {
                    let o = a.get(i).unwrap();
                    let mut v = o.to_variant()?;
                    sa.set(i as i32, &mut v)?;
                }
                variant.set_safearray(&mut sa)
            },
            Object::Variant(v) => variant = v.0.clone(),
            Object::SafeArray(sa) => variant.set_safearray(sa),
            o => return Err(UError::new(
                UErrorKind::ConversionError,
                UErrorMessage::VariantConvertionError(o.clone()),
            )),
        }
        Ok(variant)
    }
}

/* IDispatchの拡張 */

const LOCALE_USER_DEFAULT: u32 = 0x0400;
const LOCALE_SYSTEM_DEFAULT: u32 = 0x0800;
type ComResult<T> = Result<T, ComError>;

// unsafe impl Send for IDispatch {}

pub trait IDispatchHelper {
    fn get(&self, name: &str, keys: Option<Vec<VARIANT>>) -> ComResult<VARIANT>;
    fn set(&self, name: &str, value: VARIANT, keys: Option<Vec<VARIANT>>) -> ComResult<VARIANT>;
    fn run(&self, name: &str, args: &mut Vec<VARIANT>) -> ComResult<VARIANT>;
    fn invoke_wrapper(&self, name: &str, dp: *mut DISPPARAMS, wflags: u16) -> ComResult<VARIANT>;
}

impl IDispatchHelper for IDispatch {
    fn get(&self, name: &str, keys: Option<Vec<VARIANT>>) -> ComResult<VARIANT> {
        let mut dp = DISPPARAMS::default();
        if keys.is_some() {
            let mut args = keys.unwrap();
            args.reverse();
            dp.cArgs = args.len() as u32;
            dp.rgvarg = args.as_mut_ptr();
        }
        self.invoke_wrapper(name, &mut dp, DISPATCH_PROPERTYGET as u16)
    }

    fn set(&self, name: &str, mut value: VARIANT, keys: Option<Vec<VARIANT>>) -> ComResult<VARIANT> {
        let mut dp = DISPPARAMS::default();
        if keys.is_some() {
            let mut args = keys.unwrap();
            args.push(value);
            args.reverse();
            dp.cArgs = args.len() as u32;
            dp.rgvarg = args.as_mut_ptr();
        } else {
            // プロパティにセットする値
            dp.cArgs = 1;
            dp.rgvarg = &mut value;
        }
        dp.cNamedArgs = 1;
        let mut dispid_propertyput = DISPID_PROPERTYPUT;
        dp.rgdispidNamedArgs = &mut dispid_propertyput as *mut i32;
        self.invoke_wrapper(name, &mut dp, DISPATCH_PROPERTYPUT as u16)
    }

    fn run(&self, name: &str, args: &mut Vec<VARIANT>) -> ComResult<VARIANT> {
        let mut dp = DISPPARAMS::default();
        // 引数をセット
        // 引数の入った配列は逆順で渡す
        args.reverse();
        dp.cArgs = args.len() as u32;
        dp.rgvarg = args.as_mut_ptr();
        match self.invoke_wrapper(name, &mut dp, DISPATCH_METHOD as u16) {
            Ok(v) => {
                args.reverse(); // 順序を戻す
                Ok(v)
            },
            Err(e) => if e.code == DISP_E_MEMBERNOTFOUND.0 {
                // メソッドが存在しなかった場合はプロパティである可能性があるのでget()する
                args.reverse();
                let keys = args.iter().map(|v|v.to_owned()).collect();
                self.get(name, Some(keys))
            } else {
                Err(e)
            }
        }
    }

    fn invoke_wrapper(&self, name: &str, dp: *mut DISPPARAMS, wflags: u16) -> ComResult<VARIANT> {
        unsafe {
            let mut member: Vec<u16> = to_wide_string(name);
            let mut dispidmember = 0;
            self.GetIDsOfNames(
                &windows::core::GUID::default(),
                &mut PWSTR(member.as_mut_ptr()),
                1,
                LOCALE_USER_DEFAULT,
                &mut dispidmember
            )?;

            let mut excepinfo = EXCEPINFO::default();
            let mut argerr = 0;
            let mut result = VARIANT::default();

            match self.Invoke(
                dispidmember,
                &windows::core::GUID::default(),
                LOCALE_SYSTEM_DEFAULT,
                wflags,
                dp,
                &mut result,
                &mut excepinfo,
                &mut argerr
            ) {
                Ok(()) => Ok(result),
                Err(e) => {
                    let com_err = ComError::new(&e, Some(excepinfo.bstrDescription.to_string()));
                    Err(com_err)
                },
            }
        }
    }
}

/* VARENUMを拡張*/
pub trait VARENUMHelper {
    fn as_u16(&self) -> u16;
}

impl VARENUMHelper for VARENUM {
    fn as_u16(&self) -> u16 {
        *self as u16
    }
}

// unsafe impl Send for VARIANT {}

pub trait VARIANTHelper {
    fn vt(&self) -> u16;
    fn set_vt(&mut self, vt: VARENUM);
    fn set_double(&mut self, n: f64);
    fn set_bstr(&mut self, bstr: BSTR);
    fn set_bool(&mut self, b: &bool);
    fn set_idispatch(&mut self, idispatch: &IDispatch);
    fn _set_variant(&mut self, variant: &mut VARIANT);
    fn set_safearray(&mut self, sa: &SAFEARRAY);
    fn get_double(&self) -> f64;
    fn get_bstr(&self) -> ComResult<BSTR>;
    fn get_idispatch(&self) -> ComResult<IDispatch>;
    fn get_bool(&self) -> bool;
    fn change_type(&self, var_enum: VARENUM) -> ComResult<VARIANT>;
    fn copy(&self) -> ComResult<VARIANT>;
    fn is_equal(&self, other: &VARIANT) -> bool;
}

impl VARIANTHelper for VARIANT {
    fn vt(&self) -> u16 {
        unsafe {self.Anonymous.Anonymous.vt}
    }
    fn set_vt(&mut self, vt: VARENUM) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = vt.as_u16();
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn set_double(&mut self, n: f64) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_R8.as_u16();
        v00.Anonymous.dblVal = n;
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn set_bstr(&mut self, bstr: BSTR) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_BSTR.as_u16();
        v00.Anonymous.bstrVal = ManuallyDrop::new(bstr);
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn set_bool(&mut self, b: &bool) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_BOOL.as_u16();
        v00.Anonymous.boolVal = if *b {-1} else {0};
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn set_idispatch(&mut self, idispatch: &IDispatch) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_DISPATCH.as_u16();
        v00.Anonymous.pdispVal = ManuallyDrop::new(Some(idispatch.clone()));
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn _set_variant(&mut self, variant: &mut VARIANT) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_VARIANT.as_u16();
        v00.Anonymous.pvarVal = variant as *mut VARIANT;
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn set_safearray(&mut self, sa: &SAFEARRAY) {
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_VARIANT.as_u16() | VT_ARRAY.as_u16();
        v00.Anonymous.parray = sa as *const SAFEARRAY as *mut SAFEARRAY;
        self.Anonymous.Anonymous = ManuallyDrop::new(v00);
    }
    fn get_double(&self) -> f64 {
        unsafe {
            self.Anonymous.Anonymous.Anonymous.dblVal
        }
    }
    fn get_bstr(&self) -> ComResult<BSTR> {
        unsafe {
            let mut copy = self.copy()?;
            let mut v00 = ManuallyDrop::take(&mut copy.Anonymous.Anonymous);
            let bstr = ManuallyDrop::take(&mut v00.Anonymous.bstrVal);
            drop(copy);
            Ok(bstr)
        }
    }
    fn get_idispatch(&self) -> ComResult<IDispatch> {
        unsafe {
            let p = &self.Anonymous.Anonymous.Anonymous.pdispVal;
            // let disp = IDispatch::from_abi(p)?;
            // let disp = &*p.cast::<IDispatch>();
            let disp = p.as_ref().unwrap();
            Ok(disp.clone())
        }
    }
    fn get_bool(&self) -> bool {
        unsafe {
            self.Anonymous.Anonymous.Anonymous.boolVal != 0
        }
    }
    fn change_type(&self, var_enum: VARENUM) -> ComResult<VARIANT> {
        unsafe {
            let mut dest = VARIANT::default();
            VariantChangeType(&mut dest, self, 0, var_enum.as_u16())?;
            Ok(dest)
        }
    }
    fn copy(&self) -> ComResult<VARIANT> {
        let mut dest = VARIANT::default();
        unsafe {
            VariantCopy(&mut dest, self)?;
        }
        Ok(dest)
    }
    fn is_equal(&self, other: &VARIANT) -> bool {
        // if self.vt() == other.vt() {
        //     match self.vt() as i32 {
        //         VT_R8 => self.get_double() == other.get_double(),
        //         VT_BSTR => {
        //             let b1 = self.get_bstr().unwrap_or_default();
        //             let b2 = other.get_bstr().unwrap_or_default();
        //             b1 == b2
        //         },
        //         VT_BOOL => self.get_bool() == other.get_bool(),
        //         _ => unsafe {self.Anonymous.decVal == other.Anonymous.decVal}
        //     }
        // } else {
        //     false
        // }
        self == other
    }
}

/* SafeArrayを拡張 */

// unsafe impl Send for SAFEARRAY {}

pub trait SAFEARRAYHelper {
    fn new(lbound: i32, ubound: i32) -> Self;
    fn new2(lbound: i32, ubound: i32, lbound2: i32, ubound2: i32) -> Self;
    fn get(&mut self, index: i32) -> ComResult<VARIANT>;
    fn set(&mut self, index: i32, variant: &mut VARIANT) -> ComResult<()>;
    fn len(&self, get_dim: bool) -> ComResult<usize>;
}

impl SAFEARRAYHelper for SAFEARRAY {
    fn new(lbound: i32, ubound: i32) -> Self {
        let vt = VT_VARIANT as u16;
        let cdims = 1;
        let mut rgsabound = SAFEARRAYBOUND::new(lbound, ubound);
        let sa = unsafe {
            let p = SafeArrayCreate(vt, cdims, &mut rgsabound);
            *p
        };
        sa
    }

    fn new2(lbound: i32, ubound: i32, lbound2: i32, ubound2: i32) -> Self {
        let vt = VT_VARIANT as u16;
        let cdims = 2;
        let mut rgsabound = vec![
            SAFEARRAYBOUND::new(lbound2, ubound2),
            SAFEARRAYBOUND::new(lbound, ubound),
        ];
        let sa = unsafe {
            let p = SafeArrayCreate(vt, cdims, rgsabound.as_mut_ptr() as *mut SAFEARRAYBOUND);
            *p
        };
        sa
    }

    fn get(&mut self, mut index: i32) -> ComResult<VARIANT> {
        let psa = self as *mut SAFEARRAY;
        let rgindices = &mut index as *mut i32;
        let mut variant = VARIANT::default();
        let pv = &mut variant as *mut VARIANT as *mut c_void;
        unsafe {
            SafeArrayGetElement(psa, rgindices, pv)?;
        };
        Ok(variant)
    }

    fn set(&mut self, mut index: i32, variant: &mut VARIANT) -> ComResult<()> {
        let psa = self as *mut SAFEARRAY;
        let rgindices = &mut index as *mut i32;
        let pv = variant as *mut VARIANT as *mut c_void;
        unsafe {
            SafeArrayPutElement(psa, rgindices, pv)?;
        };
        Ok(())
    }

    fn len(&self, get_dim: bool) -> ComResult<usize> {
        let psa = self as *const _ as *mut SAFEARRAY;
        let size = unsafe {
            let ndim = SafeArrayGetDim(psa);
            if get_dim {
                ndim as usize
            } else {
                let lb = SafeArrayGetLBound(psa, ndim)?;
                let ub = SafeArrayGetUBound(psa, ndim)?;
                (ub - lb + 1) as usize
            }
        };
        Ok(size)
    }
}

pub trait SAFEARRAYBOUNDHelper {
    fn new(lbound: i32, ubound: i32) -> Self;
}

impl SAFEARRAYBOUNDHelper for SAFEARRAYBOUND {
    fn new(lbound: i32, ubound: i32) -> Self {
        let size = (ubound - lbound + 1) as u32;
        Self {cElements: size, lLbound: lbound}
    }
}