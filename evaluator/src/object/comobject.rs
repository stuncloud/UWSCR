use windows::{
    core::{self, BSTR, HRESULT, HSTRING, PCWSTR, PWSTR, ComInterface, GUID, Interface, implement, IUnknown},
    Win32::{
        Foundation::{VARIANT_BOOL, DISP_E_MEMBERNOTFOUND, HWND, LPARAM, BOOL},
        UI::{
            WindowsAndMessaging::{
                SetForegroundWindow,
                GetWindowTextW, EnumWindows, EnumChildWindows,
                OBJID_NATIVEOM,
            },
            Accessibility::AccessibleObjectFromWindow,
        },
        System::{
            Com::{
                CLSCTX_ALL, CLSCTX_LOCAL_SERVER,
                IDispatch, IDispatch_Impl, //IDispatch_Vtbl,
                CLSIDFromString, CLSIDFromProgID, CoCreateInstance,
                DISPPARAMS,
                DISPATCH_FLAGS, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPATCH_METHOD,
                EXCEPINFO,
                SAFEARRAY, SAFEARRAYBOUND,
                ITypeInfo, //ELEMDESC,
                IConnectionPoint, IConnectionPointContainer,
            },
            Ole::{
                GetActiveObject,
                DISPID_PROPERTYPUT, DISPID_NEWENUM,
                SafeArrayCreate, SafeArrayPutElement, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayDestroy,
                IEnumVARIANT,
                IDispatchEx,fdexNameCaseInsensitive,
            },
            Variant::{
                VARIANT, VARIANT_0_0,
                VARENUM, VT_ARRAY,VT_BYREF,VT_BOOL,VT_BSTR,VT_CY,VT_DATE,VT_DECIMAL,VT_DISPATCH,VT_EMPTY,VT_I1,VT_I2,VT_I4,VT_INT,VT_NULL,VT_R4,VT_R8,VT_UI1,VT_UI2,VT_UI4,VT_UINT,VT_UNKNOWN,VT_VARIANT,
                // VT_PTR, VT_SAFEARRAY,
                VAR_CHANGE_FLAGS,
                VariantChangeType, VariantClear,
            },
            Wmi::{
                ISWbemObject, ISWbemProperty,
            }
        },
    }
};

use crate::{Object, Evaluator, EvalResult, Function};
use crate::error::{UError, UErrorKind, UErrorMessage};
use parser::ast::{Expression, Identifier};
use util::winapi::WString;

use std::mem::ManuallyDrop;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock};

use num_traits::FromPrimitive;

static IE_CLSID: OnceLock<Option<GUID>> = OnceLock::new();
const LOCALE_SYSTEM_DEFAULT: u32 = 0x0800;
const LOCALE_USER_DEFAULT: u32 = 0x400;
pub type ComResult<T> = Result<T, ComError>;

#[derive(Debug)]
pub enum ComError {
    WindowsError {
        message: String,
        code: i32,
        description: Option<String>
    },
    UError(UError),
    IENotAllowed,
}
impl ComError {
    fn new(err: core::Error, description: Option<String>) -> Self {
        Self::WindowsError {
            message: err.message().to_string(),
            code: err.code().0,
            description,
        }
    }
    fn new_u(kind: UErrorKind, message: UErrorMessage) -> Self {
        Self::UError(UError::new(kind, message))
    }
    fn from_variant_error(vt: VARENUM) -> Self {
        let e = UError::new(UErrorKind::VariantError, UErrorMessage::FromVariant(vt.0));
        Self::UError(e)
    }
    pub fn is_member_not_found(&self) -> bool {
        match self {
            ComError::WindowsError { message: _, code, description: _ } => {
                *code == DISP_E_MEMBERNOTFOUND.0
            },
            ComError::UError(_) |
            ComError::IENotAllowed => false,
        }
    }
    fn as_windows_error(self) -> core::Error {
        match self {
            ComError::WindowsError { message, code, description: _ } => {
                core::Error::new(HRESULT(code), HSTRING::from(message))
            },
            ComError::UError(err) => {
                if let UErrorKind::ComError(n) = err.kind {
                    core::Error::from(HRESULT(n))
                } else {
                    let code = HRESULT(-1);
                    let message = HSTRING::from(err.to_string());
                    core::Error::new(code, message)
                }
            },
            ComError::IENotAllowed => {
                let code = HRESULT(-1);
                let message = HSTRING::from("Internet Explorer not allowed");
                core::Error::new(code, message)
            }
        }
    }
}
impl From<core::Error> for ComError {
    fn from(e: core::Error) -> Self {
        let code = e.code().0;
        let message = e.message().to_string();
        let description = match e.info() {
            Some(info) => unsafe {
                let mut description = BSTR::new();
                let mut error = HRESULT(0);
                let mut restricteddescription = BSTR::new();
                let mut capabilitysid = BSTR::new();
                if info.GetErrorDetails(&mut description, &mut error, &mut restricteddescription, &mut capabilitysid).is_ok() {
                    let details = format!("{error}: {description}, {restricteddescription}, {capabilitysid}");
                    Some(details)
                } else {
                    None
                }
            },
            None => None,
        };
        ComError::WindowsError { message, code, description }
    }
}

impl From<ComError> for UError {
    fn from(e: ComError) -> Self {
        match e {
            ComError::WindowsError { message, code, description } => {
                Self::new_com_error(
                    UErrorKind::ComError(code),
                    UErrorMessage::ComError(message, description)
                )
            },
            ComError::UError(mut e) => {
                e.is_com_error = true;
                e
            },
            ComError::IENotAllowed => {
                Self::new(UErrorKind::ProgIdError, UErrorMessage::InternetExplorerNotAllowed)
            }
        }
    }
}

#[derive(Clone)]
pub struct ComObject {
    idispatch: IDispatch,
    handlers: Arc<Mutex<EventHandlers>>
}
impl std::fmt::Display for ComObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_type_name() {
            Some(type_name) => {
                if self.is_collection() {
                    write!(f, "ComObject({type_name}[])")
                } else {
                    write!(f, "ComObject({type_name})")
                }
            },
            None => {
                // 型名が得られない場合ポインタを表示
                let ptr = self.idispatch.as_raw() as isize;
                #[cfg(target_arch="x86_64")]
                {
                    write!(f, "ComObject(0x{ptr:016X})")
                }
                #[cfg(target_arch="x86")]
                {
                    write!(f, "ComObject(0x{ptr:08X})")
                }
            },
        }
    }
}
impl std::fmt::Debug for ComObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComObject")
            .field("idispatch", &self.idispatch)
            .finish()
    }
}
impl PartialEq for ComObject {
    fn eq(&self, other: &Self) -> bool {
        self.idispatch == other.idispatch
    }
}

impl From<IDispatch> for ComObject {
    fn from(idispatch: IDispatch) -> Self {
        let handlers = Arc::new(Mutex::new(EventHandlers::new()));
        Self { idispatch, handlers }
    }
}

