use crate::{Evaluator, EvalResult, UError, UErrorKind, UErrorMessage};
use crate::error::DefinitionType;
use crate::object::{
    Object,
    Function,
    ustruct::{StructDef, MemberDefVec, UStruct, MemberType},
    comobject::SafeArray
};

use parser::ast::{self, DefDllParam, DefDllParamSize, Expression};
use util::winapi::{
    to_ansi_bytes, to_wide_string, from_ansi_bytes, from_wide_string,
};

use libffi::middle::{Arg, Type, Cif, CodePtr, Closure};
use std::vec::IntoIter;
use std::ffi::c_void;
use std::ptr::copy_nonoverlapping;

use num_traits::FromPrimitive;

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::{
    Foundation::HANDLE,
    System::Memory::{
        HeapCreate, HeapAlloc, HeapFree,
        HEAP_GENERATE_EXCEPTIONS, HEAP_ZERO_MEMORY, HEAP_NONE,
        HeapDestroy,
    }
};

#[derive(Debug, Clone, PartialEq)]
pub enum DllType {
    Int,
    Long,
    Bool,
    Uint,
    Hwnd,
    Handle,
    String,
    Wstring,
    Float,
    Double,
    Word,
    Dword,
    Byte,
    Char,
    Pchar,
    Wchar,
    PWchar,
    Boolean,
    Longlong,
    SafeArray,
    Void,
    Pointer,
    Size,
    UStruct,
    CallBack,
}
impl DllType {
    pub fn size(&self) -> usize {
        match self {
            DllType::Int |
            DllType::Long |
            DllType::Bool => std::mem::size_of::<i32>(),
            DllType::Uint |
            DllType::Dword => std::mem::size_of::<u32>(),
            DllType::Float => std::mem::size_of::<f32>(),
            DllType::Double => std::mem::size_of::<f64>(),
            DllType::Word |
            DllType::Wchar => std::mem::size_of::<u16>(),
            DllType::Byte |
            DllType::Boolean |
            DllType::Char => std::mem::size_of::<u8>(),
            DllType::Longlong => std::mem::size_of::<i64>(),
            DllType::Hwnd |
            DllType::Handle |
            DllType::String |
            DllType::Wstring |
            DllType::Pchar |
            DllType::PWchar |
            DllType::Pointer |
            DllType::Size |
            DllType::UStruct |
            DllType::CallBack |
            DllType::SafeArray => std::mem::size_of::<usize>(),
            DllType::Void => 0,
        }
    }
}
impl std::fmt::Display for DllType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DllType::Int => write!(f, "int"),
            DllType::Long => write!(f, "long"),
            DllType::Bool => write!(f, "bool"),
            DllType::Uint => write!(f, "uint"),
            DllType::Hwnd => write!(f, "hwnd"),
            DllType::Handle => write!(f, "handle"),
            DllType::String => write!(f, "string"),
            DllType::Wstring => write!(f, "wstring"),
            DllType::Float => write!(f, "float"),
            DllType::Double => write!(f, "double"),
            DllType::Word => write!(f, "word"),
            DllType::Dword => write!(f, "dword"),
            DllType::Byte => write!(f, "byte"),
            DllType::Char => write!(f, "char"),
            DllType::Pchar => write!(f, "pchar"),
            DllType::Wchar => write!(f, "wchar"),
            DllType::PWchar => write!(f, "pwchar"),
            DllType::Boolean => write!(f, "boolean"),
            DllType::Longlong => write!(f, "longlong"),
            DllType::SafeArray => write!(f, "safearray"),
            DllType::Void => write!(f, "void"),
            DllType::Pointer => write!(f, "pointer"),
            DllType::Size => write!(f, "size"),
            DllType::UStruct => write!(f, "struct"),
            DllType::CallBack => write!(f, "callback"),
        }
    }
}
impl From<ast::DllType> for DllType {
    fn from(val: ast::DllType) -> Self {
        match val {
            ast::DllType::Int => DllType::Int,
            ast::DllType::Long => DllType::Long,
            ast::DllType::Bool => DllType::Bool,
            ast::DllType::Uint => DllType::Uint,
            ast::DllType::Hwnd => DllType::Hwnd,
            ast::DllType::Handle => DllType::Handle,
            ast::DllType::String => DllType::String,
            ast::DllType::Wstring => DllType::Wstring,
            ast::DllType::Float => DllType::Float,
            ast::DllType::Double => DllType::Double,
            ast::DllType::Word => DllType::Word,
            ast::DllType::Dword => DllType::Dword,
            ast::DllType::Byte => DllType::Byte,
            ast::DllType::Char => DllType::Char,
            ast::DllType::Pchar => DllType::Pchar,
            ast::DllType::Wchar => DllType::Wchar,
            ast::DllType::PWchar => DllType::PWchar,
            ast::DllType::Boolean => DllType::Boolean,
            ast::DllType::Longlong => DllType::Longlong,
            ast::DllType::SafeArray => DllType::SafeArray,
            ast::DllType::Void => DllType::Void,
            ast::DllType::Pointer => DllType::Pointer,
            ast::DllType::Size => DllType::Size,
            ast::DllType::UStruct => DllType::UStruct,
            ast::DllType::CallBack => DllType::CallBack,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DllParam {
    Param {
        dll_type: DllType,
        is_ref: bool,
        size: Option<usize>
    },
    Struct(Vec<Self>),
    Callback(Vec<DllType>, DllType),
}
impl std::fmt::Display for DllParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DllParam::Param { dll_type, is_ref, size } => {
                let r = if *is_ref {"var "} else {""};
                let s = match size {
                    Some(n) => format!("[{n}]"),
                    None => String::new(),
                };
                write!(f, "{r}{dll_type}{s}")
            },
            DllParam::Struct(v) => {
                let s = v.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{{{s}}}")
            },
            DllParam::Callback(argtypes, rtype) => {
                let types = argtypes.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "callback({}):{}", types, rtype)
            },
        }
    }
}
impl DllParam {
    #[allow(clippy::len_without_is_empty)]
    /// パラメータの数を得る
    pub fn len(&self) -> usize {
        match self {
            Self::Param { dll_type:_, is_ref:_, size:_ } => 1,
            Self::Struct(params) => {
                params.iter()
                    .map(|p| p.len())
                    .reduce(|a,b| a + b)
                    .unwrap_or_default()
            },
            Self::Callback(_, _) => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefDll {
    pub name: String,
    pub alias: Option<String>,
    path: String,
    params: Vec<DllParam>,
    rtype: DllType,
}
impl std::fmt::Display for DefDll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params = self.params.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        match &self.alias {
            Some(alias) => write!(f, "{}({}):{}:{} as {}", self.name, params, self.rtype, self.path, alias),
            None => write!(f, "{}({}):{}:{}", self.name, params, self.rtype, self.path)
        }
    }
}
impl DefDll {
    pub fn convert_params(params: Vec<DefDllParam>, e: &mut Evaluator) -> EvalResult<Vec<DllParam>> {
        params.into_iter()
            .map(|p| {
                match p {
                    DefDllParam::Param { dll_type, is_ref, size } => {
                        let size = match size {
                            DefDllParamSize::Const(c) => {
                                match e.env.get_const_num(&c) {
                                    Some(n) => Ok(Some(n)),
                                    None => Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgConstSizeIsNotValid)),
                                }
                            },
                            DefDllParamSize::Size(n) => Ok(Some(n)),
                            DefDllParamSize::None => Ok(None),
                        }?;
                        Ok(DllParam::Param { dll_type: dll_type.into(), is_ref, size })
                    },
                    DefDllParam::Struct(p) => {
                        let p = Self::convert_params(p, e)?;
                        Ok(DllParam::Struct(p))
                    },
                    DefDllParam::Callback(a, r) => {
                        let arg_types = a.into_iter()
                            .map(|t| t.into())
                            .collect();
                        Ok(DllParam::Callback(arg_types, r.into()))
                    },
                }
            })
            .collect()
    }
    pub fn new<T>(name: String, alias: Option<String>, path: String, params: Vec<DllParam>, rtype: T) -> EvalResult<Self>
        where T: Into<DllType>
    {
        match rtype.into() {
            DllType::SafeArray |
            DllType::UStruct |
            DllType::CallBack => Err(UError::new(UErrorKind::DefinitionError(DefinitionType::DefDll), UErrorMessage::DllResultTypeNotAllowed)),
            rtype => Ok(Self { name, alias, path, params, rtype })
        }
    }
    fn param_len(&self) -> usize {
        self.params.iter().map(|p| p.len()).reduce(|a,b| a + b).unwrap_or_default()
    }
    pub fn invoke(&self, arguments: Vec<(Option<Expression>, Object)>, e: &mut Evaluator) -> EvalResult<Object> {
        unsafe {
            if self.param_len() != arguments.len() {
                return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch));
            }
            /* 引数などの準備 */
            let mut dllargs = DllArgs::new(&self.params, arguments, e)?;

            let result = {
                // 実際に渡す引数とその型情報
                let mut args = vec![];
                let mut types = vec![];
                let mut closures = vec![];
                for dllarg in &mut dllargs.args {
                    types.push(dllarg.get_type());
                    let arg = match &mut dllarg.value {
                        DllArgVal::Int(v) => Arg::new(v),
                        DllArgVal::IntV(v) => Arg::new(v),
                        DllArgVal::Uint(v) => Arg::new(v),
                        DllArgVal::UintV(v) => Arg::new(v),
                        DllArgVal::Word(v) => Arg::new(v),
                        DllArgVal::WordV(v) => Arg::new(v),
                        DllArgVal::Byte(v) => Arg::new(v),
                        DllArgVal::ByteV(v) => Arg::new(v),
                        DllArgVal::LongLong(v) => Arg::new(v),
                        DllArgVal::LongLongV(v) => Arg::new(v),
                        DllArgVal::Float(v) => Arg::new(v),
                        DllArgVal::FloatV(v) => Arg::new(v),
                        DllArgVal::Double(v) => Arg::new(v),
                        DllArgVal::DoubleV(v) => Arg::new(v),
                        DllArgVal::Size(v) => Arg::new(v),
                        DllArgVal::SizeV(v) => Arg::new(v),
                        DllArgVal::Struct(sarg) => {
                            Arg::new(&sarg.ustruct.address)
                        },
                        DllArgVal::UStruct(ust) => {
                            Arg::new(&ust.address)
                        },
                        DllArgVal::NullPtr => Arg::new(&0),
                        DllArgVal::SafeArray(sa) => {
                            let ptr = sa.as_ptr();
                            Arg::new(&ptr)
                        },
                        DllArgVal::CallBack(cb) => {
                            let c = cb.get_closure();
                            let arg = Arg::new(c.code_ptr());
                            closures.push(c);
                            arg
                        },
                        DllArgVal::ArgValPtr(p) => Arg::new(&p.ptr),
                    };
                    args.push(arg);
                }

                /* dll関数の実行 */

                // dllを開く
                let lib = dlopen::raw::Library::open(&self.path)?;
                // 関数シンボルを得る
                let symbol = lib.symbol(&self.name)?;
                // 戻り値の型
                let result_type = Type::from(&self.rtype);
                // 関数定義
                let cif = Cif::new(types, result_type);
                // 関数ポインタ
                let fun = CodePtr::from_ptr(symbol);

                // 実行
                match self.rtype {
                    DllType::Int |
                    DllType::Long => {
                        let n = cif.call::<i32>(fun, &args);
                        n.into()
                    }
                    DllType::Bool => {
                        let n = cif.call::<i32>(fun, &args);
                        (n != 0).into()
                    },
                    DllType::Uint |
                    DllType::Dword => {
                        let n = cif.call::<u32>(fun, &args);
                        n.into()
                    },
                    DllType::Hwnd |
                    DllType::Handle |
                    DllType::Size |
                    DllType::Pointer => {
                        let n = cif.call::<usize>(fun, &args);
                        n.into()
                    },
                    DllType::Byte => {
                        let n = cif.call::<u8>(fun, &args);
                        n.into()
                    }
                    DllType::Boolean => {
                        let n = cif.call::<u8>(fun, &args);
                        (n != 0).into()
                    },
                    DllType::Char => {
                        let n = cif.call::<u8>(fun, &args);
                        let ptr = &n as *const u8;
                        let s = Self::string_from_ansi_ptr(ptr);
                        s.into()
                    },
                    DllType::Word => {
                        let n = cif.call::<u16>(fun, &args);
                        n.into()
                    },
                    DllType::Wchar => {
                        let n = cif.call::<u16>(fun, &args);
                        let ptr = &n as *const u16;
                        let s = Self::string_from_wide_ptr(ptr);
                        s.into()
                    },
                    DllType::Float => {
                        let n = cif.call::<f32>(fun, &args);
                        n.into()
                    },
                    DllType::Double => {
                        let n = cif.call::<f64>(fun, &args);
                        n.into()
                    },
                    DllType::Longlong => {
                        let n = cif.call::<i64>(fun, &args);
                        n.into()
                    },
                    DllType::String |
                    DllType::Pchar => {
                        let ptr = cif.call::<*const u8>(fun, &args);
                        let s = Self::string_from_ansi_ptr(ptr);
                        s.into()
                    },
                    DllType::Wstring |
                    DllType::PWchar => {
                        let ptr = cif.call::<*const u16>(fun, &args);
                        let s = Self::string_from_wide_ptr(ptr);
                        s.into()
                    },
                    DllType::SafeArray => {
                        let ptr = cif.call::<*mut c_void>(fun, &args);
                        let sa = SafeArray::from_raw(ptr);
                        sa.to_object()?
                    }
                    // 戻り値型として使用不可
                    DllType::UStruct |
                    DllType::CallBack |
                    // 戻り値なし
                    DllType::Void => {
                        cif.call::<u8>(fun, &args);
                        Object::Empty
                    },
                }
            };

            /* 参照渡し */
            for arg in dllargs.args {
                if let DllArgVal::CallBack(cb) = arg.value {
                    // コールバックなら関数のエラーがあるかチェック
                    if let Some(err) = cb.user_func.result {
                        return Err(err);
                    }
                } else {
                    // var/refへの代入
                    // エラーは握りつぶす
                    let _ = arg.assign(e);
                }
            }

            Ok(result)
        }
    }
    unsafe fn string_from_ansi_ptr(ptr: *const u8) -> String {
        let pcstr = PCSTR::from_raw(ptr);
        let ansi = unsafe { pcstr.as_bytes() };
        from_ansi_bytes(ansi)
    }
    unsafe fn string_from_wide_ptr(ptr: *const u16) -> String {
        let pcwstr = PCWSTR::from_raw(ptr);
        let wide = unsafe { pcwstr.as_wide() };
        from_wide_string(wide)
    }
}
impl From<&DllType> for Type {
    fn from(t: &DllType) -> Self {
        match t {
            DllType::Int |
            DllType::Long |
            DllType::Bool => Self::i32(),
            DllType::Uint |
            DllType::Dword => Self::u32(),
            DllType::Float => Self::f32(),
            DllType::Double => Self::f64(),
            DllType::Longlong => Self::i64(),
            DllType::Byte |
            DllType::Char |
            DllType::Boolean => Self::u8(),
            DllType::Word |
            DllType::Wchar => Self::u16(),
            DllType::Hwnd |
            DllType::Handle |
            DllType::Size |
            DllType::Pointer => Self::usize(),
            DllType::String |
            DllType::Wstring |
            DllType::Pchar |
            DllType::PWchar |
            DllType::SafeArray |
            DllType::CallBack => Self::pointer(),
            DllType::UStruct => Self::usize(),
            DllType::Void => Self::void(),
        }
    }
}
impl From<&DllType> for MemberType {
    fn from(t: &DllType) -> Self {
        match t {
            DllType::Int => Self::Int,
            DllType::Long => Self::Long,
            DllType::Bool => Self::Bool,
            DllType::Uint => Self::Uint,
            DllType::Hwnd => Self::Hwnd,
            DllType::Handle => Self::Handle,
            DllType::String => Self::String,
            DllType::Wstring => Self::Wstring,
            DllType::Float => Self::Float,
            DllType::Double => Self::Double,
            DllType::Word => Self::Word,
            DllType::Dword => Self::Dword,
            DllType::Byte => Self::Byte,
            DllType::Char => Self::Char,
            DllType::Pchar => Self::Pchar,
            DllType::Wchar => Self::Wchar,
            DllType::PWchar => Self::PWchar,
            DllType::Boolean => Self::Boolean,
            DllType::Longlong => Self::Longlong,
            DllType::Pointer => Self::Pointer,
            DllType::Size => Self::Size,
            DllType::UStruct => Self::Pointer,
            DllType::SafeArray => Self::Pointer,
            DllType::CallBack => Self::Pointer,
            DllType::Void => Self::Pointer
        }
    }
}

