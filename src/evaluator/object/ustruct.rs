use super::Object;
use super::super::{EvalResult, Evaluator};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::ast::{Expression, DefDllParamSize};
use crate::winapi::{
    to_ansi_bytes, from_ansi_bytes, to_wide_string, from_wide_string,
};

use std::ffi::c_void;
use std::str::FromStr;
use std::{mem, ptr};
use std::sync::{Arc, Mutex};

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::{
    Foundation::HANDLE,
    System::Memory::{
        HeapAlloc, HeapFree, HeapCreate, HeapDestroy,
        HEAP_ZERO_MEMORY, HEAP_NONE, HEAP_GENERATE_EXCEPTIONS,
    }
};

use num_traits::FromPrimitive;

pub struct MemberDefVec(pub Vec<(String, MemberType, Option<usize>, bool)>);
impl MemberDefVec {
    pub fn new(members: Vec<(String, String, DefDllParamSize, bool)>, e: &mut Evaluator) -> EvalResult<Self> {
        let members = members.into_iter()
            .map(|(name, type_name, size, is_ref)| {
                let r#type = match MemberType::from_str(&type_name) {
                    Ok(t) => Ok(t),
                    Err(err) => {
                        if let Some(Object::StructDef(sdef)) = e.env.get_struct(&type_name) {
                            Ok(MemberType::Struct(sdef))
                        } else {
                            Err(err)
                        }
                    },
                }?;
                let size = match size {
                    DefDllParamSize::Const(name) => {
                        e.env.get_const_num(&name)
                            .ok_or(UError::new(UErrorKind::StructDefError, UErrorMessage::DllArgConstSizeIsNotValid))
                            .map(|n| Some(n))
                    },
                    DefDllParamSize::Size(n) => Ok(Some(n)),
                    DefDllParamSize::None => Ok(None),
                }?;
                Ok((name, r#type, size, is_ref))
            })
            .collect::<EvalResult<Vec<_>>>()?;
        Ok(Self(members))
    }
}
/// 構造体メンバ定義
#[derive(Debug, Clone, PartialEq)]
pub struct MemberDef {
    name: String,
    r#type: MemberType,
    /// 配列指定であればそのサイズ
    len: Option<usize>,
    offset: usize,
    is_ref: bool,
}
impl MemberDef {
    fn new(name: String, r#type: MemberType, len: Option<usize>, offset: usize, is_ref: bool) -> Self {
        Self {name, r#type, len, offset, is_ref }
    }
    fn size(&self) -> usize {
        if self.is_ref {
            Self::p_size()
        } else {
            match &self.r#type {
                MemberType::String |
                MemberType::Pchar |
                MemberType::Wstring |
                MemberType::PWchar |
                MemberType::Struct(_) => self.r#type.size(),
                t => t.size() * self.len.unwrap_or(1)
            }
        }
    }
    fn alignment(&self) -> usize {
        if self.is_ref {
            Self::p_size()
        } else {
            self.r#type.alignment()
        }
    }
    fn p_size() -> usize {
        mem::size_of::<usize>()
    }
}
/// 構造体定義
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub size: usize,
    members: Vec<MemberDef>
}
impl StructDef {
    pub fn new(name: String, memberdef: MemberDefVec) -> Self {
        let members = memberdef.0;
        // アラインメントの最大値を得る
        let max_alignment = members.iter().map(|(_,t,_,_)| t.alignment()).reduce(|a,b| a.max(b)).unwrap_or_default();
        let mut offset = 0;
        let mut last_alignment = None::<usize>;

        let members = members.into_iter()
            .map(|(name, mut r#type, len, is_ref)| {
                if let MemberType::Struct(sdef) = &mut r#type {
                    let soffset = sdef.fix_layout(max_alignment, &mut offset, &mut last_alignment);
                    MemberDef::new(name, r#type, len, soffset, is_ref)
                } else {
                    let alignment = if is_ref {
                        mem::size_of::<usize>()
                    } else {
                        r#type.alignment()
                    };
                    offset = Self::pad_offset(alignment, max_alignment, offset, last_alignment);
                    let mdef = MemberDef::new(name, r#type, len, offset, is_ref);
                    // 合計サイズ分オフセットを進める
                    last_alignment = Some(alignment);
                    offset += mdef.size();
                    mdef
                }
            })
            .collect::<Vec<_>>();

        // 最後のオフセット位置を最大アラインメントの倍数に丸めてそれをサイズとする
        let size = if offset % max_alignment == 0 {
            offset
        } else {
            max_alignment - offset % max_alignment + offset
        };
        Self {name, size, members}
    }
    /// 必要に応じてオフセットを補正 (パディングする)
    fn pad_offset(alignment: usize, max_alignment: usize, mut offset: usize, last_alignment: Option<usize>) -> usize {
        if let Some(last) = last_alignment {
            // 前メンバよりアラインメント大きい
            if last < alignment {
                // 最大アラインメント単位でのオフセット
                let unit_offset = offset % max_alignment;
                if unit_offset > 0 {
                    // 0でなければ補正
                    if (max_alignment - unit_offset) >= alignment {
                        // 現単位にねじ込める場合
                        offset += alignment - unit_offset % alignment;
                    } else {
                        // 次の単位へ
                        offset += max_alignment - unit_offset % max_alignment;
                    }
                }
            }
        }
        offset
    }
    /// ネストした構造体のレイアウトを修正する
    ///
    /// 構造体自身のオフセットを返す
    fn fix_layout(&mut self, max_alignment: usize, offset: &mut usize, last_alignment: &mut Option<usize>) -> usize {
        let mut o = 0;
        for member in self.members.iter_mut() {
            match member.r#type.as_mut() {
                MemberType::Struct(sdef) => {
                    member.offset = sdef.fix_layout(max_alignment, &mut o, last_alignment);
                },
                _ => {
                    let alignment = member.alignment();
                    member.offset = Self::pad_offset(alignment, max_alignment, o, *last_alignment);
                    o += alignment * member.len.unwrap_or(1);
                    *last_alignment = Some(alignment)
                }
            }
        }
        let soffset = self.members.iter().next().map(|m| m.offset).unwrap_or_default() + *offset;
        *offset += o;
        soffset
    }
    pub fn layout(&self, parent: Option<&str>) -> String {
        self.members.iter().map(|m| {
            let name = match parent {
                Some(pname) => format!("{}.{}", pname, m.name),
                None => m.name.clone(),
            };
            match &m.r#type {
                MemberType::Struct(sdef) => {
                    let nested = sdef.layout(Some(&name));
                    format!("{}: {}\r\n{}", name, m.offset, nested)
                },
                _ => format!("{}: alignment: {}, size: {}, offset: {}", name, m.alignment(), m.size(), m.offset)
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n")
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
pub struct RefVal {
    ptr: *mut c_void,
    hheap: HANDLE,
    len: usize
}
impl Drop for RefVal {
    fn drop(&mut self) {
        unsafe {
            let _ = HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            let _ = HeapDestroy(self.hheap);
        }
    }
}
impl RefVal {
    fn new<T>(len: usize) -> EvalResult<Self> {
        unsafe {
            let bytes = mem::size_of::<T>() * len;
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, bytes, bytes)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, bytes);
            Ok(Self { ptr, hheap, len })
        }
    }
    fn new_ptr() -> EvalResult<Self> {
        Self::new::<usize>(1)
    }
    fn set<T>(&self, v: &Vec<T>) {
        unsafe {
            let dst = self.ptr as *mut T;
            ptr::copy_nonoverlapping(v.as_ptr(), dst, v.len());
        }
    }
    fn set_ptr(&self, addr: usize) {
        self.set::<usize>(&vec![addr]);
    }
    fn address(&self) -> usize {
        self.ptr as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringBuffer {
    ptr: *mut c_void,
    hheap: HANDLE,
    len :usize,
    ansi: bool,
}
impl Drop for StringBuffer {
    fn drop(&mut self) {
        unsafe {
            let _ = HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            let _ = HeapDestroy(self.hheap);
        }
    }
}
impl StringBuffer {
    // const DEFAULT_LENGTH: usize = 1024;
    fn new(len: usize, is_ansi: bool) -> EvalResult<Self> {
        unsafe {
            let bytes = if is_ansi { mem::size_of::<u8>() } else { mem::size_of::<u16>() } * len;
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, bytes, bytes)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, bytes);
            Ok(Self { ptr, hheap, len, ansi: is_ansi })
        }
    }
    fn from_str(string: &str, is_ansi: bool) -> EvalResult<Self> {
        unsafe {
            if is_ansi {
                let ansi = to_ansi_bytes(string);
                let len = ansi.len();
                let buf = Self::new(len, is_ansi)?;
                let dst = buf.ptr as *mut u8;
                ptr::copy_nonoverlapping(ansi.as_ptr(), dst, len);
                Ok(buf)
            } else {
                let wide = to_wide_string(string);
                let len = wide.len();
                let buf = Self::new(len, is_ansi)?;
                let dst = buf.ptr as *mut u16;
                ptr::copy_nonoverlapping(wide.as_ptr(), dst, len);
                Ok(buf)
            }
        }
    }
    fn set_string(&self, s: &str, is_ansi: bool) -> EvalResult<()> {
        if is_ansi {
            let ansi = to_ansi_bytes(s);
            let len = ansi.len();
            if self.len >= len {
                unsafe {
                    let dst = self.ptr as *mut u8;
                    ptr::write_bytes(dst, 0, self.len);
                    ptr::copy_nonoverlapping(ansi.as_ptr(), dst, len);
                }
                Ok(())
            } else {
                Err(UError::new(
                    UErrorKind::UStructError,
                    UErrorMessage::UStructStringMemberSizeOverflow(self.len, len)
                ))
            }
        } else {
            let wide = to_wide_string(s);
            let len = wide.len();
            if self.len >= len {
                unsafe {
                    let dst = self.ptr as *mut u16;
                    ptr::write_bytes(dst, 0, self.len);
                    ptr::copy_nonoverlapping(wide.as_ptr(), dst, len);
                }
                Ok(())
            } else {
                Err(UError::new(
                    UErrorKind::UStructError,
                    UErrorMessage::UStructStringMemberSizeOverflow(self.len, len)
                ))
            }
        }
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
    /// メンバが示すポインタとバッファ自身のポインタが一致するかどうか
    ///
    /// - addr: メンバのアドレス
    /// - is_ref: ポインタのポインタかどうか
    fn check_ptr(&self, addr: usize, is_ref: bool) -> bool {
        unsafe {
            let src = if is_ref {
                // バッファのポインタを得る
                let mut dst = 0usize;
                let src = addr as *const usize;
                ptr::copy_nonoverlapping(src, &mut dst, 1);
                dst as *const usize
            } else {
                addr as *const usize
            };
            let mut dst = 0usize;
            ptr::copy_nonoverlapping(src, &mut dst, 1);
            dst == self.address()
        }
    }
    fn get_string_from_pointer(addr: usize, is_ansi: bool, is_char: bool) -> Option<String> {
        unsafe {
            if is_ansi {
                let mut dst = 0usize;
                let src = addr as *const usize;
                ptr::copy_nonoverlapping(src, &mut dst, 1);
                if dst > 0 {
                    let ptr = dst as *const u8;
                    let pcstr = PCSTR::from_raw(ptr);
                    let ansi = pcstr.as_bytes();
                    let s = from_ansi_bytes(ansi);
                    Some(StringBuffer::fix_string(s, is_char))
                } else {
                    None
                }
            } else {
                let mut dst = 0usize;
                let src = addr as *const usize;
                ptr::copy_nonoverlapping(src, &mut dst, 1);
                if dst > 0 {
                    let ptr = dst as *const u16;
                    let pcwstr = PCWSTR::from_raw(ptr);
                    let wide = pcwstr.as_wide();
                    let s = from_wide_string(wide);
                    Some(StringBuffer::fix_string(s, is_char))
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct UStructMember {
    name: String,
    r#type: MemberType,
    offset: usize,
    len: Option<usize>,
    is_ref: bool,
    refval: Arc<Mutex<Option<RefVal>>>,
    buffer: Arc<Mutex<Option<StringBuffer>>>,
    ustruct: Option<UStruct>,
    /// def_dllの構造体内包定義でrefパラメータだった場合にExpressionを持つ
    pub refexpr: Option<Expression>
}
impl PartialEq for UStructMember {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.r#type == other.r#type && self.offset == other.offset && self.len == other.len
    }
}
impl UStructMember {
    pub fn new(memdef: &MemberDef) -> Self {
        Self {
            name: memdef.name.to_ascii_lowercase(),
            r#type: memdef.r#type.clone(),
            offset: memdef.offset,
            len: memdef.len,
            is_ref: memdef.is_ref,
            refval: Arc::new(Mutex::new(None)),
            buffer: Arc::new(Mutex::new(None)),
            ustruct: None,
            refexpr: None,
        }
    }
    pub fn size(&self) -> usize {
        match &self.r#type {
            MemberType::String |
            MemberType::Pchar |
            MemberType::Wstring |
            MemberType::PWchar => self.r#type.size(),
            MemberType::Struct(sdef) => sdef.size,
            t => t.size() * self.len.unwrap_or(1)
        }
    }
    fn buffer_size(&self) -> usize {
        let guard = self.buffer.lock().unwrap();
        match &*guard {
            Some(buf) => buf.len,
            None => 0,
        }
    }
    fn matches(&self, name: &str) -> bool {
        name.to_ascii_lowercase() == self.name
    }
    /// 新たな文字列バッファをセットし、バッファとそのアドレスを返す
    fn set_new_string(&self, string: &str, is_ansi: bool) -> EvalResult<(StringBuffer, usize)> {
        let buf = StringBuffer::from_str(&string, is_ansi)?;
        let addr = if self.is_ref {
            // バッファのポインタのポインタ
            self.set_string_ptr_ref(buf.address())?
        } else {
            // バッファのポインタ
            buf.address()
        };
        Ok((buf, addr))
    }
    /// 新たな文字列バッファをセットし、バッファとそのアドレスを返す
    ///
    /// サイズ指定版
    fn set_new_string_sized(&self, string: &str, len: usize, is_ansi: bool) -> EvalResult<(StringBuffer, usize)> {
        let buf = StringBuffer::new(len, is_ansi)?;
        buf.set_string(&string, is_ansi)?;
        let addr = if self.is_ref {
            self.set_string_ptr_ref(buf.address())?
        } else {
            buf.address()
        };
        Ok((buf, addr))
    }
    /// 文字列バッファのポインタをRefValにセットしてRefValのポインタを返す
    fn set_string_ptr_ref(&self, addr: usize) -> EvalResult<usize> {
        let refval = RefVal::new_ptr()?;
        refval.set_ptr(addr);
        let r_addr = refval.address();
        let mut guard = self.refval.lock().unwrap();
        *guard = Some(refval);
        Ok(r_addr)
    }
    /// addr: 親構造体のアドレス
    ///
    /// - 子構造体の場合はアドレスからUStructを作る
    /// - 構造体ポインタの場合は構造体を新規に作りそのポインタを書き込む
    fn init_child_ustruct(&mut self, addr: usize) -> EvalResult<()> {
        let mut ust = match &self.r#type {
            MemberType::Struct(sdef) => {
                let ptr = (addr + self.offset) as *mut c_void;
                let ust = UStruct::new_from_pointer(ptr, sdef);
                ust
            },
            MemberType::UStruct(sdef) => {
                let ust = UStruct::try_from(sdef)?;
                unsafe {
                    let src = &ust.address;
                    let dst = (addr + self.offset) as *mut _;
                    ptr::copy_nonoverlapping(src, dst, 1);
                }
                ust
            },
            _ => {
                return Ok(());
            },
        };
        for member in ust.members.iter_mut() {
            member.init_child_ustruct(ust.address)?;
        }
        self.ustruct = Some(ust);
        Ok(())
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
    pub fn get_ustruct_mut(&mut self) -> Option<&mut UStruct> {
        self.ustruct.as_mut()
    }
    pub fn get_ustruct(&self) -> Option<&UStruct> {
        self.ustruct.as_ref()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct UStructPointer {
    ptr: *mut c_void,
    hheap: HANDLE
}
impl UStructPointer {
    pub fn new(ptr: *mut c_void, hheap: HANDLE) -> Self {
        Self { ptr, hheap }
    }
}
impl Drop for UStructPointer {
    fn drop(&mut self) {
        unsafe {
            let _ = HeapFree(self.hheap, HEAP_NONE, Some(self.ptr));
            let _ = HeapDestroy(self.hheap);
        }
    }
}
#[derive(Debug, Clone)]
pub struct UStruct {
    pub name: String,
    members: Vec<UStructMember>,
    size: usize,
    pub address: usize,
    pub pointer: Option<Arc<Mutex<UStructPointer>>>,
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
        let mut ustruct = Self::new(sdef);
        ustruct.init()?;
        Ok(ustruct)
    }
}
impl UStruct {
    fn new(sdef: &StructDef) -> Self {
        let members = sdef.members.iter()
            .map(|mdef| UStructMember::new(mdef))
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
    fn init(&mut self) -> EvalResult<()> {
        unsafe {
            let size = self.size;
            let hheap = HeapCreate(HEAP_GENERATE_EXCEPTIONS, size, size)?;
            let ptr = HeapAlloc(hheap, HEAP_ZERO_MEMORY, size);
            self.address = ptr as usize;
            // メンバの初期化
            for member in self.members.iter_mut() {
                // 構造体メンバであればUstructを作る
                member.init_child_ustruct(self.address)?;
            }
            let pointer = UStructPointer::new(ptr, hheap);
            self.pointer = Some(Arc::new(Mutex::new(pointer)));
            Ok(())
        }
    }
    fn get_member(&self, name: &str) -> EvalResult<&UStructMember> {
        self.members.iter()
            .find(|m| m.matches(name))
            .ok_or(UError::new(
                UErrorKind::UStructError,
                UErrorMessage::StructMemberNotFound(self.name.clone(), name.into())
            ))
    }
    pub fn get_member_mut(&mut self, index: usize) -> Option<&mut UStructMember> {
        self.members.get_mut(index)
    }
    pub fn get_members(&self) -> &Vec<UStructMember> {
        self.members.as_ref()
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
    pub fn get_by_name(&self, name: &str) -> EvalResult<Object> {
        let member = self.get_member(name)?;
        self.get(member)
    }
    pub fn get_by_index(&self, index: usize) -> EvalResult<Object> {
        match self.members.get(index) {
            Some(member) => {
                self.get(member)
            },
            None => Err(UError::new(
                UErrorKind::UStructError,
                UErrorMessage::None
            )),
        }
    }
    fn get_num<T>(&self, member: &UStructMember) -> EvalResult<Vec<T>>
        where T: FromPrimitive + Default + Clone
    {
        unsafe {
            let addr = self.address + member.offset;
            let count = member.len.unwrap_or(1);
            let mut vec = vec![T::default(); count];
            let dst = vec.as_mut_ptr();
            if member.is_ref {
                let mut ptr = 0usize;
                let src = addr as *const usize;
                ptr::copy_nonoverlapping(src, &mut ptr, 1);
                if ptr == 0 {
                    Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberWasNullPointer))
                } else {
                    let src = ptr as *const T;
                    ptr::copy_nonoverlapping(src, dst, count);
                    Ok(vec)
                }
            } else {
                let src = addr as *const T;
                ptr::copy_nonoverlapping(src, dst, count);
                Ok(vec)
            }
        }
    }
    pub fn get(&self, member: &UStructMember) -> EvalResult<Object> {
        let as_array = member.len.is_some();
        match &member.r#type {
            MemberType::Int |
            MemberType::Long => {
                let vec = self.get_num::<i32>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Bool => {
                let vec = self.get_num::<i32>(member)?;
                Ok(Object::from_vec_t_bool(vec, as_array))
            },
            MemberType::Uint |
            MemberType::Dword => {
                let vec = self.get_num::<u32>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Float => {
                let vec = self.get_num::<f32>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Double => {
                let vec = self.get_num::<f64>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Word => {
                let vec = self.get_num::<u16>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Wchar => {
                let wide = self.get_num::<u16>(member)?;
                let s = from_wide_string(&wide);
                Ok(s.into())
            },
            MemberType::Byte => {
                let vec = self.get_num::<u8>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Char => {
                let ansi = self.get_num::<u8>(member)?;
                let s = from_ansi_bytes(&ansi);
                Ok(s.into())
            },
            MemberType::Boolean => {
                let vec = self.get_num::<u8>(member)?;
                Ok(Object::from_vec_t_bool(vec, as_array))
            },
            MemberType::Longlong => {
                let vec = self.get_num::<i64>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::String |
            MemberType::Pchar |
            MemberType::Wstring |
            MemberType::PWchar => {
                let is_char = member.r#type.is_char();
                let is_ansi = member.r#type.is_ansi();
                let addr = self.address + member.offset;
                let guard = member.buffer.lock().unwrap();
                match &*guard {
                    Some(buf) => {
                        if buf.check_ptr(addr, member.is_ref) {
                            let s = buf.to_string(is_char);
                            Ok(s.into())
                        } else {
                            let s = StringBuffer::get_string_from_pointer(addr, is_ansi, is_char);
                            Ok(s.into())
                        }
                    },
                    None => {
                        let s = StringBuffer::get_string_from_pointer(addr, is_ansi, is_char);
                        Ok(s.into())
                    },
                }
            },
            MemberType::Hwnd |
            MemberType::Handle |
            MemberType::Pointer |
            MemberType::Size => {
                let vec = self.get_num::<usize>(member)?;
                Ok(Object::from_vec_t(vec, as_array))
            },
            MemberType::Struct(sdef) => {
                if let Some(ustruct) = &member.ustruct {
                    Ok(Object::UStruct(ustruct.clone()))
                } else {
                    Err(UError::new(
                        UErrorKind::UStructError,
                        UErrorMessage::Any(format!("{} is null; {} was not allocated", member.name, sdef.name))
                    ))
                }
            },
            MemberType::Void => Ok(Object::Empty),
            MemberType::UStruct(_) => {
                unreachable!()
            },
        }
    }
    fn set_array_num<T: FromPrimitive>(&self, member: &UStructMember, index: usize, value: Object) -> EvalResult<()> {
        let t = value.cast2::<T>()?;
        let offset = mem::size_of::<T>() * index;
        if member.is_ref {
            let mut ptr = 0usize;
            let addr = self.address + member.offset;
            let src = addr as *mut _;
            unsafe {
                ptr::copy_nonoverlapping(src, &mut ptr, 1)
            }
            if ptr == 0 {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberWasNullPointer))
            } else {
                let addr = ptr + offset;
                let dst = addr as *mut _;
                unsafe {
                    ptr::copy_nonoverlapping(&t, dst, 1);
                }
                Ok(())
            }
        } else {
            let addr = self.address + member.offset + offset;
            let dst = addr as *mut _;
            unsafe {
                ptr::copy_nonoverlapping(&t, dst, 1)
            }
            Ok(())
        }
    }
    fn set_num<T>(&self, member: &UStructMember, value: Object) -> EvalResult<()>
        where T: FromPrimitive,
    {
        let addr = self.address + member.offset;
        let count = member.len.unwrap_or(1);
        let v = value.to_num_vec::<T>()?;
        if v.len() > count {
            Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)))
        } else {
            if member.is_ref {
                let refval = RefVal::new::<T>(count)?;
                refval.set(&v);
                unsafe {
                    let src = refval.ptr as usize;
                    let dst = addr as *mut _;
                    ptr::copy_nonoverlapping(&src, dst, 1);
                }
                let mut guard = member.refval.lock().unwrap();
                *guard = Some(refval);
            } else {
                let src = v.as_ptr();
                let dst = addr as *mut _;
                unsafe {
                    ptr::copy_nonoverlapping(src, dst, v.len());
                }
            }
            Ok(())
        }
    }
    fn set_char(&self, member: &UStructMember, is_ansi: bool, value: Object) -> EvalResult<()> {
        let s = value.to_string_nullable().unwrap_or_default();
        let addr = self.address + member.offset;
        let count = member.len.unwrap_or(1);
        if is_ansi {
            let ansi = to_ansi_bytes(&s);
            if ansi.len() > count {
                return Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)));
            } else {
                if member.is_ref {
                    let refval = RefVal::new::<u8>(count)?;
                    refval.set(&ansi);
                    unsafe {
                        let src = refval.ptr as usize;
                        let dst = addr as *mut _;
                        ptr::copy_nonoverlapping(&src, dst, 1);
                    }
                } else {
                    let src = ansi.as_ptr();
                    let dst = addr as *mut u8;
                    unsafe {
                        ptr::copy_nonoverlapping(src, dst, ansi.len());
                    }
                }
            }
        } else {
            let wide = to_wide_string(&s);
            if wide.len() > count {
                return Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberSizeError(count)));
            } else {
                if member.is_ref {
                    let refval = RefVal::new::<u16>(count)?;
                    refval.set(&wide);
                    unsafe {
                        let src = refval.ptr as usize;
                        let dst = addr as *mut _;
                        ptr::copy_nonoverlapping(&src, dst, 1);
                    }
                } else {
                    let src = wide.as_ptr();
                    let dst = addr as *mut u16;
                    unsafe {
                        ptr::copy_nonoverlapping(src, dst, wide.len());
                    }
                }
            }
        }
        Ok(())
    }
    fn set_string(&self, member: &UStructMember, value: Object) -> EvalResult<()> {
        let addr = self.address + member.offset;
        let is_ansi = if member.is_ansi_string() {
            true
        } else if member.is_wide_string() {
            false
        } else {
            return Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError));
        };
        let opt_str = value.to_string_nullable();
        match opt_str {
            Some(string) => {
                match member.len {
                    Some(len) => {
                        let mut guard = member.buffer.lock().unwrap();
                        match guard.as_ref() {
                            Some(buf) => {
                                if buf.check_ptr(addr, member.is_ref) {
                                    // バッファに上書き
                                    buf.set_string(&string, is_ansi)?;
                                } else {
                                    // ポインタがバッファを示していないので新規にセット
                                    let (buf, src) = member.set_new_string_sized(&string, len, is_ansi)?;
                                    unsafe {
                                        let dst = addr as *mut _;
                                        ptr::copy_nonoverlapping(&src, dst, 1);
                                    }
                                    *guard = Some(buf);
                                }
                            },
                            None => {
                                // バッファがないので新規に作る
                                let (buf, src) = member.set_new_string_sized(&string, len, is_ansi)?;
                                unsafe {
                                    let dst = addr as *mut _;
                                    ptr::copy_nonoverlapping(&src, dst, 1);
                                }
                                *guard = Some(buf);
                            },
                        }
                    },
                    None => {
                        // サイズ指定がない場合は常に新規バッファを作ってセット
                        let (buf, src) = member.set_new_string(&string, is_ansi)?;
                        unsafe {
                            let dst = addr as *mut _;
                            ptr::copy_nonoverlapping(&src, dst, 1);
                        }
                        let mut guard = member.buffer.lock().unwrap();
                        *guard = Some(buf);
                    },
                }
            },
            None => {
                // NULL代入
                let mut guard = member.buffer.lock().unwrap();
                if guard.is_some() {
                    // StringBufferがあったら消して構造体にはNULLポインタをセット
                    let src = 0usize;
                    let dst = addr as *mut _;
                    unsafe {
                        ptr::copy_nonoverlapping(&src, dst, 1)
                    }
                    *guard = None;
                }
            },
        }
        Ok(())
    }
    pub fn set_by_name(&self, name: &str, value: Object) -> EvalResult<()> {
        let member = self.get_member(name)?;
        self.set(member, value)
    }
    pub fn set_array_member_by_name(&self, name: &str, index: Object, value: Object) -> EvalResult<()> {
        let index = match &index.as_f64(true) {
            Some(n) => {
                match usize::from_f64(*n) {
                    Some(index) => Ok(index),
                    None => Err(UError::new(UErrorKind::UStructError, UErrorMessage::CastError2(*n, "usize".into()))),
                }
            },
            None => Err(UError::new(UErrorKind::UStructError, UErrorMessage::NotANumber(index))),
        }?;
        let member = self.get_member(name)?;
        self.set_array(member, index, value)
    }
    /// def_dllの引数の値をセットするために使う
    ///
    /// refパラメータのExpressionも渡す
    pub fn set_by_index(&mut self, index: usize, value: Object, refexpr: Option<Expression>) -> EvalResult<()> {
        if let Some(member) = self.members.get(index) {
            self.set(member, value)?;
        } else {
            return Err(UError::default());
        }
        if let Some(member) = self.members.get_mut(index) {
            member.refexpr = refexpr;
        }
        Ok(())
    }
    fn set(&self, member: &UStructMember, value: Object) -> EvalResult<()> {
        match &member.r#type {
            MemberType::Int |
            MemberType::Long |
            MemberType::Bool => {
                self.set_num::<i32>(member, value)
            },
            MemberType::Uint |
            MemberType::Dword => {
                self.set_num::<u32>(member, value)
            },
            MemberType::Float => {
                self.set_num::<f32>(member, value)
            },
            MemberType::Double => {
                self.set_num::<f64>(member, value)
            },
            MemberType::Word => {
                self.set_num::<u16>(member, value)
            },
            MemberType::Wchar => {
                self.set_char(member, false, value)
            },
            MemberType::Byte |
            MemberType::Boolean => {
                self.set_num::<u8>(member, value)
            },
            MemberType::Char => {
                self.set_char(member, true, value)
            },
            MemberType::Longlong => {
                self.set_num::<i64>(member, value)
            },
            MemberType::String |
            MemberType::Pchar |
            MemberType::Wstring |
            MemberType::PWchar => {
                self.set_string(member, value)
            },
            MemberType::Hwnd |
            MemberType::Handle |
            MemberType::Pointer |
            MemberType::Size => {
                self.set_num::<usize>(member, value)
            },
            MemberType::Struct(_) |
            MemberType::UStruct(_) => {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
            },
            MemberType::Void => {
                // 何もしない
                Ok(())
            },
        }
    }
    fn set_array(&self, member: &UStructMember, index: usize, value: Object) -> EvalResult<()> {
        if index < member.len.unwrap_or(0) {
            match &member.r#type {
                MemberType::Int |
                MemberType::Long |
                MemberType::Bool => {
                    self.set_array_num::<i32>(member, index, value)
                },
                MemberType::Uint |
                MemberType::Dword => {
                    self.set_array_num::<u32>(member, index, value)
                },
                MemberType::Float => {
                    self.set_array_num::<f32>(member, index, value)
                },
                MemberType::Double => {
                    self.set_array_num::<f64>(member, index, value)
                },
                MemberType::Word => {
                    self.set_array_num::<u16>(member, index, value)
                },
                MemberType::Byte |
                MemberType::Boolean => {
                    self.set_array_num::<u8>(member, index, value)
                },
                MemberType::Longlong => {
                    self.set_array_num::<i64>(member, index, value)
                },
                MemberType::Hwnd |
                MemberType::Handle |
                MemberType::Pointer |
                MemberType::Size => {
                    self.set_array_num::<usize>(member, index, value)
                },
                MemberType::Wchar |
                MemberType::Char |
                MemberType::String |
                MemberType::Pchar |
                MemberType::Wstring |
                MemberType::PWchar |
                MemberType::Struct(_) |
                MemberType::UStruct(_) => {
                    Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberIsNotArray))
                },
                MemberType::Void => {
                    // 何もしない
                    Ok(())
                },
            }
        } else {
            Err(UError::new(UErrorKind::UStructError, UErrorMessage::IndexOutOfBounds(index.into())))
        }
    }
    pub fn invoke_method(&self, name: &str, args: Vec<Object>) -> EvalResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "size" | "length" => {
                let size = self.size();
                Ok(size.into())
            },
            "address" => {
                let addr = self.address;
                Ok(addr.into())
            },
            "bufsize" => {
                let obj = args.get(0).ok_or(UError::new(UErrorKind::UStructError, UErrorMessage::BuiltinArgRequiredAt(1)))?;
                let name = obj.to_string();
                let member = self.get_member(&name)?;
                let size = member.buffer_size();
                Ok(size.into())
            }
            _ => {
                Err(UError::new(UErrorKind::UStructError, UErrorMessage::CanNotCallMethod(name.to_string())))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberType {
    Int,
    Long,
    Bool,
    Uint,
    Dword,
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
    Hwnd,
    Handle,
    Pointer,
    Size,
    /// ネストされた構造体
    Struct(StructDef),
    /// 構造体ポインタ
    ///
    /// def_dllでネスト定義された場合のみ使用される
    UStruct(StructDef),
    Void,
}
impl AsMut<MemberType> for MemberType {
    fn as_mut(&mut self) -> &mut MemberType {
        self
    }
}
impl MemberType {
    fn size(&self) -> usize {
        match self {
            MemberType::Int |
            MemberType::Long |
            MemberType::Bool => mem::size_of::<i32>(),
            MemberType::Uint |
            MemberType::Dword => mem::size_of::<u32>(),
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
            MemberType::Hwnd |
            MemberType::Handle |
            MemberType::Pointer |
            MemberType::Size => mem::size_of::<usize>(),
            MemberType::Struct(sdef) => sdef.size,
            MemberType::UStruct(_) => mem::size_of::<usize>(),
            MemberType::Void => mem::size_of::<c_void>(),
        }
    }
    fn alignment(&self) -> usize {
        match self {
            MemberType::Struct(sdef) => {
                sdef.members.iter()
                    .map(|m| match &m.r#type {
                        MemberType::Struct(_) => m.alignment(),
                        t => t.size()
                    })
                    .reduce(|a,b| a.max(b))
                    .unwrap_or_default()
            },
            mt => mt.size()
        }
    }
    fn is_ansi(&self) -> bool {
        match self {
            MemberType::String |
            MemberType::Pchar => true,
            _ => false
        }
    }
    fn is_char(&self) -> bool {
        match self {
            MemberType::PWchar |
            MemberType::Pchar => true,
            _ => false
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
            MemberType::Hwnd => write!(f, "hwnd"),
            MemberType::Handle => write!(f, "handle"),
            MemberType::Pointer => write!(f, "pointer"),
            MemberType::Size => write!(f, "size"),
            MemberType::Struct(sdef) => write!(f, "{}", sdef.name),
            MemberType::UStruct(sdef) => write!(f, "*{}", sdef.name),
            MemberType::Void => write!(f, "void"),
        }
    }
}
impl std::str::FromStr for MemberType {
    type Err = UError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "int" => Ok(MemberType::Int),
            "long" => Ok(MemberType::Long),
            "bool" => Ok(MemberType::Bool),
            "uint" => Ok(MemberType::Uint),
            "string" => Ok(MemberType::String),
            "wstring" => Ok(MemberType::Wstring),
            "float" => Ok(MemberType::Float),
            "double" => Ok(MemberType::Double),
            "word" => Ok(MemberType::Word),
            "dword" => Ok(MemberType::Dword),
            "byte" => Ok(MemberType::Byte),
            "char" => Ok(MemberType::Char),
            "pchar" => Ok(MemberType::Pchar),
            "wchar" => Ok(MemberType::Wchar),
            "pwchar" => Ok(MemberType::PWchar),
            "boolean" => Ok(MemberType::Boolean),
            "longlong" => Ok(MemberType::Longlong),
            "hwnd" => Ok(MemberType::Hwnd),
            "handle" => Ok(MemberType::Handle),
            "pointer" => Ok(MemberType::Pointer),
            "size" => Ok(MemberType::Size),
            "void" => Ok(MemberType::Void),
            other => Err(UError::new(UErrorKind::StructDefError, UErrorMessage::UnknownDllType(other.into())))
        }
    }
}

