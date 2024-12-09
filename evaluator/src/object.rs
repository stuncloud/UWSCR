pub mod hashtbl;
pub mod version;
pub mod utask;
pub mod ustruct;
pub mod module;
pub mod function;
pub mod uobject;
pub mod fopen;
pub mod class;
pub mod browser;
mod web;
pub mod comobject;
mod variant;

pub use self::hashtbl::{HashTbl, HashTblEnum};
pub use self::version::Version;
pub use self::utask::UTask;
pub use self::ustruct::{StructDef, UStruct, UStructMember};
pub use self::module::Module;
pub use self::function::Function;
pub use self::uobject::UObject;
pub use self::fopen::*;
pub use self::class::ClassInstance;
pub use variant::Variant;
use browser::{BrowserBuilder, Browser, TabWindow, RemoteObject};
pub use web::{WebRequest, WebResponse, HtmlNode};
pub use comobject::{ComObject, ComError, ComArg, Unknown, Excel, ExcelOpenFlag, ObjectTitle, VariantExt};

use util::settings::USETTINGS;
use crate::environment::Layer;
use crate::def_dll::DefDll;
use crate::builtins::BuiltinFunction;
use crate::error::{UError, UErrorKind, UErrorMessage};
use crate::gui::form::{WebViewForm, WebViewRemoteObject};
use parser::ast::*;

use windows::Win32::Foundation::HWND;

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use std::ops::{Add, Sub, Mul, Div, Rem, BitOr, BitAnd, BitXor};
use std::cmp::Ordering;

use num_traits::Zero;
use strum_macros::{VariantNames, Display, EnumString, EnumProperty};
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
    UObject(UObject),
    DynamicVar(fn()->Object), // 特殊変数とか
    Version(Version),
    ExpandableTB(String),
    Enum(UEnum),
    Task(UTask),
    DefDllFunction(DefDll),
    StructDef(StructDef), // 構造体定義
    UStruct(UStruct), // 構造体インスタンス
    ComObject(ComObject),
    Unknown(Unknown),
    Variant(Variant),
    BrowserBuilder(Arc<Mutex<BrowserBuilder>>),
    Browser(Browser),
    TabWindow(TabWindow),
    RemoteObject(RemoteObject),
    Fopen(Arc<Mutex<Fopen>>),
    ByteArray(Vec<u8>),
    /// 参照渡しされたパラメータ変数
    Reference(Expression, Arc<Mutex<Layer>>),
    /// WebRequestオブジェクト
    WebRequest(Arc<Mutex<WebRequest>>),
    /// WebResponseオブジェクト
    WebResponse(WebResponse),
    HtmlNode(HtmlNode),
    /// 組み込みオブジェクトのメソッド呼び出し
    MemberCaller(MemberCaller, String),
    /// Form
    WebViewForm(WebViewForm),
    /// FormのRemoteObject
    WebViewRemoteObject(WebViewRemoteObject),
    /// PARAM_STR
    ParamStr(Vec<String>)
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
            Object::UObject(arg0) => f.debug_tuple("UObject").field(arg0).finish(),
            Object::DynamicVar(arg0) => f.debug_tuple("DynamicVar").field(arg0).finish(),
            Object::Version(arg0) => f.debug_tuple("Version").field(arg0).finish(),
            Object::ExpandableTB(arg0) => f.debug_tuple("ExpandableTB").field(arg0).finish(),
            Object::Enum(arg0) => f.debug_tuple("Enum").field(arg0).finish(),
            Object::Task(arg0) => f.debug_tuple("Task").field(arg0).finish(),
            Object::DefDllFunction(arg0) => f.debug_tuple("DefDllFunction").field(arg0).finish(),
            Object::StructDef(arg0) => f.debug_tuple("StructDef").field(arg0).finish(),
            Object::UStruct(arg0) => f.debug_tuple("UStruct").field(arg0).finish(),
            Object::BrowserBuilder(arg0) => f.debug_tuple("BrowserBuilder").field(arg0).finish(),
            Object::Browser(arg0) => f.debug_tuple("Browser").field(arg0).finish(),
            Object::TabWindow(arg0) => f.debug_tuple("TabWindow").field(arg0).finish(),
            Object::RemoteObject(arg0) => f.debug_tuple("RemoteObject").field(arg0).finish(),
            Object::Fopen(arg0) => f.debug_tuple("Fopen").field(arg0).finish(),
            Object::ByteArray(arg0) => f.debug_tuple("ByteArray").field(arg0).finish(),
            Object::Reference(arg0, arg1) => f.debug_tuple("Reference").field(arg0).field(arg1).finish(),
            Object::WebRequest(arg0) => f.debug_tuple("WebRequest").field(arg0).finish(),
            Object::WebResponse(arg0) => f.debug_tuple("WebResponse").field(arg0).finish(),
            Object::HtmlNode(arg0) => f.debug_tuple("HtmlNode").field(arg0).finish(),
            Object::MemberCaller(_, _) => todo!(),
            Object::ComObject(arg0) => f.debug_tuple("ComObject").field(arg0).finish(),
            Object::Unknown(arg0) => f.debug_tuple("Unknown").field(arg0).finish(),
            Object::Variant(arg0) => f.debug_tuple("Variant").field(arg0).finish(),
            Object::WebViewForm(arg0) => f.debug_tuple("WebViewForm").field(arg0).finish(),
            Object::WebViewRemoteObject(arg0) => f.debug_tuple("WebViewRemoteObject").field(arg0).finish(),
            Object::ParamStr(vec) => f.debug_list().entries(vec.iter()).finish(),
        }
    }
}

