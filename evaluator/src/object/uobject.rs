use super::Object;
use crate::error::{UError,UErrorKind,UErrorMessage};
use crate::EvalResult;

use std::borrow::BorrowMut;
use std::sync::{Arc, RwLock};
use itertools::Itertools;
use serde_json::Value as JsonValue;
use serde_yml::Value as YamlValue;

#[derive(Clone, Debug)]
pub struct UObject {
    value: Arc<RwLock<JYValue>>,
    pointer: Option<String>
}
#[derive(Debug, Clone)]
pub enum JYValue {
    Json(JsonValue),
    Yaml(YamlValue),
}

impl UObject {
    fn new(value: JYValue) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
            pointer: None
        }
    }
    pub fn from_json_str(json: &str) -> UObjectResult<Self> {
        let value = serde_json::from_str(json)?;
        let value = JYValue::Json(value);
        Ok(Self::new(value))
    }
    pub fn from_yaml_str(yaml: &str) -> UObjectResult<Self> {
        let value = serde_yml::from_str(yaml)?;
        let value = JYValue::Yaml(value);
        Ok(Self::new(value))
    }
    pub fn to_json_string(self) -> Result<String, UObjectError> {
        let read = self.value.read()?;
        let s = match &*read {
            JYValue::Json(value) => serde_json::to_string(value)?,
            JYValue::Yaml(_) => {
                let json = JsonValue::from(read.clone());
                serde_json::to_string(&json)?
            },
        };
        Ok(s)
    }
    pub fn to_json_string_pretty(self) -> Result<String, UObjectError> {
        let read = self.value.read()?;
        let s = match &*read {
            JYValue::Json(value) => serde_json::to_string_pretty(value)?,
            JYValue::Yaml(_) => {
                let json = JsonValue::from(read.clone());
                serde_json::to_string_pretty(&json)?
            },
        };
        Ok(s)
    }
    pub fn to_yaml_string(self) -> Result<String, UObjectError> {
        let read = self.value.read()?;
        let s = match &*read {
            JYValue::Json(_) => {
                let yaml = YamlValue::from(read.clone());
                serde_yml::to_string(&yaml)?
            },
            JYValue::Yaml(value) => serde_yml::to_string(value)?,
        };
        Ok(s)
    }
    /// 任意のポインタを持った自身のクローンを作る
    pub fn clone_with_pointer(&self, pointer: Option<String>) -> Self {
        Self {
            value: Arc::clone(&self.value),
            pointer,
        }
    }
    pub fn index_to_pointer(&self, index: Option<&Object>) -> Option<String> {
        match (&self.pointer, index) {
            (None, None) => None,
            (None, Some(i)) => Some(format!("/{}", i)),
            (Some(p), None) => Some(p.to_string()),
            (Some(p), Some(i)) => Some(format!("{}/{}", p, i)),
        }
    }
    pub fn get(&self, index: &Object) -> EvalResult<Object> {
        let read = self.value.read().unwrap();
        match &*read {
            JYValue::Json(value) => {
                let parent = match &self.pointer {
                    Some(p) => value.pointer(p).unwrap_or(&JsonValue::Null),
                    None => value,
                };
                let value = match index {
                    Object::String(key) => {
                        parent.get_case_insensitive(key)
                    },
                    Object::Num(n) => {
                        parent.get(*n as usize)
                    },
                    _ => None,
                }.ok_or(UError::new(
                    UErrorKind::UObjectError,
                    UErrorMessage::InvalidMemberOrIndex(index.to_string())
                ))?;
                let obj = match value {
                    JsonValue::Null => Object::Null,
                    JsonValue::Bool(b) => (*b).into(),
                    JsonValue::Number(number) => number.as_f64().unwrap_or_default().into(),
                    JsonValue::String(s) => s.clone().into(),
                    JsonValue::Array(_) |
                    JsonValue::Object(_) => {
                        let pointer = self.index_to_pointer(Some(index));
                        let new = self.clone_with_pointer(pointer);
                        Object::UObject(new)
                    },
                };
                Ok(obj)
            },
            JYValue::Yaml(value) => {
                let parent = match &self.pointer {
                    Some(p) => value.pointer(p).unwrap_or(&YamlValue::Null),
                    None => value,
                };
                let value = match index {
                    Object::String(key) => {
                        parent.get_case_insensitive(key)
                    },
                    Object::Num(n) => {
                        parent.get(*n as usize)
                    },
                    _ => None,
                }.ok_or(UError::new(
                    UErrorKind::UObjectError,
                    UErrorMessage::InvalidMemberOrIndex(index.to_string())
                ))?;
                let obj = match value {
                    YamlValue::Null => Object::Null,
                    YamlValue::Bool(b) => (*b).into(),
                    YamlValue::Number(number) => number.as_f64().unwrap_or_default().into(),
                    YamlValue::String(s) => s.clone().into(),
                    YamlValue::Sequence(_) |
                    YamlValue::Mapping(_) |
                    YamlValue::Tagged(_) => {
                        let pointer = self.index_to_pointer(Some(index));
                        let new = self.clone_with_pointer(pointer);
                        Object::UObject(new)
                    },
                };
                Ok(obj)
            },
        }
    }
    fn pointer_to(&self, member: Option<&String>) -> Option<String> {
        match &self.pointer {
            Some(p) => match member {
                Some(m) => Some(format!("{p}/{m}")),
                None => Some(p.to_string()),
            },
            None => member.map(|m| format!("/{m}")),
        }
    }
    /// UObjectへの代入
    pub fn set(&self, index: Object, new_value: Object, member: Option<String>) -> EvalResult<()> {
        let mut write = self.value.write().unwrap();
        let pointer = self.pointer_to(member.as_ref());
        match &mut *write {
            JYValue::Json(value) => {
                let new_json_value = new_value.try_into()?;
                let mut_val = match &pointer {
                    Some(p) => value.pointer_mut(p)
                            .ok_or(UError::new(
                                UErrorKind::UObjectError,
                                UErrorMessage::InvalidMemberOrIndex(member.unwrap_or_default())
                            ))?,
                    None => value.borrow_mut(),
                };
                match index {
                    Object::String(key) => {
                        match mut_val.get_case_insensitive_mut(&key) {
                            Some(v) => {
                                *v = new_json_value;
                                Ok(())
                            },
                            None => Err(UError::new(
                                UErrorKind::UObjectError,
                                UErrorMessage::InvalidMemberOrIndex(key)
                            )),
                        }
                    },
                    Object::Num(i) => {
                        match mut_val.get_mut(i as usize) {
                            Some(v) => {
                                *v = new_json_value;
                                Ok(())
                            },
                            None => todo!(),
                        }
                    },
                    o => Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::InvalidMemberOrIndex(o.to_string())
                    )),
                }
            },
            JYValue::Yaml(value) => {
                let new_yaml_value = new_value.try_into()?;
                let mut_val = match &pointer {
                    Some(p) => value.pointer_mut(p)
                    .ok_or(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::InvalidMemberOrIndex(member.unwrap_or_default())
                    ))?,
                    None => value.borrow_mut(),
                };
                match index {
                    Object::String(key) => {
                        match mut_val.get_case_insensitive_mut(&key) {
                            Some(v) => {
                                *v = new_yaml_value;
                                Ok(())
                            },
                            None => Err(UError::new(
                                UErrorKind::UObjectError,
                                UErrorMessage::InvalidMemberOrIndex(key)
                            )),
                        }
                    },
                    Object::Num(i) => {
                        match mut_val.get_mut(i as usize) {
                            Some(v) => {
                                *v = new_yaml_value;
                                Ok(())
                            },
                            None => todo!(),
                        }
                    },
                    o => Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::InvalidMemberOrIndex(o.to_string())
                    )),
                }
            },
        }
    }
    /// 配列へのpush\
    /// 成功時true
    pub fn push(&self, new_value: Object) -> bool {
        let mut write = self.value.write().unwrap();
        match &mut *write {
            JYValue::Json(value) => {
                let Ok(new_json_value) = new_value.try_into() else {
                    return false;
                };
                let mut_array = match &self.pointer {
                    Some(p) => value.pointer_mut(p).and_then(|v| v.as_array_mut()),
                    None => value.as_array_mut(),
                };
                if let Some(array) = mut_array {
                    array.push(new_json_value);
                    true
                } else {
                    false
                }
            },
            JYValue::Yaml(value) => {
                let Ok(new_yaml_value) = new_value.try_into() else {
                    return false
                };
                let mut_array = match &self.pointer {
                    Some(p) => value.pointer_mut(p).and_then(|v| v.as_array_mut()),
                    None => value.as_array_mut(),
                };
                if let Some(array) = mut_array {
                    array.push(new_yaml_value);
                    true
                } else {
                    false
                }
            },
        }
    }
    pub fn to_object_vec(&self) -> EvalResult<Vec<Object>> {
        let read = self.value.read().unwrap();
        match &*read {
            JYValue::Json(value) => value.as_array()
                .map(|arr| {
                        (0..arr.len()).map(|i| {
                            let pointer = match &self.pointer {
                                Some(p) => format!("{p}/{i}"),
                                None => format!("/{i}"),
                            };
                            let o = self.clone_with_pointer(Some(pointer));
                            Object::UObject(o)
                        })
                        .collect()
                }),
            JYValue::Yaml(value) => value.as_array()
                .map(|arr| {
                    (0..arr.len()).map(|i| {
                        let pointer = match &self.pointer {
                            Some(p) => format!("{p}/{i}"),
                            None => format!("/{i}"),
                        };
                        let o = self.clone_with_pointer(Some(pointer));
                        Object::UObject(o)
                    })
                    .collect()
                }),
        }.ok_or(UError::new(
            UErrorKind::UObjectError,
            UErrorMessage::UObjectIsNotAnArray,
        ))
    }
    pub fn get_size(&self) -> usize {
        let read = self.value.read().unwrap();
        match &*read {
            JYValue::Json(value) => match value {
                JsonValue::Array(values) => values.len(),
                JsonValue::Object(map) => map.len(),
                _ => 0
            },
            JYValue::Yaml(value) => value.len(),
        }
    }
    fn keys(&self) -> EvalResult<Vec<Object>> {
        let read = self.value.read().unwrap();
        let keys = match &*read {
            JYValue::Json(value) => match value {
                JsonValue::Object(map) => {
                    map.keys().map(|key| key.as_str().into()).collect()
                },
                _ => Vec::new()
            },
            JYValue::Yaml(value) => match value {
                YamlValue::Mapping(map) => {
                    map.keys().map(|key| key.as_str().into()).collect()
                },
                _ => Vec::new()
            },
        };
        Ok(keys)
    }
    fn values(&self) -> EvalResult<Vec<Object>> {
        let read = self.value.read().unwrap();
        let values = match &*read {
            JYValue::Json(value) => match value {
                JsonValue::Object(map) => {
                    map.values().map(|v| v.as_str().into()).collect()
                },
                _ => Vec::new()
            },
            JYValue::Yaml(value) => match value {
                YamlValue::Mapping(map) => {
                    map.values().map(|v| v.as_str().into()).collect()
                },
                _ => Vec::new()
            },
        };
        Ok(values)
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
        let read = self.value.read().unwrap();
        let s = match &*read {
            JYValue::Json(value) => {
                let value = match &self.pointer {
                    Some(p) => value.pointer(p).unwrap_or(&JsonValue::Null),
                    None => value,
                };
                match serde_json::to_string(value) {
                    Ok(json) => json,
                    Err(e) => e.to_string(),
                }
            },
            JYValue::Yaml(value) => {
                let value = match &self.pointer {
                    Some(p) => value.pointer(p).unwrap_or(&YamlValue::Null),
                    None => value,
                };
                match serde_yml::to_string(value) {
                    Ok(yaml) => yaml,
                    Err(e) => e.to_string(),
                }
            },
        };
        write!(f, "{s}")
    }
}

