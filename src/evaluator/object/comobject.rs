use windows::{
    core::{self, BSTR, HRESULT, HSTRING, PCWSTR, PWSTR, ComInterface, GUID, Interface, implement, IUnknown},
    Win32::{
        Foundation::{VARIANT_TRUE, VARIANT_FALSE, DISP_E_MEMBERNOTFOUND},
        System::{
            Com::{
                CLSCTX_ALL, CLSCTX_LOCAL_SERVER,
                IDispatch, IDispatch_Impl, //IDispatch_Vtbl,
                CLSIDFromString, CoCreateInstance,
                DISPPARAMS,
                DISPATCH_FLAGS, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPATCH_METHOD,
                EXCEPINFO,
                VARIANT, VARIANT_0_0,
                VARENUM, VT_ARRAY,VT_BYREF,VT_BOOL,VT_BSTR,VT_CY,VT_DATE,VT_DECIMAL,VT_DISPATCH,VT_EMPTY,VT_ERROR,VT_I1,VT_I2,VT_I4,VT_INT,VT_NULL,VT_R4,VT_R8,VT_UI1,VT_UI2,VT_UI4,VT_UINT,VT_UNKNOWN,VT_VARIANT,
                // VT_PTR, VT_SAFEARRAY,
                SAFEARRAY, SAFEARRAYBOUND,
                ITypeInfo, //ELEMDESC,
                IConnectionPoint, IConnectionPointContainer,
            },
            Ole::{
                GetActiveObject,
                DISPID_PROPERTYPUT, DISPID_NEWENUM,
                VariantChangeType,
                SafeArrayCreate, SafeArrayPutElement, SafeArrayGetElement, SafeArrayGetElemsize,
                IEnumVARIANT,
                IDispatchEx,fdexNameCaseInsensitive
            },
            Wmi::{
                ISWbemObject, ISWbemProperty,
            }
        }
    }
};

use crate::evaluator::{Object, Evaluator, EvalResult, Function};
use crate::ast::{Expression, Identifier};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::winapi::WString;

use std::mem::ManuallyDrop;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

const LOCALE_SYSTEM_DEFAULT: u32 = 0x0800;
const LOCALE_USER_DEFAULT: u32 = 0x400;
type ComResult<T> = Result<T, ComError>;

