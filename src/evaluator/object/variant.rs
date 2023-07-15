use std::fmt;
use windows::Win32::System::{
    Com::{
        VARIANT, VARENUM,
    },
    Ole::{VariantClear, VarEqv, VariantChangeType},
};
use super::{
    Object,
    comobject::{VariantExt, ComError, ComResult}
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
        unsafe {
            let mut new = VARIANT::default();
            let vt = VARENUM(vt);
            VariantChangeType(&mut new, &self.0, 0, vt)?;
            Ok(Self(new))
        }
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
impl Into<Object> for Variant {
    fn into(self) -> Object {
        Object::Variant(self)
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