#[allow(unused)]
impl ComObject {
    /// IE未許可かつCLSIDがIEと一致した場合エラーを返す
    fn disallow_ie(clsid: &GUID, allow_ie: bool) -> ComResult<()> {
        if allow_ie {
            Ok(())
        } else {
            let ie_clsid = IE_CLSID.get_or_init(|| {
                unsafe {
                    let lpszprogid = HSTRING::from("InternetExplorer.Application");
                    CLSIDFromProgID(&lpszprogid).ok()
                }
            });
            match ie_clsid {
                Some(ie) => {
                    if ie == clsid {
                        Err(ComError::IENotAllowed)
                    } else {
                        Ok(())
                    }
                },
                None => Ok(()),
            }
        }
    }
    unsafe fn get_clsid(id: &str) -> ComResult<GUID> {
        // {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX} であればGUIDだが厳密な検査はしない
        let clsid = if id.len() == 38 && id.contains('{') && id.contains('-') {
            // GUID
            let lpsz = HSTRING::from(id);
            CLSIDFromString(&lpsz)?
        } else {
            // ProgID
            let lpszprogid = HSTRING::from(id);
            CLSIDFromProgID(&lpszprogid)?
        };
        Ok(clsid)
    }
    pub fn new(id: String, allow_ie: bool) -> ComResult<Self> {
        unsafe {
            let rclsid = Self::get_clsid(&id)?;
            Self::disallow_ie(&rclsid, allow_ie)?;
            let idispatch = Self::create_instance(&rclsid)?;
            let handlers = Arc::new(Mutex::new(EventHandlers::new()));
            let obj = Self {idispatch, handlers};
            Ok(obj)
        }
    }
    /// 以下の順でCoCreateInstanceを試みる
    /// 1. CLSCTX_ALL
    /// 2. CLSCTX_LOCAL_SERVER
    fn create_instance(rclsid: &GUID) -> ComResult<IDispatch> {
        unsafe {
            match CoCreateInstance(rclsid, None, CLSCTX_ALL) {
                Ok(disp) => Ok(disp),
                Err(_) => {
                    let disp = CoCreateInstance(rclsid, None, CLSCTX_LOCAL_SERVER)?;
                    Ok(disp)
                },
            }
        }
    }
    pub fn get_instance(id: String, title: Option<ObjectTitle>, allow_ie: bool) -> ComResult<Option<Self>> {
        unsafe {
            let rclsid = Self::get_clsid(&id)?;
            Self::disallow_ie(&rclsid, allow_ie)?;
            match title {
                Some(title) => {
                    let (title, nth) = title.get();
                    if let Some(mut go) = GetObject::new(title, rclsid) {
                        Ok(go.search(nth))
                    } else {
                        Ok(None)
                    }
                },
                None => {
                    let pvreserved = std::ptr::null_mut() as *mut std::ffi::c_void;
                    let mut ppunk = None;
                    GetActiveObject(&rclsid, None, &mut ppunk)?;
                    let maybe_obj = match ppunk {
                        Some(unk) => {
                            let idispatch = unk.cast::<IDispatch>()?;
                            let handlers = Arc::new(Mutex::new(EventHandlers::new()));
                            let obj = Self {idispatch, handlers};
                            Some(obj)
                        },
                        None => None,
                    };
                    Ok(maybe_obj)
                }
            }
        }
    }
    fn invoke_raw(&self, dispidmember: i32, pdispparams: *const DISPPARAMS, wflags: DISPATCH_FLAGS) -> ComResult<VARIANT> {
        unsafe {
            let riid = GUID::zeroed();
            let lcid = LOCALE_SYSTEM_DEFAULT;
            let mut result = VARIANT::default();
            let mut excepinfo = EXCEPINFO::default();
            self.idispatch.Invoke(dispidmember, &riid, lcid, wflags, pdispparams, Some(&mut result), Some(&mut excepinfo), None)
                .map_err(|err| ComError::new(err, Some(excepinfo.bstrDescription.to_string())))?;
            Ok(result)
        }
    }
    fn invoke(&self, dispidmember: i32, pdispparams: *const DISPPARAMS, wflags: DISPATCH_FLAGS) -> ComResult<Object> {
        let variant = self.invoke_raw(dispidmember, pdispparams, wflags)?;
        variant.try_into()
    }
    fn get_id_from_name(&self, name: &str) -> ComResult<i32> {
        unsafe {
            // メンバのIDを取得
            let riid = GUID::zeroed();
            let hstring = HSTRING::from(name);
            let rgsznames = PCWSTR::from_raw(hstring.as_ptr());
            let cnames = 1;
            let lcid = LOCALE_USER_DEFAULT;
            let mut dispidmember = 0;
            self.idispatch.GetIDsOfNames(&riid, &rgsznames, cnames, lcid, &mut dispidmember)?;
            Ok(dispidmember)
        }
    }
    /// プロパティの値を取得
    ///
    /// obj.prop
    pub fn get_property(&self, prop: &str) -> ComResult<Object> {
        let variant = self.get_raw_property(prop)?;
        variant.try_into()
    }
    pub fn get_raw_property(&self, prop: &str) -> ComResult<VARIANT> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        self.invoke_raw(dispidmember, &mut dp, DISPATCH_PROPERTYGET|DISPATCH_METHOD)
    }
    fn get_property_as_comobject(&self, prop: &str) -> ComResult<Self> {
        let variant = self.get_raw_property(prop)?;
        let disp = variant.to_idispatch()?;
        Ok(Self::from(disp))
    }
    pub fn get_prop_vt(&self, prop: &str) -> ComResult<u16> {
        let variant = self.get_raw_property(prop)?;
        Ok(variant.vt().0)
    }
    /// プロパティへの代入
    ///
    /// obj.prop = value
    pub fn set_property(&self, prop: &str, value: Object) -> ComResult<()> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let new = value.try_into()?;
        let mut args = vec![new];
        dp.cArgs = 1;
        dp.rgvarg = args.as_mut_ptr();
        dp.cNamedArgs = 1;
        let mut named_args = vec![DISPID_PROPERTYPUT];
        dp.rgdispidNamedArgs = named_args.as_mut_ptr();
        self.invoke(dispidmember, &mut dp, DISPATCH_PROPERTYPUT)?;
        Ok(())
    }
    /// インデックス指定でプロパティの値を得る
    /// obj.prop[index]
    pub fn get_property_by_index(&self, prop: &str, index: Vec<Object>) -> ComResult<Object> {
        match self.get_raw_property_by_index(prop, index.clone()) {
            Ok(variant) => variant.try_into(),
            Err(e) => {
                if e.is_member_not_found() {
                    // DISP_E_MEMBERNOTFOUNDの場合は
                    // foo.barがコレクションでfoo.bar[i]でItem(i)を得たい可能性がある
                    // 一旦プロパティとして取得し、それがCOMオブジェクトならItem取得を試みる
                    let com2 = self.get_property_as_comobject(prop)?;
                    com2.get_item_property(index)
                } else {
                    Err(e)
                }
            },
        }
    }
    fn get_raw_property_by_index(&self, prop: &str, index: Vec<Object>) -> ComResult<VARIANT> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let mut args = index.clone().into_iter()
            .map(|o| o.try_into())
            .collect::<ComResult<Vec<_>>>()?;
        args.reverse();
        dp.cArgs = args.len() as u32;
        dp.rgvarg = args.as_mut_ptr();
        self.invoke_raw(dispidmember, &mut dp, DISPATCH_PROPERTYGET|DISPATCH_METHOD)
    }
    fn get_property_by_index_as_comobject(&self, prop: &str, index: Vec<Object>) -> ComResult<Self> {
        let variant = self.get_raw_property_by_index(prop, index)?;
        variant.to_idispatch().map(|disp| Self::from(disp))
    }
    /// インデックス指定でプロパティへ代入
    /// obj.prop[index] = value
    pub fn set_property_by_index(&self, prop: &str, index: Object, value: Object) -> ComResult<()> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let new = value.try_into()?;
        let i = index.try_into()?;
        let mut args = vec![new, i];
        dp.cArgs = 2;
        dp.rgvarg = args.as_mut_ptr();
        dp.cNamedArgs = 1;
        let mut named_args = vec![DISPID_PROPERTYPUT];
        dp.rgdispidNamedArgs = named_args.as_mut_ptr();
        self.invoke(dispidmember, &mut dp, DISPATCH_PROPERTYPUT)?;
        Ok(())
    }
    /// オブジェクト自身にインデックス指定
    /// Itemプロパティの糖衣構文
    /// obj[index]
    pub fn get_by_index(&self, index: Vec<Object>) -> ComResult<Object> {
        self.get_item_property(index)
    }
    /// オブジェクト自身にインデックス指定で代入
    /// Itemプロパティの糖衣構文
    /// obj[index] = value
    pub fn set_by_index(&self, index: Object, value: Object) -> ComResult<()> {
        self.set_property_by_index("Item", index, value)
    }
    pub fn get_item_property(&self, index: Vec<Object>) -> ComResult<Object> {
        let dispidmember = self.get_id_from_name("Item")?;

        let mut dp = DISPPARAMS::default();
        let mut args = index.clone().into_iter()
            .map(|o| o.try_into())
            .collect::<ComResult<Vec<_>>>()?;
        args.reverse();
        dp.cArgs = args.len() as u32;
        dp.rgvarg = args.as_mut_ptr();
        self.invoke(dispidmember, &mut dp, DISPATCH_PROPERTYGET|DISPATCH_METHOD)
    }
    /// メソッドの実行
    /// obj.method(args)
    pub fn invoke_method(&self, method: &str, args: &mut Vec<ComArg>) -> ComResult<Object> {
        let variant = self.invoke_method_raw(method, args)?;
        variant.try_into()
    }
    pub fn invoke_method_raw(&self, method: &str, args: &mut Vec<ComArg>) -> ComResult<VARIANT> {
        if self.is_wmi_object() {
            // ISWbemObject(Ex)であればWMIメソッドとして処理
            let result = self.invoke_wmi_method(method, args);
            if result.is_ok() {
                return result;
            }
            // 失敗時は通常のCOMメソッドとして処理を続行する
        }
        let dispidmember = self.get_id_from_name(method)?;
        let mut dp = DISPPARAMS::default();
        let mut wargs = args.iter()
            .map(|arg| ComArgType::try_from(arg.clone()))
            .collect::<ComResult<Vec<ComArgType>>>()?;

        let info = self.get_type_info()?;
        let pnames = info.get_param_names(dispidmember)?;

        let mut named_flg = false;

        let (ids, mut vargs): (Vec<_>, Vec<_>) = wargs.iter_mut()
            .map(|arg| {
                match arg {
                    ComArgType::Arg(v) => {
                        if named_flg {
                            Err(ComError::UError(UError::new(UErrorKind::ComArgError, UErrorMessage::InvalidComMethodArgOrder)))
                        } else {
                            Ok((None, v.clone()))
                        }
                    },
                    ComArgType::ByRef(v) => {
                        if named_flg {
                            Err(ComError::UError(UError::new(UErrorKind::ComArgError, UErrorMessage::InvalidComMethodArgOrder)))
                        } else {
                            let vv = VARIANT::by_ref(v);
                            Ok((None, vv))
                        }
                    },
                    ComArgType::NamedArg(name, v) => {
                        let id = pnames.get_id_of(&name)?;
                        named_flg = true;
                        Ok((Some(id), v.clone()))
                    },
                }
            })
            .collect::<ComResult<Vec<(Option<i32>, VARIANT)>>>()
            .map(|v| v.into_iter().unzip())?;

        // 引数は逆順にする
        vargs.reverse();

        dp.cArgs = vargs.len() as u32;
        dp.rgvarg = vargs.as_mut_ptr();

        // 名前付き引数
        if ids.iter().any(|name| name.is_some()) {
            let mut named_args = ids.into_iter()
                .filter_map(|maybe_id| maybe_id)
                .collect::<Vec<_>>();
            named_args.reverse();
            if ! named_args.is_empty() {
                dp.cNamedArgs = named_args.len() as u32;
                dp.rgdispidNamedArgs = named_args.as_mut_ptr();
            }
        }
        let variant = self.invoke_raw(dispidmember, &dp, DISPATCH_METHOD|DISPATCH_PROPERTYGET)?;
        vargs.reverse();
        // 参照渡しは値を更新する
        for (arg, varg) in args.iter_mut().zip(vargs.into_iter()) {
            match arg {
                ComArg::ByRef(_, byref) => *byref = varg.try_into()?,
                _ => {}
            }
        }
        Ok(variant)
    }
    /// メソッド引数への変換
    pub fn to_comarg(evaluator: &mut Evaluator, exprs: Vec<Expression>) -> EvalResult<Vec<ComArg>> {
        exprs.into_iter()
            .map(|expr| {
                match expr {
                    Expression::Assign(left, right) => {
                        let arg = evaluator.eval_expression(*right)?;
                        if let Expression::Identifier(Identifier(name)) = *left {
                            Ok(ComArg::NamedArg(name, arg))
                        } else {
                            Ok(ComArg::Arg(arg))
                        }
                    },
                    Expression::RefArg(e) => {
                        let ref_expr = *e.clone();
                        let arg = evaluator.eval_expression(*e)?;
                        Ok(ComArg::ByRef(ref_expr, arg))
                    },
                    e => {
                        let arg = evaluator.eval_expression(e)?;
                        Ok(ComArg::Arg(arg))
                    }
                }
            })
            .collect()
    }

    fn get_type_info(&self) -> ComResult<TypeInfo> {
        TypeInfo::try_from(&self.idispatch)
    }

    fn get_type_name(&self) -> Option<String> {
        let info = TypeInfo::try_from(&self.idispatch).ok()?;
        info.get_type_name()
    }
    pub fn to_object_vec(&self) -> ComResult<Vec<Object>> {
        if let Ok(collection) = ComCollection::try_from(self) {
            // IEnumVARIANTが実装されている場合
            collection.to_object_vec()
        } else {
            // IEnumVARIANTが実装されていない場合CountとItemを試す
            let cnt = self.get_property("Count")
                .map_err(|_| ComError::new_u(UErrorKind::ComCollectionError, UErrorMessage::FailedToConvertToCollection))?;
            let cnt = cnt.as_f64(false).ok_or(ComError::new_u(UErrorKind::ComCollectionError, UErrorMessage::FailedToConvertToCollection))? as u32;
            (0..cnt).into_iter()
                .map(|i| {
                    let index = vec![i.into()];
                    self.get_property_by_index("Item", index)
                        .map_err(|_| ComError::new_u(UErrorKind::ComCollectionError, UErrorMessage::FailedToConvertToCollection))
                })
                .collect()
        }
    }
    fn is_collection(&self) -> bool {
        ComCollection::try_from(self).is_ok()
    }
    fn is_wmi_object(&self) -> bool {
        if let Some(name) = self.get_type_name().as_deref() {
            match name {
                "ISWbemObject" |
                "ISWbemObjectEx" => true,
                _ => false
            }
        } else {
            false
        }
    }
    fn invoke_wmi_method(&self, method: &str, args: &mut Vec<ComArg>) -> ComResult<VARIANT> {
        unsafe {
            let wbemobj = self.idispatch.cast::<ISWbemObject>()?;
            let strname = BSTR::from(method);
            let method = wbemobj.Methods_()?.Item(&strname, 0)?;

            let inparams = method.InParameters()?.SpawnInstance_(0)?;
            let count = inparams.Properties_()?.Count()?;
            let newenum = inparams.Properties_()?._NewEnum()?.cast::<IEnumVARIANT>()?;
            let props = ComCollection {col: newenum};

            let mut wargs = args.iter()
                .map(|arg| ComArgType::try_from(arg.clone()))
                .collect::<ComResult<Vec<ComArgType>>>()?
                .into_iter();

            for _ in (0..count) {
                if let Some(com) = props.next_idispatch()? {
                    let prop = com.cast::<ISWbemProperty>()?;
                    let value = match wargs.next() {
                        Some(w) => match w {
                            ComArgType::NamedArg(_, _) => {
                                return Err(ComError::new_u(UErrorKind::WmiError, UErrorMessage::NamedArgNotAllowed));
                            },
                            ComArgType::Arg(v) |
                            ComArgType::ByRef(v) => v,
                        },
                        None => {
                            return Err(ComError::new_u(UErrorKind::WmiError, UErrorMessage::MissingArgument));
                        },
                    };
                    prop.SetValue(&value)?;
                }
            }

            let outparam = wbemobj.ExecMethod_(&strname, &inparams, 0, None)?;
            let outparamenum = ComCollection{ col: outparam.Properties_()?._NewEnum()?.cast::<IEnumVARIANT>()? };
            let retrun_value = match outparamenum.next()? {
                Some(variant) => {
                    let prop = variant.to_t::<ISWbemProperty>()?;
                    prop.Value().map_err(|e| e.into())
                },
                None => Ok(VARIANT::default()),
            };
            for arg in args {
                if let ComArg::ByRef(_, byref) = arg {
                    let out_value = match outparamenum.next()? {
                        Some(variant) => {
                            let prop = variant.to_t::<ISWbemProperty>()?;
                            prop.Value()?.try_into()?
                        },
                        None => Object::Empty
                    };
                    *byref = out_value;
                }
            }
            retrun_value
        }
    }
    fn _invoke_ex(&self, name: &str, pdp: *const DISPPARAMS, wflags: DISPATCH_FLAGS) -> ComResult<VARIANT> {
        unsafe {
            let dispex = self.idispatch.cast::<IDispatchEx>()?;
            let bstrname = BSTR::from(name);
            let id = dispex.GetDispID(&bstrname, fdexNameCaseInsensitive as u32)?;
            let lcid = LOCALE_SYSTEM_DEFAULT;
            let mut result = VARIANT::default();
            let mut excepinfo = EXCEPINFO::default();
            dispex.InvokeEx(id, lcid, wflags.0, pdp, Some(&mut result), Some(&mut excepinfo), None)
                .map_err(|err| ComError::new(err, Some(excepinfo.bstrDescription.to_string())))?;
            Ok(result)
        }
    }
    fn cast<T: ComInterface>(&self) -> ComResult<T> {
        let t = self.idispatch.cast::<T>()?;
        Ok(t)
    }
    pub fn set_event_handler(&mut self, interface: &str, event_name: &str, func: Function, evaluator: Evaluator) -> ComResult<()> {
        let type_info = self.get_type_info()?;
        let info = type_info.get_event_interface_type_info(interface)?;
        let riid = info.get_riid()?;
        let memid = info.get_ids_of_names(event_name)?;
        let event = EventDisp::new(func, memid, evaluator);

        let container = self.cast()?;
        let handler = EventHandler::new(event, container, &riid)?;
        let mut handlers = self.handlers.lock().unwrap();
        handlers.set(handler);
        Ok(())
    }
    pub fn remove_event_handler(&mut self) -> ComResult<()> {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.remove()
    }
}