impl PartialEq for UObject {
    fn eq(&self, other: &Self) -> bool {
        // 一方をロックしもう一方もロックできれば別のオブジェクト
        let _tmp = self.value.write().unwrap();
        let is_same_object = other.value.write().is_err();
        is_same_object && self.pointer == other.pointer
    }
}

type UObjectResult<T> = Result<T, UObjectError>;
pub struct UObjectError(String);
impl<E: std::error::Error> From<E> for UObjectError {
    fn from(e: E) -> Self {
        Self(e.to_string())
    }
}
impl std::fmt::Display for UObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<UObjectError> for UError {
    fn from(e: UObjectError) -> Self {
        Self::new(UErrorKind::UObjectError, UErrorMessage::Any(e.to_string()))
    }
}

trait YamlPointer {
    fn pointer(&self, pointer: &str) -> Option<&YamlValue>;
    fn pointer_mut(&mut self, pointer: &str) -> Option<&mut YamlValue>;
    fn parse_index(s: &str) -> Option<usize> {
        if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
            return None;
        }
        s.parse().ok()
    }
}

impl YamlPointer for YamlValue {
    fn pointer(&self, pointer: &str) -> Option<&YamlValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, index| match target {
                Self::Sequence(seq) => Self::parse_index(&index).and_then(|i| seq.get(i)),
                Self::Mapping(map) => map.get(&index),
                Self::Tagged(tagged) => tagged.value.get(&index),
                _ => None,
            })
    }

    fn pointer_mut(&mut self, pointer: &str) -> Option<&mut YamlValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }

        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, index| match target {
                Self::Sequence(seq) => Self::parse_index(&index).and_then(|i| seq.get_mut(i)),
                Self::Mapping(map) => map.get_mut(&index),
                Self::Tagged(tagged) => tagged.value.get_mut(&index),
                _ => None,
            })
    }
}

