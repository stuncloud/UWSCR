use crate::ast::*;
use crate::evaluator::env::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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
    Function(Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    Procedure(Vec<Identifier>, BlockStatement, Rc<RefCell<Env>>),
    BuiltinFunction(i32, BuiltinFunction),
    Null,
    Empty,
    Nothing,
    Continue(u32),
    Break(u32),
    Result(Box<Object>),
    Error(String),
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
                        result.push_str(&format!("{}: {}", k, v))
                    } else {
                        result.push_str(&format!(", {}: {}", k, v))
                    }
                }
                write!(f, "{{{}}}", result)
            },
            Object::SortedHash(ref hash, _) => {
                let mut result = String::new();
                for (i, (k, v)) in hash.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}: {}", k, v))
                    } else {
                        result.push_str(&format!(", {}: {}", k, v))
                    }
                }
                write!(f, "{{{}}}", result)
            },
            Object::Function(ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "function({}) {{ ... }}", result)
            },
            Object::Procedure(ref params, _, _) => {
                let mut result = String::new();
                for (i, Identifier(ref s)) in params.iter().enumerate() {
                    if i < 1 {
                        result.push_str(&format!("{}", s))
                    } else {
                        result.push_str(&format!(", {}", s))
                    }
                }
                write!(f, "procedure({}) {{ ... }}", result)
            },
            Object::BuiltinFunction(_, _) => write!(f, "[builtin function]"),
            Object::Null => write!(f, "NULL"),
            Object::Empty => write!(f, "EMPTY"),
            Object::Nothing => write!(f, "NOTHING"),
            Object::Continue(ref n) => write!(f, "Continue {}", n),
            Object::Break(ref n) => write!(f, "Break {}", n),
            Object::Result(ref value) => write!(f, "{}", value),
            Object::Error(ref value) => write!(f, "{}", value),
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