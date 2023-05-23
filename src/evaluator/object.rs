pub mod hashtbl;
pub mod version;
pub mod variant;
pub mod utask;
pub mod ustruct;
pub mod module;
pub mod function;
pub mod uobject;
pub mod fopen;
pub mod class;
pub mod browser;
mod web;

pub use self::hashtbl::{HashTbl, HashTblEnum};
pub use self::version::Version;
pub use self::variant::Variant;
pub use self::utask::UTask;
pub use self::ustruct::{UStruct, UStructMember};
pub use self::module::Module;
pub use self::function::Function;
pub use self::uobject::UObject;
pub use self::fopen::*;
pub use self::class::ClassInstance;
use browser::{BrowserBuilder, Browser, TabWindow, RemoteObject, BrowserFunction};
pub use web::{WebRequest, WebResponse, WebFunction};

use crate::ast::*;
use crate::evaluator::environment::Layer;
use crate::evaluator::builtins::{
    BuiltinFunction,
    system_controls::gettime::datetime_str_to_f64,
};
use crate::evaluator::com_object::VARIANTHelper;
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use windows::{
    Win32::{
        Foundation::HWND,
        System::{
            Com::{
                IDispatch,
                SAFEARRAY,
            }
        },
    }
};

use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::ops::{Add, Sub, Mul, Div, Rem, BitOr, BitAnd, BitXor};
use std::cmp::{Ordering};

use num_traits::Zero;
use serde_json::{self, Value};

use super::EvalResult;