trait ValueExt {
    /// キーのcase一致がなければcase不一致で該当するものを探す
    /// 1. caseも含めて一致するものがあればそれを返す
    /// 2. 1. がなければ最初に見つかったものを返す
    fn get_case_insensitive(&self, key: &str) -> Option<&Self>;
    /// キーのcase一致がなければcase不一致で該当するものを探す
    /// 1. caseも含めて一致するものがあればそれを返す
    /// 2. 1. がなければ最初に見つかったものを返す
    fn get_case_insensitive_mut(&mut self, key: &str) -> Option<&mut Self>;
}
trait YamlValueExt {
    fn as_array(&self) -> Option<&Vec<Self>> where Self: Sized;
    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> where Self: Sized;
    fn len(&self) -> usize;
}
impl ValueExt for JsonValue {
    fn get_case_insensitive(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => {
                match map.get(key) {
                    // case完全一致
                    Some(found) => Some(found),
                    // case無視一致の最初のもの
                    None => map.iter()
                        .find_map(|(k, v)| {
                            k.eq_ignore_ascii_case(key).then_some(v)
                        }),
                }
            },
            _ => None,
        }
    }
    fn get_case_insensitive_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        match self {
            JsonValue::Object(map) => {
                map.iter_mut()
                    .filter(|(k, _)| {
                        k.eq_ignore_ascii_case(key)
                    })
                    .find_or_first(|(k, _)| k.eq(&key))
                    .map(|(_, k)| k)
            },
            _ => None,
        }
    }
}
impl ValueExt for YamlValue {
    fn get_case_insensitive(&self, key: &str) -> Option<&YamlValue> {
        match self {
            YamlValue::Mapping(mapping) => {
                match mapping.get(key) {
                    Some(found) => Some(found),
                    None => mapping.iter()
                        .find_map(|(k, v)| {
                            let key_str = k.as_str()?;
                            key_str.eq_ignore_ascii_case(key).then_some(v)
                        }),
                }
            },
            YamlValue::Tagged(tagged_value) => {
                tagged_value.value.get_case_insensitive(key)
            },
            _ => None
        }
    }

