pub mod hashtbl;
pub mod version;
pub mod variant;
pub mod utask;
pub mod ustruct;
pub mod module;
pub mod function;
pub mod uobject;
pub mod fopen;

pub use self::hashtbl::{HashTbl, HashTblEnum};
pub use self::version::Version;
pub use self::variant::Variant;
pub use self::utask::UTask;
pub use self::ustruct::{UStruct, UStructMember};
pub use self::module::Module;
pub use self::function::Function;
pub use self::uobject::UObject;
pub use self::fopen::*;

use crate::ast::*;
use crate::evaluator::builtins::BuiltinFunction;
use crate::evaluator::com_object::VARIANTHelper;
use crate::evaluator::devtools_protocol::{Browser, Element};

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
use std::sync::{Arc, Mutex};

use num_traits::Zero;
use serde_json::{self, Value};

#[derive(Clone, Debug)]
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
    VarArgument(Expression),
    Browser(Browser),
    BrowserFunc(Browser, String),
    Element(Element),
    ElementFunc(Element, String),
    Fopen(Arc<Mutex<Fopen>>),
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
            Object::UObject(ref uobj) => {
                write!(f, "UObject: {}", uobj)
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
            Object::Fopen(ref arc) => {
                let fopen = arc.lock().unwrap();
                write!(f, "{}", &*fopen)
            },
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Object::Num(n) => match other {
                Object::Num(n2) => n == n2,
                Object::String(s) => n.to_string() == s.to_string(),
                Object::Empty |
                Object::EmptyParam => 0.0 == *n,
                Object::Bool(b) => ! n.is_zero() && *b,
                _ => false
            },
            Object::String(s) => match other {
                Object::Num(n) => s.to_string() == n.to_string(),
                Object::String(s2) => s.to_string() == s2.to_string(),
                Object::Empty |
                Object::EmptyParam => false,
                Object::Bool(b) => b.to_string().to_ascii_lowercase() == s.to_ascii_lowercase(),
                _ => false
            },
            Object::Bool(b) => match other {
                Object::Bool(b2) => b == b2,
                Object::String(s) => b.to_string().to_ascii_lowercase() == s.to_ascii_lowercase(),
                Object::Empty |
                Object::EmptyParam => false && *b,
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
            Object::Instance(_, i) => if let Object::Instance(_,i2) = other {i==i2} else {false},
            Object::Instances(_) => false,
            Object::DestructorNotFound => false,
            Object::Null => match other {
                Object::Null => true,
                _ => false
            },
            Object::Empty |
            Object::EmptyParam => match other {
                Object::Empty | Object::EmptyParam => true,
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
            Object::Eval(_) => false,
            Object::Handle(h) => if let Object::Handle(h2) = other {h==h2} else {false},
            Object::RegEx(r) => if let Object::RegEx(r2) = other {r==r2} else {false},
            Object::Exit => false,
            Object::ExitExit(_) => false,
            Object::SpecialFuncResult(_) => false,
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
            Object::Browser(b) => if let Object::Browser(b2) = other {b == b2} else {false},
            Object::BrowserFunc(b, f) => if let Object::BrowserFunc(b2,f2) = other {
                b == b2 && f == f2
            } else {false},
            Object::Element(e) => if let Object::Element(e2) = other {e==e2} else {false},
            Object::ElementFunc(e, f) => if let Object::ElementFunc(e2,f2) = other {
                e==e2 && f==f2
            } else {false},
            Object::Fopen(f1) => if let Object::Fopen(f2) = other {
                let _tmp = f1.lock().unwrap();
                let result = f2.try_lock().is_err();
                result
            } else {false},
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

impl Default for Object {
    fn default() -> Self {
        Object::Empty
    }
}

#[derive(Clone, Debug)]
pub enum SpecialFuncResultType {
    GetEnv,
    ListModuleMember(String),
    BuiltinConstName(Option<Expression>),
    Task(Function, Vec<(Option<Expression>, Object)>),
    GetLogPrintWinId,
    Balloon(Option<crate::gui::Balloon>),
    BalloonID,
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
impl Into<Object> for i32 {
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