#[derive(Clone)]
pub enum Object {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Object>),
    HashTbl(Arc<Mutex<HashTbl>>),
    AnonFunc(Function),
    Function(Function),
    AsyncFunction(Function),
    BuiltinFunction(String, i32, BuiltinFunction),
    Module(Arc<Mutex<Module>>),
    Class(String, BlockStatement), // class定義
    Instance(Arc<Mutex<ClassInstance>>), // classインスタンス, デストラクタが呼ばれたらNothingになる
    Null,
    Empty,
    EmptyParam,
    Nothing,
    Continue(u32),
    Break(u32),
    Handle(HWND),
    RegEx(String),
    Exit,
    Global, // globalを示す
    This(Arc<Mutex<Module>>),   // thisを示す
    UObject(UObject),
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
    /// COMメソッドのvar引数
    VarArgument(Expression),
    BrowserBuilder(Arc<Mutex<BrowserBuilder>>),
    Browser(Browser),
    TabWindow(TabWindow),
    RemoteObject(RemoteObject),
    /// ブラウザ関連オブジェクトのメソッド実行用の型
    BrowserFunction(BrowserFunction),
    Fopen(Arc<Mutex<Fopen>>),
    ByteArray(Vec<u8>),
    /// 参照渡しされたパラメータ変数
    Reference(Expression, Arc<Mutex<Layer>>),
    /// WebRequestオブジェクト
    WebRequest(Arc<Mutex<WebRequest>>),
    /// WebResponseオブジェクト
    WebResponse(WebResponse),
    WebFunction(WebFunction),
}
impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Num(arg0) => f.debug_tuple("Num").field(arg0).finish(),
            Object::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Object::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Object::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Object::HashTbl(arg0) => f.debug_tuple("HashTbl").field(arg0).finish(),
            Object::AnonFunc(arg0) => f.debug_tuple("AnonFunc").field(arg0).finish(),
            Object::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Object::AsyncFunction(arg0) => f.debug_tuple("AsyncFunction").field(arg0).finish(),
            Object::BuiltinFunction(arg0, arg1, _) => f.debug_tuple("BuiltinFunction").field(arg0).field(arg1).finish(),
            Object::Module(arg0) => f.debug_tuple("Module").field(arg0).finish(),
            Object::Class(arg0, arg1) => f.debug_tuple("Class").field(arg0).field(arg1).finish(),
            Object::Instance(arg0) => f.debug_tuple("Instance").field(arg0).finish(),
            Object::Null => write!(f, "Null"),
            Object::Empty => write!(f, "Empty"),
            Object::EmptyParam => write!(f, "EmptyParam"),
            Object::Nothing => write!(f, "Nothing"),
            Object::Continue(arg0) => f.debug_tuple("Continue").field(arg0).finish(),
            Object::Break(arg0) => f.debug_tuple("Break").field(arg0).finish(),
            Object::Handle(arg0) => f.debug_tuple("Handle").field(arg0).finish(),
            Object::RegEx(arg0) => f.debug_tuple("RegEx").field(arg0).finish(),
            Object::Exit => write!(f, "Exit"),
            Object::Global => write!(f, "Global"),
            Object::This(arg0) => f.debug_tuple("This").field(arg0).finish(),
            Object::UObject(arg0) => f.debug_tuple("UObject").field(arg0).finish(),
            Object::DynamicVar(arg0) => f.debug_tuple("DynamicVar").field(arg0).finish(),
            Object::Version(arg0) => f.debug_tuple("Version").field(arg0).finish(),
            Object::ExpandableTB(arg0) => f.debug_tuple("ExpandableTB").field(arg0).finish(),
            Object::Enum(arg0) => f.debug_tuple("Enum").field(arg0).finish(),
            Object::Task(arg0) => f.debug_tuple("Task").field(arg0).finish(),
            Object::DefDllFunction(arg0, arg1, arg2, arg3) => f.debug_tuple("DefDllFunction").field(arg0).field(arg1).field(arg2).field(arg3).finish(),
            Object::Struct(arg0, arg1, arg2) => f.debug_tuple("Struct").field(arg0).field(arg1).field(arg2).finish(),
            Object::UStruct(arg0, arg1, arg2) => f.debug_tuple("UStruct").field(arg0).field(arg1).field(arg2).finish(),
            Object::ComObject(arg0) => f.debug_tuple("ComObject").field(arg0).finish(),
            Object::ComMember(arg0, arg1) => f.debug_tuple("ComMember").field(arg0).field(arg1).finish(),
            Object::Variant(arg0) => f.debug_tuple("Variant").field(arg0).finish(),
            Object::SafeArray(arg0) => f.debug_tuple("SafeArray").field(arg0).finish(),
            Object::VarArgument(arg0) => f.debug_tuple("VarArgument").field(arg0).finish(),
            Object::BrowserBuilder(arg0) => f.debug_tuple("BrowserBuilder").field(arg0).finish(),
            Object::Browser(arg0) => f.debug_tuple("Browser").field(arg0).finish(),
            Object::TabWindow(arg0) => f.debug_tuple("TabWindow").field(arg0).finish(),
            Object::RemoteObject(arg0) => f.debug_tuple("RemoteObject").field(arg0).finish(),
            Object::BrowserFunction(arg0) => f.debug_tuple("BrowserFunction").field(arg0).finish(),
            Object::Fopen(arg0) => f.debug_tuple("Fopen").field(arg0).finish(),
            Object::ByteArray(arg0) => f.debug_tuple("ByteArray").field(arg0).finish(),
            Object::Reference(arg0, arg1) => f.debug_tuple("Reference").field(arg0).field(arg1).finish(),
            Object::WebRequest(arg0) => f.debug_tuple("WebRequest").field(arg0).finish(),
            Object::WebResponse(arg0) => f.debug_tuple("WebResponse").field(arg0).finish(),
            Object::WebFunction(_) => write!(f, "WebFunction"),

        }
    }
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
            Object::Function(ref func) => {
                let title = if func.is_proc {"procedure"} else {"function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}: {}({})", title, func.name.as_ref().unwrap(), params)
            },
            Object::AsyncFunction(ref func) => {
                let title = if func.is_proc {"procedure"} else {"function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}: {}({})", title, func.name.as_ref().unwrap(), params)
            },
            Object::AnonFunc(ref func) => {
                let title = if func.is_proc {"anonymous procedure"} else {"anonymous function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}({})", title, params)
            },
            Object::BuiltinFunction(ref name, _, _) => write!(f, "builtin: {}()", name),
            Object::Null => write!(f, "NULL"),
            Object::Empty => write!(f, ""),
            Object::EmptyParam => write!(f, "__EMPTYPARAM__"),
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(ref n) => write!(f, "Continue {}", n),
            Object::Break(ref n) => write!(f, "Break {}", n),
            Object::Exit => write!(f, "Exit"),
            Object::Module(ref m) => write!(f, "module: {}", m.lock().unwrap().name()),
            Object::Class(ref name, _) => write!(f, "class: {}", name),
            Object::Instance(ref m) => {
                let ins = m.lock().unwrap();
                if ins.is_dropped {
                    write!(f, "NOTHING")
                } else {
                    write!(f, "instance of {}", ins.name)
                }
            },
            Object::Handle(h) => write!(f, "{:?}", h),
            Object::RegEx(ref re) => write!(f, "regex: {}", re),
            Object::Global => write!(f, "GLOBAL"),
            Object::This(ref m) => write!(f, "THIS ({})", m.lock().unwrap().name()),
            Object::UObject(ref uobj) => {
                write!(f, "{}", uobj)
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
            Object::Variant(ref v) => write!(f, "Variant({})", v.0.vt().0),
            Object::SafeArray(_) => write!(f, "SafeArray"),
            Object::VarArgument(_) => write!(f, "var"),
            Object::BrowserBuilder(_) => write!(f, "BrowserBuilder"),
            Object::Browser(ref b) => write!(f, "Browser: {b})"),
            Object::TabWindow(ref t) => write!(f, "TabWindow: {t}"),
            Object::RemoteObject(ref r) => write!(f, "{r}"),
            Object::BrowserFunction(_) => write!(f, "BrowserFunction"),
            Object::Fopen(ref arc) => {
                let fopen = arc.lock().unwrap();
                write!(f, "{}", &*fopen)
            },
            Object::ByteArray(ref arr) => write!(f, "{:?}", arr),
            Object::Reference(_, _) => write!(f, "Reference"),
            Object::WebRequest(ref req) => {
                let mutex = req.lock().unwrap();
                write!(f, "{mutex}")
            },
            Object::WebResponse(ref res) => write!(f, "{res}"),
            Object::WebFunction(_) => write!(f, "WebFunction"),
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Object::EmptyParam => match other {
                Object::EmptyParam => true,
                _ => false
            },
            Object::Num(n) => match other {
                Object::Num(n2) => n == n2,
                Object::String(s) => n.to_string() == s.to_string(),
                Object::Empty => 0.0 == *n,
                Object::Bool(b) => ! n.is_zero() && *b,
                _ => false
            },
            Object::String(s) => match other {
                Object::Num(n) => s.to_string() == n.to_string(),
                Object::String(s2) => s.to_string() == s2.to_string(),
                Object::Empty => false,
                Object::Bool(b) => b.to_string().to_ascii_lowercase() == s.to_ascii_lowercase(),
                _ => false
            },
            Object::Bool(b) => match other {
                Object::Bool(b2) => b == b2,
                Object::String(s) => b.to_string().to_ascii_lowercase() == s.to_ascii_lowercase(),
                Object::Empty => false && *b,
                _ => false
            },
            Object::Array(a) => if let Object::Array(a2) = other {a == a2} else {false},
            Object::HashTbl(h) => if let Object::HashTbl(h2) = other {
                let _tmp = h.lock().unwrap();
                h2.try_lock().is_err()
            } else {false},
            Object::AnonFunc(f1) => if let Object::AnonFunc(f2) = other {f1 == f2} else {false},
            Object::Function(f1) => if let Object::Function(f2) = other {f1 == f2} else {false},
            Object::AsyncFunction(f1) => if let Object::AsyncFunction(f2) = other {f1 == f2} else {false},
            Object::BuiltinFunction(n, _, _) => if let Object::BuiltinFunction(n2,_,_) = other {n == n2} else {false},
            Object::Module(m) => if let Object::Module(m2) = other {
                let _tmp = m.lock().unwrap();
                m2.try_lock().is_err()
            } else {false},
            Object::Class(n, _) => if let Object::Class(n2,_) = other {n==n2} else {false},
            Object::Instance(m1) => if let Object::Instance(m2) = other {
                let _ins = m1.lock().unwrap();
                m2.try_lock().is_err()
            } else {false},
            Object::Null => match other {
                Object::Null => true,
                _ => false
            },
            Object::Empty => match other {
                Object::Empty => true,
                Object::Num(n) => &0.0 == n,
                Object::String(_) => false,
                _ => false,
            },
            Object::Nothing => match other {
                Object::Nothing => true,
                _ => false,
            },
            Object::Continue(_) => false,
            Object::Break(_) => false,
            Object::Handle(h) => if let Object::Handle(h2) = other {h==h2} else {false},
            Object::RegEx(r) => if let Object::RegEx(r2) = other {r==r2} else {false},
            Object::Exit => false,
            Object::Global => false,
            Object::This(m) => if let Object::This(m2) = other {
                let _tmp = m.lock().unwrap();
                m2.try_lock().is_err()
            } else {false},
            Object::UObject(uobj) => if let Object::UObject(uobj2) = other {
                uobj == uobj2
            } else {false},
            Object::DynamicVar(f) => if let Object::DynamicVar(f2) = other {f() == f2()} else {false},
            Object::Version(v) => if let Object::Version(v2) = other {v == v2} else {false},
            Object::ExpandableTB(_) => false,
            Object::Enum(e) => if let Object::Enum(e2) = other {e==e2} else {false},
            Object::Task(_) => false,
            Object::DefDllFunction(n, p, v, t) => if let Object::DefDllFunction(n2,p2,v2,t2) = other {
                n==n2 && p==p2 && v==v2 && t==t2
            } else {false},
            Object::Struct(n, s, v) => if let Object::Struct(n2,s2,v2) = other {
                n==n2 && s==s2 && v==v2
            } else {false},
            Object::UStruct(n, s, u) => if let Object::UStruct(n2,s2,u2) = other {
                let _tmp = u.lock().unwrap();
                let is_same_struct = u2.try_lock().is_err();
                n==n2 && s==s2 && is_same_struct
            } else {false},
            Object::ComObject(d) => if let Object::ComObject(d2) = other {
                format!("{:?}", d) == format!("{:?}", d2)
            } else {false},
            Object::ComMember(d, n) => if let Object::ComMember(d2,n2) = other {
                format!("{:?}.{}", d, n) == format!("{:?}.{}", d2, n2)
            } else {false},
            Object::Variant(v) => if let Object::Variant(v2) = other {v == v2} else {false},
            Object::SafeArray(_) => false,
            Object::VarArgument(e) => if let Object::VarArgument(e2) = other {e==e2} else {false},
            Object::BrowserBuilder(b) => if let Object::BrowserBuilder(b2) = other {
                let _tmp = b.lock().unwrap();
                b2.try_lock().is_err()
            } else {false},
            Object::Browser(b) => if let Object::Browser(b2) = other {b == b2} else {false},
            Object::TabWindow(t) => if let Object::TabWindow(t2) = other {t == t2} else {false},
            Object::RemoteObject(r) => if let Object::RemoteObject(r2) = other {r == r2} else {false},
            Object::BrowserFunction(_) => false,
            Object::Fopen(f1) => if let Object::Fopen(f2) = other {
                let _tmp = f1.lock().unwrap();
                let result = f2.try_lock().is_err();
                result
            } else {false},
            Object::ByteArray(arr1) => if let Object::ByteArray(arr2) = other {
                arr1 == arr2
            } else {false},
            // 比較されることはない
            Object::Reference(_, _) => false,
            Object::WebRequest(req) => {
                if let Object::WebRequest(req2) = other {
                    let _tmp = req.lock();
                    req2.try_lock().is_err()
                } else {false}
            },
            Object::WebResponse(res) => {
                if let Object::WebResponse(res2) = other { res == res2 } else {false}
            },
            Object::WebFunction(_) => false,
        }
    }
}

