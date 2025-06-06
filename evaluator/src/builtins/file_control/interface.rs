
use windows::{
    core::{
        implement,
        HRESULT,
        Result as WinResult, Error as WinError,
    },
    Win32::{
        Foundation::{
            BOOL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
            DATA_S_SAMEFORMATETC, OLE_E_ADVISENOTSUPPORTED,
            E_NOTIMPL, E_INVALIDARG, E_POINTER,
            S_OK, S_FALSE,
            DV_E_FORMATETC, DV_E_DVASPECT, DV_E_TYMED, DV_E_CLIPFORMAT,
        },
        System::{
            Com::{
                IDataObject, IDataObject_Impl,
                IEnumFORMATETC, IEnumFORMATETC_Impl,
                IAdviseSink, IEnumSTATDATA,
                FORMATETC, STGMEDIUM,
                DATADIR, DATADIR_GET,
                DVASPECT_CONTENT,
            },
            Ole::{
                IDropSource, IDropSource_Impl,
                DROPEFFECT,
                ReleaseStgMedium,
            },
            SystemServices::MODIFIERKEYS_FLAGS,
        },
    }
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, AtomicPtr, Ordering::SeqCst}
};
use std::ops::ControlFlow;

#[derive(Default)]
#[implement(IDropSource)]
pub struct DropSource;
impl DropSource {
    pub fn new() -> Self {
        Self
    }
    pub fn as_interface(&self) -> WinResult<IDropSource> {
        unsafe { self.cast() }
    }
}
impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(&self,_: BOOL,_: MODIFIERKEYS_FLAGS) ->  HRESULT {
        DRAGDROP_S_DROP
    }

    fn GiveFeedback(&self,_:DROPEFFECT) ->  HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

#[derive(Default)]
#[implement(IDataObject)]
pub struct DropFiles {
    format: Arc<AtomicPtr<Vec<FORMATETC>>>,
    medium: Arc<AtomicPtr<Vec<STGMEDIUM>>>,
}
impl DropFiles {
    pub fn new() -> Self {
        let pfmt = Box::into_raw(Box::new(Vec::new()));
        let pmed = Box::into_raw(Box::new(Vec::new()));
        Self {
            format: Arc::new(AtomicPtr::new(pfmt)),
            medium: Arc::new(AtomicPtr::new(pmed)),
        }
    }
    unsafe fn push_format(&self, format: FORMATETC) -> WinResult<()> {
        unsafe {
            match self.format.load(SeqCst).as_mut(){
                Some(fmts) => {
                    fmts.push(format);
                    Ok(())
                },
                None => Err(E_POINTER.into()),
            }
        }
    }
    unsafe fn push_medium(&self, medium: STGMEDIUM) -> WinResult<()> {
        unsafe {
            match self.medium.load(SeqCst).as_mut() {
                Some(meds) => {
                    meds.push(medium);
                    Ok(())
                },
                None => Err(E_POINTER.into()),
            }
        }
    }
    pub fn as_interface(&self) -> WinResult<IDataObject> {
        unsafe { self.cast() }
    }
}
impl IDataObject_Impl for DropFiles {
    fn GetData(&self,pformatetcin: *const FORMATETC) ->  WinResult<STGMEDIUM> {
        unsafe {
            let format = pformatetcin.as_ref().ok_or(WinError::from(E_INVALIDARG))?;
            let fmts = self.format.load(SeqCst).as_ref()
                .ok_or(WinError::from(E_POINTER))?;
            let meds = self.medium.load(SeqCst).as_ref()
                .ok_or(WinError::from(E_POINTER))?;
            fmts.iter().zip(meds.iter())
                .find_map(|(fmt, med)| {
                    let is_matched = (format.tymed & fmt.tymed) > 0
                        && format.dwAspect.eq(&fmt.dwAspect)
                        && format.cfFormat.eq(&fmt.cfFormat);

                    is_matched.then_some(med.clone())
                })
                .ok_or(DV_E_FORMATETC.into())
        }
    }

