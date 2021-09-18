use windows::Abi;

use crate::winapi::{
    bindings::Windows::Win32::{
        Foundation::{PWSTR, BSTR, DISP_E_MEMBERNOTFOUND},
        System::{
            // Com::{
            //     COINIT_APARTMENTTHREADED, CLSCTX_ALL,
            //     CLSIDFromProgID, CoInitializeEx, CoCreateInstance,
            // },
            OleAutomation::{
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
                VARENUM, SAFEARRAY, SAFEARRAYBOUND,
                DISPPARAMS, EXCEPINFO,
                VARIANT, VARIANT_abi, VARIANT_0, VARIANT_0_0_abi, VARIANT_0_0_0_abi, VARIANT_0_0_0_0_abi,
                IDispatch,
                VariantInit, VariantChangeType,
                SafeArrayCreate, SafeArrayGetElement, SafeArrayPutElement,
                SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayGetDim,
            },
        },
    },
    to_wide_string,
};
use crate::evaluator::{
    UError, EvalResult,
    object::Object,
};

use std::{
    ffi::c_void,
    fmt, ptr,
    // sync::{Arc, Mutex},
};

/* COMエラー */

pub struct ComError {
    pub message: String,
    pub code: u32,
    pub description: Option<String>
}

impl ComError {
    pub fn new_with_description(e: &windows::Error, desc: &str) -> Self {
        Self {
            message: e.message(),
            code: e.code().0,
            description: Some(desc.into())
        }
    }
    pub fn new(e: &windows::Error) -> Self {
        Self {
            message: e.message(),
            code: e.code().0,
            description: None
        }
    }
}

impl From<windows::Error> for ComError {
    fn from(e: windows::Error) -> Self {
        Self::new(&e)
    }
}

impl From<ComError> for UError {
    fn from(e: ComError) -> Self {
        let mut uerr = Self::new(
            &format!("Com Error(0x{:08X})", e.code),
            &e.message,
            match e.description {
            Some(ref s) => Some(s),
            None => None
        });
        uerr.is_com_error = true;
        uerr
    }
}

/* COMに渡す値の実体 */

#[derive(Clone, Debug)]
pub enum ComArg {
    Num(f64),
    Bstr(BSTR),
    Bool(bool),
    Dispatch(IDispatch),
    Variant(VARIANT),
    Empty,
    Null,
    // Nothing,
    // Invalid
}

impl ComArg {
    pub fn from_object(object: Object) -> EvalResult<Self> {
        let comarg = match object {
            Object::Num(n) => Self::Num(n),
            Object::String(ref s) => {
                let wide = to_wide_string(s);
                Self::Bstr(BSTR::from_wide(&wide))
            },
            Object::ComObject(ref d) => Self::Dispatch(d.clone()),
            Object::Variant(ref v) => Self::Variant(v.clone()),
            Object::Null => Self::Null,
            Object::Bool(b) => Self::Bool(b),
            o => return Err(UError::new(
                "COM conversion error",
                &format!("can not convert {} to VARIANT", o),
                None
            )),
        };
        Ok(comarg)
    }

    pub fn to_variant(&self) -> VARIANT {
        let mut variant = VARIANT::default();
        match self {
            ComArg::Num(n) => variant.set_double(*n),
            ComArg::Bstr(bstr) => variant.set_bstr(bstr),
            ComArg::Bool(b) => variant.set_bool(b),
            ComArg::Dispatch(d) => variant.set_idispatch(d),
            ComArg::Variant(v) => variant.set_variant(v),
            ComArg::Empty => {}, // そのまま
            ComArg::Null => variant.set_vt(VT_NULL),
        }
        variant
    }
}


/* Objectの拡張 */
pub trait ObjectHelper {
    fn from_variant(variant: VARIANT) -> EvalResult<Object>;
}

impl ObjectHelper for Object {
    fn from_variant(variant: VARIANT) -> EvalResult<Self> {
        // VT_ARRAYの場合
        let is_array = (variant.vt() & VT_ARRAY.0 as u16) > 0;
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
                arr.push(Object::from_variant(v)?);
            }
            return Ok(Object::Array(arr))
        }

        // let is_ref = (variant.vt() & VT_BYREF.0 as u16) > 0;
        let vt = variant.vt() & VT_TYPEMASK.0 as u16;
        let obj = match VARENUM(vt as i32) {
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
                let mut dest = VARIANT::default();
                unsafe {
                    VariantChangeType(&mut dest, &variant, 0, VT_R8.0 as u16)?;
                }
                Object::Num(dest.get_double())
            },
            VT_BSTR => {
                let bstr = variant.get_bstr()?;
                Object::String(bstr.to_string())
            },
            VT_DATE |
            VT_LPSTR |
            VT_LPWSTR => {
                let mut dest = VARIANT::default();
                unsafe {
                    VariantChangeType(&mut dest, &variant, 0, VT_BSTR.0 as u16)?;
                }
                let bstr = dest.get_bstr()?;
                Object::String(bstr.to_string())
            },
            VT_DISPATCH => {
                let disp = variant.get_idispatch()?;
                Object::ComObject(disp)
            },
            VT_BOOL => Object::Bool(variant.get_bool()),
            _ => Object::Variant(variant),
        };
        Ok(obj)
    }
}

