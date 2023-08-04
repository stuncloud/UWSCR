use super::Object;
use super::super::{EvalResult, Evaluator};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::winapi::{
    to_ansi_bytes, from_ansi_bytes, to_wide_string, from_wide_string,
};

use std::ffi::c_void;
use std::{mem, ptr};
use std::sync::{Arc, Mutex};

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::{
    System::{
        Memory::{
            HeapHandle,
            HeapAlloc, HeapFree, HeapCreate, HeapDestroy,
            HEAP_ZERO_MEMORY, HEAP_NONE, HEAP_GENERATE_EXCEPTIONS,
        }
    }
};


/// 構造体定義
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub size: usize,
    members: Vec<MemberDef>
}
#[derive(Debug, Clone, PartialEq)]
pub struct MemberDef {
    name: String,
    r#type: MemberType,
    /// 配列指定であればそのサイズ
    len: Option<usize>,
}
impl StructDef {
    pub fn new(name: String, members: Vec<(String, String, Option<usize>)>, e: &mut Evaluator) -> EvalResult<Self> {
        let members = members.into_iter()
            .map(|(name, t, len)| MemberDef::mew(name, &t, len, e))
            .collect::<EvalResult<Vec<_>>>()?;
        let size = members.iter()
            .map(|m| m.size())
            .reduce(|a, b| a + b)
            .unwrap_or_default();
        Ok(Self {name, size, members})
    }
}
impl MemberDef {
    fn size(&self) -> usize {
        match &self.r#type {
            MemberType::String |
            MemberType::Pchar |
            MemberType::Wstring |
            MemberType::PWchar => mem::size_of::<usize>(),
            MemberType::UStruct(sdef) => sdef.size,
            t => {
                t.size() * self.len.unwrap_or(1)
            },
        }
    }
    fn mew(name: String, t: &str, len: Option<usize>, e: &mut Evaluator) -> EvalResult<Self> {
        let r#type = match t {
            "int" => MemberType::Int,
            "long" => MemberType::Long,
            "bool" => MemberType::Bool,
            "uint" => MemberType::Uint,
            "hwnd" => MemberType::Hwnd,
            "string" => MemberType::String,
            "wstring" => MemberType::Wstring,
            "float" => MemberType::Float,
            "double" => MemberType::Double,
            "word" => MemberType::Word,
            "dword" => MemberType::Dword,
            "byte" => MemberType::Byte,
            "char" => MemberType::Char,
            "pchar" => MemberType::Pchar,
            "wchar" => MemberType::Wchar,
            "pwchar" => MemberType::PWchar,
            "boolean" => MemberType::Boolean,
            "longlong" => MemberType::Longlong,
            "pointer" => MemberType::Pointer,
            other => {
                if let Some(Object::StructDef(sdef)) = e.env.get_struct(other) {
                    MemberType::UStruct(sdef)
                } else {
                    let err = UError::new(UErrorKind::StructDefError, UErrorMessage::UnknownDllType(other.into()));
                    return Err(err);
                }
            },
        };
        Ok(Self {name, r#type, len})
    }
}
impl std::fmt::Display for StructDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let members = self.members.iter()
            .map(|m| m.to_string())
            .reduce(|a, b| format!("{a}, {b}"))
            .unwrap_or_default();
        write!(f, "{} {{{}}}", self.name, members)
    }
}
impl std::fmt::Display for MemberDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.len {
            Some(l) => write!(f, "{}: {}[{}]", self.name, self.r#type, l),
            None => write!(f, "{}: {}", self.name, self.r#type)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringBuffer {
    ptr: *mut c_void,
    hheap: HeapHandle,
    len :usize,
    ansi: bool,
}
impl Drop for StringBuffer {
    fn drop(&mut self) {
        unsafe {
            HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            HeapDestroy(self.hheap);
        }
    }
}
impl StringBuffer {
    fn new(len: Option<usize>, is_ansi: bool) -> EvalResult<Self> {
        unsafe {
            let len = len.unwrap_or(1024);
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, len, len)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, len);
            Ok(Self { ptr, hheap, len, ansi: is_ansi })
        }
    }
    fn ansi_from(s: &str) -> EvalResult<Self> {
        let ansi = to_ansi_bytes(s);
        let len = ansi.len();
        let buf = Self::new(Some(len), true)?;
        unsafe {
            ptr::copy_nonoverlapping(ansi.as_ptr(), buf.ptr as *mut u8, len);
        }
        Ok(buf)
    }
    fn wide_from(s: &str) -> EvalResult<Self> {
        let wide = to_wide_string(s);
        let len = wide.len();
        let buf = Self::new(Some(len), false)?;
        unsafe {
            ptr::copy_nonoverlapping(wide.as_ptr(), buf.ptr as *mut u16, len);
        }
        Ok(buf)
    }
    fn to_string(&self, is_char: bool) -> String {
        unsafe {
            let s = if self.ansi {
                let ansi = std::slice::from_raw_parts(self.ptr as *const u8, self.len);
                from_ansi_bytes(ansi)
            } else {
                let wide = std::slice::from_raw_parts(self.ptr as *const u16, self.len);
                from_wide_string(wide)
            };
            Self::fix_string(s, is_char)
        }
    }
    fn fix_string(s: String, is_char: bool) -> String {
        if is_char {
            s
        } else {
            match s.split_once('\0') {
                Some((s,_)) => s.to_string(),
                None => s,
            }
        }
    }
    fn _len(&self) -> usize {
        self.len
    }
    fn address(&self) -> usize {
        self.ptr as usize
    }
    // fn from_ptr_to_string(&mut self, ptr: *const c_void, trim_from_null: bool) -> Option<String> {
    //     unsafe {
    //         let s = match self {
    //             StringBuffer::Wide(wide) => {
    //                 let src = ptr as *const u16;
    //                 let dst = wide.as_mut_ptr();
    //                 ptr::copy_nonoverlapping(src, dst, wide.len());
    //                 from_wide_string(wide)
    //             },
    //             StringBuffer::Ansi(ansi) => {
    //                 let src = ptr as *const u8;
    //                 let dst = ansi.as_mut_ptr();
    //                 ptr::copy_nonoverlapping(src, dst, ansi.len());
    //                 from_ansi_bytes(ansi)
    //             },
    //             StringBuffer::None => None?,
    //         };
    //         let s = if trim_from_null {
    //             match s.split_once('\0') {
    //                 Some((s, _)) => s.to_string(),
    //                 None => s,
    //             }
    //         } else {
    //             s
    //         };
    //         Some(s)
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct UStructMember {
    name: String,
    r#type: MemberType,
    offset: usize,
    len: Option<usize>,
    buffer: Arc<Mutex<Option<StringBuffer>>>
}
impl PartialEq for UStructMember {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.r#type == other.r#type && self.offset == other.offset && self.len == other.len
    }
}
impl UStructMember {
    fn new(name: &str, r#type: &MemberType, offset: usize, len: Option<usize>) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            r#type: r#type.clone(),
            offset,
            len,
            buffer: Arc::new(Mutex::new(None)),
        }
    }
    fn matches(&self, name: &str) -> bool {
        name.to_ascii_lowercase() == self.name
    }
    fn set_string(&self, addr: usize, string: Option<String>) -> EvalResult<bool> {
        unsafe {
            let is_string = if self.is_ansi_string() {
                Some(true)
            } else if self.is_wide_string() {
                Some(false)
            } else {
                None
            };
            if let Some(is_ansi) = is_string {
                let buf = if let Some(s) = &string {
                    if is_ansi {
                        StringBuffer::ansi_from(s)?
                    } else {
                        StringBuffer::wide_from(s)?
                    }
                } else {
                    StringBuffer::new(None, is_ansi)?
                };
                let pbuf = buf.address();
                let src = pbuf as *const usize;
                let dst = addr as *mut usize;
                ptr::copy_nonoverlapping(src, dst, 1);
                let mut buffer = self.buffer.lock().unwrap();
                *buffer = Some(buf);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
    fn is_wide_string(&self) -> bool {
        match self.r#type {
            MemberType::Wstring |
            MemberType::PWchar => true,
            _ => false,
        }
    }
    fn is_ansi_string(&self) -> bool {
        match self.r#type {
            MemberType::String |
            MemberType::Pchar => true,
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct UStructPointer {
    ptr: *mut c_void,
    hheap: HeapHandle
}
impl Drop for UStructPointer {
    fn drop(&mut self) {
        unsafe {
            HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            HeapDestroy(self.hheap);
        }
    }
}
#[derive(Debug, Clone)]
pub struct UStruct {
    pub name: String,
    members: Vec<UStructMember>,
    size: usize,
    address: usize,
    pointer: Option<Arc<Mutex<UStructPointer>>>,
}
impl PartialEq for UStruct {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.name == other.name && self.members == other.members && self.size == other.size && self.address == other.address;
        let b2 = match (&self.pointer, &other.pointer) {
            (Some(p1), Some(p2)) => {
                let _tmp = p1.lock();
                p2.try_lock().is_err()
            },
            _ => false,
        };
        b1 && b2
    }
}
impl std::fmt::Display for UStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(target_arch="x86_64")]
        {
            write!(f, "{}(0x{:016X})", self.name, self.address)
        }
        #[cfg(target_arch="x86")]
        {
            write!(f, "{}(0x{:08X})", self.name, self.address)
        }
    }
}
impl TryFrom<&StructDef> for UStruct {
    type Error = UError;

