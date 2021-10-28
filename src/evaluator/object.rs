use crate::ast::*;
use crate::evaluator::environment::{NamedObject, Module};
use crate::evaluator::builtins::BuiltinFunction;
use crate::evaluator::{EvalResult};
use crate::evaluator::def_dll::DllArg;
use crate::evaluator::com_object::VARIANTHelper;
use crate::error::evaluator::{UError,UErrorKind,UErrorMessage};
use crate::evaluator::devtools_protocol::{Browser, Element};

use crate::winapi::{
    to_ansi_bytes, from_ansi_bytes, to_wide_string,
};
use windows::{
    runtime::Handle,
    Win32::{
        Foundation::HWND,
        System::OleAutomation::{
            VARIANT, SAFEARRAY, IDispatch,
        },
    }
};

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::thread::JoinHandle;
use std::mem;

use indexmap::IndexMap;
use libc::{c_void};
use num_traits::Zero;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use serde_json::{self, Value};
use cast;

#[derive(Clone, Debug)]
pub enum Object {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Object>),
    HashTbl(Arc<Mutex<HashTbl>>),
    AnonFunc(Vec<Expression>, BlockStatement, Arc<Mutex<Vec<NamedObject>>>, bool),
    Function(String, Vec<Expression>, BlockStatement, bool, Option<Arc<Mutex<Module>>>),
    AsyncFunction(String, Vec<Expression>, BlockStatement, bool, Option<Arc<Mutex<Module>>>),
    BuiltinFunction(String, i32, BuiltinFunction),
    Module(Arc<Mutex<Module>>),
    Class(String, BlockStatement), // class定義
    Instance(Arc<Mutex<Module>>, u32), // classインスタンス, デストラクタが呼ばれたらNothingになる
    Instances(Vec<String>), // ローカルのインスタンス参照リスト
    DestructorNotFound, // デストラクタがなかった場合に返る、これが来たらエラーにせず終了する
    Null,
    Empty,
    EmptyParam,
    Nothing,
    Continue(u32),
    Break(u32),
    Eval(String),
    Handle(HWND),
    RegEx(String),
    Exit,
    ExitExit(i32),
    SpecialFuncResult(SpecialFuncResultType),
    Global, // globalを示す
    This(Arc<Mutex<Module>>),   // thisを示す
    UObject(Arc<Mutex<serde_json::Value>>),
    UChild(Arc<Mutex<serde_json::Value>>, String),
    DynamicVar(fn()->Object), // 特殊変数とか
    Version(Version),
    ExpandableTB(String),
    Enum(UEnum),
    Task(UTask),
    DefDllFunction(String, String, Vec<DefDllParam>, DllType), // 関数名, dllパス, 引数の型, 戻り値の型
    Struct(String, usize, Vec<(String, DllType)>), // 構造体定義: name, size, [(member name, type)]
    UStruct(String, usize, Arc<Mutex<UStruct>>), // 構造体インスタンス
    ComObject(IDispatch),
    ComMember(IDispatch, String),
    Variant(Variant),
    // ComObject(Arc<Mutex<IDispatch>>),
    // Variant(Arc<Mutex<VARIANT>>),
    SafeArray(SAFEARRAY),
    VarArgument(Expression),
    Browser(Browser),
    BrowserFunc(Browser, String),
    Element(Element),
    ElementFunc(Element, String),
}