impl Object {
    fn cast2<T: FromPrimitive>(&self) -> EvalResult<T> {
        match self.as_f64(true) {
            Some(f) => {
                T::from_f64(f)
                    .ok_or(UError::new(
                        UErrorKind::UStructError,
                        UErrorMessage::CastError2(f, std::any::type_name::<T>().to_string())
                    ))
            },
            None => Err(UError::new(UErrorKind::UStructError, UErrorMessage::StructMemberTypeError))
        }
    }
    fn to_num_vec<T>(&self) -> EvalResult<Vec<T>>
        where T: FromPrimitive,
    {
        match self {
            Object::Array(arr) => {
                arr.iter()
                    .map(|o| o.cast2::<T>())
                    .collect::<EvalResult<Vec<T>>>()
            },
            obj => {
                obj.cast2::<T>().map(|t| vec![t])
            }
        }
    }
    pub fn to_string_nullable(&self) -> Option<String> {
        match self {
            Object::Empty |
            Object::EmptyParam |
            Object::Nothing |
            Object::Null => None,
            o => Some(o.to_string())
        }
    }
    fn from_vec_t<T>(vec: Vec<T>, as_array: bool) -> Object
        where T: Into<Object> + Copy
    {
        if as_array {
            let arr = vec.into_iter().map(|n| n.into()).collect();
            Object::Array(arr)
        } else {
            let n = vec[0];
            n.into()
        }
    }
    fn from_vec_t_bool<T>(vec: Vec<T>, as_array: bool) -> Object
        where T: FromPrimitive + Default + PartialEq
    {
        let zero = T::from_i32(0).unwrap_or_default();
        if as_array {
            let arr = vec.into_iter().map(|n| (n != zero).into()).collect();
            Object::Array(arr)
        } else {
            let b = vec[0] != zero;
            b.into()
        }
    }
}