    fn try_from(sdef: &StructDef) -> Result<Self, Self::Error> {
        unsafe {
            let mut ustruct = Self::new(sdef);
            let size = ustruct.size();
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, size, size)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, sdef.size);
            ustruct.address = ptr as usize;
            for member in ustruct.members.iter_mut() {
                // 文字列メンバであればデフォルトサイズのバッファを作っておく
                let addr = ustruct.address + member.offset;
                member.set_string(addr, None)?;
            }
            let pointer = UStructPointer { ptr, hheap };
            ustruct.pointer = Some(Arc::new(Mutex::new(pointer)));
            Ok(ustruct)
        }
    }
}
impl UStruct {
    fn new(sdef: &StructDef) -> Self {
        let mut offset = 0;
        let members = sdef.members.iter()
            .map(|mdef| {
                let member = UStructMember::new(&mdef.name, &mdef.r#type, offset, mdef.len);
                offset += match mdef.len {
                    Some(len) => mdef.r#type.size() * len,
                    None => mdef.r#type.size(),
                };
                member
            })
            .collect();
        Self {
            name: sdef.name.clone(),
            members,
            size: sdef.size,
            address: 0,
            pointer: None,
        }
    }
    pub fn new_from_pointer(ptr: *const c_void, sdef: &StructDef) -> Self {
        let mut ustruct = Self::new(sdef);
        ustruct.address = ptr as usize;
        ustruct
    }
    fn get_member(&self, name: &str) -> Option<&UStructMember> {
        self.members.iter()
            .find(|m| m.matches(name))
    }
    pub fn as_ptr(&self) -> *mut c_void {
        match &self.pointer {
            Some(mutex) => {
                let p = mutex.lock().unwrap();
                p.ptr
            },
            None => ptr::null_mut() as *mut c_void,
        }
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn get(&self, name: &str) -> EvalResult<Object> {
        unsafe {
            match self.get_member(name) {
                Some(member) => {
                    let addr = self.address + member.offset;
                    let count = member.len.unwrap_or(1);
                    match &member.r#type {
                        MemberType::Int |
                        MemberType::Long => {
                            let mut dst = vec![0i32; count];
                            let src = addr as *const i32;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Bool => {
                            let mut dst = vec![0i32; count];
                            let src = addr as *const i32;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let b = dst[0] != 0;
                                Ok(b.into())
                            } else {
                                let arr = dst.into_iter().map(|n| (n != 0).into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Uint |
                        MemberType::Dword => {
                            let mut dst = vec![0u32; count];
                            let src = addr as *const u32;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Float => {
                            let mut dst = vec![0f32; count];
                            let src = addr as *const f32;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok((n as f64).into())
                            } else {
                                let arr = dst.into_iter().map(|n| (n as f64).into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Double => {
                            let mut dst = vec![0f64; count];
                            let src = addr as *const f64;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Word => {
                            let mut dst = vec![0u16; count];
                            let src = addr as *const u16;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Wchar => {
                            let mut dst = vec![0u16; count];
                            let src = addr as *const u16;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            let s = from_wide_string(&dst);
                            Ok(s.into())
                        },
                        MemberType::Byte => {
                            let mut dst = vec![0u8; count];
                            let src = addr as *const u8;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok((n as u16).into())
                            } else {
                                let arr = dst.into_iter().map(|n| (n as u16).into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Char => {
                            let mut dst = vec![0u8; count];
                            let src = addr as *const u8;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            let s = from_ansi_bytes(&dst);
                            Ok(s.into())
                        },
                        MemberType::Boolean => {
                            let mut dst = vec![0u8; count];
                            let src = addr as *const u8;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok((n != 0).into())
                            } else {
                                let arr = dst.into_iter().map(|n| (n != 0).into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::Longlong => {
                            let mut dst = vec![0i64; count];
                            let src = addr as *const i64;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::String |
                        MemberType::Pchar => {
                            let is_char = member.r#type == MemberType::Pchar;
                            let guard = member.buffer.lock().unwrap();
                            match &*guard {
                                Some(buf) => {
                                    let s = buf.to_string(is_char);
                                    Ok(s.into())
                                },
                                None => {
                                    let mut dst = 0usize;
                                    let src = addr as *const usize;
                                    ptr::copy_nonoverlapping(src, &mut dst, 1);
                                    let ptr = dst as *const u8;
                                    let pcstr = PCSTR::from_raw(ptr);
                                    let ansi = pcstr.as_bytes();
                                    let s = from_ansi_bytes(ansi);
                                    let s = StringBuffer::fix_string(s, is_char);
                                    Ok(s.into())
                                },
                            }
                        },
                        MemberType::Wstring |
                        MemberType::PWchar => {
                            let is_char = member.r#type == MemberType::PWchar;
                            let guard = member.buffer.lock().unwrap();
                            match &*guard {
                                Some(buf) => {
                                    let s = buf.to_string(is_char);
                                    Ok(s.into())
                                },
                                None => {
                                    let mut dst = 0usize;
                                    let src = addr as *const usize;
                                    ptr::copy_nonoverlapping(src, &mut dst, 1);
                                    let ptr = dst as *const u16;
                                    let pcwstr = PCWSTR::from_raw(ptr);
                                    let wide = pcwstr.as_wide();
                                    let s = from_wide_string(wide);
                                    let s = StringBuffer::fix_string(s, is_char);
                                    Ok(s.into())
                                },
                            }
                        },
                        MemberType::Hwnd |
                        MemberType::Pointer => {
                            let mut dst = vec![0usize; count];
                            let src = addr as *const usize;
                            ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), count);
                            if count == 1 {
                                let n = dst[0];
                                Ok(n.into())
                            } else {
                                let arr = dst.into_iter().map(|n| n.into()).collect();
                                Ok(Object::Array(arr))
                            }
                        },
                        MemberType::UStruct(sdef) => {
                            let ptr = addr as *const c_void;
                            let ustruct = Self::new_from_pointer(ptr, sdef);
                            Ok(Object::UStruct(ustruct))
                        },
                    }
                },
                None => Err(UError::new(
                    UErrorKind::UStructError,
                    UErrorMessage::StructMemberNotFound(self.name.clone(), name.into())
                )),
            }
        }
    }
    fn set_num<T>(addr: usize, count: usize, value: Object) -> EvalResult<()>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        let v = value.to_num_vec::<T>()?;
        if v.len() > count {
            Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)))
        } else {
            let src = v.as_ptr();
            let dst = addr as *mut _;
            unsafe {
                ptr::copy_nonoverlapping(src, dst, v.len());
            }
            Ok(())
        }
    }
    fn set_f64(addr: usize, count: usize, value: Object) -> EvalResult<()> {
        let v = value.to_f64_vec()?;
        if v.len() > count {
            Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)))
        } else {
            let src = v.as_ptr();
            let dst = addr as *mut _;
            unsafe {
                ptr::copy_nonoverlapping(src, dst, v.len());
            }
            Ok(())
        }
    }
    fn set_char(addr: usize, is_ansi: bool, count: usize, value: Object) -> EvalResult<()> {
        let s = value.to_string();
        if is_ansi {
            let ansi = to_ansi_bytes(&s);
            if ansi.len() > count {
                return Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)));
            } else {
                let src = ansi.as_ptr();
                let dst = addr as *mut u8;
                unsafe {
                    ptr::copy_nonoverlapping(src, dst, ansi.len());
                }
            }
        } else {
            let wide = to_wide_string(&s);
            if wide.len() > count {
                return Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)));
            } else {
                let src = wide.as_ptr();
                let dst = addr as *mut u16;
                unsafe {
                    ptr::copy_nonoverlapping(src, dst, wide.len());
                }
            }
        }
        Ok(())
    }
    pub fn set(&self, name: &str, value: Object) -> EvalResult<()> {
        match self.get_member(name) {
            Some(member) => {
                let addr = self.address + member.offset;
                let count = member.len.unwrap_or(1);
                match &member.r#type {
                    MemberType::Int |
                    MemberType::Long |
                    MemberType::Bool => {
                        Self::set_num::<i32>(addr, count, value)
                    },
                    MemberType::Uint |
                    MemberType::Dword => {
                        Self::set_num::<u32>(addr, count, value)
                    },
                    MemberType::Float => {
                        Self::set_num::<f32>(addr, count, value)
                    },
                    MemberType::Double => {
                        Self::set_f64(addr, count, value)
                    },
                    MemberType::Word => {
                        Self::set_num::<u16>(addr, count, value)
                    },
                    MemberType::Wchar => {
                        Self::set_char(addr, false, count, value)
                    },
                    MemberType::Byte |
                    MemberType::Boolean => {
                        Self::set_num::<u8>(addr, count, value)
                    },
                    MemberType::Char => {
                        Self::set_char(addr, true, count, value)
                    },
                    MemberType::Longlong => {
                        Self::set_num::<i64>(addr, count, value)
                    },
                    MemberType::String |
                    MemberType::Pchar |
                    MemberType::Wstring |
                    MemberType::PWchar => {
                        let string = Some(value.to_string());
                        if member.set_string(addr, string)? {
                            Ok(())
                        } else {
                            Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
                        }
                    },
                    MemberType::Hwnd |
                    MemberType::Pointer => {
                        Self::set_num::<usize>(addr, count, value)
                    },
                    MemberType::UStruct(_) => {
                        Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
                    },
                }
            },
            None => Err(UError::new(
                UErrorKind::UStructError,
                UErrorMessage::StructMemberNotFound(self.name.clone(), name.into())
            )),
        }
    }
    pub fn invoke_method(&self, name: &str) -> EvalResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "size" | "length" => {
                let size = self.size();
                Ok(size.into())
            },
            "address" => {
                let addr = self.address;
                Ok(addr.into())
            }
            _ => {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::CanNotCallMethod(name.to_string())))
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
enum MemberType {
    Int,
    Long,
    Bool,
    Uint,
    Dword,
    Hwnd,
    Float,
    Double,
    Word,
    Wchar,
    Byte,
    Char,
    Boolean,
    Longlong,
    String,
    Pchar,
    Wstring,
    PWchar,
    Pointer,
    UStruct(StructDef),
}
impl MemberType {
    fn size(&self) -> usize {
        match self {
            MemberType::Int |
            MemberType::Long |
            MemberType::Bool => mem::size_of::<i32>(),
            MemberType::Uint |
            MemberType::Dword => mem::size_of::<u32>(),
            MemberType::Hwnd => mem::size_of::<usize>(),
            MemberType::Float => mem::size_of::<f32>(),
            MemberType::Double => mem::size_of::<f64>(),
            MemberType::Word |
            MemberType::Wchar => mem::size_of::<u16>(),
            MemberType::Byte |
            MemberType::Char |
            MemberType::Boolean => mem::size_of::<u8>(),
            MemberType::Longlong => mem::size_of::<i64>(),
            MemberType::String |
            MemberType::Pchar |
            MemberType::Wstring |
            MemberType::PWchar |
            MemberType::Pointer => mem::size_of::<usize>(),
            MemberType::UStruct(sdef) => sdef.size,
        }
    }
}
impl std::fmt::Display for MemberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberType::Int => write!(f, "int"),
            MemberType::Long => write!(f, "long"),
            MemberType::Bool => write!(f, "bool"),
            MemberType::Uint => write!(f, "uint"),
            MemberType::Dword => write!(f, "dword"),
            MemberType::Hwnd => write!(f, "hwnd"),
            MemberType::Float => write!(f, "float"),
            MemberType::Double => write!(f, "double"),
            MemberType::Word => write!(f, "word"),
            MemberType::Wchar => write!(f, "wchar"),
            MemberType::Byte => write!(f, "byte"),
            MemberType::Char => write!(f, "char"),
            MemberType::Boolean => write!(f, "boolean"),
            MemberType::Longlong => write!(f, "longlong"),
            MemberType::String => write!(f, "string"),
            MemberType::Pchar => write!(f, "pchar"),
            MemberType::Wstring => write!(f, "wstring"),
            MemberType::PWchar => write!(f, "pwchar"),
            MemberType::Pointer => write!(f, "pointer"),
            MemberType::UStruct(sdef) => write!(f, "{}", sdef.name),
        }
    }
}

impl Object {
    fn to_num_vec<T>(&self) -> EvalResult<Vec<T>>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        match self {
            Object::Num(f) => {
                let n = T::cast(*f)?;
                Ok(vec![n])
            },
            Object::Bool(b) => {
                let n = if *b {1.0} else {0.0};
                let n= T::cast(n)?;
                Ok(vec![n])
            }
            Object::Array(arr) => {
                let vec = arr.iter()
                    .filter_map(|o| o.as_f64(false))
                    .map(|f| T::cast(f))
                    .collect::<Result<Vec<T>, cast::Error>>()?;
                Ok(vec)
            },
            _ => {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
            }
        }
    }
    fn to_f64_vec(&self) -> EvalResult<Vec<f64>> {
        match self {
            Object::Num(n) => {
                Ok(vec![*n])
            },
            Object::Bool(b) => {
                let n = if *b {1.0} else {0.0};
                Ok(vec![n])
            }
            Object::Array(arr) => {
                let vec = arr.iter()
                    .filter_map(|o| o.as_f64(false))
                    .collect();
                Ok(vec)
            },
            _ => {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
            }
        }
    }
}