unsafe impl Send for Object {}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Object::Num(value) => write!(f, "{}", value),
            Object::String(value) => write!(f, "{}", value),
            Object::Bool(b) => write!(f, "{}", if *b {"True"} else {"False"}),
            Object::Array(objects) => {
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
            Object::HashTbl(hash) => {
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
            Object::Function(func) => {
                let title = if func.is_proc {"procedure"} else {"function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}: {}({})", title, func.name.as_ref().unwrap(), params)
            },
            Object::AsyncFunction(func) => {
                let title = if func.is_proc {"procedure"} else {"function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}: {}({})", title, func.name.as_ref().unwrap(), params)
            },
            Object::AnonFunc(func) => {
                let title = if func.is_proc {"anonymous procedure"} else {"anonymous function"};
                let params = func.params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}({})", title, params)
            },
            Object::BuiltinFunction(name, _, _) => write!(f, "builtin: {}()", name),
            Object::Null => write!(f, "\0"),
            Object::Empty => write!(f, ""),
            Object::EmptyParam => write!(f, "__EMPTYPARAM__"),
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(n) => write!(f, "Continue {}", n),
            Object::Break(n) => write!(f, "Break {}", n),
            Object::Exit => write!(f, "Exit"),
            Object::Module(m) => write!(f, "module: {}", m.lock().unwrap().name()),
            Object::Class(name, _) => write!(f, "class: {}", name),
            Object::Instance(m) => {
                let ins = m.lock().unwrap();
                if ins.is_dropped {
                    write!(f, "NOTHING")
                } else {
                    write!(f, "instance of {}", ins.name)
                }
            },
            Object::Handle(h) => write!(f, "{:?}", h),
            Object::RegEx(re) => write!(f, "regex: {}", re),
            Object::Global => write!(f, "GLOBAL"),
            Object::UObject(uobj) => {
                write!(f, "{}", uobj)
            },
            Object::DynamicVar(func) => write!(f, "{}", func()),
            Object::Version(v) => write!(f, "{}", v),
            Object::ExpandableTB(_) => write!(f, "expandable textblock"),
            Object::Enum(e) => write!(f, "Enum {}", e.name),
            Object::Task(t) => write!(f, "Task [{}]", t),
            Object::DefDllFunction(defdll) => write!(f, "{defdll}"),
            Object::StructDef(s) => write!(f, "{s}"),
            Object::UStruct(ustruct) => write!(f, "{ustruct}"),
            Object::BrowserBuilder(_) => write!(f, "BrowserBuilder"),
            Object::Browser(b) => write!(f, "Browser: {b})"),
            Object::TabWindow(t) => write!(f, "TabWindow: {t}"),
            Object::RemoteObject(r) => write!(f, "{r}"),
            Object::Fopen(arc) => {
                let fopen = arc.lock().unwrap();
                write!(f, "{}", &*fopen)
            },
            Object::ByteArray(arr) => write!(f, "{:?}", arr),
            Object::Reference(_, _) => write!(f, "Reference"),
            Object::WebRequest(req) => {
                let mutex = req.lock().unwrap();
                write!(f, "{mutex}")
            },
            Object::WebResponse(res) => write!(f, "{res}"),
            Object::HtmlNode(node) => write!(f, "{node}"),
            Object::MemberCaller(method, member) => {
                match method {
                    MemberCaller::Module(m) => {
                        match m.try_lock() {
                            Ok(m) => write!(f, "{}.{member}", m.name()),
                            Err(_) => write!(f, "{{Module}}.{member}"),
                        }
                    },
                    MemberCaller::ClassInstance(ins) => {
                        match ins.try_lock() {
                            Ok(g) => write!(f, "{}.{member}", g.name),
                            Err(_) => write!(f, "{{ClassInstance}}.{member}"),
                        }
                    },
                    MemberCaller::BrowserBuilder(_) => write!(f, "BrowserBuilder.{member}"),
                    MemberCaller::Browser(_) => write!(f, "Browser.{member}"),
                    MemberCaller::TabWindow(_) => write!(f, "TabWindow.{member}"),
                    MemberCaller::RemoteObject(_) => write!(f, "RemoteObject.{member}"),
                    MemberCaller::WebRequest(_) => write!(f, "WebRequest.{member}"),
                    MemberCaller::WebResponse(_) => write!(f, "WebResponse.{member}"),
                    MemberCaller::HtmlNode(_) => write!(f, "HtmlNode.{member}"),
                    MemberCaller::ComObject(_) => write!(f, "ComObject.{member}"),
                    MemberCaller::UStruct(ust) => write!(f, "{}.{member}", ust.name),
                    MemberCaller::WebViewForm(_) => write!(f, "WebViewForm.{member}"),
                    MemberCaller::WebViewRemoteObject(_) => write!(f, "WebViewRemoteObject.{member}"),
                    MemberCaller::UObject(_) => write!(f, "UObject.{member}"),
                }
            },
            Object::ComObject(com) => write!(f, "{com}"),
            Object::Unknown(unk) => write!(f, "{unk}"),
            Object::Variant(variant) => write!(f, "{variant}"),
            Object::WebViewForm(form) => write!(f, "{form}"),
            Object::WebViewRemoteObject(remote) => write!(f, "{remote}"),
            Object::ParamStr(vec) => write!(f, "{vec:?}"),
        }
    }
}