#[derive(Debug)]
struct DllArgs {
    args: Vec<DllArg>,
}
impl DllArgs {
    fn new(params: &[DllParam], arguments: Vec<(Option<Expression>, Object)>, e: &mut Evaluator) -> EvalResult<Self> {
        let mut iter_args = arguments.into_iter();
        let args = params.iter()
            .map(|param| DllArg::new(param, &mut iter_args, e) )
            .collect::<EvalResult<Vec<DllArg>>>()?;
        Ok(Self { args })
    }
}

#[derive(Debug)]
struct DllArg {
    refexpr: Option<Expression>,
    value: DllArgVal,
}
impl DllArg {
    fn new(param: &DllParam, iter_args: &mut IntoIter<(Option<Expression>, Object)>, e: &mut Evaluator) -> EvalResult<Self> {
        match param {
            DllParam::Param { dll_type, is_ref, size } => {
                let (expr, value) = iter_args.next()
                        .ok_or(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch))?;
                    let value = DllArgVal::new(dll_type, *size, *is_ref, Some(value))?;
                let refexpr = if *is_ref { expr } else { None };
                Ok(Self { refexpr, value })
            },
            DllParam::Struct(params) => {
                let sarg = StructArg::from(params, iter_args)?;
                let value = DllArgVal::Struct(sarg);
                Ok(Self { refexpr: None, value })
            },
            DllParam::Callback(arg_types, rtype) => {
                let (_, value) = iter_args.next()
                        .ok_or(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch))?;
                match value {
                    Object::Function(f) |
                    Object::AnonFunc(f) => {
                        let user_func = UserFunc::new(arg_types.clone(), f, e);
                        let cb = UCallback::new(user_func, rtype.clone())?;
                        // cb.set_closure()?;
                        Ok(Self { refexpr: None, value: DllArgVal::CallBack(cb) })
                    },
                    o => Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::NotAFunction(o)))
                }
            }
        }
    }
    fn get_type(&self) -> Type {
        match &self.value {
            DllArgVal::Int(_) |
            DllArgVal::IntV(_) => Type::i32(),
            DllArgVal::Uint(_) |
            DllArgVal::UintV(_) => Type::u32(),
            DllArgVal::Word(_) |
            DllArgVal::WordV(_) => Type::u16(),
            DllArgVal::Byte(_) |
            DllArgVal::ByteV(_) => Type::u8(),
            DllArgVal::LongLong(_) |
            DllArgVal::LongLongV(_) => Type::i64(),
            DllArgVal::Float(_) |
            DllArgVal::FloatV(_) => Type::f32(),
            DllArgVal::Double(_) |
            DllArgVal::DoubleV(_) => Type::f64(),
            DllArgVal::Size(_) |
            DllArgVal::SizeV(_) => Type::usize(),
            DllArgVal::Struct(_) |
            DllArgVal::UStruct(_) => Type::usize(),
            DllArgVal::NullPtr => Type::usize(),
            DllArgVal::CallBack(_) |
            DllArgVal::SafeArray(_) |
            DllArgVal::ArgValPtr(_) => Type::pointer(),
        }
    }
    fn assign(self, e: &mut Evaluator) -> EvalResult<()> {
        let DllArg { refexpr, value } = self;
        match value {
            DllArgVal::Struct(sarg) => {
                sarg.assign(e)?;
            },
            value => {
                if let Some(expr) = refexpr {
                    if let Some(obj) = value.into_object() {
                        e.eval_assign_expression(expr, obj)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ArgValPtr {
    ptr: *mut c_void,
    hheap: HANDLE,
    count: usize,
    r#type: DllType,
}
impl Drop for ArgValPtr {
    fn drop(&mut self) {
        unsafe {
            let _ = HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            let _ = HeapDestroy(self.hheap);
        }
    }
}
impl ArgValPtr {
    fn new<T>(r#type: &DllType, vec: Vec<T>) -> EvalResult<Self> {
        unsafe {
            let count = vec.len();
            let heapsize = r#type.size() * count;
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, heapsize, heapsize)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, heapsize);

            let src = vec.as_ptr();
            let dst = ptr as *mut T;
            copy_nonoverlapping(src, dst, count);

            let avptr = Self { ptr, hheap, count, r#type: r#type.clone() };
            Ok(avptr)
        }
    }
    fn null_ptr() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            hheap: HANDLE(0),
            count: 0,
            r#type: DllType::Void,
        }
    }
    fn _addr(&self) -> usize {
        self.ptr as usize
    }
    fn into_object<T: Default + Clone + Into<Object>>(self) -> Object {
        let mut dst = vec![T::default(); self.count];
        let src = self.ptr as *mut T;
        unsafe {
            copy_nonoverlapping(src, dst.as_mut_ptr(), self.count);
        }
        let mut iter = dst.into_iter().map(|n| n.into());
        if self.count == 1 {
            iter.next().unwrap_or_default()
        } else {
            let arr = iter.collect();
            Object::Array(arr)
        }
    }
    fn into_string_object(self, ansi: bool, char: bool) -> Object {
        let s = if ansi {
            let mut dst = vec![0u8; self.count];
            let src = self.ptr as *mut u8;
            unsafe {
                copy_nonoverlapping(src, dst.as_mut_ptr(), self.count);
            }
            from_ansi_bytes(&dst)
        } else {
            let mut dst = vec![0u16; self.count];
            let src = self.ptr as *mut u16;
            unsafe {
                copy_nonoverlapping(src, dst.as_mut_ptr(), self.count);
            }
            from_wide_string(&dst)
        };
        let s = if char {
            s
        } else {
            match s.split_once('\0') {
                Some((s, _)) => s.into(),
                None => s,
            }
        };
        s.into()
    }
}

#[derive(Debug)]
enum DllArgVal {
    /// int/long/bool
    Int(i32),
    /// int/long/bool配列
    IntV(Vec<i32>),
    /// uint/dword
    Uint(u32),
    /// uint/dword配列
    UintV(Vec<u32>),
    /// word/wchar
    Word(u16),
    /// word/wchar配列
    WordV(Vec<u16>),
    /// byte/char/boolean
    Byte(u8),
    /// byte/char/boolean配列
    ByteV(Vec<u8>),
    /// longlong
    LongLong(i64),
    /// longlong配列
    LongLongV(Vec<i64>),
    /// float
    Float(f32),
    /// float配列
    FloatV(Vec<f32>),
    /// double
    Double(f64),
    /// double配列
    DoubleV(Vec<f64>),
    /// pointer/hwnd/handle/size
    Size(usize),
    /// pointer/hwnd/handle/size配列
    SizeV(Vec<usize>),

    /// {}による構造体定義
    Struct(StructArg),
    /// ユーザー定義構造体
    UStruct(UStruct),
    /// SafeArray
    SafeArray(SafeArray),
    /// コールバック関数
    CallBack(UCallback),
    /// ポインタ
    ArgValPtr(ArgValPtr),
    /// ぬるぽ
    NullPtr,
}
impl DllArgVal {
    fn check_size<U>(vec: &[U], size: usize) -> EvalResult<()> {
        if size > 0 && size != vec.len() {
            Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArrayArgLengthMismatch))
        } else {
            Ok(())
        }
    }
    fn new_num_arg<T>(r#type: &DllType, size: Option<usize>, is_ref: bool, value: Object) -> EvalResult<Option<NumArg<T>>>
        where T: FromPrimitive
    {
        let arg = match size {
            Some(size) => {
                match value.to_vec::<T>() {
                    Some(vec) => {
                        Self::check_size(&vec, size)?;
                        if is_ref {
                            let ptr = ArgValPtr::new(r#type, vec)?;
                            Some(NumArg::Ptr(ptr))
                        } else {
                            Some(NumArg::Vec(vec))
                        }
                    },
                    None => None,
                }
            },
            None => {
                match value.cast::<T>() {
                    Some(t) => {
                        if is_ref {
                            let ptr = ArgValPtr::new(r#type, vec![t])?;
                            Some(NumArg::Ptr(ptr))
                        } else {
                            Some(NumArg::Num(t))
                        }
                    },
                    None => None,
                }
            },
        };
        Ok(arg)
    }
    fn new(r#type: &DllType, size: Option<usize>, is_ref: bool, value: Option<Object>) -> EvalResult<Self> {
        let value = value
            .ok_or(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch))?;
        let value_type = value.get_type();
        let argval = match r#type {
            DllType::Int |
            DllType::Long |
            DllType::Bool => {
                    Self::new_num_arg::<i32>(r#type, size, is_ref, value)?
                    .map(|arg| match arg {
                        NumArg::Num(n) => Self::Int(n),
                        NumArg::Vec(v) => Self::IntV(v),
                        NumArg::Ptr(p) => Self::ArgValPtr(p),
                    })
            },
            DllType::Uint |
            DllType::Dword => {
                Self::new_num_arg::<u32>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Uint(n),
                    NumArg::Vec(v) => Self::UintV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Word => {
                Self::new_num_arg::<u16>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Word(n),
                    NumArg::Vec(v) => Self::WordV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Wchar => {
                let wide = match value.to_string_nullable() {
                    Some(s) => to_wide_string(&s),
                    None => {
                        let size = size.unwrap_or(1);
                        vec![0u16; size]
                    },
                };
                if is_ref {
                    let ptr = ArgValPtr::new(r#type, wide)?;
                    Some(Self::ArgValPtr(ptr))
                } else {
                    Some(Self::WordV(wide))
                }
            },
            DllType::Byte |
            DllType::Boolean => {
                Self::new_num_arg::<u8>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Byte(n),
                    NumArg::Vec(v) => Self::ByteV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Char => {
                let ansi = match value.to_string_nullable() {
                    Some(s) => to_ansi_bytes(&s),
                    None => {
                        let size = size.unwrap_or(1);
                        vec![0u8; size]
                    },
                };
                if is_ref {
                    let ptr = ArgValPtr::new(r#type, ansi)?;
                    Some(Self::ArgValPtr(ptr))
                } else {
                    Some(Self::ByteV(ansi))
                }
            },
            DllType::Longlong => {
                Self::new_num_arg::<i64>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::LongLong(n),
                    NumArg::Vec(v) => Self::LongLongV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Float => {
                Self::new_num_arg::<f32>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Float(n),
                    NumArg::Vec(v) => Self::FloatV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Double => {
                Self::new_num_arg::<f64>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Double(n),
                    NumArg::Vec(v) => Self::DoubleV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::Hwnd |
            DllType::Handle |
            DllType::Pointer |
            DllType::Size => {
                Self::new_num_arg::<usize>(r#type, size, is_ref, value)?
                .map(|arg| match arg {
                    NumArg::Num(n) => Self::Size(n),
                    NumArg::Vec(v) => Self::SizeV(v),
                    NumArg::Ptr(p) => Self::ArgValPtr(p),
                })
            },
            DllType::String |
            DllType::Pchar => {
                let ptr = match value.to_string_nullable() {
                    Some(s) => {
                        let mut ansi = to_ansi_bytes(&s);
                        if let Some(size) = size {
                            if ansi.len() > size {
                                return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllStringArgToLarge(size, ansi.len())));
                            } else {
                                ansi.resize(size, 0);
                            }
                        }
                        ArgValPtr::new(r#type, ansi)?
                    },
                    None => ArgValPtr::null_ptr(),
                };
                Some(Self::ArgValPtr(ptr))
            },
            DllType::Wstring |
            DllType::PWchar => {
                let ptr = match value.to_string_nullable() {
                    Some(s) => {
                        let mut wide = to_wide_string(&s);
                        if let Some(size) = size {
                            if wide.len() > size {
                                return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllStringArgToLarge(size, wide.len())));
                            } else {
                                wide.resize(size, 0);
                            }
                        }
                        ArgValPtr::new(r#type, wide)?
                    },
                    None => ArgValPtr::null_ptr(),
                };
                Some(Self::ArgValPtr(ptr))
            },
            DllType::UStruct => {
                match value {
                    Object::UStruct(ust) => Some(Self::UStruct(ust)),
                    Object::Null => Some(Self::NullPtr),
                    _ => None
                }
            }
            DllType::SafeArray => {
                if let Object::Array(_) = &value {
                    let sa = SafeArray::try_from(value)?;
                    Some(Self::SafeArray(sa))
                } else {
                    None
                }
            },
            DllType::CallBack => None,
            DllType::Void => None,
        };
        match argval {
            Some(v) => Ok(v),
            None => if size.is_some() {
                Err(UError::new(
                    UErrorKind::DllFuncError,
                    UErrorMessage::DllArrayArgTypeMismatch(r#type.to_string())
                ))
            } else {
                Err(UError::new(
                    UErrorKind::DllFuncError,
                    UErrorMessage::DllArgTypeMismatch(r#type.to_string(), value_type)
                ))
            },
        }
    }
    /// var/refされたときに返すオブジェクトを得る
    fn into_object(self) -> Option<Object> {
        match self {
            DllArgVal::ArgValPtr(p) => {
                let obj = match &p.r#type {
                    DllType::Int |
                    DllType::Long => p.into_object::<i32>(),
                    DllType::Bool => p.into_object::<i32>().into_bool_obj(),
                    DllType::Uint |
                    DllType::Dword => p.into_object::<u32>(),
                    DllType::Hwnd |
                    DllType::Handle |
                    DllType::Size |
                    DllType::Pointer => p.into_object::<usize>(),
                    DllType::Float => p.into_object::<f32>(),
                    DllType::Double => p.into_object::<f64>(),
                    DllType::Word => p.into_object::<u16>(),
                    DllType::Byte => p.into_object::<u8>(),
                    DllType::Boolean => p.into_object::<u8>().into_bool_obj(),
                    DllType::Longlong => p.into_object::<i64>(),
                    DllType::Char |
                    DllType::Pchar => p.into_string_object(true, true),
                    DllType::String => p.into_string_object(true, false),
                    DllType::Wchar |
                    DllType::PWchar => p.into_string_object(false, true),
                    DllType::Wstring => p.into_string_object(false, false),
                    DllType::CallBack |
                    DllType::Void => Object::Empty,
                    // ここには来ないはず
                    DllType::SafeArray => todo!(),
                    DllType::UStruct => todo!(),
                };
                Some(obj)
            },
            DllArgVal::SafeArray(sa) => sa.to_object().ok(),
            _ => None
        }
    }
}
enum NumArg<T> {
    Num(T),
    Vec(Vec<T>),
    Ptr(ArgValPtr)
}

#[derive(Debug)]
struct StructArg {
    ustruct: UStruct,
}
impl StructArg {
    fn set_values(params: &[DllParam], ustruct: &mut UStruct, iter_args: &mut IntoIter<(Option<Expression>, Object)>) -> EvalResult<()> {
        for (index, param) in params.iter().enumerate() {
            match param {
                DllParam::Param { dll_type:_, is_ref:_, size:_ } => {
                    if let Some((refexpr, value)) = iter_args.next() {
                        ustruct.set_by_index(index, value, refexpr)
                            .map_err(|e|
                                if e.kind == UErrorKind::UnknownError {
                                    UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch)
                                } else {
                                    e
                                }
                            )?;
                    } else {
                        return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch));
                    }
                },
                DllParam::Struct(subparams) => {
                    if let Some(member) = ustruct.get_member_mut(index) {
                        if let Some(ust) = member.get_ustruct_mut() {
                            Self::set_values(subparams, ust, iter_args)?;
                            continue;
                        }
                    }
                    return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch));
                },
                DllParam::Callback(_, _) => {
                    todo!()
                }
            }
        }
        Ok(())
    }
    fn from(params: &Vec<DllParam>, iter_args: &mut IntoIter<(Option<Expression>, Object)>) -> EvalResult<Self> {

        let sdef = StructDef::from(params);
        let mut ustruct = UStruct::try_from(&sdef)?;
        Self::set_values(params, &mut ustruct, iter_args)?;

        Ok(StructArg { ustruct })

    }
    fn assign(&self, e: &mut Evaluator) -> EvalResult<()> {
        Self::assign_member_values(&self.ustruct, e)
    }
    fn assign_member_values(ustruct: &UStruct, e: &mut Evaluator) -> EvalResult<()> {
        for member in ustruct.get_members() {
            if let Some(ust) = member.get_ustruct() {
                Self::assign_member_values(ust, e)?;
            } else if let Some(expr) = &member.refexpr {
                let value = ustruct.get(member)?;
                e.eval_assign_expression(expr.clone(), value)?;
            }
        }
        Ok(())
    }
}

impl From<&Vec<DllParam>> for StructDef {
    fn from(params: &Vec<DllParam>) -> Self {
        let members = params.iter()
            .map(|param| {
                match param {
                    DllParam::Param { dll_type, is_ref, size } => {
                        let member_type = MemberType::from(dll_type);
                        (String::default(), member_type, *size, *is_ref)
                    },
                    DllParam::Struct(params) => {
                        let sdef = Self::from(params);
                        (String::default(), MemberType::UStruct(sdef), None, false)
                    },
                    DllParam::Callback(_, _) => {
                        todo!()
                    }
                }
            })
            .collect();
        let memberdef = MemberDefVec(members);
        Self::new(String::default(), memberdef)
    }
}

impl Object {
    fn cast<T: FromPrimitive>(&self) -> Option<T> {
        let n = self.as_f64(true)?;
        T::from_f64(n)
    }
    fn to_vec<T: FromPrimitive>(&self) -> Option<Vec<T>> {
        match self {
            Object::Array(arr) => {
                arr.iter()
                    .map(|o| o.as_f64(true))
                    .map(|n| match n {
                        Some(n) => T::from_f64(n),
                        None => None,
                    })
                    .collect()
            },
            o => {
                let n = o.as_f64(true)?;
                let t = T::from_f64(n)?;
                Some(vec![t])
            },
        }
    }
    fn into_bool_obj(self) -> Object {
        if let Object::Array(arr) = self {
            let arr = arr.into_iter().map(|o| o.is_truthy().into()).collect();
            Object::Array(arr)
        } else {
            self.is_truthy().into()
        }
    }
}

#[derive(Debug)]
struct UCallback {
    user_func: UserFunc,
    rtype: DllType,
}
impl UCallback {
    fn new(user_func: UserFunc, rtype: DllType) -> EvalResult<Self> {
        match &rtype {
            DllType::String |
            DllType::Wstring |
            DllType::Char |
            DllType::Pchar |
            DllType::Wchar |
            DllType::PWchar |
            DllType::SafeArray |
            DllType::UStruct |
            DllType::CallBack => {
                Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::InvalidCallbackReturnType(rtype)))
            },
            _ => {
                Ok(Self { user_func, rtype })
            }
        }
    }
    fn get_closure(&mut self) -> Closure {
        self.user_func.get_closure(&self.rtype)
    }
    unsafe extern "C" fn callback_u8(_cif: &libffi::low::ffi_cif, result: &mut u8, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<u8>(args) };
    }
    unsafe extern "C" fn callback_u16(_cif: &libffi::low::ffi_cif, result: &mut u16, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<u16>(args) };
    }
    unsafe extern "C" fn callback_u32(_cif: &libffi::low::ffi_cif, result: &mut u32, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<u32>(args) };
    }
    unsafe extern "C" fn callback_i32(_cif: &libffi::low::ffi_cif, result: &mut i32, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<i32>(args) };
    }
    unsafe extern "C" fn callback_i64(_cif: &libffi::low::ffi_cif, result: &mut i64, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<i64>(args) };
    }
    unsafe extern "C" fn callback_f32(_cif: &libffi::low::ffi_cif, result: &mut f32, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<f32>(args) };
    }
    unsafe extern "C" fn callback_f64(_cif: &libffi::low::ffi_cif, result: &mut f64, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<f64>(args) };
    }
    unsafe extern "C" fn callback_usize(_cif: &libffi::low::ffi_cif, result: &mut usize, args: *const *const c_void, userdata: &mut UserFunc) {
        *result = unsafe { userdata.invoke::<usize>(args) };
    }
}