unsafe impl Send for Object {}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Object::Num(ref value) => write!(f, "{}", value),
            Object::String(ref value) => write!(f, "{}", value),
            Object::Bool(b) => write!(f, "{}", if b {"True"} else {"False"}),
            Object::Array(ref objects) => {
                let mut members = String::new();
                for (i, obj) in objects.iter().enumerate() {
                    if i < 1 {
                        members.push_str(&format!("{}", obj))
                    } else {
                        members.push_str(&format!(", {}", obj))
                    }
                }
                write!(f, "[{}]", members)
            },
            Object::HashTbl(ref hash) => {
                let mut key_values = String::new();
                for (i, (k, v)) in hash.lock().unwrap().map().iter().enumerate() {
                    if i < 1 {
                        key_values.push_str(&format!("\"{}\": {}", k, v))
                    } else {
                        key_values.push_str(&format!(", \"{}\": {}", k, v))
                    }
                }
                write!(f, "{{{}}}", key_values)
            },
            Object::Function(ref name, ref params, _, is_proc, ref instance) => {
                let mut arguments = String::new();
                let func_name = if instance.is_some() {
                    instance.clone().unwrap().lock().unwrap().name()
                } else {
                    name.to_string()
                };
                for (i, e) in params.iter().enumerate() {
                    match e {
                        Expression::Params(ref p) => if i < 1 {
                            arguments.push_str(&format!("{}", p))
                        } else {
                            arguments.push_str(&format!(", {}", p))
                        },
                        _ => ()
                    }
                }
                if is_proc {
                    write!(f, "procedure: {}({})", func_name, arguments)
                } else {
                    write!(f, "function: {}({})", func_name, arguments)
                }
            },
            Object::AsyncFunction(ref name, ref params, _, is_proc, ref instance) => {
                let mut arguments = String::new();
                let func_name = if instance.is_some() {
                    instance.clone().unwrap().lock().unwrap().name()
                } else {
                    name.to_string()
                };
                for (i, e) in params.iter().enumerate() {
                    match e {
                        Expression::Params(ref p) => if i < 1 {
                            arguments.push_str(&format!("{}", p))
                        } else {
                            arguments.push_str(&format!(", {}", p))
                        },
                        _ => ()
                    }
                }
                if is_proc {
                    write!(f, "async procedure: {}({})", func_name, arguments)
                } else {
                    write!(f, "async function: {}({})", func_name, arguments)
                }
            },
            Object::AnonFunc(ref params, _, _, is_proc) => {
                let mut arguments = String::new();
                for (i, e) in params.iter().enumerate() {
                    match e {
                        Expression::Params(ref p) => if i < 1 {
                            arguments.push_str(&format!("{}", p))
                        } else {
                            arguments.push_str(&format!(", {}", p))
                        },
                        _ => ()
                    }
                }
                if is_proc {
                    write!(f, "anonymous_proc({})", arguments)
                } else {
                    write!(f, "anonymous_func({})", arguments)
                }
            },
            Object::BuiltinFunction(ref name, _, _) => write!(f, "builtin: {}()", name),
            Object::Null => write!(f, "NULL"),
            Object::Empty => write!(f, ""),
            Object::EmptyParam => write!(f, "__EMPTYPARAM__"),
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(ref n) => write!(f, "Continue {}", n),
            Object::Break(ref n) => write!(f, "Break {}", n),
            Object::Exit => write!(f, "Exit"),
            Object::ExitExit(ref n) => write!(f, "ExitExit ({})", n),
            Object::Eval(ref value) => write!(f, "{}", value),
            Object::SpecialFuncResult(_) => write!(f, "特殊関数の戻り値"),
            Object::Module(ref m) => write!(f, "module: {}", m.lock().unwrap().name()),
            Object::Class(ref name, _) => write!(f, "class: {}", name),
            Object::Instance(ref m, id) => {
                let ins = m.lock().unwrap();
                if ins.is_disposed() {
                    write!(f, "NOTHING")
                } else {
                    write!(f, "instance of {} [{}]", ins.name(), id)
                }
            },
            Object::Instances(ref v) => write!(f, "auto disposable instances: {}", v.len()),
            Object::DestructorNotFound => write!(f, "no destructor"),
            Object::Handle(h) => write!(f, "{:?}", h),
            Object::RegEx(ref re) => write!(f, "regex: {}", re),
            Object::Global => write!(f, "GLOBAL"),
            Object::This(ref m) => write!(f, "THIS ({})", m.lock().unwrap().name()),
            Object::UObject(ref v) => {
                let value = v.lock().unwrap();
                write!(f, "UObject: {}", serde_json::to_string(&value.clone()).map_or_else(|e| format!("{}", e), |j| j))
            },
            Object::UChild(ref u, ref p) => {
                let v = u.lock().unwrap().pointer(p.as_str()).unwrap_or(&serde_json::Value::Null).clone();
                write!(f, "UObject: {}", serde_json::to_string(&v).map_or_else(|e| format!("{}", e), |j| j))
            },
            Object::DynamicVar(func) => write!(f, "{}", func()),
            Object::Version(ref v) => write!(f, "{}", v),
            Object::ExpandableTB(_) => write!(f, "expandable textblock"),
            Object::Enum(ref e) => write!(f, "Enum {}", e.name),
            Object::Task(ref t) => write!(f, "Task [{}]", t),
            Object::DefDllFunction(ref name, _, _, _) => write!(f, "def_dll: {}", name),
            Object::Struct(ref name, _, _) => write!(f, "struct: {}", name),
            Object::UStruct(ref name, _, _) => write!(f, "instance of struct: {}", name),
            // Object::UStruct(_, _, ref m) => {
            //     let u = m.lock().unwrap();
            //     write!(f, "{:?}", u)
            // },
            Object::ComObject(ref d) => write!(f, "{:?}", d),
            Object::ComMember(_, _) => write!(f, "Com member"),
            Object::Variant(ref v) => write!(f, "Variant({})", v.0.vt()),
            Object::SafeArray(_) => write!(f, "SafeArray"),
            Object::VarArgument(_) => write!(f, "var"),
            Object::Browser(ref b) => write!(f, "Browser: {}:{} ({})", b.btype, b.port, b.id),
            Object::BrowserFunc(_,ref s) => write!(f, "Browser.{}", s),
            Object::Element(ref e) => write!(f, "Element: {}", e.node_id),
            Object::ElementFunc(_, ref s) => write!(f, "Element.{}", s),
        }
    }
}

