use crate::ast::{DllType};
use crate::evaluator::{EvalResult, UError};
use crate::evaluator::object::Object;
use crate::winapi::{
    to_ansi_bytes, to_wide_string, from_ansi_bytes,
};
use libffi::middle::{Arg, arg};
use std::ffi::c_void;
use cast;

#[derive(Debug)]
pub enum DllArg {
    Int(i32), // int, long, bool,
    IntArray(Vec<i32>), // int, long, bool,
    Uint(u32), // uint, dword
    UintArray(Vec<u32>), // uint, dword
    Hwnd(isize),
    HwndArray(Vec<isize>),
    Float(f32),
    FloatArray(Vec<f32>),
    Double(f64),
    DoubleArray(Vec<f64>),
    Word(u16), // word, wchar
    WordArray(Vec<u16>), // word, wchar
    Byte(u8), // byte, char, boolean
    ByteArray(Vec<u8>), // byte, char, boolean
    LongLong(i64),
    LongLongArray(Vec<i64>),
    String(Vec<u8>, bool), // string, pchar boolはnullで切るかどうか
    WString(Vec<u16>, bool), // wstring, wpchar boolはnullで切るかどうか
    Pointer(usize),
    Struct(*mut c_void),
    SafeArray,
    Null, // null
}

impl DllArg {
    pub fn new(obj: &Object, dll_type: &DllType) -> Result<Self, String> {
        let dll_arg = match obj {
            Object::Array(_) => return Err("array".into()),
            Object::Num(n) => match dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => Self::Int(*n as i32),
                DllType::Uint |
                DllType::Dword => Self::Uint(*n as u32),
                DllType::Hwnd => Self::Hwnd(*n as isize),
                DllType::Float => Self::Float(*n as f32),
                DllType::Double => Self::Double(*n),
                DllType::Word |
                DllType::Wchar => Self::Word(*n as u16),
                DllType::Byte |
                DllType::Char => Self::Byte(*n as u8),
                DllType::Longlong => Self::LongLong(*n as i64),
                DllType::String => {
                    let s = to_ansi_bytes(&format!("{}", n));
                    Self::String(s, true)
                },
                DllType::Pchar => {
                    let s = to_ansi_bytes(&format!("{}", n));
                    Self::String(s, false)
                },
                DllType::Wstring => {
                    let s = to_wide_string(&format!("{}", n));
                    Self::WString(s, true)
                },
                DllType::PWchar => {
                    let s = to_wide_string(&format!("{}", n));
                    Self::WString(s, false)
                },
                DllType::Pointer => Self::Pointer(*n as usize),
                _ => return Err("number".into())
            },
            Object::Empty => match dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => Self::Int(0),
                DllType::Uint |
                DllType::Dword => Self::Uint(0),
                DllType::Hwnd => Self::Hwnd(0),
                DllType::Float => Self::Float(0.0),
                DllType::Double => Self::Double(0.0),
                DllType::Word |
                DllType::Wchar => Self::Word(0),
                DllType::Byte |
                DllType::Char => Self::Byte(0),
                DllType::Longlong => Self::LongLong(0),
                DllType::String => {
                    let s = to_ansi_bytes("");
                    Self::String(s, true)
                },
                DllType::Pchar => {
                    let s = to_ansi_bytes("");
                    Self::String(s, false)
                },
                DllType::Wstring => {
                    let s = to_wide_string("");
                    Self::WString(s, true)
                },
                DllType::PWchar => {
                    let s = to_wide_string("");
                    Self::WString(s, false)
                },
                _ => return Err("EMPTY".into())
            },
            Object::Null => match dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => Self::Int(0),
                DllType::Uint |
                DllType::Dword => Self::Uint(0),
                DllType::Hwnd => Self::Hwnd(0),
                DllType::Float => Self::Float(0.0),
                DllType::Double => Self::Double(0.0),
                DllType::Word |
                DllType::Wchar => Self::Word(0),
                DllType::Byte |
                DllType::Char => Self::Byte(0),
                DllType::Longlong => Self::LongLong(0),
                DllType::String |
                DllType::Pchar |
                DllType::Wstring |
                DllType::PWchar => Self::Null,
                _ => return Err("NULL".into())
            },
            Object::String(ref s) => match dll_type {
                DllType::String => {
                    let s = to_ansi_bytes(s);
                    Self::String(s, true)
                },
                DllType::Pchar => {
                    let s = to_ansi_bytes(s);
                    Self::String(s, false)
                },
                DllType::Wstring => {
                    let s = to_wide_string(s);
                    Self::WString(s, true)
                },
                DllType::PWchar => {
                    let s = to_wide_string(s);
                    Self::WString(s, false)
                },
                _ => return Err("string".into())
            },
            Object::Bool(b) => match dll_type {
                DllType::Bool => Self::Int(*b as i32),
                DllType::Boolean => Self::Byte(*b as u8),
                _ => return Err("bool".into())
            },
            o => return Err(format!("{}", o))
        };
        Ok(dll_arg)
    }

    pub fn new_array(obj: &Object, dll_type: &DllType) -> EvalResult<Self> {
        let dll_arg = match obj {
            Object::Array(arr) => match dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => {
                    let v = object_vec_to_premitive_vec::<i32>(arr)?;
                    DllArg::IntArray(v)
                },
                DllType::Uint |
                DllType::Dword => {
                    let v = object_vec_to_premitive_vec::<u32>(arr)?;
                    DllArg::UintArray(v)
                },
                DllType::Hwnd => {
                    let v = object_vec_to_premitive_vec::<isize>(arr)?;
                    DllArg::HwndArray(v)
                },
                DllType::Float => {
                    let v = object_vec_to_premitive_vec::<f32>(arr)?;
                    DllArg::FloatArray(v)
                },
                DllType::Double => {
                    let v = object_vec_to_f64_vec(arr)?;
                    DllArg::DoubleArray(v)
                },
                DllType::Word |
                DllType::Wchar => {
                    let v = object_vec_to_premitive_vec::<u16>(arr)?;
                    DllArg::WordArray(v)
                },
                DllType::Byte |
                DllType::Char |
                DllType::Boolean => {
                    let v = object_vec_to_premitive_vec::<u8>(arr)?;
                    DllArg::ByteArray(v)
                },
                DllType::Longlong => {
                    let v = object_vec_to_premitive_vec::<i64>(arr)?;
                    DllArg::LongLongArray(v)
                },
                _ => return Err(UError::default())
            },
            _ => return Err(UError::default())
        };
        Ok(dll_arg)
    }

    pub fn to_arg(&self) -> Arg {
        match self {
            DllArg::Int(v) => arg(v),
            DllArg::IntArray(v) => arg(v),
            DllArg::Uint(v) => arg(v),
            DllArg::UintArray(v) => arg(v),
            DllArg::Hwnd(v) => arg(v),
            DllArg::HwndArray(v) => arg(v),
            DllArg::Float(v) => arg(v),
            DllArg::FloatArray(v) => arg(v),
            DllArg::Double(v) => arg(v),
            DllArg::DoubleArray(v) => arg(v),
            DllArg::Word(v) => arg(v),
            DllArg::WordArray(v) => arg(v),
            DllArg::Byte(v) => arg(v),
            DllArg::ByteArray(v) => arg(v),
            DllArg::LongLong(v) => arg(v),
            DllArg::LongLongArray(v) => arg(v),
            DllArg::String(v, _) => arg(v),
            DllArg::WString(v, _) => arg(v),
            DllArg::Struct(v) => arg(v),
            DllArg::Pointer(v) => arg(v),
            DllArg::SafeArray => arg(&0),
            DllArg::Null => arg(&0),
        }
    }

    pub fn to_object(&self) -> Object {
        match self {
            DllArg::Int(v) => Object::Num(*v as f64),
            DllArg::IntArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::Uint(v) => Object::Num(*v as f64),
            DllArg::UintArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::Hwnd(v) => Object::Num(*v as f64),
            DllArg::HwndArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::String(v, is_null_end) => {
                let str = from_ansi_bytes(v);
                if *is_null_end {
                    let null_end_str = str.split("\0").collect::<Vec<&str>>();
                    Object::String(null_end_str[0].to_string())
                } else {
                    Object::String(str)
                }
            },
            DllArg::WString(ref v, is_null_end) => {
                let str = String::from_utf16_lossy(v);
                if *is_null_end {
                    let null_end_str = str.split("\0").collect::<Vec<&str>>();
                    Object::String(null_end_str[0].to_string())
                } else {
                    Object::String(str)
                }
            },
            DllArg::Float(v) => Object::Num(*v as f64),
            DllArg::FloatArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::Double(v) => Object::Num(*v as f64),
            DllArg::DoubleArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::Word(v) => Object::Num(*v as f64),
            DllArg::WordArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::Byte(v) => Object::Num(*v as f64),
            DllArg::ByteArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::LongLong(v) => Object::Num(*v as f64),
            DllArg::LongLongArray(v) => {
                let arr = v.into_iter().map(|n|  Object::Num(*n as f64)).collect::<Vec<Object>>();
                Object::Array(arr)
            },
            DllArg::SafeArray => Object::Null,
            DllArg::Null => Object::Null,
            DllArg::Pointer(v) => Object::Num(*v as f64),
            DllArg::Struct(_) => Object::Null
        }
    }
}

fn object_vec_to_premitive_vec<T>(vec: &Vec<Object>) -> EvalResult<Vec<T>>
    where T: cast::From<f64, Output=Result<T, cast::Error>>
{
    let mut result = Vec::<T>::new();
    for obj in vec {
        match obj {
            Object::Num(n) => result.push(T::cast(*n)?),
            Object::Bool(b) => if *b {
                result.push(T::cast(1.0)?)
            } else {
                result.push(T::cast(0.0)?)
            },
            Object::Empty |
            Object::Null => result.push(T::cast(0.0)?),
            _ => return Err(UError::default())
        }
    }
    Ok(result)
}
fn object_vec_to_f64_vec(vec: &Vec<Object>) -> EvalResult<Vec<f64>> {
    let mut result = Vec::new();
    for obj in vec {
        match obj {
            Object::Num(n) => result.push(*n),
            Object::Bool(b) => if *b {
                result.push(1.0)
            } else {
                result.push(0.0)
            },
            Object::Empty |
            Object::Null => result.push(0.0),
            _ => return Err(UError::default())
        }
    }
    Ok(result)
}