#[derive(Debug, Clone)]
pub enum ComArg {
    Arg(Object),
    ByRef(Expression, Object),
    NamedArg(String, Object),
}
impl Into<Object> for ComArg {
    fn into(self) -> Object {
        match self {
            ComArg::Arg(o) |
            ComArg::ByRef(_, o) |
            ComArg::NamedArg(_, o) => o,
        }
    }
}
enum ComArgType {
    Arg(VARIANT),
    ByRef(VARIANT),
    NamedArg(String, VARIANT),
}
impl TryFrom<ComArg> for ComArgType {
    type Error = ComError;

    fn try_from(arg: ComArg) -> Result<Self, Self::Error> {
        match arg {
            ComArg::Arg(obj) => {
                let variant = obj.try_into()?;
                Ok(Self::Arg(variant))
            },
            ComArg::ByRef(_, obj) => {
                let variant = obj.try_into()?;
                Ok(Self::ByRef(variant))
            },
            ComArg::NamedArg(name, obj) => {
                let variant = obj.try_into()?;
                Ok(Self::NamedArg(name, variant))
            },
        }
    }
}

impl TryFrom<Object> for VARIANT {
    type Error = ComError;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        let variant = match obj {
            Object::Num(n) => {
                if n.fract() == 0.0 {
                    match i32::from_f64(n) {
                        Some(i) => VARIANT::from_i32(i),
                        None => VARIANT::from_f64(n),
                    }
                } else {
                    VARIANT::from_f64(n)
                }
            },
            Object::String(s) => VARIANT::from_string(s),
            Object::Bool(b) => VARIANT::from_bool(b),
            Object::Null => VARIANT::null(),
            Object::EmptyParam => VARIANT::null(),
            Object::Empty => VARIANT::default(),
            Object::ComObject(disp) => VARIANT::from_idispatch(disp.idispatch),
            Object::Unknown(unk) => VARIANT::from_iunknown(unk.0),
            Object::Variant(variant) => variant.get(),
            Object::Array(_) => {
                let sa = SafeArray::try_from(obj)?;
                sa.as_variant()
            }
            o => {
                let t = o.get_type().to_string();
                let e = UError::new(UErrorKind::VariantError, UErrorMessage::ToVariant(t));
                Err(ComError::UError(e))?
            }
        };
        Ok(variant)
    }
}