static OPTION_SAME_STR: OnceLock<bool> = OnceLock::new();
fn compare_string<A, B>(a: A, b: B) -> bool
    where A: fmt::Display, B: fmt::Display
{
    let same_str = OPTION_SAME_STR.get_or_init(|| {
        let s = USETTINGS.lock().unwrap();
        s.options.same_str
    });
    if *same_str {
        a.to_string() == b.to_string()
    } else {
        a.to_string().eq_ignore_ascii_case(&b.to_string())
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
                Object::String(s) => compare_string(n, s),
                Object::Empty => false,
                Object::Bool(b) => {
                    let n2 = b.then_some(1.0).unwrap_or(0.0);
                    *n == n2
                },
                _ => false
            },
            Object::String(s) => match other {
                Object::Num(n) => compare_string(n, s),
                Object::String(s2) => compare_string(s, s2),
                Object::Empty => false,
                Object::Bool(b) => if *b {
                    compare_string(s, "True")
                } else {
                    compare_string(s, "False")
                },
                _ => false
            },
            Object::Bool(b) => match other {
                Object::Bool(b2) => b == b2,
                Object::String(s) => if *b {
                    compare_string("True", s)
                } else {
                    compare_string("False", s)
                },
                Object::Num(n) => {
                    let n2 = b.then_some(1.0).unwrap_or(0.0);
                    *n == n2
                },
                Object::Empty => false,
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
            Object::UObject(uobj) => if let Object::UObject(uobj2) = other {
                uobj == uobj2
            } else {false},
            Object::DynamicVar(f) => if let Object::DynamicVar(f2) = other {f() == f2()} else {false},
            Object::Version(v) => if let Object::Version(v2) = other {v == v2} else {false},
            Object::ExpandableTB(_) => false,
            Object::Enum(e) => if let Object::Enum(e2) = other {e==e2} else {false},
            Object::Task(_) => false,
            Object::DefDllFunction(d1) => if let Object::DefDllFunction(d2) = other { d1 == d2 } else {false},
            Object::StructDef(s1) => if let Object::StructDef(s2) = other { s1 == s2 } else {false},
            Object::UStruct(u1) => if let Object::UStruct(u2) = other { u1 == u2 } else {false},
            Object::BrowserBuilder(b) => if let Object::BrowserBuilder(b2) = other {
                let _tmp = b.lock().unwrap();
                b2.try_lock().is_err()
            } else {false},
            Object::Browser(b) => if let Object::Browser(b2) = other {b == b2} else {false},
            Object::TabWindow(t) => if let Object::TabWindow(t2) = other {t == t2} else {false},
            Object::RemoteObject(r) => if let Object::RemoteObject(r2) = other {r == r2} else {false},
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
            Object::HtmlNode(node) => {
                if let Object::HtmlNode(node2) = other {node == node2} else {false}
            },
            Object::MemberCaller(method, member) => {
                if let Object::MemberCaller(method2, member2) = other {
                    method == method2 && member == member2
                } else {false}
            },
            Object::ComObject(com1) => {
                if let Object::ComObject(com2) = other {com1 == com2} else {false}
            },
            Object::Unknown(unk1) => {
                if let Object::Unknown(unk2) = other {unk1 == unk2} else {false}
            },
            Object::Variant(var1) => {
                if let Object::Variant(var2) = other {var1 == var2} else {false}
            },
            Object::WebViewForm(form1) => {
                if let Object::WebViewForm(form2) = other {form1 == form2} else {false}
            },
            Object::WebViewRemoteObject(r1) => {
                if let Object::WebViewRemoteObject(r2) = other {r1 == r2} else {false}
            },
            Object::ParamStr(v1) => {
                if let Object::ParamStr(v2) = other {v1 == v2} else {false}
            }
        }
    }
}

