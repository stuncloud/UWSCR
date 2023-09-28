use windows::{
    core::{self},
    Win32::System::Com::{
        CoInitializeEx, CoUninitialize,
        COINIT_MULTITHREADED,
    },
};

pub struct Com;

impl Com {
    /// COMを初期化する
    pub fn init() -> Result<Self, core::Error> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
    /// COMを解除、自身drop時に呼ばれる
    pub fn uninit(&self) {
        unsafe {
            CoUninitialize();
        }
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        self.uninit()
    }
}