impl TryInto<Object> for VARIANT {
    type Error = ComError;

    fn try_into(self) -> Result<Object, Self::Error> {
        unsafe {
            let v00 = &self.Anonymous.Anonymous;
            let vt = v00.vt;
            let is_array = vt.0 & VT_ARRAY.0 == VT_ARRAY.0;
            let is_ref = vt.0 & VT_BYREF.0 == VT_BYREF.0;
            if is_array {
                if is_ref {
                    v00.Anonymous.pparray.try_into()
                } else {
                    v00.Anonymous.parray.try_into()
                }
            } else {
                let vt = if is_ref {
                    VARENUM(vt.0 ^ VT_BYREF.0)
                } else {
                    vt
                };
                match vt {
                    VT_BOOL => if is_ref {
                        match v00.Anonymous.pboolVal.as_mut() {
                            Some(b) => Ok(b.as_bool().into()),
                            None => Err(ComError::from_variant_error(vt)),
                        }
                    } else {
                        let b = v00.Anonymous.boolVal;
                        Ok(b.as_bool().into())
                    },
                    VT_BSTR => if is_ref {
                        match v00.Anonymous.pbstrVal.as_mut() {
                            Some(bstr) => Ok(bstr.to_string().into()),
                            None => Err(ComError::from_variant_error(vt)),
                        }
                    } else {
                        Ok(v00.Anonymous.bstrVal.to_string().into())
                    },
                    VT_DATE => {
                        let variant = self.change_type(VT_BSTR)?;
                        variant.try_into()
                    },
                    // 数値系
                    VT_CY | // 通貨
                    VT_DECIMAL |
                    VT_I1 |
                    VT_I2 |
                    VT_I4 |
                    VT_INT |
                    VT_UI1 |
                    VT_UI2 |
                    VT_UI4 |
                    VT_UINT |
                    VT_R4 => {
                        let variant = self.change_type(VT_R8)?;
                        variant.try_into()
                    },
                    VT_R8 => if is_ref {
                        let r8 = v00.Anonymous.pdblVal;
                        Ok((*r8).into())
                    } else {
                        Ok(v00.Anonymous.dblVal.into())
                    },
                    VT_DISPATCH => if is_ref {
                        match v00.Anonymous.ppdispVal.as_mut() {
                            Some(maybe_disp) => match maybe_disp {
                                Some(disp) => {
                                    let obj = ComObject::from(disp.clone());
                                    Ok(Object::ComObject(obj))
                                },
                                None => Ok(Object::Nothing),
                            },
                            None => Ok(Object::Nothing),
                        }
                    } else {
                        match &*v00.Anonymous.pdispVal {
                            Some(disp) => {
                                let obj = ComObject::from(disp.clone());
                                Ok(Object::ComObject(obj))
                            },
                            None => Ok(Object::Nothing),
                        }
                    },
                    VT_EMPTY => Ok(Object::Empty),
                    VT_NULL => Ok(Object::Null),
                    VT_UNKNOWN => if is_ref {
                        match v00.Anonymous.ppunkVal.as_mut() {
                            Some(maybe_unk) => match maybe_unk {
                                Some(unk) => {
                                    let unk = unk.clone();
                                    Ok(Object::Unknown(unk.into()))
                                },
                                None => Ok(Object::Nothing),
                            },
                            None => Ok(Object::Nothing),
                        }
                    } else {
                        match &*v00.Anonymous.punkVal {
                            Some(unk) => {
                                let unk = unk.clone();
                                Ok(Object::Unknown(unk.into()))
                            },
                            None => Err(ComError::from_variant_error(vt)),
                        }
                    },
                    VT_VARIANT => {
                        match v00.Anonymous.pvarVal.as_mut() {
                            Some(variant) => variant.clone().try_into(),
                            None => Err(ComError::from_variant_error(vt)),
                        }
                    },
                    _ => Ok(Object::Variant(self.into()))
                }
            }
        }
    }
}

impl TryInto<Object> for *mut SAFEARRAY {
    type Error = ComError;

    fn try_into(self) -> Result<Object, Self::Error> {
        unsafe {
            let lbound = SafeArrayGetLBound(self, 1)?;
            let ubound = SafeArrayGetUBound(self, 1)?;
            let size = ubound - lbound + 1;
            let arr = (0..size).into_iter()
                .map(|rgindices| {
                    let mut variant = VARIANT::default();
                    let pv = &mut variant as *mut _ as *mut c_void;
                    SafeArrayGetElement(self, &rgindices, pv)?;
                    variant.try_into()
                })
                .collect::<ComResult<Vec<Object>>>()?;
            Ok(Object::Array(arr))
        }
    }
}
impl TryInto<Object> for *mut *mut SAFEARRAY {
    type Error = ComError;

    fn try_into(self) -> Result<Object, Self::Error> {
        unsafe {
            let psa = *self;
            psa.try_into()
        }
    }
}

pub trait VariantExt {
    fn null() -> VARIANT;
    /// 他のVARIANTの参照を持つVARIANT型を新たに作る
    fn by_ref(var_val: *mut VARIANT) -> VARIANT;
    fn from_f64(n: f64) -> VARIANT;
    fn from_i32(n: i32) -> VARIANT;
    fn from_string(s: String) -> VARIANT;
    fn from_bool(b: bool) -> VARIANT;
    fn from_idispatch(disp: IDispatch) -> VARIANT;
    fn from_iunknown(unk: IUnknown) -> VARIANT;
    fn from_safearray(psa: *mut SAFEARRAY) -> VARIANT;
    fn vt(&self) -> VARENUM;
    fn to_idispatch(&self) -> ComResult<IDispatch>;
    fn to_t<T: ComInterface>(&self) -> ComResult<T>;
    fn to_i32(&self) -> ComResult<i32>;
    fn to_string(&self) -> ComResult<String>;
    fn to_bool(&self) -> ComResult<bool>;
    fn change_type(&self, vt: VARENUM) -> ComResult<VARIANT>;
}

