pub mod bindings {
    windows::include_bindings!();
}

use crate::evaluator::UError;

// convert windows::Error to UError
impl From<windows::Error> for UError {
    fn from(e: windows::Error) -> Self {
        UError::new(
            "Windows Api Error".into(),
            e.message(),
            Some(format!("{:?}", e.code()))
        )
    }
}

