use std::fmt;
use windows::Win32::System::{
    Ole::VarEqv,
    Variant::{
        VariantClear, VARENUM, VARIANT, VT_BOOL, VT_BSTR, VT_CY, VT_DATE, VT_DECIMAL, VT_DISPATCH, VT_ERROR, VT_I1, VT_I2, VT_I4, VT_I8, VT_INT, VT_NULL, VT_R4, VT_R8, VT_UI1, VT_UI2, VT_UI4, VT_UI8, VT_UINT, VT_UNKNOWN,
    }
};
use super::{
    Object,
    comobject::{VariantExt, ComError, ComResult, Unknown, ComObject},
    UError, UErrorKind, UErrorMessage,
};

#[derive(Clone)]
pub struct Variant(pub VARIANT);

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            match VarEqv(&self.0, &other.0) {
                Ok(variant) => variant.to_bool().unwrap_or(false),
                Err(_) => false,
            }
        }
    }
}

impl Variant {
    pub fn get(&self) -> VARIANT {
        self.0.clone()
    }
    pub fn get_type(&self) -> u16 {
        let vt = self.0.vt();
        vt.0
    }
    pub fn change_type(&self, vt: u16) -> ComResult<Self> {
        let new = self.0.change_type(VARENUM(vt))?;
        Ok(Self(new))
    }
    pub fn into_object(self) -> Object {
        Object::Variant(self)
    }
}

impl From<VARIANT> for Variant {
    fn from(value: VARIANT) -> Self {
        Self(value)
    }
}
impl TryFrom<Object> for Variant {
    type Error = ComError;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        let variant = obj.try_into()?;
        Ok(Self(variant))
    }
}
// impl From<Variant> for Object {
//     fn from(val: Variant) -> Self {
//         Object::Variant(val)
//     }
// }
impl TryFrom<Variant> for Object {
    type Error = UError;
    fn try_from(value: Variant) -> Result<Self, Self::Error> {
        unsafe {
            let v00 = &value.0.Anonymous.Anonymous;
            let obj = match value.0.vt() {
                VT_BOOL => v00.Anonymous.boolVal.as_bool().into(),
                VT_BSTR => v00.Anonymous.bstrVal.to_string().into(),
                VT_CY => v00.Anonymous.cyVal.int64.into(),
                VT_DATE => v00.Anonymous.date.into(),
                VT_DECIMAL => {
                    let r8 = value.change_type(VT_R8.0)?;
                    return r8.try_into();
                },
                VT_ERROR => v00.Anonymous.scode.into(),
                VT_I1 => v00.Anonymous.cVal.into(),
                VT_I2 => (v00.Anonymous.iVal as i32).into(),
                VT_I4 => v00.Anonymous.lVal.into(),
                VT_I8 => v00.Anonymous.llVal.into(),
                VT_UI1 => v00.Anonymous.bVal.into(),
                VT_UI2 => v00.Anonymous.uiVal.into(),
                VT_UI4 => v00.Anonymous.ulVal.into(),
                VT_UI8 => (v00.Anonymous.ullVal as f64).into(),
                VT_R4 => v00.Anonymous.fltVal.into(),
                VT_R8 => v00.Anonymous.dblVal.into(),
                VT_INT => v00.Anonymous.intVal.into(),
                VT_UINT => v00.Anonymous.uintVal.into(),
                VT_UNKNOWN => match v00.Anonymous.punkVal.as_ref() {
                    Some(unk) => Object::Unknown(Unknown::from(unk.clone())),
                    None => Object::Nothing,
                },
                VT_DISPATCH => match v00.Anonymous.pdispVal.as_ref() {
                    Some(disp) => Object::ComObject(ComObject::from(disp.clone())),
                    None => Object::Nothing,
                },
                VARENUM(vt) => {
                    return Err(UError::new(UErrorKind::VariantError, UErrorMessage::FromVariant(vt)));
                },
            };
            Ok(obj)
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VARIANT")
            .field("vt", &self.0.vt())
            .finish()
    }
}
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vt = self.0.vt().0;
        write!(f, "VARIANT({vt})")
    }
}