impl VariantExt for VARIANT {
    fn null() -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_NULL;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn by_ref(var_val: *mut VARIANT) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VARENUM(VT_VARIANT.0|VT_BYREF.0);
        v00.Anonymous.pvarVal = var_val;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_f64(n: f64) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_R8;
        v00.Anonymous.dblVal = n;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_i32(n: i32) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_I4;
        v00.Anonymous.intVal = n;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_string(s: String) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_BSTR;
        let bstr = BSTR::from(s);
        v00.Anonymous.bstrVal = ManuallyDrop::new(bstr);
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_bool(b: bool) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_BOOL;
        v00.Anonymous.boolVal = VARIANT_BOOL::from(b);
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_idispatch(disp: IDispatch) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_DISPATCH;
        v00.Anonymous.pdispVal = ManuallyDrop::new(Some(disp));
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_iunknown(unk: IUnknown) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_UNKNOWN;
        v00.Anonymous.punkVal = ManuallyDrop::new(Some(unk));
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn from_safearray(psa: *mut SAFEARRAY) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VARENUM(VT_ARRAY.0|VT_VARIANT.0);
        v00.Anonymous.parray = psa;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn vt(&self) -> VARENUM {
        unsafe {
            let v00 = &self.Anonymous.Anonymous;
            v00.vt
        }
    }
    fn to_idispatch(&self) -> ComResult<IDispatch> {
        unsafe {
            let v00 = &self.Anonymous.Anonymous;
            let maybe_disp = if v00.vt == VT_DISPATCH {
                (*v00.Anonymous.pdispVal).clone()
            } else {
                None
            };
            maybe_disp.ok_or(ComError::new_u(
                UErrorKind::VariantError,
                UErrorMessage::VariantIsNotIDispatch
            ))
        }
    }
    fn to_t<T: ComInterface>(&self) -> ComResult<T> {
        let disp = self.to_idispatch()?;
        let t = disp.cast::<T>()?;
        Ok(t)
    }
    fn to_i32(&self) -> ComResult<i32> {
        unsafe {
            let mut new = self.change_type(VT_I4)?;
            let v00 = &new.Anonymous.Anonymous;
            let n = v00.Anonymous.lVal;
            VariantClear(&mut new)?;
            Ok(n)
        }
    }
    fn to_string(&self) -> ComResult<String> {
        unsafe {
            let mut new = self.change_type(VT_BSTR)?;
            let v00 = &new.Anonymous.Anonymous;
            let str = v00.Anonymous.bstrVal.to_string();
            VariantClear(&mut new)?;
            Ok(str)
        }
    }
    fn to_bool(&self) -> ComResult<bool> {
        unsafe {
            let mut new = self.change_type(VT_BOOL)?;
            let v00 = &new.Anonymous.Anonymous;
            let b = v00.Anonymous.boolVal.as_bool();
            VariantClear(&mut new)?;
            Ok(b)
        }
    }
    fn change_type(&self, vt: VARENUM) -> ComResult<VARIANT> {
        unsafe {
            let mut new = VARIANT::default();
            VariantChangeType(&mut new, self, VAR_CHANGE_FLAGS(0), vt)?;
            Ok(new)
        }
    }
}


