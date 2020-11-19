use crate::ast::*;
use crate::evaluator::env::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use winapi::shared::windef::HWND;

pub type BuiltinFunction = fn(Vec<Object>) -> Object;

#[derive(PartialEq, Clone, Debug)]
pub enum Object {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Object>),
    Hash(HashMap<String, Object>, bool),
    SortedHash(BTreeMap<String, Object>, bool),
    AnonFunc(Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    AnonProc(Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    Function(String, Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    Procedure(String, Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    ModuleFunction(String, String, Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    ModuleProcedure(String, String, Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    GlobalMember(String),
    BuiltinFunction(i32, BuiltinFunction),
    BuiltinConst(Box<Object>),
    Module(String, HashMap<String, Object>),
    Null,
    Empty,
    Nothing,
    Continue(u32),
    Break(u32),
    Error(String),
    Eval(String),
    Handle(HWND),
    RegEx(String),
    Exit,
    Debug(DebugType),
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Object::Num(ref value) => write!(f, "{}", value),
            Object::String(ref value) => write!(f, "{}", value),
            Object::Bool(b) => write!(f, "{}", if b {"True"} else {"False"}),
            Object::Array(ref objects) => {
                let mut result = String::new();
                for (i, obj) in objects.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", obj))
                    } else {
                        result.push_str(&format!(", {}", obj))
                    }
                }
                write!(f, "[{}]", result)
            },
            Object::Hash(ref hash, _) => {
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
            Object::Function(ref name, ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "{}({})", name, result)
            },
            Object::Procedure(ref name, ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "{}({})", name, result)
            },
            Object::ModuleFunction(ref module_name, ref name, ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "{}.{}({})", module_name, name, result)
            },
            Object::ModuleProcedure(ref module_name, ref name, ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "{}.{}({})", module_name, name, result)
            },
            Object::AnonFunc(ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "_function_({})", result)
            },
            Object::AnonProc(ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "_procedure_({})", result)
            },
            Object::BuiltinFunction(_, _) => write!(f, "builtin_function()"),
            Object::BuiltinConst(ref o) => write!(f, "builtin constant: {}", o),
            Object::GlobalMember(ref name) => write!(f, "global: {}", name),
            Object::Null => write!(f, "NULL"),
            Object::Empty => write!(f, ""),
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(ref n) => write!(f, "Continue {}", n),
            Object::Break(ref n) => write!(f, "Break {}", n),
            Object::Exit => write!(f, "Exit"),
            Object::Eval(ref value) => write!(f, "{}", value),
            Object::Error(ref value) => write!(f, "{}", value),
            Object::Debug(_) => write!(f, "debug"),
            Object::Module(ref name, _) => write!(f, "module {}", name),
            Object::Handle(h) => write!(f, "{:?}", h),
            Object::RegEx(ref re) => write!(f, "regex: {}", re)
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
    PrintEnv(String),
}