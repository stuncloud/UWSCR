use super::Object;
use crate::error::{UError,UErrorKind,UErrorMessage};
use crate::EvalResult;

use std::ops::Deref;
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
#[derive(Debug, PartialEq)]
enum JYValueRef<'a> {
    Json(&'a JsonValue),
    Yaml(&'a YamlValue),
}
#[derive(Debug)]
enum JYValueMut<'a> {
    Json(&'a mut JsonValue),
    Yaml(&'a mut YamlValue),
}
impl JYValue {
    const NULL_JSON: JsonValue = JsonValue::Null;
    const NULL_YAML: YamlValue = YamlValue::Null;
    fn value_from_pointer(&self, pointer: Option<&str>) -> JYValueRef {
        match pointer {
            Some(p) => match self {
                JYValue::Json(value) => value.pointer(p)
                    .map(JYValueRef::Json).unwrap_or(JYValueRef::Json(&Self::NULL_JSON)),
                JYValue::Yaml(value) => value.pointer(p)
                    .map(JYValueRef::Yaml).unwrap_or(JYValueRef::Yaml(&Self::NULL_YAML)),
            },
            None => match self {
                JYValue::Json(value) => JYValueRef::Json(value),
                JYValue::Yaml(value) => JYValueRef::Yaml(value),
            },
        }
    }
    fn set_value_on_pointer<F, T>(&mut self, pointer: Option<&str>, f: F) -> T
    where
        F: Fn(JYValueMut) -> T,
    {
        match pointer {
            Some(p) => {
                match self {
                    JYValue::Json(value) => match value.pointer_mut(p) {
                        Some(v) => f(JYValueMut::Json(v)),
                        None => f(JYValueMut::Json(&mut JsonValue::Null)),
                    },
                    JYValue::Yaml(value) => match value.pointer_mut(p) {
                        Some(v) => f(JYValueMut::Yaml(v)),
                        None => f(JYValueMut::Yaml(&mut YamlValue::Null)),
                    },
                }
            },
            None => {
                let jymut = match self {
                    JYValue::Json(value) => JYValueMut::Json(value),
                    JYValue::Yaml(value) => JYValueMut::Yaml(value),
                };
                f(jymut)
            },
        }
    }
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
        let jyref = read.value_from_pointer(self.pointer.as_deref());
        let value = JsonValue::from(jyref);
        let s = serde_json::to_string(&value)?;
        Ok(s)
    }
    pub fn to_json_string_pretty(self) -> Result<String, UObjectError> {
        let read = self.value.read()?;
        let jyref = read.value_from_pointer(self.pointer.as_deref());
        let value = JsonValue::from(jyref);
        let s = serde_json::to_string_pretty(&value)?;
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
    pub fn pointer_to<I: std::fmt::Display + ?Sized>(&self, member: Option<&I>) -> Option<String> {
        match (&self.pointer, member) {
            (None, None) => None,
            (None, Some(i)) => Some(format!("/{i}")),
            (Some(p), None) => Some(p.to_string()),
            (Some(p), Some(i)) => Some(format!("{p}/{i}")),
        }
    }
    fn json_value_to_object<I: std::fmt::Display>(&self, value: &JsonValue, member: Option<&I>) -> Object {
        match value {
            JsonValue::Null => Object::Null,
            JsonValue::Bool(b) => (*b).into(),
            JsonValue::Number(number) => number.into(),
            JsonValue::String(s) => s.deref().into(),
            JsonValue::Array(_) |
            JsonValue::Object(_) => {
                let pointer = self.pointer_to(member);
                let obj = self.clone_with_pointer(pointer);
                Object::UObject(obj)
            },
        }
    }
    fn yaml_value_to_object<I: std::fmt::Display + ?Sized>(&self, value: &YamlValue, member: Option<&I>) -> Object {
        match value {
            YamlValue::Null => Object::Null,
            YamlValue::Bool(b) => (*b).into(),
            YamlValue::Number(number) => number.into(),
            YamlValue::String(s) => s.deref().into(),
            YamlValue::Sequence(_) |
            YamlValue::Mapping(_) |
            YamlValue::Tagged(_) => {
                let pointer = self.pointer_to(member);
                let obj = self.clone_with_pointer(pointer);
                Object::UObject(obj)
            },
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
                let obj = self.json_value_to_object(value, Some(index));
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
                let obj = self.yaml_value_to_object(value, Some(index));
                Ok(obj)
            },
        }
    }
    /// UObjectへの代入
    pub fn set(&self, index: Object, new_value: Object, member: Option<String>) -> EvalResult<()> {
        let new_jy = JYValue::try_from(new_value)?;
        let mut write = self.value.write().unwrap();
        let index = &index;
        let pointer = self.pointer_to(member.as_ref());
        write.set_value_on_pointer(pointer.as_deref(), move |jymut| {
            match jymut {
                JYValueMut::Json(value) => {
                    let new_j = JsonValue::from(new_jy.clone());
                    match index {
                        Object::String(key) => {
                            let old = value.get_case_insensitive_mut(key)
                                .ok_or(UError::new(UErrorKind::UObjectError, UErrorMessage::InvalidMemberOrIndex(key.to_string())))?;
                            *old = new_j;
                            Ok(())
                        },
                        Object::Num(i) => {
                            let old = value.get_mut(*i as usize)
                                .ok_or(UError::new(UErrorKind::UObjectError, UErrorMessage::InvalidMemberOrIndex(i.to_string())))?;
                            *old = new_j;
                            Ok(())
                        },
                        _ => Err(UError::new(
                            UErrorKind::UObjectError,
                            UErrorMessage::InvalidMemberOrIndex(index.to_string())
                        ))
                    }
                },
                JYValueMut::Yaml(value) => {
                    let new_y = YamlValue::from(new_jy.clone());
                    match index {
                        Object::String(key) => {
                            let old = value.get_case_insensitive_mut(key)
                                .ok_or(UError::new(UErrorKind::UObjectError, UErrorMessage::InvalidMemberOrIndex(key.to_string())))?;
                            *old = new_y;
                            Ok(())
                        },
                        Object::Num(i) => {
                            let old =value.get_mut(*i as usize)
                                .ok_or(UError::new(UErrorKind::UObjectError, UErrorMessage::InvalidMemberOrIndex(i.to_string())))?;
                            *old = new_y;
                            Ok(())
                        },
                        _ => Err(UError::new(
                            UErrorKind::UObjectError,
                            UErrorMessage::InvalidMemberOrIndex(index.to_string())
                        ))
                    }
                },
            }
        })
    }
    /// 配列へのpush\
    /// 成功時true
    pub fn push(&self, new_value: Object) -> bool {
        let Ok(new_jy) = JYValue::try_from(new_value) else {
            return false;
        };
        let mut write = self.value.write().unwrap();
        write.set_value_on_pointer(self.pointer.as_deref(), move |jymut| {
            match jymut {
                JYValueMut::Json(value) => {
                    let new_j = JsonValue::from(new_jy.clone());
                    if let JsonValue::Array(arr) = value {
                        arr.push(new_j);
                        true
                    } else {
                        false
                    }
                },
                JYValueMut::Yaml(value) => {
                    let new_y = YamlValue::from(new_jy.clone());
                    if let YamlValue::Sequence(seq) = value {
                        seq.push(new_y);
                        true
                    } else {
                        false
                    }
                },
            }
        })
    }
    pub fn to_object_vec(&self) -> EvalResult<Vec<Object>> {
        let read = self.value.read().unwrap();
        let jy = read.value_from_pointer(self.pointer.as_deref());
        match jy {
            JYValueRef::Json(JsonValue::Array(vec)) => {
                let vec = vec.iter().enumerate()
                    .map(|(i, v)| self.json_value_to_object(v, Some(&i)))
                    .collect();
                Ok(vec)
            },
            JYValueRef::Yaml(YamlValue::Sequence(vec)) => {
                let vec = vec.iter().enumerate()
                    .map(|(i, v)| self.yaml_value_to_object(v, Some(&i)))
                    .collect();
                Ok(vec)
            }
            _ => Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::UObjectIsNotAnArray,
            ))
        }
    }
    pub fn get_size(&self) -> usize {
        let read = self.value.read().unwrap();
        let jyref = read.value_from_pointer(self.pointer.as_deref());
        match jyref {
            JYValueRef::Json(value) => match value {
                JsonValue::Array(arr) => arr.len(),
                JsonValue::Object(map) => map.len(),
                _ => 0,
            },
            JYValueRef::Yaml(value) => value.len(),
        }
    }
    fn keys(&self) -> EvalResult<Vec<Object>> {
        let read = self.value.read().unwrap();
        let jyref = read.value_from_pointer(self.pointer.as_deref());
        let keys = match jyref {
            JYValueRef::Json(value) => match value {
                JsonValue::Object(map) => {
                    map.keys().map(|key| key.as_str().into()).collect()
                },
                _ => Vec::new()
            },
            JYValueRef::Yaml(value) => match value {
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
        let jyref = read.value_from_pointer(self.pointer.as_deref());
        let values = match jyref {
            JYValueRef::Json(value) => match value {
                JsonValue::Object(map) => {
                    map.keys().zip(map.values())
                        .map(|(key, value)| self.json_value_to_object(value, Some(key)))
                        .collect()
                },
                _ => Vec::new()
            },
            JYValueRef::Yaml(value) => match value {
                YamlValue::Mapping(map) => {
                    map.keys().zip(map.values())
                        .map(|(key, value)| self.yaml_value_to_object(value, key.as_str()))
                        .collect()
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
        match self.clone().to_json_string() {
            Ok(j) => write!(f, "{j}"),
            Err(e) => write!(f, "{e}"),
        }
    }
}

impl PartialEq for UObject {
    fn eq(&self, other: &Self) -> bool {
        let read1 = self.value.read().unwrap();
        let value1 = read1.value_from_pointer(self.pointer.as_deref());
        let read2 = other.value.read().unwrap();
        let value2 = read2.value_from_pointer(other.pointer.as_deref());
        value1 == value2
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
    // fn as_array(&self) -> Option<&Vec<Self>> where Self: Sized;
    // fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> where Self: Sized;
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
        // fn as_array(&self) -> Option<&Vec<YamlValue>> {
        //     match self {
        //         YamlValue::Sequence(seq) => Some(seq),
        //         _ => None
        //     }
        // }

        // fn as_array_mut(&mut self) -> Option<&mut Vec<YamlValue>> {
        //     match self {
        //         YamlValue::Sequence(seq) => Some(seq),
        //         _ => None
        //     }
        // }
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
                YamlValue::Number(yml_num) => {
                    if yml_num.is_i64() {
                        JsonValue::Number(yml_num.as_i64().map(serde_json::Number::from).unwrap())
                    } else if yml_num.is_u64() {
                        JsonValue::Number(yml_num.as_u64().map(serde_json::Number::from).unwrap())
                    } else if yml_num.is_f64() {
                        let f = yml_num.as_f64().unwrap();
                        match serde_json::Number::from_f64(f) {
                            Some(n) => JsonValue::Number(n),
                            None => JsonValue::Null,
                        }
                    } else {
                        JsonValue::Null
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

impl From<JYValueRef<'_>> for JsonValue {
    fn from(r: JYValueRef) -> Self {
        match r {
            JYValueRef::Json(value) => value.clone(),
            JYValueRef::Yaml(value) => JYValue::Yaml(value.clone()).into(),
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
                JsonValue::Number(jsn_num) => {
                    if jsn_num.is_i64() {
                        YamlValue::Number(jsn_num.as_i64().map(serde_yml::Number::from).unwrap())
                    } else if jsn_num.is_u64() {
                        YamlValue::Number(jsn_num.as_u64().map(serde_yml::Number::from).unwrap())
                    } else if jsn_num.is_f64() {
                        YamlValue::Number(jsn_num.as_f64().map(serde_yml::Number::from).unwrap())
                    } else {
                        YamlValue::Null
                    }
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

impl TryFrom<Object> for JYValue {
    type Error = UError;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        match object {
            Object::Null => Ok(JYValue::Json(JsonValue::Null)),
            Object::Bool(b) => Ok(JYValue::Json(JsonValue::Bool(b))),
            Object::Num(n) => {
                let number = new_json_number(n)?;
                Ok(JYValue::Json(JsonValue::Number(number)))
            },
            Object::String(s) => Ok(JYValue::Json(JsonValue::String(s))),
            Object::UObject(uo) => {
                let read = uo.value.read().unwrap();
                match &*read {
                    JYValue::Json(value) => {
                        match &uo.pointer {
                            Some(p) => match value.pointer(p) {
                                Some(value) => Ok(JYValue::Json(value.clone())),
                                None => Ok(JYValue::Json(JsonValue::Null)),
                            },
                            None => Ok(JYValue::Json(value.clone())),
                        }
                    },
                    JYValue::Yaml(value) => {
                        match &uo.pointer {
                            Some(p) => match value.pointer(p) {
                                Some(value) => Ok(JYValue::Yaml(value.clone())),
                                None => Ok(JYValue::Yaml(YamlValue::Null)),
                            },
                            None => Ok(JYValue::Yaml(value.clone())),
                        }
                    },
                }
            },
            Object::Array(arr) => {
                let arr = arr.into_iter()
                    .map(|o| o.try_into())
                    .collect::<Result<_, Self::Error>>()?;
                Ok(JYValue::Json(JsonValue::Array(arr)))
            }
            o => Err(UError::new(
                UErrorKind::UObjectError,
                UErrorMessage::CanNotConvertToUObject(o)
            ))
        }
    }
}

impl TryFrom<Object> for YamlValue {
    type Error = UError;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        match object {
            Object::Null => Ok(YamlValue::Null),
            Object::Bool(b) => Ok(YamlValue::Bool(b)),
            Object::Num(n) => {
                let number = new_yaml_number(n);
                Ok(YamlValue::Number(number))
            },
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
                let number = new_json_number(f)?;
                Ok(JsonValue::Number(number))
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
        match value {
            JsonValue::Null => Object::Null,
            JsonValue::Bool(b) => b.into(),
            JsonValue::Number(n) => n.as_f64().unwrap_or_default().into(),
            JsonValue::String(s) => s.into(),
            value => Object::UObject(value.into())
        }
    }
}
impl From<&serde_json::Number> for Object {
    fn from(n: &serde_json::Number) -> Self {
        n.as_f64().unwrap_or_default().into()
    }
}
impl From<&serde_yml::Number> for Object {
    fn from(n: &serde_yml::Number) -> Self {
        n.as_f64().unwrap_or_default().into()
    }
}

fn new_json_number(n: f64) -> Result<serde_json::Number, UError> {
    if n.fract() == 0.0 {
        if n.is_sign_negative() {
            Ok(serde_json::Number::from(n as i64))
        } else {
            Ok(serde_json::Number::from(n as u64))
        }
    } else {
        serde_json::Number::from_f64(n).ok_or(UError::new(
            UErrorKind::UObjectError,
            UErrorMessage::CanNotConvertToUObject(Object::Num(n))
        ))
    }
}
fn new_yaml_number(n: f64) -> serde_yml::Number {
    if n.fract() == 0.0 {
        if n.is_sign_negative() {
            serde_yml::Number::from(n as i64)
        } else {
            serde_yml::Number::from(n as u64)
        }
    } else {
        serde_yml::Number::from(n)
    }
}