impl Object {
    // 二次元までの配列サイズを得る、配列でない場合はNone
    pub fn get_array_size(&self) -> Option<(usize, Option<usize>)> {
        if let Object::Array(arr) = self {
            let size = arr.len();
            let size2 = arr.iter()
                .filter_map(|obj| obj.get_array_size())
                .map(|(s, _)| s)
                .reduce(|a, b| a.max(b));
            Some((size, size2))
        } else {
            None
        }
    }
    fn flatten(self, index: Vec<i32>) -> Option<Vec<(Object, Vec<i32>)>>{
        if let Object::Array(arr) = self {
            let is_all_array = arr.iter().all(|o| if let Object::Array(_) = o {true} else {false});
            let is_all_value = arr.iter().all(|o| if let Object::Array(_) = o {false} else {true});
            if is_all_array {
                let mut arr2 = vec![];
                let mut i = 0;
                for obj in arr {
                    let mut index2 = index.clone();
                    index2.push(i);
                    let v = obj.flatten(index2)?;
                    arr2.extend(v);
                    i += 1;
                }
                Some(arr2)
            } else if is_all_value {
                let mut arr2 = vec![];
                let mut i = 0;
                for obj in arr {
                    let mut index2 = index.clone();
                    index2.push(i);
                    arr2.push((obj, index2));
                    i += 1;
                }
                Some(arr2)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
struct ParamName {
    name: String,
    vt: Option<VARENUM>
}
// impl ParamName {
//     fn is_vt_variant(&self) -> bool {
//         self.vt == Some(VT_VARIANT)
//     }
// }
#[derive(Debug)]
struct ParamNames {
    names: Vec<ParamName>
}
impl ParamNames {
    fn new<S: std::fmt::Display>(names: Vec<S>, vts: Option<Vec<VARENUM>>) -> Self {
        let names = names.into_iter()
            .map(|s| s.to_string().to_ascii_uppercase());
        let names = match vts {
            Some(vts) => {
                names.zip(vts.into_iter())
                    .map(|(name, vt)| ParamName {name, vt: Some(vt)})
                    .collect()
            },
            None => {
                names.map(|name| ParamName {name, vt: None})
                    .collect()
            },
        };
        Self {names}
    }
    fn get_id_of(&self, name: &str) -> ComResult<i32> {
        let upper = name.to_ascii_uppercase();
        let id = self.names.iter()
            .position(|p| p.name == upper)
            .map(|i| i as i32)
            .ok_or(ComError::new_u(UErrorKind::ComArgError, UErrorMessage::NamedArgNotFound(name.to_string())))?;
        Ok(id)
    }
}

struct TypeInfo {
    info: ITypeInfo,
    // _cnt: u32
}
impl TypeInfo {
    fn get_param_names(&self, memid: i32) -> ComResult<ParamNames> {
        unsafe {
            // let vts = self.get_param_vts(memid)?;
            let mut rgbstrnames = vec![BSTR::new(); 100];
            let mut pcnames = 0;
            self.info.GetNames(memid, &mut rgbstrnames, &mut pcnames)?;
            rgbstrnames.resize(pcnames as usize, Default::default());
            // 名前の1つ目は関数名なので除外
            let names = rgbstrnames.drain(1..).collect();
            // Ok(ParamNames::new(names, vts))
            Ok(ParamNames::new(names, None))
        }
    }
    // fn get_func_count(&self) -> ComResult<u32> {
    //     unsafe {
    //         let ptypeattr = self.info.GetTypeAttr()?;
    //         let count = (*ptypeattr).cFuncs as u32;
    //         self.info.ReleaseTypeAttr(ptypeattr);
    //         Ok(count)
    //     }
    // }
    // fn get_param_vts(&self, memid: i32) -> ComResult<Option<Vec<VARENUM>>> {
    //     unsafe {
    //         let count = self.get_func_count()?;
    //         for index in 0..count {
    //             let desc = self.info.GetFuncDesc(index)?;
    //             if (*desc).memid == memid {
    //                 println!("\u{001b}[33m[debug] index: {index:?}\u{001b}[0m");
    //                 // let desc = *desc;
    //                 let len = (*desc).cParams as usize;
    //                 let ptr = (*desc).lprgelemdescParam;
    //                 println!("\u{001b}[35m[debug] {len}: {ptr:?}\u{001b}[0m");
    //                 let elems = Vec::from_raw_parts(ptr, len, len);
    //                 let vts = elems.into_iter()
    //                     .map(|elem| Self::vt_from_elemdesc(elem))
    //                     .collect();
    //                 self.info.ReleaseFuncDesc(desc);
    //                 return Ok(Some(vts));
    //             } else {
    //                 self.info.ReleaseFuncDesc(desc);
    //             }
    //         }
    //         Ok(None)
    //     }
    // }
    // fn vt_from_elemdesc(elem: ELEMDESC) -> VARENUM {
    //     unsafe {
    //         match elem.tdesc.vt {
    //             VT_PTR => {
    //                 let desc = elem.tdesc.Anonymous.lptdesc;
    //                 (*desc).vt
    //             },
    //             VT_SAFEARRAY => {
    //                 let desc = elem.tdesc.Anonymous.lptdesc;
    //                 (*desc).vt
    //             },
    //             vt => vt
    //         }
    //     }
    // }
    fn get_type_name(&self) -> Option<String> {
        unsafe {
            let mut pbstrname = BSTR::new();
            self.info.GetDocumentation(-1, Some(&mut pbstrname), None, &mut 0, None).ok()?;
            let name = pbstrname.to_string();
            Some(name)
        }
    }
    fn get_event_interface_type_info(&self, interface: &str) -> ComResult<Self> {
        unsafe {
            let mut pptlib = None;
            self.info.GetContainingTypeLib(&mut pptlib, &mut 0)?;
            let lib = pptlib
                .ok_or(ComError::new_u(UErrorKind::ComEventError, UErrorMessage::EventInterfaceNotFound))?;

            // let hstring = HSTRING::from(interface);
            // let ptr = hstring.as_ptr() as *mut _;
            let mut wide = interface.to_wide_null_terminated();
            let sznamebuf = PWSTR::from_raw(wide.as_mut_ptr());
            let mut pptinfo = None;
            lib.FindName(sznamebuf, 0, &mut pptinfo, &mut 0, &mut 1)?;
            let info = pptinfo
                .ok_or(ComError::new_u(UErrorKind::ComEventError, UErrorMessage::EventInterfaceNotFound))?;
            Ok(Self {info})
        }
    }
    fn get_riid(&self) -> ComResult<GUID> {
        unsafe {
            let attr = self.info.GetTypeAttr()?;
            let riid = (*attr).guid;
            self.info.ReleaseTypeAttr(attr);
            Ok(riid)
        }
    }
    fn get_ids_of_names(&self, name: &str) -> ComResult<i32> {
        unsafe {
            let hstring = HSTRING::from(name);
            let rgsznames = PCWSTR::from_raw(hstring.as_ptr());
            let mut pmemid = 0;
            self.info.GetIDsOfNames(&rgsznames, 1, &mut pmemid)?;
            Ok(pmemid)
        }
    }
}

impl TryFrom<&IDispatch> for TypeInfo {
    type Error = ComError;

    fn try_from(disp: &IDispatch) -> Result<Self, Self::Error> {
        unsafe {
            // let _cnt = disp.GetTypeInfoCount()?;
            let info = disp.GetTypeInfo(0, 0)?;
            Ok(Self { info })
        }
    }
}

struct ComCollection {
    col: IEnumVARIANT
}
impl TryFrom<&ComObject> for ComCollection {
    type Error = ComError;

    fn try_from(com: &ComObject) -> Result<Self, Self::Error> {
        let mut pdispparams = DISPPARAMS::default();
        let v = com.invoke_raw(DISPID_NEWENUM, &mut pdispparams, DISPATCH_PROPERTYGET|DISPATCH_METHOD)?;
        unsafe {
            let v00 = &v.Anonymous.Anonymous;
            if let Some(unk) = &*v00.Anonymous.punkVal {
                let col = unk.cast()?;
                Ok(Self {col})
            } else {
                Err(ComError::new_u(UErrorKind::ComCollectionError, UErrorMessage::FailedToConvertToCollection))
            }
        }
    }
}
impl ComCollection {
    fn _skip(&self, celt: u32) -> core::Result<()> {
        unsafe {
            let hr = self.col.Skip(celt);
            if hr.is_ok() {
                Ok(())
            } else {
                Err(hr.into())
            }
        }
    }
    fn next(&self) -> core::Result<Option<VARIANT>> {
        unsafe {
            let mut rgvar = [VARIANT::default()];
            let mut pceltfetched = 0;
            let hr = self.col.Next(&mut rgvar, &mut pceltfetched);
            if hr.is_ok() {
                if pceltfetched > 0 {
                    let variant = rgvar[0].to_owned();
                    Ok(Some(variant))
                } else {
                    Ok(None)
                }
            } else {
                Err(hr.into())
            }
        }
    }
    fn next_idispatch(&self) -> ComResult<Option<ComObject>> {
        match self.next()? {
            Some(variant) => {
                variant.to_idispatch()
                    .map(|idispatch| Some(ComObject::from(idispatch)))
            },
            None => Ok(None),
        }
    }
    fn reset(&self) -> core::Result<()> {
        unsafe {
            self.col.Reset()
        }
    }
    fn _get(&self, index: u32) -> ComResult<Object> {
        self._skip(index)?;
        let maybe_variant = self.next()?;
        self.reset()?;
        match maybe_variant {
            Some(variant) => variant.try_into(),
            None => {
                let err = UError::new(UErrorKind::ComCollectionError, UErrorMessage::IndexOutOfBounds(index.into()));
                Err(ComError::UError(err))
            },
        }
    }
    fn to_comobject_vec(&self) -> ComResult<Vec<ComObject>> {
        let mut vec = vec![];
        loop {
            if let Some(variant) = self.next()? {
                let disp = variant.to_idispatch()?;
                vec.push(ComObject::from(disp));
            } else {
                break;
            }
        }
        self.reset()?;
        Ok(vec)
    }
    fn to_object_vec(&self) -> ComResult<Vec<Object>> {
        let mut vec = vec![];
        loop {
            if let Some(variant) = self.next()? {
                let obj = variant.try_into()?;
                vec.push(obj);
            } else {
                break;
            }
        }
        self.reset()?;
        Ok(vec)
    }
}

impl TryFrom<Option<VARIANT>> for Object {
    type Error = ComError;

    fn try_from(value: Option<VARIANT>) -> Result<Self, Self::Error> {
        match value {
            Some(variant) => variant.try_into(),
            None => Ok(Object::Empty),
        }
    }
}

pub struct EventHandlers {
    handlers: Vec<EventHandler>
}
impl EventHandlers {
    fn new() -> Self {
        Self {handlers: Vec::new()}
    }
    fn set(&mut self, handler: EventHandler) {
        self.handlers.push(handler);
    }
    fn remove(&mut self) -> ComResult<()>{
        for handler in &self.handlers {
            handler.unset()?;
        }
        self.handlers.clear();
        Ok(())
    }
}

pub struct EventHandler {
    _event: EventDisp,
    cp: IConnectionPoint,
    cookie: u32,
}
impl EventHandler {
    fn new(_event: EventDisp, container: IConnectionPointContainer, riid: &GUID) -> ComResult<Self> {
        unsafe {
            let disp = _event.cast::<IDispatch>()?;

            let cp = container.FindConnectionPoint(riid)?;
            let cookie = cp.Advise(&disp)?;
            Ok(Self {_event, cp, cookie})
        }
    }
    fn unset(&self) -> ComResult<()> {
        unsafe {
            self.cp.Unadvise(self.cookie)?;
            Ok(())
        }
    }
}

#[implement(IDispatch)]
pub struct EventDisp {
    evaluator: Evaluator,
    func: Function,
    memid: i32,
}

impl EventDisp {
    fn new(func: Function, memid: i32, evaluator: Evaluator) -> Self {
        Self { evaluator, func, memid }
    }
}
#[allow(non_snake_case)]
impl IDispatch_Impl for EventDisp {
    fn GetTypeInfoCount(&self) ->  ::windows::core::Result<u32> {
        unimplemented!()
    }

    fn GetTypeInfo(&self,_itinfo:u32,_lcid:u32) ->  ::windows::core::Result<ITypeInfo> {
        unimplemented!()
    }

    fn GetIDsOfNames(&self,_riid: *const ::windows::core::GUID,_rgsznames: *const ::windows::core::PCWSTR,_cnames:u32,_lcid:u32,_rgdispid: *mut i32) ->  ::windows::core::Result<()> {
        unimplemented!()
    }

    fn Invoke(&self,dispidmember:i32,_riid: *const ::windows::core::GUID,_lcid:u32,_wflags:DISPATCH_FLAGS,pdispparams: *const DISPPARAMS,pvarresult: *mut VARIANT,_pexcepinfo: *mut EXCEPINFO,_puargerr: *mut u32) ->  ::windows::core::Result<()> {
        unsafe {
            if self.memid == dispidmember {
                // DISPPARAMSをObject配列に変換
                let dp = &*pdispparams;
                let args = dp.as_object_vec().map_err(|e| e.as_windows_error())?;
                // DISPPARAMSの値をEVENT_PRMをセット
                let new = Object::Array(args.clone());
                let mut evaluator = self.evaluator.clone();
                evaluator.assign_identifier("EVENT_PRM", new)
                    .map_err(|e| ComError::UError(e).as_windows_error())?;
                // 関数の引数にもセット
                let arguments = args.into_iter()
                    .map(|o| (None, o))
                    .collect();
                // イベントハンドラ関数を実行
                let obj = self.func.invoke(&mut evaluator, arguments, None)
                    .map_err(|e| ComError::UError(e).as_windows_error())?;
                // 関数の戻り値をpvarresultに渡す
                *pvarresult = VARIANT::try_from(obj).map_err(|e| e.as_windows_error())?;
            } else {
                *pvarresult = VARIANT::from_bool(false);
            }
            Ok(())
        }
    }
}
// unsafe impl Interface for EventDisp {
//     type Vtable = IDispatch_Vtbl;
// }
// impl Clone for EventDisp {
//     fn clone(&self) -> Self {
//         Self { evaluator: self.evaluator.clone(), func: self.func.clone(), memid: self.memid.clone() }
//     }
// }

trait DispParamsExt {
    fn as_object_vec(&self) -> ComResult<Vec<Object>>;
}

impl DispParamsExt for DISPPARAMS {
    fn as_object_vec(&self) -> ComResult<Vec<Object>> {
        unsafe {
            let len = self.cArgs as usize;
            let ptr = self.rgvarg;
            let variants = Vec::from_raw_parts(ptr, len, len);
            variants.into_iter()
                .map(|v| v.try_into())
                .collect()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unknown(IUnknown);

impl std::fmt::Display for Unknown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self.0.as_raw() as isize;
        #[cfg(target_arch="x86_64")]
        {
            write!(f, "IUnknown(0x{ptr:016X})")
        }
        #[cfg(target_arch="x86")]
        {
            write!(f, "IUnknown(0x{ptr:08X})")
        }
    }
}
impl From<IUnknown> for Unknown {
    fn from(unk: IUnknown) -> Self {
        Self(unk)
    }
}
pub struct ObjectTitle {
    title: String,
    nth: u32,
}
impl ObjectTitle {
    pub fn new(title: String, nth: u32) -> Self {
        Self {title, nth}
    }
    fn get(self) -> (String, u32) {
        (self.title, self.nth)
    }
}
static CLSIDS: OnceLock<Clsids> = OnceLock::new();
struct Clsids {
    excel: Option<GUID>,
    word: Option<GUID>,
    access: Option<GUID>,
}
impl Clsids {
    fn get_type(&self, clsid: GUID) -> GetObjectType {
        if Some(clsid) == self.excel {
            GetObjectType::Excel
        } else if Some(clsid) == self.word {
            GetObjectType::Word
        } else if Some(clsid) == self.access {
            GetObjectType::Access
        } else {
            GetObjectType::Other
        }
    }
}
#[derive(PartialEq)]
enum GetObjectType {
    Excel,
    Word,
    Access,
    Other,
}
impl From<GUID> for GetObjectType {
    fn from(clsid: GUID) -> Self {
        let ids = CLSIDS.get_or_init(|| {
            unsafe {
                let lpsz = HSTRING::from("Excel.Application");
                let excel = CLSIDFromString(&lpsz).ok();
                let lpsz = HSTRING::from("Word.Application");
                let word = CLSIDFromString(&lpsz).ok();
                let lpsz = HSTRING::from("Access.Application");
                let access = CLSIDFromString(&lpsz).ok();
                Clsids { excel, word, access }
            }
        });
        ids.get_type(clsid)
    }
}
impl GetObjectType {
    fn compare(&self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        match self {
            GetObjectType::Excel => name.contains("excel"),
            GetObjectType::Word => name.contains("word"),
            GetObjectType::Access => name.contains("access"),
            GetObjectType::Other => false,
        }
    }
}
struct GetObject {
    hwnds: Vec<HWND>,
    title: String,
    r#type: GetObjectType
}
impl GetObject {
    fn new(title: String, clsid: GUID) -> Option<Self> {
        let title = title.to_ascii_lowercase();
        let r#type = GetObjectType::from(clsid);
        if r#type == GetObjectType::Other {
            None
        } else {
            Some(Self {hwnds: vec![], title, r#type})
        }
    }
    fn search(&mut self, mut nth: u32) -> Option<ComObject> {
        unsafe {
            let lparam = self as *mut Self as isize;
            let _ = EnumWindows(Some(Self::callback), LPARAM(lparam));
            for hwnd in &self.hwnds {
                let mut ec = EnumChildren::new(*hwnd);
                ec.run();
                for child in &ec.children {
                    let mut pvobject = std::ptr::null_mut() as *mut c_void;
                    if AccessibleObjectFromWindow(*child, OBJID_NATIVEOM.0 as u32, &IDispatch::IID, &mut pvobject).is_ok() {
                        let disp = IDispatch::from_raw(pvobject);
                        let com = ComObject::from(disp);
                        let name = com.get_type_name().unwrap_or_default();
                        match name.as_str() {
                            "Window" => {
                                if let Ok(app) = com.get_property_as_comobject("Application") {
                                    if let Ok(variant) = app.get_raw_property("Name") {
                                        let name = variant.to_string().unwrap_or_default();
                                        if self.r#type.compare(&name) {
                                            nth -= 1;
                                            if nth == 0 {
                                                return Some(app);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            None
        }
    }
    fn compare(&self, title: &String) -> bool {
        let title = title.to_ascii_lowercase();
        title.contains(&self.title)
    }
    fn push(&mut self, hwnd: HWND) {
        self.hwnds.push(hwnd);
    }
    unsafe extern "system"
    fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let go = lparam.0 as *mut Self;

        let mut buf = [0; 512];
        let len = GetWindowTextW(hwnd, &mut buf) as usize;
        let title = String::from_utf16_lossy(&buf[..len]);
        if (*go).compare(&title) {
            (*go).push(hwnd);
        }
        true.into()
    }
}
struct EnumChildren {
    parent: HWND,
    children: Vec<HWND>
}
impl EnumChildren {
    fn new(parent: HWND) -> Self {
        Self {parent, children: Vec::new()}
    }
    fn push(&mut self, hwnd: HWND) {
        self.children.push(hwnd);
    }
    fn run(&mut self) {
        unsafe {
            let lparam = self as *mut Self as isize;
            EnumChildWindows(self.parent, Some(Self::callback), LPARAM(lparam));
        }
    }
    unsafe extern "system"
    fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let vec = lparam.0 as *mut Self;
        (*vec).push(hwnd);
        true.into()
    }
}

/* Excel */
use num_derive::FromPrimitive;

#[derive(FromPrimitive, Default)]
pub enum ExcelOpenFlag {
    /// Excelが存在すればそれを使いなければ新規
    #[default]
    Default    = 0,
    /// 常に新規
    New        = 1,
    /// Workbookを返す、起動はDefaultと同じ条件
    Book       = 2,
    /// 互換製品を起動、未対応
    ThirdParty = 3,
}
enum ObjectType {
    Application,
    Workbook,
    Other,
}
pub struct Excel {
    obj: ComObject,
    hwnd: HWND,
    r#type: ObjectType
}

impl Excel {
    const EXCEL_PROGID: &str = "Excel.Application";
    fn create(file: Option<String>, params: Vec<String>) -> ComResult<Self> {
        let obj = ComObject::new(Self::EXCEL_PROGID.into(), false)?;
        let e = Self::new(obj)?;
        e.open_book(file, params)?;
        e.show()?;
        Ok(e)

    }
    fn get_or_create(file: Option<String>, params: Vec<String>) -> ComResult<Self> {
        let obj = match ComObject::get_instance(Self::EXCEL_PROGID.into(), None, false).ok().flatten() {
            Some(obj) => obj,
            None => ComObject::new(Self::EXCEL_PROGID.into(), false)?,
        };
        let e = Self::new(obj)?;
        e.show()?;
        e.open_book(file, params)?;
        Ok(e)
    }
    fn get_or_create_book(file: Option<String>, params: Vec<String>) -> ComResult<Self> {
        let obj = match ComObject::get_instance(Self::EXCEL_PROGID.into(), None, false).ok().flatten() {
            Some(obj) => obj,
            None => ComObject::new(Self::EXCEL_PROGID.into(), false)?,
        };
        let e = Self::new(obj)?;
        e.show()?;
        if let Object::ComObject(book) = e.open_book(file, params)? {
            Ok(Self::new_book(book, e.hwnd))
        } else {
            Ok(e)
        }
    }
    pub fn open(file: Option<String>, flg: ExcelOpenFlag, params: Vec<String>) -> ComResult<ComObject> {
        let excel = match flg {
            ExcelOpenFlag::Default => Self::get_or_create(file, params),
            ExcelOpenFlag::New => Self::create(file, params),
            ExcelOpenFlag::Book => Self::get_or_create_book(file, params),
            ExcelOpenFlag::ThirdParty => Err(ComError::new_u(UErrorKind::ExcelError, UErrorMessage::ThirdPartyNotImplemented)),
        }?;
        excel.activate();
        Ok(excel.obj)
    }
    pub fn new(obj: ComObject) -> ComResult<Self> {
        let hwnd = match obj.get_raw_property("Hwnd") {
            Ok(variant) => variant.to_i32()?,
            Err(_) => {
                let app = obj.get_property_as_comobject("Application")?;
                app.get_raw_property("Hwnd")?.to_i32()?
            }
        };
        let hwnd = HWND(hwnd as isize);
        let r#type = match obj.get_type_name() {
            Some(name) => match name.as_str() {
                "_Application" => ObjectType::Application,
                "_Workbook" => ObjectType::Workbook,
                _ => ObjectType::Other
            },
            None => ObjectType::Other,
        };
        Ok(Self {obj, hwnd, r#type})
    }
    fn new_book(obj: ComObject, hwnd: HWND) -> Self {
        Self {obj, hwnd, r#type: ObjectType::Workbook}
    }
    /// - ファイル名指定あり
    ///     1. Open
    /// - ファイル名指定なし
    ///     - Workbookが0
    ///         1. Add
    ///     - Workbookがある
    ///         1. なにもしない
    fn open_book(&self, file: Option<String>, params: Vec<String>) -> ComResult<Object> {
        match file {
            Some(path) => {
                let books = self.workbooks()?;
                let mut args = vec![ComArg::Arg(path.into())];
                let named = params.into_iter()
                    .filter_map(|p| {
                        p.split_once(":=")
                            .map(|(param, value)| ComArg::NamedArg(param.into(), value.into()))
                    });
                args.extend(named);
                books.invoke_method("Open", &mut args)
            },
            None => {
                self.add_book_if_none()?;
                Ok(Object::Empty)
            }
        }
    }
    fn add_book_if_none(&self) -> ComResult<()> {
        if self.count()? == 0 {
            let books = self.workbooks()?;
            let mut args = vec![];
            books.invoke_method("Add", &mut args)?;
        }
        Ok(())
    }
    fn count(&self) -> ComResult<i32> {
        let books = self.workbooks()?;
        books.get_raw_property("Count")?.to_i32()
    }
    fn workbooks(&self) -> ComResult<ComObject> {
        self.obj.get_property_as_comobject("Workbooks")
    }
    fn show(&self) -> ComResult<()> {
        self.obj.set_property("Visible", true.into())
    }
    fn activate(&self) {
        unsafe {
            SetForegroundWindow(self.hwnd);
        }
    }
    /// 自身からWorkbookオブジェクトを得る
    /// 自身が_Applicationの場合にWorkbookを得る為の関数を渡す
    fn get_wookbook<F: Fn() -> ComResult<ComObject>>(&self, from_app: F) -> ComResult<ComObject> {
        match self.r#type {
            ObjectType::Application => from_app(),
            ObjectType::Workbook => Ok(self.obj.clone()),
            ObjectType::Other => Err(ComError::new_u(
                UErrorKind::ExcelError,
                UErrorMessage::IsNotValidExcelObject
            )),
        }
    }
    /// ブック自身またはアクティブブックから指定シートを得る
    fn get_sheet(&self, sheet_id: Option<Object>) -> ComResult<ComObject> {
        let book = self.get_wookbook(|| {
            self.obj.get_property_as_comobject("ActiveWorkbook")
        })?;
        match sheet_id {
            Some(id) => book.get_property_by_index_as_comobject("WorkSheets", vec![id]),
            None => book.get_property_as_comobject("ActiveSheet"),
        }
    }
    /// path
    /// - None: 強制終了
    /// - Some(None): 上書き保存
    /// - Some(Some(path)): pathに保存
    pub fn close(&self, path: Option<Option<String>>) -> Option<()> {
        let book = self.get_wookbook(|| {
            self.obj.get_property_as_comobject("ActiveWorkbook")
        }).ok()?;
        let app = book.get_property_as_comobject("Application").ok()?;
        match path {
            Some(Some(path)) => {
                app.set_property("DisplayAlerts", false.into()).ok()?;
                let mut args = vec![
                    ComArg::Arg(path.into()),
                ];
                book.invoke_method("SaveAs", &mut args).ok()?;
                app.set_property("DisplayAlerts", true.into()).ok()?;
            },
            Some(None) => {
                let mut args = vec![];
                book.invoke_method("Save", &mut args).ok()?;
            },
            None => {
                let mut args = vec![
                    ComArg::Arg(false.into())
                ];
                book.invoke_method("Close", &mut args).ok()?;
            },
        }
        let mut args = vec![];
        app.invoke_method("Quit", &mut args).ok()?;
        Some(())
    }
    pub fn activate_sheet(&self, sheet_id: Object, book_id: Option<Object>) -> Option<()> {
        let book = self.get_wookbook(|| {
            match book_id.to_owned() {
                Some(index) => {
                    self.obj.get_property_by_index_as_comobject("Workbooks", vec![index])
                },
                None => self.obj.get_property_as_comobject("ActiveWorkbook")
            }
        }).ok()?;
        let sheet = book.get_property_by_index_as_comobject("Worksheets", vec![sheet_id]).ok()?;
        sheet.invoke_method("Activate", &mut vec![]).ok()?;
        Some(())
    }
    pub fn add_sheet(&self, sheet_id: &str) -> Option<()> {
        let book = self.get_wookbook(|| {
            self.obj.get_property_as_comobject("ActiveWorkbook")
        }).ok()?;
        let sheets = book.get_property_as_comobject("WorkSheets").ok()?;
        if sheets.get_property_by_index_as_comobject("Item", vec![sheet_id.into()]).is_err() {
            // 同名シートが存在しない場合は追加
            let count = sheets.get_raw_property("Count").ok()?.to_i32().ok()?;
            let after = sheets.get_property_by_index_as_comobject("Item", vec![count.into()]).ok()?;
            let mut args = vec![
                ComArg::NamedArg("After".into(), Object::ComObject(after)),
            ];
            if let Ok(Object::ComObject(sheet)) = sheets.invoke_method("Add", &mut args) {
                sheet.set_property("Name", sheet_id.into()).ok()
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn delete_sheet(&self, sheet_id: Object) -> Option<()> {
        let book = self.get_wookbook(|| {
            self.obj.get_property_as_comobject("ActiveWorkbook")
        }).ok()?;
        let sheet = book.get_property_by_index_as_comobject("WorkSheets", vec![sheet_id]).ok()?;
        sheet.invoke_method("Delete", &mut vec![]).map(|_| ()).ok()
    }
    pub fn get_a1_range(&self, a1: Option<String>, sheet_id: Option<Object>) -> ComResult<ComObject> {
        let sheet = self.get_sheet(sheet_id)?;
        match a1 {
            Some(a1) => {
                sheet.get_property_by_index_as_comobject("Range", vec![a1.into()])
            },
            None => {
                sheet.invoke_method("Activate", &mut vec![])?;
                self.obj.get_property_as_comobject("Selection")
            },
        }
    }
    pub fn get_cell_range(&self, row: f64, column: f64, sheet_id: Option<Object>) -> ComResult<ComObject> {
        let sheet = self.get_sheet(sheet_id)?;
        let row = sheet.get_property_by_index_as_comobject("Rows", vec![row.into()])?;
        row.get_property_by_index_as_comobject("Columns", vec![column.into()])
    }
    pub fn get_range_value(&self, a1: Option<String>, sheet_id: Option<Object>) -> ComResult<Object> {
        let range = self.get_a1_range(a1, sheet_id)?;
        let col = ComCollection::try_from(&range)?.to_comobject_vec()?;
        let arr = col.into_iter()
            .map(|com| com.get_property("Value"))
            .collect::<ComResult<Vec<Object>>>()?;
        if arr.len() == 1 {
            let obj = arr[0].to_owned();
            Ok(obj)
        } else {
            Ok(Object::Array(arr))
        }
    }
    pub fn get_cell_value(&self, row: f64, column: f64, sheet_id: Option<Object>) -> ComResult<Object> {
        let cell = self.get_cell_range(row, column, sheet_id)?;
        cell.get_property("Value")
    }
    pub fn set_range(&self, value: Object, range: ComObject, color: Option<i32>, bg_color: Option<i32>) -> Option<()> {
        let count = range.get_raw_property("Count")
            .unwrap_or_default()
            .to_i32()
            .unwrap_or_default();
        let range = match count {
            1 => {
                // Rangeが単一セルの場合
                match value.get_array_size() {
                    // セットする値が配列の場合はそれに合ったサイズのRangeにする
                    Some((s, s2)) => {
                        let mut args = match s2 {
                            Some(s2) => vec![ComArg::Arg(s.into()), ComArg::Arg(s2.into())],
                            None => vec![ComArg::Arg(1.into()), ComArg::Arg(s.into())],
                        };
                        // let mut args = vec![
                        //     ComArg::Arg(s.into()),
                        //     ComArg::Arg(s2.unwrap_or(1).into()),
                        // ];
                        let variant = range.invoke_method_raw("Resize", &mut args).ok()?;
                        let disp = variant.to_idispatch().ok()?;
                        let new_range = ComObject::from(disp);
                        Some(new_range)
                    },
                    // セットする値が配列ではないのでそのままセット
                    None => {
                        Some(range)
                    }
                }
            },
            0 => {
                // Rangeが0要素の場合は失敗
                None
            },
            _ => {
                // 範囲
                Some(range)
            }
        }?;
        range.set_property("Value", value).ok()?;
        if let Some(color) = color {
            let font = range.get_property_as_comobject("Font").ok()?;
            font.set_property("Color", color.into()).ok()?;
        }
        if let Some(bg_color) = bg_color {
            let interior = range.get_property_as_comobject("Interior").ok()?;
            interior.set_property("Color", bg_color.into()).ok()?;
        }
        Some(())
    }
}

#[derive(Debug)]
pub struct SafeArray(*mut SAFEARRAY);
impl TryFrom<Object> for SafeArray {
    type Error = ComError;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        unsafe {
            let flat = obj.flatten(vec![])
                .ok_or(ComError::new_u(UErrorKind::SafeArrayError, UErrorMessage::CanNotConvertToSafeArray))?;
            let sizes = flat.iter()
                .map(|(_, i)| i.clone())
                .reduce(|a, b| {
                    a.into_iter().zip(b.into_iter())
                        .map(|(a, b)| a.max(b))
                        .collect()
                })
                .ok_or(ComError::new_u(UErrorKind::SafeArrayError, UErrorMessage::CanNotConvertToSafeArray))?;
            let cdims = sizes.len() as u32;
            let rgsabound = sizes.into_iter()
                .map(|i| SAFEARRAYBOUND { cElements: i as u32 + 1, lLbound: 0 })
                .collect::<Vec<_>>();
            let psa = SafeArrayCreate(VT_VARIANT, cdims, rgsabound.as_ptr());
            for (obj, index) in flat {
                let v = VARIANT::try_from(obj)?;
                let pv = &v as *const VARIANT as *const c_void;
                SafeArrayPutElement(psa, index.as_ptr(), pv)?;
            }
            Ok(Self(psa))
        }
    }
}
impl SafeArray {
    pub fn from_raw(ptr: *mut c_void) -> Self {
        Self(ptr as *mut SAFEARRAY)
    }
    pub fn as_ptr(&self) -> *mut c_void {
        self.0 as *mut c_void
    }
    fn as_variant(&self) -> VARIANT {
        VARIANT::from_safearray(self.0)
    }
    pub fn to_object(&self) -> ComResult<Object> {
        self.0.try_into()
    }
    pub fn destroy(&self) {
        unsafe {
            let _ = SafeArrayDestroy(self.0);
        }
    }
}