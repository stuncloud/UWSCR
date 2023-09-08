use crate::ast::{DllType, DefDllParam, Expression};
use crate::evaluator::{Evaluator, EvalResult, UError, UErrorKind, UErrorMessage};
use crate::error::evaluator::DefinitionType;
use crate::evaluator::object::{
    Object,
    ustruct::{StructDef, MemberDefVec, UStruct, MemberType}
};
use crate::winapi::{
    to_ansi_bytes, to_wide_string, from_ansi_bytes, from_wide_string,
};
use libffi::middle::{Arg, Type, Cif, CodePtr};
use std::vec::IntoIter;
use std::ffi::c_void;
use std::ptr::copy_nonoverlapping;
use cast;

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::System::Memory::{
    HeapCreate, HeapAlloc, HeapFree,
    HEAP_GENERATE_EXCEPTIONS, HEAP_ZERO_MEMORY, HEAP_NONE,
    HeapHandle, HeapDestroy,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DefDll {
    name: String,
    path: String,
    params: Vec<DefDllParam>,
    rtype: DllType,
}
impl std::fmt::Display for DefDll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params = self.params.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{}({}):{}:{}", self.name, params, self.rtype, self.path)
    }
}
impl DefDll {
    pub fn new(name: String, path: String, params: Vec<DefDllParam>, rtype: DllType) -> EvalResult<Self> {
        match rtype {
            DllType::SafeArray |
            DllType::UStruct |
            DllType::CallBack => Err(UError::new(UErrorKind::DefinitionError(DefinitionType::DefDll), UErrorMessage::DllResultTypeNotAllowed)),
            rtype => Ok(Self { name, path, params, rtype })
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
            let dllargs = DllArgs::new(&self.params, arguments)?;

            // 実際に渡す引数とその型情報
            let (args, types): (Vec<Arg>, Vec<Type>) = dllargs.as_args_and_types().into_iter().unzip();

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

            let result = match self.rtype {
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
                DllType::Void => Object::Empty,
                // 戻り値型として使用不可
                DllType::SafeArray |
                DllType::UStruct |
                DllType::CallBack => Object::Empty,
            };

            /* 参照渡し */
            for arg in dllargs.args {
                // エラーは握りつぶす
                let _ = arg.assign(e);
            }

            Ok(result)
        }
    }
    unsafe fn string_from_ansi_ptr(ptr: *const u8) -> String {
        let pcstr = PCSTR::from_raw(ptr);
        let ansi = pcstr.as_bytes();
        from_ansi_bytes(ansi)
    }
    unsafe fn string_from_wide_ptr(ptr: *const u16) -> String {
        let pcwstr = PCWSTR::from_raw(ptr);
        let wide = pcwstr.as_wide();
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
    args: Vec<DllArg>
}
impl DllArgs {
    fn new(params: &Vec<DefDllParam>, arguments: Vec<(Option<Expression>, Object)>) -> EvalResult<Self> {
        let mut iter_args = arguments.into_iter();
        let args = params.iter()
            .map(|param| DllArg::new(param, &mut iter_args) )
            .collect::<EvalResult<Vec<DllArg>>>()?;
        Ok(Self { args })
    }
    fn as_args_and_types(&self) -> Vec<(Arg, Type)> {
        self.args.iter()
            .map(|arg| arg.to_arg_and_type() )
            .collect()
    }
}

#[derive(Debug)]
struct DllArg {
    refexpr: Option<Expression>,
    value: DllArgVal,
}
impl DllArg {
    fn new(param: &DefDllParam, iter_args: &mut IntoIter<(Option<Expression>, Object)>) -> EvalResult<Self> {
        match param {
            DefDllParam::Param { dll_type, is_ref, size } => {
                let (expr, value) = iter_args.next()
                        .ok_or(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch))?;
                let value = DllArgVal::new(dll_type, *size, *is_ref, Some(value))?;
                let refexpr = if *is_ref { expr } else { None };
                Ok(Self { refexpr, value })
            },
            DefDllParam::Struct(params) => {
                let sarg = StructArg::from(params, iter_args)?;
                let value = DllArgVal::Struct(sarg);
                Ok(Self { refexpr: None, value })
            },
        }
    }
    fn to_arg_and_type(&self) -> (Arg, Type) {
        let t = Type::from(self);
        let arg = match &self.value {
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
                // let ptr = ust.as_ptr();
                // println!("\u{001b}[36m[debug] ptr: {ptr:?}\u{001b}[0m");
                Arg::new(&ust.address)
            },
            DllArgVal::NullPtr => Arg::new(&0),
            DllArgVal::SafeArray => todo!(),
            DllArgVal::CallBack => todo!(),
            DllArgVal::ArgValPtr(p) => Arg::new(&p.ptr),
        };
        (arg, t)
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
impl From<&DllArg> for Type {
    fn from(arg: &DllArg) -> Self {
        match &arg.value {
            DllArgVal::Int(_) |
            DllArgVal::IntV(_) => Self::i32(),
            DllArgVal::Uint(_) |
            DllArgVal::UintV(_) => Self::u32(),
            DllArgVal::Word(_) |
            DllArgVal::WordV(_) => Self::u16(),
            DllArgVal::Byte(_) |
            DllArgVal::ByteV(_) => Self::u8(),
            DllArgVal::LongLong(_) |
            DllArgVal::LongLongV(_) => Self::i64(),
            DllArgVal::Float(_) |
            DllArgVal::FloatV(_) => Self::f32(),
            DllArgVal::Double(_) |
            DllArgVal::DoubleV(_) => Self::f64(),
            DllArgVal::Size(_) |
            DllArgVal::SizeV(_) => Self::usize(),
            // DllArgVal::String(_) |
            // DllArgVal::PChar(_) |
            // DllArgVal::WString(_) |
            // DllArgVal::PWChar(_) => Self::pointer(),
            DllArgVal::Struct(_) |
            DllArgVal::UStruct(_) => Self::usize(),
            DllArgVal::NullPtr => Self::usize(),
            DllArgVal::SafeArray => todo!(),
            DllArgVal::CallBack => todo!(),
            DllArgVal::ArgValPtr(_) => Self::pointer(),
        }
        // if arg.refexpr.is_some() {
        //     Self::pointer()
        // } else {
        // }
    }
}

#[derive(Debug)]
struct ArgValPtr {
    ptr: *mut c_void,
    hheap: HeapHandle,
    count: usize,
    r#type: DllType,
}
impl Drop for ArgValPtr {
    fn drop(&mut self) {
        unsafe {
            HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            HeapDestroy(self.hheap);
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
            hheap: HeapHandle(0),
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
    SafeArray,
    /// コールバック関数
    CallBack,
    /// ポインタ
    ArgValPtr(ArgValPtr),
    /// ぬるぽ
    NullPtr,
}
impl DllArgVal {
    fn check_size<U>(vec: &Vec<U>, size: usize) -> EvalResult<()> {
        if size > 0 && size != vec.len() {
            Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArrayArgLengthMismatch))
        } else {
            Ok(())
        }
    }
    fn new_num_arg<T>(r#type: &DllType, size: Option<usize>, is_ref: bool, value: Object) -> EvalResult<Option<NumArg<T>>>
        where T: cast::From<f64, Output = Result<T, cast::Error>>
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
    fn new_f64_arg(r#type: &DllType, size: Option<usize>, is_ref: bool, value: Object) -> EvalResult<Option<Self>> {
        let arg = match size {
            Some(size) => {
                match value.to_vecf64() {
                    Some(vec) => {
                        Self::check_size(&vec, size)?;
                        if is_ref {
                            let ptr = ArgValPtr::new(r#type, vec)?;
                            Some(Self::ArgValPtr(ptr))
                        } else {
                            Some(Self::DoubleV(vec))
                        }
                    },
                    None => None,
                }
            },
            None => {
                match value.as_f64(true) {
                    Some(t) => {
                        if is_ref {
                            let ptr = ArgValPtr::new(r#type, vec![t])?;
                            Some(Self::ArgValPtr(ptr))
                        } else {
                            Some(Self::Double(t))
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
            DllType::Double => Self::new_f64_arg(r#type, size, is_ref, value)?,
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
            DllType::SafeArray => todo!(),
            DllType::CallBack => todo!(),
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
    fn into_object(self) -> Option<Object> {
        let p = match self {
            DllArgVal::ArgValPtr(p) => Some(p),
            _ => None
        }?;
        let obj = match &p.r#type {
            DllType::Int |
            DllType::Long => p.into_object::<i32>(),
            DllType::Bool => p.into_object::<i32>().to_bool_obj(),
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
            DllType::Boolean => p.into_object::<u8>().to_bool_obj(),
            DllType::Longlong => p.into_object::<i64>(),
            DllType::Char |
            DllType::Pchar => p.into_string_object(true, true),
            DllType::String => p.into_string_object(true, false),
            DllType::Wchar |
            DllType::PWchar => p.into_string_object(false, true),
            DllType::Wstring => p.into_string_object(false, false),
            DllType::SafeArray => todo!(),
            DllType::Void => todo!(),
            DllType::CallBack => todo!(),
            // ここには来ないはず
            DllType::UStruct => todo!(),
        };
        Some(obj)
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
    fn set_values(params: &Vec<DefDllParam>, ustruct: &mut UStruct, iter_args: &mut IntoIter<(Option<Expression>, Object)>) -> EvalResult<()> {
        for (index, param) in params.iter().enumerate() {
            match param {
                DefDllParam::Param { dll_type:_, is_ref:_, size:_ } => {
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
                DefDllParam::Struct(subparams) => {
                    if let Some(member) = ustruct.get_member_mut(index) {
                        if let Some(ust) = member.get_ustruct_mut() {
                            Self::set_values(subparams, ust, iter_args)?;
                            continue;
                        }
                    }
                    return Err(UError::new(UErrorKind::DllFuncError, UErrorMessage::DllArgCountMismatch));
                },
            }
        }
        Ok(())
    }
    fn from(params: &Vec<DefDllParam>, iter_args: &mut IntoIter<(Option<Expression>, Object)>) -> EvalResult<Self> {

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
            } else {
                if let Some(expr) = &member.refexpr {
                    let value = ustruct.get(member)?;
                    e.eval_assign_expression(expr.clone(), value)?;
                }
            }
        }
        Ok(())
    }
}

impl From<&Vec<DefDllParam>> for StructDef {
    fn from(params: &Vec<DefDllParam>) -> Self {
        let members = params.iter()
            .map(|param| {
                match param {
                    DefDllParam::Param { dll_type, is_ref: _, size } => {
                        let member_type = MemberType::from(dll_type);
                        (String::default(), member_type, *size)
                    },
                    DefDllParam::Struct(params) => {
                        let sdef = Self::from(params);
                        (String::default(), MemberType::UStruct(sdef), None)
                    },
                }
            })
            .collect();
        let memberdef = MemberDefVec(members);
        Self::new(String::default(), memberdef)
    }
}

impl Object {
    fn cast<T: cast::From<f64, Output=Result<T, cast::Error>>>(&self) -> Option<T> {
        let n = self.as_f64(true)?;
        T::cast(n).ok()
    }
    fn to_vec<T: cast::From<f64, Output=Result<T, cast::Error>>>(&self) -> Option<Vec<T>> {
        match self {
            Object::Array(arr) => {
                arr.iter()
                    .map(|o| o.as_f64(true))
                    .map(|n| match n {
                        Some(n) => T::cast(n).ok(),
                        None => None,
                    })
                    .collect()
            },
            o => {
                let n = o.as_f64(true)?;
                let t = T::cast(n).ok()?;
                Some(vec![t])
            },
        }
    }
    fn to_vecf64(&self) -> Option<Vec<f64>> {
        match self {
            Object::Array(arr) => {
                arr.iter()
                    .map(|o| o.as_f64(true))
                    .collect()
            },
            o => {
                let n = o.as_f64(true)?;
                Some(vec![n])
            },
        }
    }
    fn to_bool_obj(self) -> Object {
        if let Object::Array(arr) = self {
            let arr = arr.into_iter().map(|o| o.is_truthy().into()).collect();
            Object::Array(arr)
        } else {
            self.is_truthy().into()
        }
    }
}