impl Object {
    pub fn is_equal(&self, other: &Object) -> bool {
        format!("{}", self) == format!("{}", other)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Object::Empty |
            Object::EmptyParam |
            Object::Bool(false) |
            Object::Nothing => false,
            Object::Instance(m, _) => {
                let ins = m.lock().unwrap();
                ! ins.is_disposed()
            },
            Object::String(s) |
            Object::ExpandableTB(s) => s.len() > 0,
            Object::Array(arr) => arr.len() > 0,
            Object::Num(n) => ! n.is_zero(),
            Object::Handle(h) => ! h.is_invalid(),
            _ => true
        }
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            Object::Num(ref n) => {
                n.to_string().hash(state)
            },
            Object::Bool(ref b) => b.hash(state),
            Object::String(ref s) => s.hash(state),
            _ => "".hash(state),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SpecialFuncResultType {
    GetEnv,
    ListModuleMember(String),
    BuiltinConstName(Option<Expression>),
    Task(Box<Object>, Vec<(Option<Expression>, Object)>),
}

// hashtbl

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum HashTblEnum {
    HASH_CASECARE = 0x1000,
    HASH_SORT = 0x2000,
    HASH_EXISTS = -103,
    HASH_REMOVE = -104,
    HASH_KEY = -101,
    HASH_VAL = -102,
    HASH_REMOVEALL = -109,
    HASH_UNKNOWN = 0,
}

#[derive(Clone, Debug)]
pub struct HashTbl {
    map: IndexMap<String, Object>,
    sort: bool,
    casecare: bool,
}

impl HashTbl {
    pub fn new(sort: bool, casecare: bool) -> Self {
        HashTbl {
            map: IndexMap::new(),
            sort,
            casecare
        }
    }

    pub fn map(&self) -> IndexMap<String, Object> {
        self.map.clone()
    }