/* IDispatchの拡張 */

const LOCALE_USER_DEFAULT: u32 = 0x0400;
const LOCALE_SYSTEM_DEFAULT: u32 = 0x0800;
type ComResult<T> = Result<T, ComError>;

unsafe impl Send for IDispatch {}

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
            Ok(v) => Ok(v),
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
            let dispidmember = match self.GetIDsOfNames(
                &windows::Guid::default(),
                &mut PWSTR(member.as_mut_ptr()),
                1,
                LOCALE_USER_DEFAULT
            ) {
                Ok(id) => id,
                Err(e) => return Err(ComError::new(&e)),
            };

            let mut excepinfo = EXCEPINFO::default();
            let mut argerr = 0;
            let mut result = VARIANT::default();

            match self.Invoke(
                dispidmember,
                &windows::Guid::default(),
                LOCALE_SYSTEM_DEFAULT,
                wflags,
                dp,
                &mut result,
                &mut excepinfo,
                &mut argerr
            ) {
                Ok(()) => Ok(result),
                Err(e) => {
                    let com_err = ComError::new_with_description(&e, &excepinfo.bstrDescription.to_string());
                    Err(com_err)
                },
            }
        }
    }
}

/* VARIANTに不足しているTraitを実装 */

impl Default for VARIANT {
    fn default() -> Self {
        let mut variant = VARIANT {
            Anonymous: VARIANT_0 {
                Anonymous: VARIANT_0_0_abi {
                    vt: 0, // VT_EMPTY
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: VARIANT_0_0_0_abi {
                        Anonymous: VARIANT_0_0_0_0_abi {
                            pvRecord: ptr::null_mut() as *mut c_void,
                            pRecInfo: ptr::null_mut() as *mut c_void
                        }
                    }
                }
            }
        };
        unsafe {
            VariantInit(&mut variant);
        }
        variant
    }
}

impl Clone for VARIANT {
    fn clone(&self) -> Self {
        let mut variant = VARIANT::default();
        variant.Anonymous.Anonymous = unsafe {self.Anonymous.Anonymous.clone()};
        variant
    }
}

impl std::fmt::Debug for VARIANT {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VARIANT")
            .field("vt", &self.vt())
            .finish()
    }
}

unsafe impl Send for VARIANT {}

pub trait VARIANTHelper {
    fn vt(&self) -> u16;
    fn set_vt(&mut self, vt: VARENUM);
    fn set_double(&mut self, n: f64);
    fn set_bstr(&mut self, bstr: &BSTR);
    fn set_idispatch(&mut self, idispatch: &IDispatch);
    fn set_variant(&mut self, variant: &VARIANT);
    fn get_double(&self) -> f64;
    fn get_bstr(&self) -> ComResult<BSTR>;
    fn get_idispatch(&self) -> ComResult<IDispatch>;
    fn get_bool(&self) -> bool;
}

