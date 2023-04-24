use windows::{
    core::{HSTRING, GUID, self, PCWSTR, PWSTR, ComInterface, HRESULT},
    Win32::{
        Media::{
            Audio::{
                PlaySoundW,
                SND_ASYNC, SND_SYNC, SND_NODEFAULT,
            },
            Speech::{
                ISpVoice, SPF_ASYNC, SPF_PURGEBEFORESPEAK, SPF_IS_NOT_XML,
                ISpRecognizer, ISpRecoContext, ISpRecoGrammar, ISpRecoResult, ISpObjectToken, ISpObjectTokenCategory,
                SPCAT_RECOGNIZERS, SPCAT_AUDIOIN,
                SPLO_STATIC,
                SPRS_ACTIVE,
                SPRAF_TopLevel, SPRAF_Active,
                SPWT_LEXICAL,
                SPGS_ENABLED,
                SPEVENT, SPEVENTENUM, SPEI_RECOGNITION, SPEI_RESERVED1, SPEI_RESERVED2, SPEI_UNDEFINED,
                SPEVENTLPARAMTYPE, SPET_LPARAM_IS_OBJECT
            }
        },
        System::{
            Com::{
                CoCreateInstance, CLSCTX_INPROC_SERVER, CLSCTX_ALL,
            },
            Threading::INFINITE,
            Diagnostics::Debug::Beep,
        },
    }
};
use std::rc::Rc;
// use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::ptr;
use std::ffi::c_void;

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

thread_local! {
    static SPEAK: Rc<RefCell<Speak>> = Rc::new(RefCell::new(Speak::new()));
}

struct Speak(Option<ISpVoice>);
impl Speak {
    fn new() -> Self {
        Self(None)
    }
    pub fn speak(&mut self, text: String, unsync: bool, interrupt: bool) -> core::Result<()> {
        unsafe {
            let voice = match &self.0 {
                Some(voice) => voice.clone(),
                None => {
                    let rclsid = GUID::from_u128(0x96749377_3391_11D2_9EE3_00C04F797396);
                    let voice = CoCreateInstance::<_, ISpVoice>(&rclsid, None, CLSCTX_ALL)?;
                    voice
                },
            };

            let pwcs = HSTRING::from(text);
            let mut dwflags = SPF_IS_NOT_XML.0 as u32;
            if interrupt {
                dwflags |= SPF_PURGEBEFORESPEAK.0 as u32;
            } else {
                voice.WaitUntilDone(INFINITE)?;
            }
            if unsync {
                // 非同期
                dwflags |= SPF_ASYNC.0 as u32;
                voice.Speak(&pwcs, dwflags, None)?;
                self.0 = Some(voice);
            } else {
                voice.Speak(&pwcs, dwflags, None)?;
                self.0 = None;
            }
            Ok(())
        }
    }
}

pub fn speak(text: String, unsync: bool, interrupt: bool) -> core::Result<()> {
    let cell = SPEAK.with(|rc| rc.clone());
    let r = cell.borrow_mut().speak(text, unsync, interrupt);
    r
}

#[allow(non_upper_case_globals)]
const CLSID_SpInprocRecognizer: GUID = GUID::from_u128(0x41B89B6B_9399_11D2_9623_00C04F8EE628);
#[allow(non_upper_case_globals)]
const CLSID_SpObjectTokenCategory: GUID = GUID::from_u128(0xA910187F_0C7A_45AC_92CC_59EDAFB77B53);

thread_local! {
    static RECOGNIZER: Rc<RefCell<Recognizer>> = Rc::new(RefCell::new(Recognizer::new()));
}

struct Recognizer {
    recognizer: Option<ISpRecognizer>,
    context: Option<ISpRecoContext>,
    grammar: Option<ISpRecoGrammar>,
}

