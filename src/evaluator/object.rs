use crate::ast::*;
use crate::evaluator::environment::{NamedObject, Module};
use crate::evaluator::builtins::BuiltinFunction;
use crate::evaluator::{EvalResult};
use crate::evaluator::def_dll::DllArg;
use crate::winapi::{
    bindings::Windows::Win32::UI::WindowsAndMessaging::HWND
};

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::thread::JoinHandle;

use indexmap::IndexMap;
use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use serde_json;

use super::UError;

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
    UStruct(String, usize, Arc<Mutex<UStruct>>) // 構造体インスタンス
}

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
        }
    }
}

impl Object {
    pub fn is_equal(&self, other: &Object) -> bool {
        format!("{}", self) == format!("{}", other)
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
    members: Vec<UStructMember>
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
            members: vec![]
        }
    }

    pub fn add(&mut self, name: String, object: Object, dll_type: DllType) -> EvalResult<()> {
        match DllArg::new(&object, &dll_type) {
            Ok(_) => {},
            Err(e) => return Err(UError::new(
                "UStruct error",
                &format!("type of {} should be {} but got {}", &name, dll_type, e),
                None
            ))
        };
        self.members.push(UStructMember {
            name,
            object,
            dll_type
        });
        Ok(())
    }

    pub fn get(&self, name: String) -> EvalResult<Object> {
        for member in &self.members {
            if member.name == name {
                return Ok(member.object.clone())
            }
        }
        Err(UError::new(
            "UStruct error",
            &format!("{} has no member named {}", &self.name, &name),
            None
        ))
    }

    pub fn set(&mut self, name: String, object: Object) -> EvalResult<()> {
        for member in self.members.iter_mut() {
            if member.name == name {
                match DllArg::new(&object, &member.dll_type) {
                    Ok(_) => {},
                    Err(e) => return Err(UError::new(
                        "UStruct error",
                        &format!("type of {} should be {} but got {}", &name, &member.dll_type, e),
                        None
                    ))
                };
                member.object = object;
                return Ok(())
            }
        }
        Err(UError::new(
            "UStruct error",
            &format!("{} has no member named {}", &self.name, &name),
            None
        ))
    }
}