impl Object {
    pub fn get_type(&self) -> ObjectType {
        match self {
            Object::Num(_) => ObjectType::TYPE_NUMBER,
            Object::String(_) => ObjectType::TYPE_STRING,
            Object::Bool(_) => ObjectType::TYPE_BOOL,
            Object::Array(_) => ObjectType::TYPE_ARRAY,
            Object::HashTbl(_) => ObjectType::TYPE_HASHTBL,
            Object::AnonFunc(_) => ObjectType::TYPE_ANONYMOUS_FUNCTION,
            Object::Function(_) => ObjectType::TYPE_FUNCTION,
            Object::BuiltinFunction(_,_,_) => ObjectType::TYPE_BUILTIN_FUNCTION,
            Object::AsyncFunction(_) => ObjectType::TYPE_ASYNC_FUNCTION,
            Object::Module(_) => ObjectType::TYPE_MODULE,
            Object::Class(_,_) => ObjectType::TYPE_CLASS,
            Object::Instance(ref m) => {
                let ins = m.lock().unwrap();
                if ins.is_dropped {
                    ObjectType::TYPE_NOTHING
                } else {
                    ObjectType::TYPE_CLASS_INSTANCE
                }
            },
            Object::Null => ObjectType::TYPE_NULL,
            Object::Empty => ObjectType::TYPE_EMPTY,
            Object::Nothing => ObjectType::TYPE_NOTHING,
            Object::Handle(_) => ObjectType::TYPE_HWND,
            Object::RegEx(_) => ObjectType::TYPE_REGEX,
            Object::Global => ObjectType::TYPE_GLOBAL,
            Object::UObject(_) => ObjectType::TYPE_UOBJECT,
            Object::Version(_) => ObjectType::TYPE_VERSION,
            Object::ExpandableTB(_) => ObjectType::TYPE_STRING,
            Object::Enum(_) => ObjectType::TYPE_ENUM,
            Object::Task(_) => ObjectType::TYPE_TASK,
            Object::DefDllFunction(_) => ObjectType::TYPE_DLL_FUNCTION,
            Object::StructDef(_) => ObjectType::TYPE_STRUCT_DEFINITION,
            Object::UStruct(_) => ObjectType::TYPE_STRUCT_INSTANCE,
            Object::ComObject(_) => ObjectType::TYPE_COM_OBJECT,
            Object::Unknown(_) => ObjectType::TYPE_IUNKNOWN,
            Object::Variant(_) => ObjectType::TYPE_VARIANT,
            Object::BrowserBuilder(_) => ObjectType::TYPE_BROWSERBUILDER_OBJECT,
            Object::Browser(_) => ObjectType::TYPE_BROWSER_OBJECT,
            Object::TabWindow(_) => ObjectType::TYPE_TABWINDOW_OBJECT,
            Object::RemoteObject(_) => ObjectType::TYPE_REMOTE_OBJECT,
            Object::Fopen(_) => ObjectType::TYPE_FILE_ID,
            Object::ByteArray(_) => ObjectType::TYPE_BYTE_ARRAY,
            Object::Reference(_, _) => ObjectType::TYPE_REFERENCE,
            Object::WebRequest(_) => ObjectType::TYPE_WEB_REQUEST,
            Object::WebResponse(_) => ObjectType::TYPE_WEB_RESPONSE,
            Object::HtmlNode(_) => ObjectType::TYPE_HTML_NODE,
            Object::MemberCaller(_, _) => ObjectType::TYPE_MEMBER_CALLER,
            Object::WebViewForm(_) => ObjectType::TYPE_WEBVIEW_FORM,
            Object::WebViewRemoteObject(_) => ObjectType::TYPE_WEBVIEW_REMOTEOBJECT,

            Object::ParamStr(_) |
            Object::EmptyParam |
            Object::DynamicVar(_) |
            Object::Continue(_) |
            Object::Break(_) |
            Object::Exit => ObjectType::TYPE_NOT_VALUE_TYPE,
        }
    }
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
    pub fn as_uwsc_cond(self) -> EvalResult<bool> {
        match self {
            Object::Null => Ok(true),
            obj => {
                let variant = Variant::try_from(obj)?;
                let variant_double = variant.0.change_type(windows::Win32::System::Variant::VT_R8)?;
                let double = Object::try_from(Some(variant_double))?;
                if let Object::Num(n) = double {
                    let b = ! n.is_zero();
                    Ok(b)
                } else {
                    // 多分Unreachableなんだけど
                    Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::SyntaxError))
                }
            }
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
                s.parse::<f64>().ok()
            },
            Object::Null => if null_as_zero {
                Some(0.0)
            } else {
                None
            },
            _ => None
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
impl Into<Object> for u8 {
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
impl Into<Object> for f32 {
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
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
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
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n - n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Minus),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は-演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::Num(n1 - n2)),
                    (Err(_), _) => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::LeftSideTypeInvalid(Infix::Minus)
                    )),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Minus),
                    )),
                }
            },
            Object::Bool(b) => {
                if let Some(n) = rhs.as_f64(false) {
                    let new = if b {1.0 - n} else {n};
                    Ok(Object::Num(new))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Minus),
                    ))
                }
            },
            Object::Empty => {
                Object::Num(0.0) - rhs
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() - n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Minus),
                    ))
                }
            },
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
            Object::RemoteObject(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Minus),
                ))
            },
        }
    }
}
impl Mul for Object {
    type Output = Result<Object, UError>;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Object::Num(n) => {
                if let Some(n2) = rhs.as_f64(false) {
                    Ok(Object::Num(n * n2))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() * n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Multiply),
                    ))
                }
            },
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Multiply),
                ))
            },
        }
    }
}
impl Div for Object {
    type Output = Result<Object, UError>;