#[derive(Debug)]
struct UserFunc {
    evaluator: Evaluator,
    function: Function,
    arg_types: Vec<DllType>,
    result: Option<UError>,
}
impl UserFunc {
    fn new(arg_types: Vec<DllType>, function: Function, evaluator: &mut Evaluator) -> Self {
        Self {
            evaluator: evaluator.clone(),
            function,
            arg_types,
            result: None,
            // _marker: std::marker::PhantomData
        }
    }
    unsafe fn invoke<T>(&mut self, args: *const *const c_void) -> T
        where T: FromPrimitive + Default
    {
        unsafe {
            let len = self.arg_types.len();
            let arg_ptrs = std::slice::from_raw_parts(args, len);
            let arguments = arg_ptrs.iter().zip(self.arg_types.iter())
                .map(|(ptr, t)| (Some(Expression::Callback), Self::ptr_as_object(*ptr, t)))
                .collect();
            match self.function.invoke(&mut self.evaluator, arguments, None) {
                Ok(obj) => {
                    match obj.as_f64(true) {
                        Some(n) => match T::from_f64(n) {
                            Some(t) => t,
                            None => {
                                self.result = Some(UError::new(UErrorKind::DllFuncError, UErrorMessage::CallbackReturnValueCastError));
                                T::from_i32(0).unwrap_or_default()
                            },
                        },
                        None => {
                            self.result = Some(UError::new(UErrorKind::DllFuncError, UErrorMessage::CallbackReturnValueCastError));
                            T::from_i32(0).unwrap_or_default()
                        },
                    }
                },
                Err(err) => {
                    self.result = Some(err);
                    T::from_i32(0).unwrap_or_default()
                },
            }
        }
    }
    unsafe fn ptr_as_object(ptr: *const c_void, t: &DllType) -> Object {
        unsafe {
            match t {
                DllType::Int |
                DllType::Long => {
                    let n: i32 = Self::copy(ptr);
                    n.into()
                },
                DllType::Bool => {
                    let n: i32 = Self::copy(ptr);
                    (n != 0).into()
                },
                DllType::Uint |
                DllType::Dword => {
                    let n: u32 = Self::copy(ptr);
                    n.into()
                },
                DllType::Word => {
                    let n: u16 = Self::copy(ptr);
                    n.into()
                },
                DllType::Byte => {
                    let n: u8 = Self::copy(ptr);
                    n.into()
                },
                DllType::Boolean => {
                    let n: u8 = Self::copy(ptr);
                    (n != 0).into()
                },
                DllType::Float => {
                    let n: f32 = Self::copy(ptr);
                    n.into()
                },
                DllType::Double => {
                    let n: f64 = Self::copy(ptr);
                    n.into()
                },
                DllType::Longlong => {
                    let n: i64 = Self::copy(ptr);
                    n.into()
                },
                DllType::Hwnd |
                DllType::Handle |
                DllType::Pointer |
                DllType::Size => {
                    let n: usize = Self::copy(ptr);
                    n.into()
                },
                DllType::Void => Object::Empty,
                DllType::SafeArray |
                DllType::Char |
                DllType::Wchar |
                DllType::String |
                DllType::Wstring |
                DllType::Pchar |
                DllType::PWchar |
                DllType::UStruct |
                DllType::CallBack => unimplemented!(),
            }
        }
    }
    unsafe fn copy<T: Default>(ptr: *const c_void) -> T {
        let mut dst: T = T::default();
        let src = ptr as *const T;
        unsafe { copy_nonoverlapping(src, &mut dst, 1) };
        dst
    }
    fn get_closure(&mut self, rtype: &DllType) -> Closure {
        let args = self.arg_types.iter().map(Type::from).collect::<Vec<_>>();
        let result = Type::from(rtype);
        let cif = Cif::new(args, result);
        match rtype {
            DllType::Int |
            DllType::Long |
            DllType::Bool => {
                Closure::new_mut(cif, UCallback::callback_i32, self)
            },
            DllType::Uint |
            DllType::Dword => {
                Closure::new_mut(cif, UCallback::callback_u32, self)
            },
            DllType::Word => {
                Closure::new_mut(cif, UCallback::callback_u16, self)
            },
            DllType::Byte |
            DllType::Boolean => {
                Closure::new_mut(cif, UCallback::callback_u8, self)
            },
            DllType::Float => {
                Closure::new_mut(cif, UCallback::callback_f32, self)
            },
            DllType::Double => {
                Closure::new_mut(cif, UCallback::callback_f64, self)
            },
            DllType::Longlong => {
                Closure::new_mut(cif, UCallback::callback_i64, self)
            },
            DllType::Void |
            DllType::Hwnd |
            DllType::Handle |
            DllType::Pointer |
            DllType::Size => {
                Closure::new_mut(cif, UCallback::callback_usize, self)
            },
            _ => {
                // return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::InvalidCallbackReturnType(t.clone())));
                unimplemented!()
            },
        }
    }
}