    fn GetDataHere(&self,_: *const FORMATETC,_: *mut STGMEDIUM) ->  WinResult<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self,pformatetc: *const FORMATETC) ->  HRESULT {
        unsafe {
            let Some(format) = pformatetc.as_ref() else {return E_INVALIDARG};
            if (DVASPECT_CONTENT.0 & format.dwAspect).gt(&0) {
                let Some(fmts) = self.format.load(SeqCst).as_ref() else {
                    return E_POINTER;
                };
                let flow = fmts.iter()
                    .try_fold(S_OK, |_, fmt| {
                        if (format.tymed & fmt.tymed).ge(&0) {
                            if format.cfFormat.eq(&fmt.cfFormat) {
                                ControlFlow::Break(S_OK)
                            } else {
                                ControlFlow::Continue(DV_E_CLIPFORMAT)
                            }
                        } else {
                            ControlFlow::Continue(DV_E_TYMED)
                        }
                    });
                match flow {
                    ControlFlow::Continue(hres) |
                    ControlFlow::Break(hres) => hres,
                }
            } else {
                DV_E_DVASPECT
            }
        }
    }

    fn GetCanonicalFormatEtc(&self,_: *const FORMATETC,pformatetcout: *mut FORMATETC) ->  HRESULT {
        if pformatetcout.is_null() {
            E_INVALIDARG
        } else {
            DATA_S_SAMEFORMATETC
        }
    }

    fn SetData(&self,pformatetc: *const FORMATETC,pmedium: *const STGMEDIUM,frelease: BOOL) ->  WinResult<()> {
        unsafe {
            let format = *pformatetc.as_ref()
                .ok_or(WinError::from(E_INVALIDARG))?;
            let medium = pmedium.as_ref()
                .ok_or(WinError::from(E_INVALIDARG))?
                .clone();
            if frelease.as_bool() {
                ReleaseStgMedium(pmedium as *mut _);
            }
            self.push_format(format)?;
            self.push_medium(medium)?;
            Ok(())
        }
    }

    fn EnumFormatEtc(&self,dwdirection:u32) ->  WinResult<IEnumFORMATETC> {
        match DATADIR(dwdirection as i32) {
            DATADIR_GET => {
                let efe = EnumFormatEtc::new(Arc::clone(&self.format));
                unsafe { efe.cast() }
            }
            _ => Err(E_NOTIMPL.into())
        }
    }

    fn DAdvise(&self,_: *const FORMATETC,_:u32,_: Option<&IAdviseSink>) ->  WinResult<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self,_:u32) ->  WinResult<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) ->  WinResult<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}


#[implement(IEnumFORMATETC)]
#[derive(Clone)]
pub struct EnumFormatEtc {
    format: Arc<AtomicPtr<Vec<FORMATETC>>>,
    index: Arc<AtomicUsize>,
}
impl EnumFormatEtc {
    pub fn new(format: Arc<AtomicPtr<Vec<FORMATETC>>>) -> Self {
        Self {
            format,
            index: Default::default(),
        }
    }
    /// Nextで返せるアイテムが有るかどうか
    unsafe fn has_remaining_item(&self) -> bool {
        unsafe {
            let Some(fmts) = self.format.load(SeqCst).as_mut() else {
                return false;
            };
            let index = self.index.load(SeqCst);
            fmts.len() > index
        }
    }
    fn increase_index(&self) -> usize {
        self.index.fetch_add(1, SeqCst)
    }
    fn skip_index(&self, celt: u32) -> bool {
        let celt = celt as usize;
        self.index.fetch_add(celt, SeqCst);
        true
    }
    fn reset_index(&self) {
        self.index.store(0, SeqCst);
    }
}
impl IEnumFORMATETC_Impl for EnumFormatEtc {
    fn Next(&self,celt:u32,rgelt: *mut FORMATETC,pceltfetched: *mut u32) ->  WinResult<()> {
        unsafe {
            // インデックスチェック
            let true = self.has_remaining_item() else {
                return Err(S_FALSE.into());
            };
            // 引数チェック
            if ! (celt.eq(&0) || rgelt.is_null()) {
                return Err(S_FALSE.into());
            }
            // celtが1の場合のみpceltfetchedのnullを許容
            if !(pceltfetched.is_null() && celt == 1) {
                return Err(S_FALSE.into());
            }
            // ヌルポなら終了
            let Some(fmts) = self.format.load(SeqCst).as_mut() else {
                return Err(S_FALSE.into())
            };
            for count in 0..celt as isize {
                let index = self.increase_index();
                if let Some(fmt) = fmts.get(index) {
                    let out = rgelt.offset(count);
                    out.replace(*fmt);
                } else {
                    break;
                }
            }
            Ok(())
        }
    }

    fn Skip(&self, celt: u32) -> WinResult<()> {
        self.skip_index(celt).then_some(())
            .ok_or(S_FALSE.into())
    }

    fn Reset(&self) -> WinResult<()> {
        self.reset_index();
        Ok(())
    }

    fn Clone(&self) -> WinResult<IEnumFORMATETC> {
        unsafe {
            self.cast()
        }
    }
}