    pub fn keys(&self) -> Vec<Object> {
        self.map.keys().map(|key| Object::String(key.clone())).collect::<Vec<Object>>()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn insert(&mut self, name: String, value: Object) {
        let key = if ! self.casecare { name.to_ascii_uppercase() } else { name };
        let new = self.map.contains_key(&key);
        self.map.insert(key, value);
        if self.sort && ! new { // sort がtrueでかつ追加した場合はソートする
            self.map.sort_keys();
        }
    }

    pub fn get(&self, name: String) -> Object {
        let key = if ! self.casecare { name.to_ascii_uppercase() } else { name };
        self.map.get(&key).unwrap_or(&Object::Empty).clone()
    }
    // hash[i, hash_key]
    pub fn get_key(&self, index: usize) -> Object {
        self.map.get_index(index).map_or(
            Object::Empty,
            |(s, _)| Object::String(s.clone())
        )
    }
    // hash[i, hash_val]
    pub fn get_value(&self, index: usize) -> Object {
        self.map.get_index(index).map_or(
            Object::Empty,
            |(_, v)| v.clone()
        )
    }
    // hash[key, hash_exists]
    pub fn check(&self, name: String) -> Object {
        let key = if ! self.casecare { name.to_ascii_uppercase() } else { name };
        Object::Bool(self.map.contains_key(&key))

    }
    // hash[key, hash_remove]
    pub fn remove(&mut self, name: String) -> Object {
        let key = if ! self.casecare { name.to_ascii_uppercase() } else { name };
        let removed = self.map.remove(&key).is_some();
        Object::Bool(removed)
    }
    // hash = hash_removeall
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[derive(Debug, Clone)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version{major, minor, patch}
    }
    pub fn parse(&self) -> f64 {
        format!("{}.{}{}", self.major, self.minor, self.patch).parse().unwrap_or(0.0)
    }
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.split('.').collect::<Vec<&str>>();
        let major = v[0].parse::<u32>()?;
        let minor = v[1].parse::<u32>()?;
        let patch = v[2].parse::<u32>()?;
        Ok(Version{major, minor, patch})
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major &&
        self.minor == other.minor &&
        self.patch == other.patch
    }
}

impl PartialEq<String> for Version {
    fn eq(&self, other: &String) -> bool {
        self.to_string() == *other
    }
}

impl PartialEq<f64> for Version {
    fn eq(&self, other: &f64) -> bool {
        self.parse() == *other
    }
}

#[derive(Clone)]
pub struct Variant(pub VARIANT);

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VARIANT")
            .field("vt", &self.0.vt())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct UTask {
    pub handle: Arc<Mutex<Option<JoinHandle<EvalResult<Object>>>>>,
}

impl fmt::Display for UTask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let flag = self.handle.lock().unwrap().is_none();
        write!(f, "{}", if flag {"done"} else {"running"})
    }
}

#[derive(Debug, Clone)]
pub struct UStruct {
    name: String,
    members: Vec<UStructMember>,
    size: usize,
}

#[derive(Debug, Clone)]
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

impl Into<Object> for String {
    fn into(self) -> Object {
        Object::String(self)
    }
}
impl Into<Object> for f64 {
    fn into(self) -> Object {
        Object::Num(self)
    }
}
impl Into<Object> for Value {
    fn into(self) -> Object {
        match self {
            serde_json::Value::Null => Object::Null,
            serde_json::Value::Bool(b) => Object::Bool(b),
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(f) => Object::Num(f),
                None => Object::Num(f64::NAN)
            },
            serde_json::Value::String(s) =>Object::String(s),
            serde_json::Value::Array(_) |
            serde_json::Value::Object(_) => Object::UObject(Arc::new(Mutex::new(self))),
        }
    }
}

impl Into<i32> for Object {
    fn into(self) -> i32 {
        match self {
            Object::Num(n) => n as i32,
            Object::Bool(b) => b as i32,
            Object::String(ref s) => match s.parse::<i32>() {
                Ok(n) => n,
                Err(_) => 0
            },
            _ => 0
        }
    }
}

pub trait ValueHelper {
    fn get_case_insensitive(&self, key: &str) -> Option<Value>;
}

impl ValueHelper for Value {
    fn get_case_insensitive(&self, key: &str) -> Option<Value> {
        match self {
            Value::Object(map) => {
                let upper = key.to_ascii_uppercase();
                let filtered = map.iter()
                                        .filter(|(k, _)| k.to_ascii_uppercase() == upper)
                                        .collect::<Vec<(&String, &Value)>>();
                if filtered.len() == 0 {
                    None
                } else if filtered.len() == 1 {
                    Some(filtered[0].1.clone())
                } else {
                    // 複数あった場合は完全一致を返す
                    // 完全一致がなければ1つ目を返す
                    let matched = filtered.iter()
                                        .filter(|(k, _)| k.as_str() == key)
                                        .map(|(_,v)| v.clone())
                                        .collect::<Vec<_>>();
                    if matched.len() > 0 {
                        Some(matched[0].clone())
                    } else {
                        Some(filtered[0].1.clone())
                    }
                }
            },
            _ => None,
        }
    }
}