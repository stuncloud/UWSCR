use super::Object;

use indexmap::IndexMap;
use strum_macros::{EnumString, VariantNames, EnumProperty};
use num_derive::{ToPrimitive, FromPrimitive};

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum HashTblEnum {
    HASH_CASECARE = 0x1000,
    HASH_SORT = 0x2000,
    HASH_EXISTS = -103,
    HASH_REMOVE = -104,
    HASH_KEY = -101,
    HASH_VAL = -102,
    HASH_REMOVEALL = -109,
    // HASH_UNKNOWN = 0,
}

#[derive(Clone, Debug, PartialEq)]
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
        self.map.keys().map(|key| Object::String(key.clone())).collect()
    }
    pub fn values(&self) -> Vec<Object> {
        self.map.values().map(|val| val.clone()).collect()
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

    pub fn get(&self, name: &String) -> Object {
        let key = if ! self.casecare { name.to_ascii_uppercase() } else { name.to_string() };
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
        let removed = self.map.shift_remove(&key).is_some();
        Object::Bool(removed)
    }
    // hash = hash_removeall
    pub fn clear(&mut self) {
        self.map.clear();
    }
}