impl Recognizer {
    fn new() -> Self {
        Self {
            recognizer: None,
            context: None,
            grammar: None,
        }
    }
    fn set(&mut self, words: Vec<String>) -> core::Result<()> {
        unsafe {
            let recognizer = Self::create_inproc_server_instance::<ISpRecognizer>(&CLSID_SpInprocRecognizer)?;
            let engine = Self::get_token(SPCAT_RECOGNIZERS, "Recognizer not found")?;
            recognizer.SetRecognizer(&engine)?;
            let audio = Self::get_token(SPCAT_AUDIOIN, "Audio input not found")?;
            recognizer.SetInput(&audio, true)?;

            let context = recognizer.CreateRecoContext()?;
            context.SetNotifyWin32Event()?;
            let interest = Self::spfei(SPEI_RECOGNITION);
            context.SetInterest(interest, interest)?;

            let grammar = context.CreateGrammar(0)?;
            let mut hfromstate = ptr::null_mut();
            let dwattributes = (SPRAF_TopLevel.0|SPRAF_Active.0) as u32;
            grammar.GetRule(None, 1, dwattributes, true, &mut hfromstate)?;
            grammar.ClearRule(hfromstate)?;
            if words.is_empty() {
                // 単語リストが空なら標準辞書をロード
                grammar.LoadDictation(None, SPLO_STATIC)?;
                grammar.SetDictationState(SPRS_ACTIVE)?;
            } else {
                // 単語リストを登録
                for word in words {
                    let psz = HSTRING::from(word);
                    grammar.AddWordTransition(hfromstate, ptr::null_mut(), &psz, None, SPWT_LEXICAL, 1.0, ptr::null())?;
                }
                grammar.Commit(0)?;
                grammar.SetGrammarState(SPGS_ENABLED)?;
                grammar.SetRuleState(None, ptr::null_mut(), SPRS_ACTIVE)?;
            }

            self.recognizer = Some(recognizer);
            self.context = Some(context);
            self.grammar = Some(grammar);
            Ok(())
        }
    }
    fn remove(&mut self) -> core::Result<()> {
        self.grammar = None;
        self.context = None;
        self.recognizer = None;
        Ok(())
    }
    fn dictate(&self, wait: bool, timeout: u32) -> core::Result<Option<String>> {
        unsafe {
            if let Some(context) = &self.context {
                let dwmilliseconds = if timeout == 0 {INFINITE} else {timeout};
                loop {
                    if wait {
                        context.WaitForNotifyEvent(dwmilliseconds)?;
                    }

                    let mut event = SPEVENT::default();
                    context.GetEvents(1, &mut event, ptr::null_mut())?;
                    let lparam_type = SPEVENTLPARAMTYPE((event._bitfield >> 16) & 0xFFFF);
                    let event_id = SPEVENTENUM(event._bitfield & 0xFFFF);
                    match event_id {
                        SPEI_RECOGNITION => {
                            if lparam_type == SPET_LPARAM_IS_OBJECT {
                                let ptr = event.lParam.0 as *mut c_void;
                                let result: ISpRecoResult = std::mem::transmute(ptr);
                                let mut buffer = [0; 260];
                                let mut text = PWSTR::from_raw(buffer.as_mut_ptr());
                                result.GetText(0xFFFFFFFF, 0xFFFFFFFF, true, &mut text, None)?;
                                let text = text.to_hstring()?.to_string_lossy();
                                break Ok(Some(text));
                            }
                        },
                        SPEI_UNDEFINED => {
                            break Ok(None);
                        }
                        _ => {}
                    }
                    if ! wait {
                        break Ok(None);
                    }
                }
            } else {
                Ok(None)
            }
        }
    }

    fn get_token(category_id: PCWSTR, err_msg: &str) -> core::Result<ISpObjectToken> {
        unsafe {
            let category = Self::create_inproc_server_instance::<ISpObjectTokenCategory>(&CLSID_SpObjectTokenCategory)?;
            category.SetId(category_id, true)?;

            let tokens = category.EnumTokens(None, None)?;
            let mut pelt = None;
            tokens.Next(1, &mut pelt, None)?;
            pelt.ok_or(Self::error(-1, err_msg))
        }
    }
    pub fn get_engine_name(&self) -> core::Result<Option<String>> {
        unsafe {
            if let Some(recognizer) = &self.recognizer {
                let token = recognizer.GetRecognizer()?;
                let pwstr = token.GetStringValue(None)?;
                let name = pwstr.to_hstring()?.to_string_lossy();
                Ok(Some(name))
            } else {
                Ok(None)
            }
        }
    }
    fn create_inproc_server_instance<T: ComInterface>(clsid: &GUID) -> core::Result<T> {
        unsafe {
            CoCreateInstance(clsid, None, CLSCTX_INPROC_SERVER)
        }
    }
    fn spfei(event_enum: SPEVENTENUM) -> u64 {
        let flag_check = (1u64 << SPEI_RESERVED1.0) | (1u64 << SPEI_RESERVED2.0);
        (1u64 << event_enum.0) | flag_check
    }
    fn error(code: i32, message: &str) -> core::Error {
        core::Error::new(HRESULT(code), HSTRING::from(message))
    }
}

impl Drop for Recognizer {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

pub fn remove_recognizer() {
    let cell = RECOGNIZER.with(|rc| rc.clone());
    let _ = cell.borrow_mut().remove();
}

pub fn recostate(words: Option<Vec<String>>) -> core::Result<Option<String>> {
    let cell = RECOGNIZER.with(|rc| rc.clone());
    if let Some(words) = words {
        cell.borrow_mut().set(words)?;
        cell.borrow_mut().get_engine_name()
    } else {
        cell.borrow_mut().remove()?;
        Ok(None)
    }
}

pub fn dictate(wait: bool, timeout: u32) -> core::Result<Option<String>> {
    let cell = RECOGNIZER.with(|rc| rc.clone());
    let text = cell.borrow().dictate(wait, timeout);
    text
}