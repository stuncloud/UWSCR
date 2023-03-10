use windows::{
    core::{HSTRING},
    Win32::{
        Media::{
            Audio::{
                PlaySoundW,
                SND_ASYNC, SND_SYNC, SND_NODEFAULT,
            },
        },
        System::Diagnostics::Debug::Beep,
    }
};

pub fn play_sound(name: &str, sync: bool, _device: u32) {
    unsafe {
        let pszsound = HSTRING::from(name);
        let fdwsound = if sync {SND_SYNC} else {SND_ASYNC};
        PlaySoundW(&pszsound, None, fdwsound);
    }
}
pub fn stop_sound() {
    unsafe {
        PlaySoundW(None, None, SND_NODEFAULT);
    }
}

pub fn beep(duration: u32, freq: u32, count: u32) {
    unsafe {
        for _ in 0..count {
            Beep(freq, duration);
        }
    }
}