    fn div(self, rhs: Self) -> Self::Output {
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Divide),
                    ))
                }
            },
            Object::String(s) => {
                // 自身が数値変換可能でかつ右辺が数値の場合は/演算可能
                match (s.parse::<f64>(), rhs.as_f64(false)) {
                    (Ok(n1), Some(n2)) => Ok(Object::Num(n1 / n2)),
                    (Err(_), _) => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::LeftSideTypeInvalid(Infix::Divide),
                    )),
                    _ => Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Divide),
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
                        UErrorMessage::RightSideTypeInvalid(Infix::Divide),
                    ))
                }
            },
            Object::Empty => {
                if let Some(_) = rhs.as_f64(false) {
                    Ok(Object::Num(0.0))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Divide),
                    ))
                }
            },
            Object::Version(v) => {
                if let Some(n) = rhs.as_f64(false) {
                    Ok(Object::Num(v.parse() / n))
                } else {
                    Err(UError::new(
                        UErrorKind::OperatorError,
                        UErrorMessage::RightSideTypeInvalid(Infix::Divide),
                    ))
                }
            },
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Divide),
                ))
            },
        }
    }
}
impl Rem for Object {
    type Output = Result<Object, UError>;

    fn rem(self, rhs: Self) -> Self::Output {
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
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
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
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
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
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
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
            // 以下はエラー
            Object::ParamStr(_) |
            Object::WebViewForm(_) |
            Object::WebViewRemoteObject(_) |
            Object::Variant(_) |
            Object::Unknown(_) |
            Object::ComObject(_) |
            Object::MemberCaller(_, _) |
            Object::HtmlNode(_) |
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
            Object::UObject(_) |
            Object::DynamicVar(_) |
            Object::ExpandableTB(_) |
            Object::Enum(_) |
            Object::Task(_) |
            Object::DefDllFunction(_) |
            Object::StructDef(_) |
            Object::UStruct(_) |
            Object::BrowserBuilder(_) |
            Object::Browser(_) |
            Object::TabWindow(_) |
            Object::Fopen(_) |
            Object::WebRequest(_) |
            Object::WebResponse(_) |
            Object::Reference(_, _) => {
                Err(UError::new(
                    UErrorKind::OperatorError,
                    UErrorMessage::LeftSideTypeInvalid(Infix::Plus),
                ))
            },
        }
    }
}