impl Object {
    pub fn is_equal(&self, other: &Object) -> bool {
        self == other
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Object::Empty |
            Object::EmptyParam |
            Object::Bool(false) |
            Object::Nothing => false,
            Object::Instance(m) => {
                let ins = m.lock().unwrap();
                ! ins.is_dropped
            },
            Object::String(s) |
            Object::ExpandableTB(s) => s.len() > 0,
            Object::Array(arr) => arr.len() > 0,
            Object::Num(n) => ! n.is_zero(),
            Object::Handle(h) => h.0 > 0,
            _ => true
        }
    }
    pub fn as_f64(&self, null_as_zero: bool) -> Option<f64> {
        match self {
            Object::Num(n) => Some(*n),
            Object::Bool(b) => {
                let n = if *b {1.0} else {0.0};
                Some(n)
            },
            Object::Empty => Some(0.0),
            Object::String(s) => {
                // 文字列はf64への変換を試みる
                match s.parse::<f64>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        // だめなら日時にしてみる
                        match datetime_str_to_f64(s) {
                            Some(n) => Some(n),
                            None => {
                                // さらにダメならバージョンにしてみる
                                let version = Version::from_str(s).ok()?.parse();
                                Some(version)
                            }
                        }
                    },
                }
            },
            Object::Null => if null_as_zero {
                Some(0.0)
            } else {
                None
            },
            _ => None
        }
    }
    /// 以下を通常の値型に変換する
    /// - Variant
    fn to_uwscr_object(self) -> Result<Object, UError> {
        match self {
            Object::Variant(v) => {
                Object::from_variant(&v.0)
            },
            obj => Ok(obj)
        }
    }
    pub fn logical_and(&self, other: &Object) -> EvalResult<Object> {
        let result = self.is_truthy() && other.is_truthy();
        Ok(result.into())
    }
    pub fn logical_or(&self, other: &Object) -> EvalResult<Object> {
        let result = self.is_truthy() || other.is_truthy();
        Ok(result.into())
    }
    pub fn logical_xor(&self, other: &Object) -> EvalResult<Object> {
        let result = self.is_truthy() != other.is_truthy();
        Ok(result.into())
    }
    pub fn equal(&self, other: &Object) -> EvalResult<Object> {
        let result = self.eq(other);
        Ok(result.into())
    }
    pub fn not_equal(&self, other: &Object) -> EvalResult<Object> {
        let result = self.ne(other);
        Ok(result.into())
    }
    pub fn greater_than_equal(&self, other: &Object) -> EvalResult<Object> {
        let result = self.ge(other);
        Ok(result.into())
    }
    pub fn greater_than(&self, other: &Object) -> EvalResult<Object> {
        let result = self.gt(other);
        Ok(result.into())
    }
    pub fn less_than_equal(&self, other: &Object) -> EvalResult<Object> {
        let result = self.le(other);
        Ok(result.into())
    }
    pub fn less_than(&self, other: &Object) -> EvalResult<Object> {
        let result = self.lt(other);
        Ok(result.into())
    }
    fn bit_or(n1: f64, n2: f64) -> Object {
        let new = n1 as i64 | n2 as i64;
        Object::Num(new as f64)
    }
    fn bit_and(n1: f64, n2: f64) -> Object {
        let new = n1 as i64 & n2 as i64;
        Object::Num(new as f64)
    }
    fn bit_xor(n1: f64, n2: f64) -> Object {
        let new = n1 as i64 ^ n2 as i64;
        Object::Num(new as f64)
    }
    fn is_zero(&self) -> bool {
        if let Some(n) = self.as_f64(false) {
            n.is_zero()
        } else {
            false
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

impl Default for Object {
    fn default() -> Self {
        Object::Empty
    }
}

impl Into<Object> for String {
    fn into(self) -> Object {
        Object::String(self)
    }
}
impl Into<Object> for &str {
    fn into(self) -> Object {
        Object::String(self.to_string())
    }
}
impl Into<Object> for f64 {
    fn into(self) -> Object {
        Object::Num(self)
    }
}
impl Into<Object> for u16 {
    fn into(self) -> Object {
        Object::Num(self as f64)
    }
}
impl Into<Object> for i32 {
    fn into(self) -> Object {
        Object::Num(self as f64)
    }
}
impl Into<Object> for i64 {
    fn into(self) -> Object {
        Object::Num(self as f64)
    }
}
impl Into<Object> for u32 {
    fn into(self) -> Object {
        Object::Num(self as f64)
    }
}
impl Into<Object> for usize {
    fn into(self) -> Object {
        Object::Num(self as f64)
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
            serde_json::Value::Object(_) => Object::UObject(UObject::new(self)),
        }
    }
}
impl From<&Value> for Object {
    fn from(v: &Value) -> Self {
        match v {
            Value::Null => Object::Null,
            Value::Bool(b) => Object::Bool(*b),
            Value::Number(n) => match n.as_f64() {
                Some(f) => Object::Num(f),
                None => Object::Num(f64::NAN)
            },
            Value::String(s) => Object::String(s.to_string()),
            Value::Array(_) |
            Value::Object(_) => {
                let uobj = UObject::new(v.clone());
                Object::UObject(uobj)
            },
        }
    }
}
impl Into<Object> for Option<String> {
    fn into(self) -> Object {
        if let Some(s) = self {
            Object::String(s)
        } else {
            Object::Empty
        }
    }
}
impl Into<Object> for Option<&str> {
    fn into(self) -> Object {
        if let Some(s) = self {
            Object::String(s.to_string())
        } else {
            Object::Empty
        }
    }
}
impl Into<Object> for Vec<String> {
    fn into(self) -> Object {
        let arr = self.into_iter()
            .map(|s| s.into())
            .collect();
        Object::Array(arr)
    }
}
impl Into<Object> for bool {
    fn into(self) -> Object {
        Object::Bool(self)
    }
}
impl  Into<Object> for Vec<u8> {
    fn into(self) -> Object {
        Object::ByteArray(self)
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

pub trait ValueExt {
    fn get_case_insensitive(&self, key: &str) -> Option<&Value>;
    fn get_case_insensitive_mut(&mut self, key: &str) -> Option<&mut Value>;
}

impl ValueExt for Value {
    fn get_case_insensitive(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => {
                let upper = key.to_ascii_uppercase();
                let map2 = map.clone();
                let keys_found = map2.iter()
                                            .filter(|(k, _)| k.to_ascii_uppercase() == upper)
                                            .map(|(k,_)| k.as_str())
                                            .collect::<Vec<_>>();
                if keys_found.len() == 0 {
                    None
                } else {
                    // 複数あった場合は完全一致を返す
                    // 完全一致がなければ1つ目を返す
                    if keys_found.contains(&key) {
                        map.get(key)
                    } else {
                        map.get(keys_found[0])
                    }
                }
            },
            _ => None,
        }
    }
    fn get_case_insensitive_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Object(ref mut map) => {
                let upper = key.to_ascii_uppercase();
                let map2 = map.clone();
                let keys_found = map2.iter()
                                            .filter(|(k, _)| k.to_ascii_uppercase() == upper)
                                            .map(|(k,_)| k.as_str())
                                            .collect::<Vec<_>>();
                if keys_found.len() == 0 {
                    None
                } else {
                    // 複数あった場合は完全一致を返す
                    // 完全一致がなければ1つ目を返す
                    if keys_found.contains(&key) {
                        map.get_mut(key)
                    } else {
                        map.get_mut(keys_found[0])
                    }
                }
            },
            _ => None,
        }
    }
}