    fn get_case_insensitive_mut(&mut self, key: &str) -> Option<&mut YamlValue> {
        match self {
            YamlValue::Mapping(mapping) => {
                mapping.iter_mut()
                    .filter_map(|(k, v)| {
                        let key_str = k.as_str()?;
                        key_str.eq_ignore_ascii_case(key)
                            .then_some((key_str, v))
                    })
                    .find_or_first(|(k, _)| k.eq(&key))
                    .map(|(_, v)| v)
            },
            YamlValue::Tagged(tagged_value) => {
                tagged_value.value.get_case_insensitive_mut(key)
            },
            _ => None
        }
    }
}
impl YamlValueExt for YamlValue {
        fn as_array(&self) -> Option<&Vec<YamlValue>> {
            match self {
                YamlValue::Sequence(seq) => Some(seq),
                _ => None
            }
        }

        fn as_array_mut(&mut self) -> Option<&mut Vec<YamlValue>> {
            match self {
                YamlValue::Sequence(seq) => Some(seq),
                _ => None
            }
        }
        fn len(&self) -> usize {
            match self {
                YamlValue::Sequence(values) => values.len(),
                YamlValue::Mapping(mapping) => mapping.len(),
                YamlValue::Tagged(tagged) => match &tagged.value {
                    YamlValue::Mapping(mapping) => mapping.len(),
                    YamlValue::Sequence(seq) => seq.len(),
                    _ => 0,
                },
                _ => 0
            }
        }

}