impl AsMut<Object> for Object {
    fn as_mut(&mut self) -> &mut Object {
        self
    }
}

#[derive(Debug, Clone)]
pub enum MemberCaller {
    Module(Arc<Mutex<Module>>),
    ClassInstance(Arc<Mutex<ClassInstance>>),
    BrowserBuilder(Arc<Mutex<BrowserBuilder>>),
    Browser(Browser),
    TabWindow(TabWindow),
    RemoteObject(RemoteObject),
    WebRequest(Arc<Mutex<WebRequest>>),
    WebResponse(WebResponse),
    HtmlNode(HtmlNode),
    ComObject(ComObject),
    UStruct(UStruct),
    WebViewForm(WebViewForm),
    WebViewRemoteObject(WebViewRemoteObject),
    UObject(UObject),
}

impl PartialEq for MemberCaller {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ClassInstance(l0), Self::ClassInstance(r0)) => {
                let _tmp = l0.lock().unwrap();
                r0.try_lock().is_err()
            },
            (Self::BrowserBuilder(l0), Self::BrowserBuilder(r0)) => {
                let _tmp = l0.lock().unwrap();
                r0.try_lock().is_err()
            },
            (Self::Browser(l0), Self::Browser(r0)) => l0 == r0,
            (Self::TabWindow(l0), Self::TabWindow(r0)) => l0 == r0,
            (Self::RemoteObject(l0), Self::RemoteObject(r0)) => l0 == r0,
            (Self::WebRequest(l0), Self::WebRequest(r0)) => {
                let _tmp = l0.lock().unwrap();
                r0.try_lock().is_err()
            },
            (Self::WebResponse(l0), Self::WebResponse(r0)) => l0 == r0,
            (Self::HtmlNode(l0), Self::HtmlNode(r0)) => l0 == r0,
            (Self::ComObject(l0), Self::ComObject(r0)) => l0 == r0,
            (Self::UStruct(l0), Self::UStruct(r0)) => l0 == r0,
            (Self::WebViewForm(l0), Self::WebViewForm(r0)) => l0 == r0,
            (Self::WebViewRemoteObject(l0), Self::WebViewRemoteObject(r0)) => l0 == r0,
            (Self::UObject(l0), Self::UObject(r0)) => l0 == r0,
            _ => false,
        }
    }
}