impl VARIANT {
    pub fn vt(&self) -> u16 {
        unsafe {self.Anonymous.Anonymous.vt}
    }
    fn set_vt(&mut self, vt: VARENUM) {
        self.Anonymous.Anonymous.vt = vt.0 as u16;
    }
    fn set_double(&mut self, n: f64) {
        self.set_vt(VT_R8);
        self.Anonymous.Anonymous.Anonymous.dblVal = n;
    }
    fn set_bstr(&mut self, bstr: &BSTR) {
        self.set_vt(VT_BSTR);
        self.Anonymous.Anonymous.Anonymous.bstrVal = bstr.abi();
    }
    fn set_bool(&mut self, b: &bool) {
        self.set_vt(VT_BOOL);
        self.Anonymous.Anonymous.Anonymous.boolVal = if *b {-1} else {0};
    }
    fn set_idispatch(&mut self, idispatch: &IDispatch) {
        self.set_vt(VT_DISPATCH);
        self.Anonymous.Anonymous.Anonymous.pdispVal = idispatch.abi();
    }
    fn set_variant(&mut self, variant: &VARIANT) {
        self.set_vt(VT_VARIANT);
        let p = &mut variant.abi() as *mut VARIANT_abi;
        self.Anonymous.Anonymous.Anonymous.pvarVal = p;
    }
    fn get_double(&self) -> f64 {
        unsafe {
            self.Anonymous.Anonymous.Anonymous.dblVal
        }
    }
    fn get_bstr(&self) -> ComResult<BSTR> {
        unsafe {
            let p = self.Anonymous.Anonymous.Anonymous.bstrVal;
            let bstr = BSTR::from_abi(p)?;
            Ok(bstr)
        }
    }
    fn get_idispatch(&self) -> ComResult<IDispatch> {
        unsafe {
            let p = self.Anonymous.Anonymous.Anonymous.pdispVal;
            let disp = IDispatch::from_abi(p)?;
            Ok(disp)
        }
    }
    fn get_bool(&self) -> bool {
        unsafe {
            self.Anonymous.Anonymous.Anonymous.boolVal != 0
        }
    }
    pub fn change_type(&self, vt: u16) -> ComResult<VARIANT> {
        unsafe {
            let mut dest = VARIANT::default();
            VariantChangeType(&mut dest, self, 0, vt)?;
            Ok(dest)
        }
    }
}

/* SafeArrayを拡張 */

unsafe impl Send for SAFEARRAY {}

pub trait SAFEARRAYHelper {
    fn new(lbound: i32, size: u32) -> SAFEARRAY;
    fn get(&mut self, index: i32) -> ComResult<VARIANT>;
    fn put(&mut self, index: i32, variant: &mut VARIANT) -> ComResult<()>;
}

impl SAFEARRAY {
    pub fn new(lbound: i32, ubound: i32) -> Self {
        let vt = VT_VARIANT.0 as u16;
        let cdims = 1;
        let mut rgsabound = SAFEARRAYBOUND::new(lbound, ubound);
        let sa = unsafe {
            let p = SafeArrayCreate(vt, cdims, &mut rgsabound);
            *p
        };
        sa
    }

    pub fn new2(lbound: i32, ubound: i32, lbound2: i32, ubound2: i32) -> Self {
        let vt = VT_VARIANT.0 as u16;
        let cdims = 2;
        let mut rgsabound = vec![
            SAFEARRAYBOUND::new(lbound, ubound),
            SAFEARRAYBOUND::new(lbound2, ubound2),
        ];
        let sa = unsafe {
            let p = SafeArrayCreate(vt, cdims, rgsabound.as_mut_ptr() as *mut SAFEARRAYBOUND);
            *p
        };
        sa
    }

    pub fn get(&mut self, mut index: i32) -> ComResult<VARIANT> {
        let psa = self as *mut SAFEARRAY;
        let rgindices = &mut index as *mut i32;
        let mut variant = VARIANT::default();
        let pv = &mut variant as *mut VARIANT as *mut c_void;
        unsafe {
            SafeArrayGetElement(psa, rgindices, pv)?;
        };
        Ok(variant)
    }

    pub fn set(&mut self, mut index: i32, variant: &mut VARIANT) -> ComResult<()> {
        let psa = self as *mut SAFEARRAY;
        let rgindices = &mut index as *mut i32;
        let pv = variant as *mut VARIANT as *mut c_void;
        unsafe {
            SafeArrayPutElement(psa, rgindices, pv)?;
        };
        Ok(())
    }

    pub fn len(&self, ndim: u32) -> ComResult<usize> {
        let psa = self as *const _ as *mut SAFEARRAY;
        let size = unsafe {
            let dim_size = SafeArrayGetDim(psa);
            if ndim == 0 {
                dim_size as usize
            // } else if dim_size > 1 && ndim == 1 {
            //     SafeArrayGetElemsize(psa) as usize
            } else {
                let lb = SafeArrayGetLBound(psa, ndim)?;
                let ub = SafeArrayGetUBound(psa, ndim)?;
                (ub - lb + 1) as usize
            }
        };
        Ok(size)
    }
}

impl SAFEARRAYBOUND {
    pub fn new(lbound: i32, ubound: i32) -> Self {
        let size = (ubound - lbound + 1) as u32;
        Self {cElements: size, lLbound: lbound}
    }

}

// // ObjectでVARIANTを表す
// #[derive(Clone, Debug)]
// pub struct UVariant(pub u16, pub Box<Object>);