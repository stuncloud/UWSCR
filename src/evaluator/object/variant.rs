use std::fmt;
use windows::Win32::System::Com::VARIANT;
use crate::evaluator::com_object::VARIANTHelper;

#[derive(Clone)]
pub struct Variant(pub VARIANT);

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_equal(&other.0)
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VARIANT")
            .field("vt", &self.0.vt())
            .finish()
    }
}