#[allow(non_camel_case_types)]
#[derive(Debug, VariantNames, EnumString, EnumProperty, Display, Clone, PartialEq)]
pub enum ObjectType {
    TYPE_NUMBER,
    TYPE_STRING,
    TYPE_BOOL,
    TYPE_ARRAY,
    TYPE_HASHTBL,
    TYPE_ANONYMOUS_FUNCTION,
    TYPE_FUNCTION,
    TYPE_BUILTIN_FUNCTION,
    TYPE_ASYNC_FUNCTION,
    TYPE_MODULE,
    TYPE_CLASS,
    TYPE_CLASS_INSTANCE,
    TYPE_NULL,
    TYPE_EMPTY,
    TYPE_NOTHING,
    TYPE_HWND,
    TYPE_REGEX,
    TYPE_UOBJECT,
    TYPE_VERSION,
    TYPE_THIS,
    TYPE_GLOBAL,
    TYPE_ENUM,
    TYPE_TASK,
    TYPE_DLL_FUNCTION,
    TYPE_STRUCT_DEFINITION,
    TYPE_STRUCT_INSTANCE,
    TYPE_COM_OBJECT,
    TYPE_IUNKNOWN,
    TYPE_VARIANT,
    TYPE_SAFEARRAY,
    TYPE_BROWSERBUILDER_OBJECT,
    TYPE_BROWSER_OBJECT,
    TYPE_TABWINDOW_OBJECT,
    TYPE_REMOTE_OBJECT,
    TYPE_FILE_ID,
    TYPE_BYTE_ARRAY,
    TYPE_REFERENCE,
    TYPE_WEB_REQUEST,
    TYPE_WEB_RESPONSE,
    TYPE_HTML_NODE,
    TYPE_WEBVIEW_FORM,
    TYPE_WEBVIEW_REMOTEOBJECT,

    TYPE_MEMBER_CALLER,
    TYPE_NOT_VALUE_TYPE,
}