/* 演算 */
impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.as_f64(true), other.as_f64(true)) {
            // ともに数値にできるなら数値として比較
            (Some(n1), Some(n2)) => n1.partial_cmp(&n2),
            // そうでなければ文字列として比較
            _ => self.to_string().partial_cmp(&other.to_string()),
        }
    }
}

impl Add for Object {
    type Output = Result<Object, UError>;

    fn add(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(true) {
                    Ok(Object::Num(n + n2))
                } else {
                    if let Object::String(s2) = rhs {
                        let mut s = n.to_string();
                        s.push_str(&s2);
                        Ok(Object::String(s))
                    } else {
                        Err(UError::new(
                            UErrorKind::OperatorError,
                            UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                        ))
                    }
                }
            },
            Object::String(mut s) => {
                if rhs == Object::Null {
                    s.push('\0');
                } else {
                    s.push_str(&rhs.to_string());
                }
                Ok(Object::String(s))
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {1.0 + n} else {n};
                    Ok(Object::Num(new))
                } else {
                    let new = format!("{}{}", self, rhs);
                    Ok(Object::String(new))
                }
            },
            Object::Array(mut arr) => {
                arr.push(rhs);
                Ok(Object::Array(arr))
            },
            Object::Null => {
                match rhs {
                    Object::Num(_) => Ok(rhs),
                    Object::Null => Ok(Object::String("\0\0".into())),
                    Object::Empty => Ok(Object::String("\0".into())),
                    Object::String(mut s) => {
                        s.push('\0');
                        Ok(Object::String(s))
                    },
                    _ => {
                        Err(UError::new(
                            UErrorKind::OperatorError,
                            UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                        ))
                    }
                }
            },
            Object::Empty => {
                Ok(rhs)
            },
            Object::ByteArray(mut arr) => {
                if let Some(n) = rhs.as_f64(true) {
                    arr.push(n as u8);
                    Ok(Object::ByteArray(arr))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(true) {
                    Ok(Object::Num(v.parse() + n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.add(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}

impl Sub for Object {
    type Output = Result<Object, UError>;

    fn sub(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n - n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {1.0 - n} else {n};
                    Ok(Object::Num(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                Ok(rhs)
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() - n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.sub(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::String(_) |
            Object::Array(_) |
            Object::Null |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}
impl Mul for Object {
    type Output = Result<Object, UError>;

    fn mul(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n * n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 右辺が数値の場合は*演算可能
                if let Some(n2) = rhs.as_f64(false) {
                    if let Ok(n) = s.parse::<f64>() {
                        // 左辺の文字列が数値変換可能であれば数値として演算
                        Ok(Object::Num(n * n2))
                    } else {
                        // 文字列の繰り返し
                        let new = s.repeat(n2 as usize);
                        Ok(Object::String(new))
                    }
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Null => {
                // 右辺が数値の場合は*演算可能
                if let Object::Num(n2) = rhs {
                    // NULL文字の繰り返し
                    let new = "\0".repeat(n2 as usize);
                    Ok(Object::String(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    // falseは0を掛ける
                    let new = if b {n} else {0.0 * n};
                    Ok(Object::Num(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(_) = rhs.as_f64(false) {
                    // 0を掛ける
                    Ok(Object::Num(0.0))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() * n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.mul(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}
impl Div for Object {
    type Output = Result<Object, UError>;

    fn div(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        if rhs.is_zero() {
            // 右辺が0の場合は0を返す
            return Ok(Object::Num(0.0));
        }
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n / n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は/演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::Num(n1 / n2)),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {1.0 / n} else {0.0};
                    Ok(Object::Num(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(_) = rhs.as_f64(false) {
                    Ok(Object::Num(0.0))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() / n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.div(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Null |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}
impl Rem for Object {
    type Output = Result<Object, UError>;

    fn rem(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        if rhs.is_zero() {
            // 右辺が0の場合はエラー
            return Err(UError::new(
                UErrorKind::OperatorError,
                UErrorMessage::DivZeroNotAllowed,
            ));
        }
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n % n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は%演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::Num(n1 % n2)),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {1.0 % n} else {0.0};
                    Ok(Object::Num(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(_) = rhs.as_f64(false) {
                    Ok(Object::Num(0.0))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() % n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.rem(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Null |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }

}
impl BitOr for Object {
    type Output = Result<Object, UError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::bit_or(n, n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::bit_or(n1, n2)),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {Object::bit_or(1.0, n)} else {Object::bit_or(0.0, n)};
                    Ok(new)
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_or(0.0, n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_or(v.parse(), n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.bitor(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Null |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}
impl BitAnd for Object {
    type Output = Result<Object, UError>;

    fn bitand(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::bit_and(n, n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::bit_and(n1, n2)),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {Object::bit_and(1.0, n)} else {Object::bit_and(0.0, n)};
                    Ok(new)
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_and(0.0, n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_and(v.parse(), n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.bitand(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Null |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}
impl BitXor for Object {
    type Output = Result<Object, UError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let rhs = rhs.to_uwscr_object()?;
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::bit_xor(n, n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::bit_xor(n1, n2)),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {Object::bit_xor(1.0, n)} else {Object::bit_xor(0.0, n)};
                    Ok(new)
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Empty => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_xor(0.0, n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::bit_xor(v.parse(), n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Plus),
                    ))
                }
            },
            Object::Variant(_) => {
                let obj = self.to_uwscr_object()?;
                obj.bitxor(rhs)
            }
            // 以下はエラー
            Object::RemoteObject(_) |
            Object::Null |
            Object::Array(_) |
            Object::ByteArray(_) |
            Object::HashTbl(_) |
            Object::AnonFunc(_) |
            Object::Function(_) |
            Object::AsyncFunction(_) |
            Object::BuiltinFunction(_, _, _) |
            Object::Module(_) |
            Object::Class(_, _) |
            Object::Instance(_) |
            Object::EmptyParam |
            Object::Nothing |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Handle(_) |
            Object::RegEx(_) |
            Object::Exit |
            Object::Global |
            Object::This(_) |
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_, _, _, _) |
            Object::Struct(_, _, _) |
            Object::UStruct(_, _, _) |
            Object::ComObject(_) |
            Object::ComMember(_, _) |
            Object::SafeArray(_) |
            Object::VarArgument(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::BrowserFunction(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::WebFunction(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}