use super::{Object, ValueExt};
use crate::error::{UError,UErrorKind,UErrorMessage};
use crate::EvalResult;

use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use serde_json::{self, Value};

#[derive(Clone, Debug)]
pub struct UObject {
    value: Arc<Mutex<Value>>,
    pointer: Option<String>
}

impl UObject {
    pub fn new(value: Value) -> Self {
        Self {
            value: Arc::new(Mutex::new(value)),
            pointer: None
        }
    }
    pub fn new_with_pointer(value: Value, pointer: String) -> Self {
        Self {
            value: Arc::new(Mutex::new(value)),
            pointer: Some(pointer),
        }
    }
    /// 任意のポインタを持った自身のクローンを作る
    pub fn clone_with_pointer(&self, pointer: String) -> Self {
        Self {
            value: self.value.clone(),
            pointer: Some(pointer),
        }
    }
    pub fn pointer(&self, index: Option<Object>) -> Option<String> {
        match (&self.pointer, index) {
            (None, None) => None,
            (None, Some(i)) => Some(format!("/{}", i)),
            (Some(ref p), None) => Some(p.to_string()),
            (Some(ref p), Some(i)) => Some(format!("{}/{}", p, i)),
        }
    }
    pub fn value(&self) -> Value {
        let m = self.value.lock().unwrap();
        match self.pointer {
            Some(ref p) => m.pointer(p).unwrap_or(&Value::Null).clone(),
            None => m.clone(),
        }
    }
    pub fn get(&self, index: &Object) -> EvalResult<Option<Value>> {
        let m = self.value.lock().unwrap();
        let parent = match self.pointer {
            Some(ref p) => m.pointer(p).unwrap_or(&Value::Null).clone(),
            None => m.clone(),
        };
        let value = match index {
            Object::String(key) => {
                parent.get_case_insensitive(key).cloned()
            },
            Object::Num(n) => parent.get(*n as usize).cloned(),
            o => return Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::InvalidMemberOrIndex(o.to_string())
            ))
        };
        Ok(value)
    }
    /// UObjectへの代入
    pub fn set(&self, index: Object, new_value: Value, member: Option<String>) -> EvalResult<()> {
        let mut m = self.value.lock().unwrap();
        let value = match (&self.pointer, member) {
            (None, None) => m.borrow_mut(),
            (None, Some(ref name)) => match m.pointer_mut(&format!("/{}", name)) {
                Some(v) => v,
                None => return Err(UError::new(
                    UErrorKind::UObjectError,
                    UErrorMessage::InvalidMemberOrIndex(name.to_string())
                )),
            },
            (Some(ref p), None) => m.pointer_mut(p).unwrap(),
            (Some(ref p), Some(ref name)) => match m.pointer_mut(&format!("{}/{}", p, name)) {
                Some(v) => v,
                None => return Err(UError::new(
                    UErrorKind::UObjectError,
                    UErrorMessage::InvalidMemberOrIndex(name.to_string())
                )),
            },
        };
        match index {
            Object::String(key) => {
                match value.get_case_insensitive_mut(&key) {
                    Some(v) => *v = new_value,
                    None => return Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::InvalidMemberOrIndex(key)
                    )),
                }
            },
            Object::Num(n) => {
                match value.get_mut(n as usize) {
                    Some(v) => *v = new_value,
                    None => return Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::InvalidMemberOrIndex(n.to_string())
                    )),
                }
            },
            o => return Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::InvalidMemberOrIndex(o.to_string())
            ))
        }
        Ok(())
    }
    /// 配列へのpush\
    /// 成功時true
    pub fn push(&self, new_value: Value) -> bool {
        let mut guard = self.value.lock().unwrap();
        match &self.pointer {
            Some(pointer) => if let Some(v) = guard.pointer_mut(pointer) {
                if let Some(arr) = v.as_array_mut() {
                    arr.push(new_value);
                    return true;
                }
            },
            None => if let Some(arr) = guard.as_array_mut() {
                arr.push(new_value);
                return true;
            },
        }
        false
    }
    pub fn to_object_vec(&self) -> EvalResult<Vec<Object>> {
        let value = self.value();
        if let Value::Array(arr) = value {
            let p = self.pointer.clone().unwrap_or_default();
            let vec = arr.iter().enumerate()
                .map(|(i, _)| {
                    let pointer = format!("{}/{}", p, i);
                    let uo = self.clone_with_pointer(pointer);
                    Object::UObject(uo)
                })
                .collect();
            Ok(vec)
        } else {
            Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::UObjectIsNotAnArray
            ))
        }
    }
    pub fn get_size(&self) -> EvalResult<usize> {
        let len = match self.value() {
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => obj.len(),
            _ => 0, // should be unreachable
        };
        Ok(len)
    }
    fn keys(&self) -> EvalResult<Vec<Object>> {
        let vec = match self.value() {
            Value::Object(obj) => {
                obj.keys().map(|key| key.as_str().into()).collect()
            },
            _ => Vec::new(),
        };
        Ok(vec)
    }
    fn values(&self) -> EvalResult<Vec<Object>> {
        let vec = match self.value() {
            Value::Object(obj) => {
                obj.values().map(|value| value.into()).collect()
            },
            _ => Vec::new(),
        };
        Ok(vec)
    }
    pub fn invoke_method(&self, method: &str) -> EvalResult<Object> {
        match method.to_ascii_lowercase().as_str() {
            "keys" => {
                let keys = self.keys()?;
                Ok(Object::Array(keys))
            },
            "values" => {
                let values = self.values()?;
                Ok(Object::Array(values))
            },
            _ => Err(UError::new(UErrorKind::UObjectError, UErrorMessage::CanNotCallMethod(method.into()))),
        }
    }
}

impl std::fmt::Display for UObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = self.value.lock().unwrap();
        let value = match self.pointer {
            Some(ref p) => m.pointer(p).unwrap_or(&Value::Null),
            None => &m,
        };
        let json = match serde_json::to_string(value) {
            Ok(j) => j,
            Err(e) => e.to_string(),
        };
        write!(f, "{}", json)
    }
}

impl PartialEq for UObject {
    fn eq(&self, other: &Self) -> bool {
        // 一方をロックしもう一方もロックできれば別のオブジェクト
        let _tmp = self.value.lock().unwrap();
        let is_same_object = other.value.try_lock().is_err();
        is_same_object && self.pointer == other.pointer
    }
}