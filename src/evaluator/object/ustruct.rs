use super::Object;
use super::super::{
    EvalResult,
    def_dll::DllArg,
};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::ast::DllType;
use crate::winapi::{
    to_ansi_bytes, from_ansi_bytes, to_wide_string,
};

use std::ffi::c_void;
use std::mem;

#[derive(Debug, Clone, PartialEq)]
pub struct UStruct {
    name: String,
    members: Vec<UStructMember>,
    size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UStructMember {
    name: String,
    object: Object,
    dll_type: DllType,
}

impl UStruct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            members: vec![],
            size: 0,
        }
    }

    pub fn add(&mut self, name: String, object: Object, dll_type: DllType) -> EvalResult<()> {
        match DllArg::new(&object, &dll_type) {
            Ok(_) => {},
            Err(e) => return Err(UError::new(
                UErrorKind::UStructError,
                UErrorMessage::StructGotBadType(name, dll_type, e)
            ))
        };
        self.size += dll_type.size();
        self.members.push(UStructMember {
            name,
            object,
            dll_type
        });
        Ok(())
    }

    pub fn add_struct(&mut self, name: String, object: Object, dll_type: DllType) {
        self.members.push(UStructMember {
            name,
            object,
            dll_type
        });
    }

    pub fn get(&self, name: String) -> EvalResult<Object> {
        for member in &self.members {
            if member.name.to_ascii_lowercase() == name.to_ascii_lowercase() {
                return Ok(member.object.clone())
            }
        }
        Err(UError::new(
            UErrorKind::UStructError,
            UErrorMessage::StructMemberNotFound(self.name.clone(), name)
        ))
    }

    pub fn set(&mut self, name: String, object: Object) -> EvalResult<()> {
        for member in self.members.iter_mut() {
            if member.name.to_ascii_lowercase() == name.to_ascii_lowercase() {
                if let DllType::Unknown(ref t) = member.dll_type {
                    if let Object::UStruct(ref n, _, _) = object {
                        if t.to_ascii_lowercase() == n.to_ascii_lowercase() {
                            member.object = object;
                        } else {
                            return Err(UError::new(
                                UErrorKind::UStructError,
                                UErrorMessage::StructMemberNotFound(self.name.clone(), name)
                            ))
                        }
                    } else {
                        return Err(UError::new(
                            UErrorKind::UStructError,
                            UErrorMessage::StructTypeNotValid(name, t.clone())
                        ))
                    }
                } else {
                    match DllArg::new(&object, &member.dll_type) {
                        Ok(_) => {},
                        Err(e) => return Err(UError::new(
                            UErrorKind::UStructError,
                            UErrorMessage::StructGotBadType(name, member.dll_type.clone(), e)
                        ))
                    };
                    member.object = object;
                }
                return Ok(())
            }
        }
        Err(UError::new(
            UErrorKind::UStructError,
            UErrorMessage::StructMemberNotFound(self.name.clone(), name)
        ))
    }

    pub fn to_pointer(&self, address: usize) -> EvalResult<()>{
        let mut offset: usize = 0;
        for member in &self.members {
            let dest = (address + offset) as *mut c_void;
            match member.dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => {
                    offset += Self::copy_number_to::<i32>(dest, &member.object)?;
                },
                DllType::Uint |
                DllType::Dword => {
                    offset += Self::copy_number_to::<u32>(dest, &member.object)?;
                },
                DllType::Hwnd => {
                    offset += Self::copy_number_to::<isize>(dest, &member.object)?;
                },
                DllType::Float => {
                    offset += Self::copy_number_to::<f32>(dest, &member.object)?;
                },
                DllType::Double => {
                    let size = mem::size_of::<f64>();
                    let mut n = if let Object::Num(v) = member.object {
                        v
                    } else {
                        0.0
                    };
                    let src = &mut n as *mut f64 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    offset += size;
                },
                DllType::Word |
                DllType::Wchar => {
                    offset += Self::copy_number_to::<u16>(dest, &member.object)?;
                },
                DllType::Byte |
                DllType::Boolean |
                DllType::Char => {
                    offset += Self::copy_number_to::<u8>(dest, &member.object)?;
                },
                DllType::Longlong => {
                    offset += Self::copy_number_to::<i64>(dest, &member.object)?;
                },
                DllType::Pointer => {
                    offset += Self::copy_number_to::<usize>(dest, &member.object)?;
                },
                DllType::Pchar |
                DllType::String => {
                    offset += Self::copy_string_to(dest, &member.object, true);
                },
                DllType::Wstring |
                DllType::PWchar => {
                    offset += Self::copy_string_to(dest, &member.object, false);
                },
                DllType::Unknown(_) => {
                    let size = mem::size_of::<usize>();
                    match member.object {
                        // メンバ構造体
                        Object::UStruct(_, _, ref m) => {
                            let u = m.lock().unwrap();
                            let p = unsafe {
                                libc::malloc(u.size)
                            };
                            u.to_pointer(p as usize)?;
                        },
                        _ => {}
                    }
                    offset += size;
                },
                _ => return Err(UError::new(
                    UErrorKind::UStructError,
                    UErrorMessage::StructTypeUnsupported(member.dll_type.clone())
                )),
            }
        }
        Ok(())
    }

    fn copy_number_to<T>(dest: *mut c_void, object: &Object) -> EvalResult<usize>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        let size = mem::size_of::<T>();
        let mut n = if let Object::Num(v) = object {
            T::cast(*v)?
        } else {
            T::cast(0.0)?
        };
        let src = &mut n as *mut T as *mut c_void;
        unsafe {
            libc::memcpy(dest, src, size);
        }
        Ok(size)
    }

    fn copy_string_to(dest: *mut c_void, object: &Object, ansi: bool) -> usize {
        let size = mem::size_of::<usize>();
        let address = match object {
            Object::String(ref s) => {
                if ansi {
                    let mut ansi = to_ansi_bytes(s);
                    ansi.as_mut_ptr() as *mut c_void as usize
                } else {
                    let mut wide = to_wide_string(s);
                    wide.as_mut_ptr() as *mut c_void as usize
                }
            },
            Object::Null => {
                let mut null: usize = 0;
                let p = &mut null as *mut usize as *mut c_void;
                p as usize
            },
            _ => {
                let mut null: usize = 0;
                let p = &mut null as *mut usize as *mut c_void;
                p as usize
            }
        };
        let src = address as *mut usize as *mut c_void;
        unsafe {
            libc::memcpy(dest, src, size);
        }
        size
    }

    pub fn from_pointer(&mut self, address: usize, free_pointer: bool) {
        let mut offset: usize = 0;
        for member in self.members.iter_mut() {
            let src = (address + offset) as *mut c_void;
            match member.dll_type {
                DllType::Int |
                DllType::Long |
                DllType::Bool => {
                    let size = mem::size_of::<i32>();
                    let mut n: i32 = 0;
                    let dest = &mut n as *mut i32 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Uint |
                DllType::Dword => {
                    let size = mem::size_of::<u32>();
                    let mut n: u32 = 0;
                    let dest = &mut n as *mut u32 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Hwnd => {
                    let size = mem::size_of::<isize>();
                    let mut n: isize = 0;
                    let dest = &mut n as *mut isize as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Float => {
                    let size = mem::size_of::<f32>();
                    let mut n: f32 = 0.0;
                    let dest = &mut n as *mut f32 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Double => {
                    let size = mem::size_of::<f64>();
                    let mut n: f64 = 0.0;
                    let dest = &mut n as *mut f64 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n);
                    offset += size;
                },
                DllType::Word |
                DllType::Wchar => {
                    let size = mem::size_of::<u16>();
                    let mut n: u16 = 0;
                    let dest = &mut n as *mut u16 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Byte |
                DllType::Boolean |
                DllType::Char => {
                    let size = mem::size_of::<u8>();
                    let mut n: u8 = 0;
                    let dest = &mut n as *mut u8 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Longlong => {
                    let size = mem::size_of::<i64>();
                    let mut n: i64 = 0;
                    let dest = &mut n as *mut i64 as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Pointer => {
                    let size = mem::size_of::<usize>();
                    let mut n: usize = 0;
                    let dest = &mut n as *mut usize as *mut c_void;
                    unsafe {
                        libc::memcpy(dest, src, size);
                    }
                    member.object = Object::Num(n as f64);
                    offset += size;
                },
                DllType::Pchar |
                DllType::String => {
                    let size = mem::size_of::<usize>();
                    if let Object::String(ref s) = member.object {
                        let mut ansi = to_ansi_bytes(s);
                        let dest = ansi.as_mut_ptr() as *mut c_void;
                        unsafe {
                            libc::memcpy(dest, src, ansi.len());
                        }
                        let str = from_ansi_bytes(&ansi);
                        member.object = if member.dll_type == DllType::String {
                            let null_end_str = str.split("\0").collect::<Vec<&str>>();
                            Object::String(null_end_str[0].to_string())
                        } else {
                            Object::String(str)
                        }
                    }
                    offset += size;
                },
                DllType::Wstring |
                DllType::PWchar => {
                    let size = mem::size_of::<usize>();
                    if let Object::String(ref s) = member.object {
                        let mut wide = to_wide_string(s);
                        let dest = wide.as_mut_ptr() as *mut c_void;
                        unsafe {
                            libc::memcpy(dest, src, wide.len());
                        }
                        let str = String::from_utf16_lossy(&wide);
                        member.object = if member.dll_type == DllType::String {
                            let null_end_str = str.split("\0").collect::<Vec<&str>>();
                            Object::String(null_end_str[0].to_string())
                        } else {
                            Object::String(str)
                        }
                    }
                    offset += size;
                },
                DllType::Unknown(_) => {
                    let size = mem::size_of::<usize>();
                    match member.object {
                        // 別の構造体
                        Object::UStruct(_, _, ref m) => {
                            // 構造体のアドレスを得る
                            let mut a: usize = 0;
                            let dest = &mut a as *mut usize as *mut c_void;
                            unsafe {
                                libc::memcpy(dest, src, mem::size_of::<usize>());
                            }
                            let mut u = m.lock().unwrap();
                            u.from_pointer(a, true);
                            if free_pointer {
                                let p = a as *mut c_void;
                                unsafe {
                                    libc::free(p);
                                }
                            }
                        },
                        _ => {}
                    }
                    offset += size;
                },
                DllType::SafeArray |
                DllType::Void |
                DllType::Struct |
                DllType::CallBack => {
                    offset += mem::size_of::<usize>();
                }
            }
        }
    }
}
