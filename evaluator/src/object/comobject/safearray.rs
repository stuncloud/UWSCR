use std::{mem::ManuallyDrop, ptr::NonNull, ffi::c_void};

use windows::{core::Interface, Win32::System::{Com::{IDispatch, CY, SAFEARRAY, SAFEARRAYBOUND}, Ole::{SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetVartype, SafeArrayUnaccessData, VarEqv}, Variant::VARIANT_0_0}};
use windows::Win32::System::Variant::{VARIANT, VT_BOOL, VT_BSTR, VT_CY, VT_DATE, VT_DECIMAL, VT_DISPATCH, VT_ERROR, VT_I1, VT_I2, VT_I4, VT_I8, VT_INT, VT_R4, VT_R8, VT_UI1, VT_UI2, VT_UI4, VT_UI8, VT_UINT, VT_UNKNOWN, VT_VARIANT};
use windows::core::{BSTR, IUnknown};
use windows::Win32::Foundation::{DECIMAL, VARIANT_BOOL};
use crate::object::{comobject::{ComError, ComResult}, Object, Variant, VariantExt};
use std::ops::Add;

#[derive(Clone, PartialEq)]
pub struct SAVec {
    /// データ
    data: Vec<SAValue>,
    /// 各次元の詳細
    bounds: Vec<SABound>
}
impl std::fmt::Debug for SAVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SafeArray")?;
        for bound in &self.bounds {
            write!(f, "[{bound}]")?
        }
        Ok(())
    }
}
impl SAVec {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(psa: *mut SAFEARRAY) -> ComResult<Self> {
        unsafe {
            if psa.is_null() {
                return Err(ComError::SafeArrayNullPointer);
            };
            let vt = SafeArrayGetVartype(psa)?;
            let this = match vt {
                VT_BOOL => {
                    let sad = SafeArrayData::<VARIANT_BOOL>::new(psa)?;
                    let data = sad.iter().map(|v| SAValue::Bool(v.as_bool())).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_BSTR => {
                    let sad = SafeArrayData::<BSTR>::new(psa)?;
                    let data = sad.iter().map(|bstr| SAValue::Bstr(bstr.clone())).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_CY => {
                    let sad = SafeArrayData::<CY>::new(psa)?;
                    let data = sad.iter().map(|cy| SAValue::Cy(cy.int64)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_DATE => {
                    let sad = SafeArrayData::<f64>::new(psa)?;
                    let data = sad.iter().map(|date| SAValue::Date(*date)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_DECIMAL => {
                    let sad = SafeArrayData::<DECIMAL>::new(psa)?;
                    let data = sad.iter().map(|decimal| SAValue::Decimal(*decimal)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_DISPATCH => {
                    let sad = SafeArrayData::<*mut c_void>::new(psa)?;
                    let data = sad.iter().map(|ptr| {
                        let disp = IDispatch::from_raw(*ptr);
                        SAValue::Dispatch(disp)
                    }).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_ERROR => {
                    let sad = SafeArrayData::<i32>::new(psa)?;
                    let data = sad.iter().map(|scode| SAValue::Error(*scode)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_I1 => {
                    let sad = SafeArrayData::<i8>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::I1(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_I2 => {
                    let sad = SafeArrayData::<i16>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::I2(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_I4 => {
                    let sad = SafeArrayData::<i32>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::I4(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_I8 => {
                    let sad = SafeArrayData::<i64>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::I8(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_INT => {
                    let sad = SafeArrayData::<i32>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Int(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_R4 => {
                    let sad = SafeArrayData::<f32>::new(psa)?;
                    let data = sad.iter().map(|f| SAValue::R4(*f)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_R8 => {
                    let sad = SafeArrayData::<f64>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::R8(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UI1 => {
                    let sad = SafeArrayData::<u8>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Ui1(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UI2 => {
                    let sad = SafeArrayData::<u16>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Ui2(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UI4 => {
                    let sad = SafeArrayData::<u32>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Ui4(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UI8 => {
                    let sad = SafeArrayData::<u64>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Ui8(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UINT => {
                    let sad = SafeArrayData::<u32>::new(psa)?;
                    let data = sad.iter().map(|n| SAValue::Uint(*n)).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_UNKNOWN => {
                    let sad = SafeArrayData::<*mut c_void>::new(psa)?;
                    let data = sad.iter().map(|ptr| {
                        let unk = IUnknown::from_raw(*ptr);
                        SAValue::Unknown(unk)
                    }).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                VT_VARIANT => {
                    let sad = SafeArrayData::<VARIANT>::new(psa)?;
                    let data = sad.iter().map(|v| SAValue::Variant(v.clone())).collect();
                    let bounds = sad.bounds();
                    Self { data, bounds }
                },
                vt => unreachable!("ここには来ないはず: {vt:?}")
            };
            SafeArrayDestroy(psa)?;
            Ok(this)
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// インデックスを指定して値を得る
    /// - indices: 多次元インデックス (SAFEARRAYなので負の数もあり得る)
    pub fn get(&self, indices: &[i32]) -> ComResult<Object> {
        let index = self.indices_to_index(indices)?;
        let t = self.data.iter().nth(index).unwrap(/* 計算違いがなければ必ず値を返す */);
        Ok(t.into())
    }
    /// 多次元インデックスから実際のインデックスを得る
    fn indices_to_index(&self, indices: &[i32]) -> ComResult<usize> {
        if self.bounds.len() == indices.len() {
            self.bounds.iter().zip(indices.iter()).enumerate()
                .try_fold(0usize, |index, (dim, (SABound { lbound, size }, i))| {
                    // インデックス下限
                    let lb = *lbound;
                    // インデックス上限
                    let ub = lb + *size as i32;
                    if (lb..ub).contains(i) {
                        // 0からのインデックス値に換算
                        let i = *i - lb;
                        Ok(index + i as usize)
                    } else {
                        Err(ComError::SafeArrayIndexOutOfBounds(dim+1, (lb, ub-1)))
                    }
                })
        } else {
            Err(ComError::SafeArrayDimensionMismatch)
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &SAValue> {
        self.data.iter()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SABound {
    /// インデックス下限
    lbound: i32,
    /// サイズ
    size: u32,
}
impl From<&SAFEARRAYBOUND> for SABound {
    fn from(sab: &SAFEARRAYBOUND) -> Self {
        SABound { lbound: sab.lLbound, size: sab.cElements }
    }
}
impl std::fmt::Display for SABound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.size)
    }
}

#[derive(Clone)]
pub enum SAValue {
    Bool(bool),
    Bstr(BSTR),
    Cy(i64),
    Date(f64),
    Decimal(DECIMAL),
    Dispatch(IDispatch),
    Error(i32),
    I1(i8),
    I2(i16),
    I4(i32),
    I8(i64),
    Int(i32),
    R4(f32),
    R8(f64),
    Ui1(u8),
    Ui2(u16),
    Ui4(u32),
    Ui8(u64),
    Uint(u32),
    Unknown(IUnknown),
    Variant(VARIANT),
}
impl PartialEq for SAValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Bstr(l0), Self::Bstr(r0)) => l0 == r0,
            (Self::Date(l0), Self::Date(r0)) => l0 == r0,
            (Self::Cy(l0), Self::Cy(r0)) => l0 == r0,
            (Self::Decimal(l0), Self::Decimal(r0)) => {
                Variant::from(*l0) == Variant::from(*r0)
            },
            (Self::Dispatch(l0), Self::Dispatch(r0)) => l0 == r0,
            (Self::Error(l0), Self::Error(r0)) => l0 == r0,
            (Self::I1(l0), Self::I1(r0)) => l0 == r0,
            (Self::I2(l0), Self::I2(r0)) => l0 == r0,
            (Self::I4(l0), Self::I4(r0)) => l0 == r0,
            (Self::I8(l0), Self::I8(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::R4(l0), Self::R4(r0)) => l0 == r0,
            (Self::R8(l0), Self::R8(r0)) => l0 == r0,
            (Self::Ui1(l0), Self::Ui1(r0)) => l0 == r0,
            (Self::Ui2(l0), Self::Ui2(r0)) => l0 == r0,
            (Self::Ui4(l0), Self::Ui4(r0)) => l0 == r0,
            (Self::Ui8(l0), Self::Ui8(r0)) => l0 == r0,
            (Self::Uint(l0), Self::Uint(r0)) => l0 == r0,
            (Self::Unknown(l0), Self::Unknown(r0)) => l0 == r0,
            (Self::Variant(l0), Self::Variant(r0)) => unsafe {
                VarEqv(l0, r0).is_ok_and(|v| v.to_bool().unwrap_or_default())
            },
            _ => false,
        }
    }
}
impl From<&SAValue> for Object {
    fn from(value: &SAValue) -> Self {
        match value {
            SAValue::Bool(b) => Object::Bool(*b),
            SAValue::Bstr(bstr) => Object::String(bstr.to_string()),
            SAValue::Cy(cy) => (*cy).into(),
            SAValue::Date(date) => Object::Num(*date),
            SAValue::Decimal(decimal) => (*decimal).into(),
            SAValue::Dispatch(idispatch) => Object::ComObject(idispatch.clone().into()),
            SAValue::Error(scode) => Object::Num((*scode).into()),
            SAValue::I1(n) => Object::Num((*n).into()),
            SAValue::I2(n) => Object::Num((*n).into()),
            SAValue::I4(n) => Object::Num((*n).into()),
            SAValue::I8(n) => Object::Num(*n as f64),
            SAValue::Int(n) => Object::Num((*n).into()),
            SAValue::R4(n) => Object::Num((*n).into()),
            SAValue::R8(n) => Object::Num(*n),
            SAValue::Ui1(n) => Object::Num((*n).into()),
            SAValue::Ui2(n) => Object::Num((*n).into()),
            SAValue::Ui4(n) => Object::Num((*n).into()),
            SAValue::Ui8(n) => Object::Num(*n as f64),
            SAValue::Uint(n) => Object::Num((*n).into()),
            SAValue::Unknown(iunknown) => Object::Unknown(iunknown.clone().into()),
            SAValue::Variant(variant) => Object::Variant(variant.clone().into()),
        }
    }
}
// impl From<CY> for Variant {
//     fn from(value: CY) -> Self {
//         let mut variant = VARIANT::default();
//         let mut v00 = VARIANT_0_0 {
//             vt: VT_CY,
//             ..Default::default()
//         };
//         v00.Anonymous.cyVal = value;
//         variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
//         Variant(variant)
//     }
// }
// impl From<CY> for Object {
//     fn from(value: CY) -> Self {
//         Object::Variant(value.into())
//     }
// }
impl From<DECIMAL> for Variant {
    fn from(value: DECIMAL) -> Self {
        let mut variant = VARIANT::default();
        variant.Anonymous.decVal = value;
        let v00 = VARIANT_0_0 {
            vt: VT_DECIMAL,
            ..Default::default()
        };
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        Variant(variant)
    }
}
impl From<DECIMAL> for Object {
    fn from(value: DECIMAL) -> Self {
        let var = Variant::from(value);
        var.try_into().unwrap_or(Object::Null)
    }
}

/// SafeArrayへのアクセスを行う
pub struct SafeArrayData<T> {
    ptr: NonNull<SAFEARRAY>,
    data: *mut T,
    /// 各次元の開始位置とサイズ
    bounds: Vec<SABound>,
}
impl<T> Drop for SafeArrayData<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = SafeArrayUnaccessData(self.ptr.as_ptr());
        }
    }
}
impl<T> SafeArrayData<T> {
    pub fn new(ptr: *mut SAFEARRAY) -> ComResult<Self>{
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        let data = Self::get_sa_data(ptr.as_ptr())?.cast::<T>();
        let bounds = Self::get_bounds(&ptr);
        Ok(Self { ptr, data, bounds })
    }
    fn get_bounds(psa: &NonNull<SAFEARRAY>) -> Vec<SABound> {
        unsafe {
            let sa = psa.as_ref();
            let pbounds = sa.rgsabound.as_ptr();
            std::slice::from_raw_parts(pbounds, sa.cDims as _)
                .iter()
                .map(SABound::from)
                .collect()
        }
    }
    fn get_sa_data(ptr: *const SAFEARRAY) -> ComResult<*mut c_void> {
        unsafe {
            let mut data = std::ptr::null_mut();
            SafeArrayAccessData(ptr, &mut data)?;
            Ok(data)
        }
    }
    /// 要素数を得る
    pub fn len(&self) -> u32 {
        self.bounds.iter().fold(0u32, |n, SABound { lbound:_, size }| n.add(size))
    }
    /// Tのイテレータを返す
    pub(crate) fn iter(&self) -> impl Iterator<Item = &'_ T> + '_ {
        unsafe {
            (0..self.len())
                .map(|i| &*self.data.offset(i as _))
        }
    }
    fn bounds(&self) -> Vec<SABound> {
        self.bounds.clone()
    }
}