// impl UserFunc {
//     fn get_closure(&'a mut self, rtype: &DllType) -> Closure<'b> {
//         let args = self.arg_types.iter().map(|t| Type::from(t)).collect::<Vec<_>>();
//         let result = Type::from(rtype);
//         let cif = Cif::new(args, result);
//         match rtype {
//             DllType::Int |
//             DllType::Long |
//             DllType::Bool => {
//                 Closure::new_mut(cif, UCallback::callback_i32, self)
//             },
//             DllType::Uint |
//             DllType::Dword => {
//                 Closure::new_mut(cif, UCallback::callback_u32, self)
//             },
//             DllType::Word => {
//                 Closure::new_mut(cif, UCallback::callback_u16, self)
//             },
//             DllType::Byte |
//             DllType::Boolean => {
//                 Closure::new_mut(cif, UCallback::callback_u8, self)
//             },
//             DllType::Float => {
//                 Closure::new_mut(cif, UCallback::callback_f32, self)
//             },
//             DllType::Double => {
//                 Closure::new_mut(cif, UCallback::callback_f64, self)
//             },
//             DllType::Longlong => {
//                 Closure::new_mut(cif, UCallback::callback_i64, self)
//             },
//             DllType::Void |
//             DllType::Hwnd |
//             DllType::Handle |
//             DllType::Pointer |
//             DllType::Size => {
//                 Closure::new_mut(cif, UCallback::callback_usize, self)
//             },
//             _t => {
//                 // return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::InvalidCallbackReturnType(_t.clone())));
//                 unimplemented!()
//             },
//         }
//     }

// }