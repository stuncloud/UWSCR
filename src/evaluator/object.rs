use crate::ast::*;
use crate::evaluator::environment::{NamedObject, Module};
use crate::evaluator::builtins::BuiltinFunction;
use crate::evaluator::UError;

use std::collections::HashMap;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::cell::RefCell;

use winapi::shared::windef::HWND;


#[derive(PartialEq, Clone, Debug)]
pub enum Object {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Object>),
    Hash(HashMap<String, Object>, bool),
    SortedHash(BTreeMap<String, Object>, bool),
    AnonFunc(Vec<Expression>, BlockStatement, Vec<NamedObject>, bool),
    Function(String, Vec<Expression>, BlockStatement, bool, Option<Box<Object>>),
    BuiltinFunction(String, i32, BuiltinFunction),
    Module(Rc<RefCell<Module>>),
    Null,
    Empty,
    Nothing,
    Continue(u32),
    Break(u32),
    Error(String),
    UError(UError),
    Eval(String),
    Handle(HWND),
    RegEx(String),
    Exit,
    Debug(DebugType),
    Global, // globalを示す
    This,   // thisを示す
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
            Object::Hash(ref hash, _) => {
                let mut key_values = String::new();
                for (i, (k, v)) in hash.iter().enumerate() {
                    if i < 1 {
                        key_values.push_str(&format!("\"{}\": {}", k, v))
                    } else {
                        key_values.push_str(&format!(", \"{}\": {}", k, v))
                    }
                }
                write!(f, "{{{}}}", key_values)
            },
            Object::SortedHash(ref hash, _) => {
                let mut result = String::new();
                for (i, (k, v)) in hash.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("\"{}\": {}", k, v))
                    } else {
                        result.push_str(&format!(", \"{}\": {}", k, v))
                    }
                }
                write!(f, "{{{}}}", result)
            },
            Object::Function(ref name, ref params, _, is_proc, ref instance) => {
                let mut arguments = String::new();
                let func_name = match instance {
                    Some(obj) => match &**obj {
                        Object::Module(m) => format!("{}.{}", m.borrow().name(), name),
                        _ => name.to_string()
                    },
                    None => name.to_string()
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
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(ref n) => write!(f, "Continue {}", n),
            Object::Break(ref n) => write!(f, "Break {}", n),
            Object::Exit => write!(f, "Exit"),
            Object::Eval(ref value) => write!(f, "{}", value),
            Object::Error(ref value) => write!(f, "{}", value),
            Object::UError(ref value) => write!(f, "{}", value),
            Object::Debug(_) => write!(f, "debug"),
            Object::Module(ref m) => write!(f, "module: {}", m.borrow().name()),
            Object::Handle(h) => write!(f, "{:?}", h),
            Object::RegEx(ref re) => write!(f, "regex: {}", re),
            Object::Global => write!(f, "GLOBAL"),
            Object::This => write!(f, "THIS"),
        }
    }
}

impl Eq for Object {}

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

#[derive(PartialEq, Clone, Debug)]
pub enum DebugType {
    GetEnv,
    ListModuleMember(String),
}