#[derive(Debug)]
pub enum ComError {
    WindowsError {
        message: String,
        code: i32,
        description: Option<String>
    },
    UError(UError),
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
            ComError::UError(_) => false,
        }
    }
    fn as_hresult(&self) -> HRESULT {
        match self {
            ComError::WindowsError { message: _, code, description: _ } => {
                HRESULT(*code)
            },
            ComError::UError(err) => {
                if let UErrorKind::ComError(n) = err.kind {
                    HRESULT(n)
                } else {
                    HRESULT(0)
                }
            },
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
    // IEの実行を防ぐ
    pub fn is_ie(input: &str) -> ComResult<bool> {
        unsafe {
            // 入力のCLSIDを得る、不正な値ならここで弾く
            let lpsz = HSTRING::from(input);
            let clsid = CLSIDFromString(&lpsz)?;
            // InternetExplorer.ApplicationのCLSIDと入力のCLSIDを比較
            let lpsz = HSTRING::from("InternetExplorer.Application");
            if let Ok(clsid_ie) = CLSIDFromString(&lpsz) {
                let is_ie = clsid_ie == clsid;
                Ok(is_ie)
            } else {
                // IEが存在しなければfalse
                Ok(false)
            }
        }
    }
    pub fn new(id: String) -> ComResult<Self> {
        unsafe {
            let lpsz = HSTRING::from(id);
            let rclsid = CLSIDFromString(&lpsz)?;
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
    pub fn get_instance(id: String) -> ComResult<Option<Self>> {
        unsafe {
            let lpsz = HSTRING::from(id);
            let rclsid = CLSIDFromString(&lpsz)?;
            let pvreserved = std::ptr::null_mut() as *mut std::ffi::c_void;
            let mut ppunk = None;
            GetActiveObject(&rclsid, pvreserved, &mut ppunk)?;
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
    /// obj.prop
    pub fn get_property(&self, prop: &str) -> ComResult<Object> {
        let variant = self.get_raw_property(prop)?;
        variant.try_into()
    }
    pub fn get_raw_property(&self, prop: &str) -> ComResult<VARIANT> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        self.invoke_raw(dispidmember, &mut dp, DISPATCH_PROPERTYGET)
    }
    /// プロパティへの代入
    /// obj.prop = value
    pub fn set_property(&self, prop: &str, value: Object) -> ComResult<()> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let wrapper = value.to_variant_wrapper()?;
        let new = wrapper.to_variant()?;
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
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let mut args = index.clone().into_iter()
            .map(|o| o.try_into())
            .collect::<ComResult<Vec<_>>>()?;
        args.reverse();
        dp.cArgs = args.len() as u32;
        dp.rgvarg = args.as_mut_ptr();
        match self.invoke(dispidmember, &mut dp, DISPATCH_PROPERTYGET|DISPATCH_METHOD) {
            Ok(obj) => Ok(obj),
            Err(e) => {
                if e.is_member_not_found() {
                    // DISP_E_MEMBERNOTFOUNDの場合は
                    // foo.barがコレクションでfoo.bar[i]でItem(i)を得たい可能性がある
                    match self.get_property(prop)? {
                        // プロパティとして取得し、COMオブジェクトならItemを得る
                        Object::ComObject(com2) => {
                            com2.get_item_property(index)
                        },
                        // それ以外はそのままエラーを返す
                        _ => Err(e)
                    }
                } else {
                    Err(e)
                }
            },
        }
    }
    /// インデックス指定でプロパティへ代入
    /// obj.prop[index] = value
    pub fn set_property_by_index(&self, prop: &str, index: Object, value: Object) -> ComResult<()> {
        let dispidmember = self.get_id_from_name(prop)?;
        let mut dp = DISPPARAMS::default();
        let wrapper = value.to_variant_wrapper()?;
        let new = wrapper.to_variant()?;
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
        let wargs = args.iter()
            .map(|arg| WrapperArg::try_from(arg.clone()))
            .collect::<ComResult<Vec<WrapperArg>>>()?;

        let info = self.get_type_info()?;
        let pnames = info.get_param_names(dispidmember)?;

        let mut named_flg = false;

        let (ids, mut vargs): (Vec<_>, Vec<_>) = wargs.iter()
            .map(|arg| {
                match arg {
                    WrapperArg::Arg(w) => {
                        if named_flg {
                            Err(ComError::UError(UError::new(UErrorKind::ComArgError, UErrorMessage::InvalidComMethodArgOrder)))
                        } else {
                            let v = w.to_variant()?;
                            Ok((None, v))
                        }
                    },
                    WrapperArg::ByRef(w) => {
                        if named_flg {
                            Err(ComError::UError(UError::new(UErrorKind::ComArgError, UErrorMessage::InvalidComMethodArgOrder)))
                        } else {
                            let mut v = w.to_variant()?;
                            let mut vv = v.to_vt_variant()?;
                            vv.as_byref()?;
                            Ok((None, vv))
                        }
                    },
                    WrapperArg::NamedArg(name, w) => {
                        let id = pnames.get_id_of(&name)?;
                        let v = w.to_variant()?;
                        named_flg = true;
                        Ok((Some(id), v))
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
        let mut ids = ids.into_iter();
        if ids.any(|name| name.is_some()) {
            let mut named_args = ids
                .filter_map(|maybe_id| maybe_id)
                .collect::<Vec<_>>();
            named_args.reverse();
            dp.cNamedArgs = named_args.len() as u32;
            dp.rgdispidNamedArgs = named_args.as_mut_ptr();
        }
        match self.invoke(dispidmember, &dp, DISPATCH_METHOD|DISPATCH_PROPERTYGET) {
            Ok(obj) => {
                vargs.reverse();
                // 参照渡しは値を更新する
                for (arg, varg) in args.iter_mut().zip(vargs.into_iter()) {
                    match arg {
                        ComArg::ByRef(_, byref) => *byref = varg.try_into()?,
                        _ => {}
                    }
                }
                Ok(obj)
            },
            Err(e) => {
                match e.as_hresult() {
                    DISP_E_MEMBERNOTFOUND => {
                        // DISP_E_MEMBERNOTFOUNDの場合は
                        // foo.barがコレクションでfoo.bar(i)でItem(i)を得たい可能性がある
                        match self.get_property(method)? {
                            // プロパティとして取得し、COMオブジェクトならItemを得る
                            Object::ComObject(com2) => {
                                let index = args.iter()
                                    .map(|comarg| comarg.clone().into())
                                    .collect();
                                let obj = com2.get_item_property(index)?;
                                // この場合余計な代入が発生しないように引数は空にする
                                args.clear();
                                Ok(obj)
                            },
                            // それ以外はそのままエラーを返す
                            _ => Err(e)
                        }
                    },
                    _ => Err(e)
                }
            },
        }

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
    fn invoke_wmi_method(&self, method: &str, args: &mut Vec<ComArg>) -> ComResult<Object> {
        unsafe {
            let wbemobj = self.idispatch.cast::<ISWbemObject>()?;
            let strname = BSTR::from(method);
            let method = wbemobj.Methods_()?.Item(&strname, 0)?;

            let inparams = method.InParameters()?.SpawnInstance_(0)?;
            let count = inparams.Properties_()?.Count()?;
            let newenum = inparams.Properties_()?._NewEnum()?.cast::<IEnumVARIANT>()?;
            let props = ComCollection {col: newenum};

            let mut wargs = args.iter()
                .map(|arg| WrapperArg::try_from(arg.clone()))
                .collect::<ComResult<Vec<WrapperArg>>>()?
                .into_iter();

            for _ in (0..count) {
                let com = props.next_idispatch()?;
                let prop = com.cast::<ISWbemProperty>()?;
                let value = match wargs.next() {
                    Some(w) => match w {
                        WrapperArg::NamedArg(_, _) => {
                            return Err(ComError::new_u(UErrorKind::WmiError, UErrorMessage::NamedArgNotAllowed));
                        },
                        WrapperArg::Arg(v) |
                        WrapperArg::ByRef(v) => v.to_variant(),
                    },
                    None => {
                        return Err(ComError::new_u(UErrorKind::WmiError, UErrorMessage::MissingArgument))
                    },
                }?;
                prop.SetValue(&value)?;
            }

            let outparam = wbemobj.ExecMethod_(&strname, &inparams, 0, None)?;
            let outparamenum = ComCollection{ col: outparam.Properties_()?._NewEnum()?.cast::<IEnumVARIANT>()? };
            let retrun_value = match outparamenum.next()? {
                Some(variant) => {
                    let prop = variant.to_t::<ISWbemProperty>()?;
                    prop.Value()?.try_into()
                },
                None => Ok(Object::Empty),
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
enum WrapperArg {
    Arg(VariantWrapper),
    ByRef(VariantWrapper),
    NamedArg(String, VariantWrapper),
}
impl TryFrom<ComArg> for WrapperArg {
    type Error = ComError;

    fn try_from(arg: ComArg) -> Result<Self, Self::Error> {
        match arg {
            ComArg::Arg(obj) => {
                let wrapper = obj.to_variant_wrapper()?;
                Ok(Self::Arg(wrapper))
            },
            ComArg::ByRef(_, obj) => {
                let wrapper = obj.to_variant_wrapper()?;
                Ok(Self::ByRef(wrapper))
            },
            ComArg::NamedArg(name, obj) => {
                let wrapper = obj.to_variant_wrapper()?;
                Ok(Self::NamedArg(name, wrapper))
            },
        }
    }
}

impl TryFrom<Object> for VARIANT {
    type Error = ComError;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        let variant = match obj {
            Object::Num(n) => VARIANT::from_f64(n),
            Object::String(s) => VARIANT::from_string(s),
            Object::Bool(b) => VARIANT::from_bool(b),
            Object::Null => VARIANT::null(),
            Object::EmptyParam => VARIANT::null(),
            Object::Empty => VARIANT::default(),
            Object::ComObject(disp) => VARIANT::from_idispatch(disp.idispatch),
            Object::Unknown(unk) => VARIANT::from_iunknown(unk.0),
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
                    // 数値系
                    VT_CY | // 通貨
                    VT_DATE | // 日付
                    VT_DECIMAL |
                    VT_I1 |
                    VT_I2 |
                    VT_I4 |
                    VT_INT |
                    VT_UI1 |
                    VT_UI2 |
                    VT_UI4 |
                    VT_UINT |
                    VT_ERROR |
                    VT_R4 => {
                        let mut variant = VARIANT::default();
                        VariantChangeType(&mut variant, &self, 0, VT_R8)?;
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
                                None => Err(ComError::from_variant_error(vt)),
                            },
                            None => Err(ComError::from_variant_error(vt)),
                        }
                    } else {
                        match &*v00.Anonymous.pdispVal {
                            Some(disp) => {
                                let obj = ComObject::from(disp.clone());
                                Ok(Object::ComObject(obj))
                            },
                            None => Err(ComError::from_variant_error(vt)),
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
                                None => Err(ComError::from_variant_error(vt)),
                            },
                            None => Err(ComError::from_variant_error(vt)),
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
                    vt => Err(ComError::from_variant_error(vt))
                }
            }
        }
    }
}

impl TryInto<Object> for *mut SAFEARRAY {
    type Error = ComError;

    fn try_into(self) -> Result<Object, Self::Error> {
        unsafe {
            let size = SafeArrayGetElemsize(self) as i32;
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

trait VariantExt {
    fn null() -> VARIANT;
    fn from_f64(n: f64) -> VARIANT;
    fn from_string(s: String) -> VARIANT;
    fn from_bool(b: bool) -> VARIANT;
    fn from_idispatch(disp: IDispatch) -> VARIANT;
    fn from_iunknown(unk: IUnknown) -> VARIANT;
    fn from_safearray(psa: *mut SAFEARRAY) -> VARIANT;
    fn vt(&self) -> VARENUM;
    fn as_byref(&mut self) -> ComResult<()>;
    fn to_vt_variant(&mut self) -> ComResult<VARIANT>;
    fn to_idispatch(&self) -> ComResult<IDispatch>;
    fn to_t<T: ComInterface>(&self) -> ComResult<T>;
}

impl VariantExt for VARIANT {
    fn null() -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_NULL;
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
        let vb = if b {VARIANT_TRUE} else {VARIANT_FALSE};
        v00.Anonymous.boolVal = vb;
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
    fn as_byref(&mut self) -> ComResult<()> {
        unsafe {
            let vt = self.vt();
            let v00 = &mut self.Anonymous.Anonymous;
            v00.vt = VARENUM(vt.0|VT_BYREF.0);
            Ok(())
        }
    }
    fn to_vt_variant(&mut self) -> ComResult<VARIANT> {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0::default();
        v00.vt = VT_VARIANT;
        v00.Anonymous.pvarVal = self;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        Ok(variant)
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
}

enum VariantWrapper {
    Array(Vec<Self>),
    Value(VARIANT),
}
impl VariantWrapper {
    fn to_variant(&self) -> ComResult<VARIANT> {
        match self {
            VariantWrapper::Array(arr) => {
                let psa = safearray_from_variant_array(arr, 1)?;
                let v = VARIANT::from_safearray(psa);
                Ok(v)
            },
            VariantWrapper::Value(v) => Ok(v.clone()),
        }
    }
}

fn safearray_from_variant_array(arr: &Vec<VariantWrapper>, dimension: u32) -> ComResult<*mut SAFEARRAY> {
    unsafe {
        let cdims = dimension;
        let rgsabound = SAFEARRAYBOUND {
            cElements: arr.len() as u32,
            lLbound: 0
        };
        let psa = SafeArrayCreate(VT_VARIANT, cdims, &rgsabound);
        let mut rgindices = 0;
        for variant in arr {
            match variant {
                VariantWrapper::Array(carr) => {
                    let p = safearray_from_variant_array(carr, 1)?;
                    let v = VARIANT::from_safearray(p);
                    let pv = &v as *const _ as *const c_void;
                    SafeArrayPutElement(psa, &rgindices, pv)?;
                },
                VariantWrapper::Value(v) => {
                    let pv = v as *const _ as *const c_void;
                    SafeArrayPutElement(psa, &rgindices, pv)?;
                },
            }
            rgindices += 1;
        }
        Ok(psa)
    }
}

impl Object {
    fn to_variant_wrapper(self) -> ComResult<VariantWrapper> {
        if let Object::Array(arr) = self {
            let varr = arr.into_iter()
                .map(|o| {
                    if let Object::Array(_) = o {
                        o.to_variant_wrapper()
                    } else {
                        let variant = o.try_into()?;
                        Ok(VariantWrapper::Value(variant))
                    }
                })
                .collect::<ComResult<Vec<VariantWrapper>>>()?;
            Ok(VariantWrapper::Array(varr))
        } else {
            let variant = self.try_into()?;
            Ok(VariantWrapper::Value(variant))
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
            let cmaxnames = rgbstrnames.len() as u32;
            let mut pcnames = 0;
            self.info.GetNames(memid, rgbstrnames.as_mut_ptr(), cmaxnames, &mut pcnames)?;
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
    fn next_idispatch(&self) -> ComResult<ComObject> {
        match self.next()? {
            Some(variant) => {
                variant.to_idispatch()
                    .map(|idispatch| ComObject::from(idispatch))
            },
            None => Err(ComError::new_u(
                UErrorKind::VariantError,
                UErrorMessage::VariantIsNotIDispatch
            )),
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
                let obj = self.func.invoke(&mut evaluator, arguments)
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