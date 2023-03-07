use windows::{
    Win32::{
        UI::{
            Input::{
                KeyboardAndMouse::{
                    BlockInput,
                },
            },
        },
    }
};


pub fn lock(flg: bool) -> bool {
    unsafe { BlockInput(flg).as_bool() }
}