impl From<JYValue> for JsonValue {
    fn from(value: JYValue) -> Self {
        match value {
            JYValue::Json(value) => value,
            JYValue::Yaml(value) => match value {
                YamlValue::Null => JsonValue::Null,
                YamlValue::Bool(b) => JsonValue::Bool(b),
                YamlValue::Number(number) => {
                    let f = number.as_f64().unwrap_or_default();
                    match serde_json::Number::from_f64(f) {
                        Some(n) => JsonValue::Number(n),
                        None => JsonValue::Null,
                    }
                },
                YamlValue::String(s) => JsonValue::String(s),
                YamlValue::Sequence(values) => {
                    let vec = values.into_iter().map(|v| JYValue::Yaml(v).into()).collect();
                    JsonValue::Array(vec)
                },
                YamlValue::Mapping(mapping) => {
                    let map = mapping.map.into_iter()
                        .map(|(k, v)| (k.as_str().unwrap_or_default().to_string(), JYValue::Yaml(v).into()))
                        .collect();
                    JsonValue::Object(map)
                },
                YamlValue::Tagged(tagged_value) => {
                    JYValue::Yaml(tagged_value.value).into()
                },
            },
        }
    }
}

impl From<JYValue> for YamlValue {
    fn from(value: JYValue) -> Self {
        match value {
            JYValue::Yaml(value) => value,
            JYValue::Json(value) => match value {
                JsonValue::Null => YamlValue::Null,
                JsonValue::Bool(b) => YamlValue::Bool(b),
                JsonValue::Number(number) => {
                    let f = number.as_f64().unwrap_or_default();
                    let n = serde_yml::Number::from(f);
                    YamlValue::Number(n)
                },
                JsonValue::String(s) => YamlValue::String(s),
                JsonValue::Array(values) => {
                    let seq = values.into_iter().map(|v| JYValue::Json(v).into()).collect();
                    YamlValue::Sequence(seq)
                },
                JsonValue::Object(map) => {
                    let mapping = map.into_iter()
                        .map(|(k, v)| (YamlValue::String(k), JYValue::Json(v).into()))
                        .collect();
                    YamlValue::Mapping(mapping)
                },
            },
        }
    }
}

impl TryFrom<Object> for YamlValue {
    type Error = UError;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        match object {
            Object::Null => Ok(YamlValue::Null),
            Object::Bool(b) => Ok(YamlValue::Bool(b)),
            Object::Num(n) => Ok(YamlValue::Number(n.into())),
            Object::String(s) => Ok(YamlValue::String(s)),
            Object::UObject(jy) => {
                let read = jy.value.read().unwrap();
                let yaml = match &*read {
                    JYValue::Json(value) => {
                        let copy = match &jy.pointer {
                            Some(p) => value.pointer(p).unwrap(),
                            None => value,
                        }.clone();
                        JYValue::Json(copy).into()
                    },
                    JYValue::Yaml(value) => value.clone(),
                };
                Ok(yaml)
            },
            Object::Array(arr) => {
                let seq = arr.into_iter()
                    .map(|o| o.try_into())
                    .collect::<Result<_, Self::Error>>()?;
                Ok(YamlValue::Sequence(seq))
            }
            o => Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::CanNotConvertToUObject(o)
            ))
        }
    }
}

impl TryFrom<Object> for JsonValue {
    type Error = UError;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        match object {
            Object::Null => Ok(JsonValue::Null),
            Object::Bool(b) => Ok(JsonValue::Bool(b)),
            Object::Num(f) => {
                match serde_json::Number::from_f64(f) {
                    Some(n) => Ok(JsonValue::Number(n)),
                    None => Err(UError::new(
                        UErrorKind::UObjectError,
                        UErrorMessage::CanNotConvertToUObject(object)
                    )),
                }
            },
            Object::String(s) => Ok(JsonValue::String(s)),
            Object::UObject(jy) => {
                let read = jy.value.read().unwrap();
                let json = match &*read {
                    JYValue::Yaml(value) => {
                        let copy = match &jy.pointer {
                            Some(p) => value.pointer(p).unwrap(),
                            None => value,
                        }.clone();
                        JYValue::Yaml(copy).into()
                    },
                    JYValue::Json(value) => value.clone(),
                };
                Ok(json)
            },
            Object::Array(arr) => {
                let vec = arr.into_iter()
                    .map(|o| o.try_into())
                    .collect::<Result<_, Self::Error>>()?;
                Ok(JsonValue::Array(vec))
            }
            o => Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::CanNotConvertToUObject(o)
            ))
        }
    }
}

impl From<JsonValue> for UObject {
    fn from(value: JsonValue) -> Self {
        UObject::new(JYValue::Json(value))
    }
}
impl From<JsonValue> for Object {
    fn from(value: JsonValue) -> Self {
        Self::UObject(value.into())
    }
}