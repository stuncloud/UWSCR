use super::{Object, Function};
use crate::evaluator::{EvalResult, Evaluator};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage, DefinitionType};
use crate::evaluator::environment::{
    NamedObject, ContainerType,
    check_special_assignment,
};

use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Module {
    name: String,
    members: Vec<NamedObject>,
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Module {
    pub fn new(name: String) -> Self {
        Module{name, members: Vec::new()}
    }

    pub fn new_with_members(name: String, members: Vec<NamedObject>) -> Self {
        Module{name, members}
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn get_members(&self) -> Vec<NamedObject> {
        self.members.clone()
    }

    pub fn get_constructor(&self) -> Option<Function> {
        match self.get(&self.name, &[ContainerType::Function])? {
            Object::Function(f) |
            Object::AnonFunc(f) => {
                Some(f)
            },
            _ => None
        }
    }

    pub fn has_destructor(&self) -> bool {
        let name = format!("_{}_", self.name());
        self.contains(&name, ContainerType::Function)
    }

    pub fn is_destructor_name(&self, name: &str) -> bool {
        name.to_string() == format!("_{}_", self.name())
    }

    pub fn get_destructor(&self) -> Option<Object> {
        let name = format!("_{}_", self.name());
        self.get(&name, &[ContainerType::Function])
    }

    pub fn add(&mut self, name: String, object: Object, container_type: ContainerType) {
        self.members.push(NamedObject::new(name.to_ascii_uppercase(), object, container_type))
    }

    fn contains(&self, name: &str, container_type: ContainerType) -> bool {
        let key = name.to_ascii_uppercase();
        self.members.iter().any(|obj| obj.name == key && container_type == obj.container_type)
    }
    pub fn has_member(&self, name: &str) -> bool {
        let key = name.to_ascii_uppercase();
        self.members.iter().any(|obj| obj.name == key)
    }

    fn get(&self, name: &str, container_type: &[ContainerType]) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        for ct in container_type {
            if let Some(o) = self.members.clone().iter().find(|o| o.name == key && o.container_type == *ct) {
                return Some(o.object.clone())
            }
        }
        None
    }

    fn set(&mut self, name: &str, value: Object, container_type: ContainerType) {
        let key = name.to_ascii_uppercase();
        for obj in self.members.iter_mut() {
            if obj.name == key && obj.container_type == container_type {
                if check_special_assignment(&obj.object, &value) {
                    obj.object = value;
                }
                break;
            }
        }
    }

    pub fn get_member(&self, name: &str) -> EvalResult<Object> {
        let container_type = [ContainerType::Variable, ContainerType::Public, ContainerType::Const];
        match self.get(name, &container_type) {
            Some(o) => Ok(o),
            None => Err(UError::new(
                UErrorKind::ModuleError,
                UErrorMessage::ModuleMemberNotFound(DefinitionType::Any, self.name.to_string(), name.to_string())
            ))
        }
    }

    pub fn get_public_member(&self, name: &str) -> EvalResult<Object> {
        let container_type = [ContainerType::Public, ContainerType::Const, ContainerType::Function];
        match self.get(name, &container_type) {
            Some(o) => Ok(o),
            None => Err(UError::new(
                UErrorKind::ModuleError,
                UErrorMessage::ModuleMemberNotFound(DefinitionType::Public, self.name.to_string(), name.to_string())
            ))
        }
    }

    pub fn get_function(&self, name: &str) -> EvalResult<Object> {
        let func = match self.get(name, &[ContainerType::Function]) {
            // 関数定義から探す
            Some(o) => Some(o),
            None => match self.get(name, &[ContainerType::Variable]) {
                // プライベート関数を探す (dim宣言した無名関数等)
                Some(o) => match o {
                    Object::AnonFunc(_) |
                    Object::Function(_) |
                    Object::BuiltinFunction(_,_,_) => Some(o),
                    _ => None
                },
                None => match self.get_public_member(name) {
                    // パブリック関数を探す
                    Ok(o) => match o {
                        Object::AnonFunc(_) |
                        Object::Function(_) |
                        Object::BuiltinFunction(_,_,_) => Some(o),
                        _ => None
                    },
                    Err(_) => None
                },
            }
        };
        func.ok_or(UError::new(
            UErrorKind::ModuleError,
            UErrorMessage::ModuleMemberNotFound(DefinitionType::Function, self.name.to_string(), name.to_string())
        ))
    }

    fn assign_index(&mut self, name: &str, new: Object, dimension: Vec<Object>, container_type: ContainerType) -> Result<(), UError> {
        let array = self.get_member(name)?;
        let (maybe_new, update) = Evaluator::update_array_object(array, dimension, &new)
            .map_err(|mut e| {
                if let UErrorMessage::NotAnArray(_) = e.message {
                    e.message = UErrorMessage::NotAnArray(name.clone().into());
                }
                e
            })?;
        if update {
            if let Some(new_array) = maybe_new {
                self.set(name, new_array, container_type);
            }
        }
        Ok(())
    }

    pub fn assign(&mut self, name: &str, value: Object, dimension: Option<Vec<Object>>) -> Result<(), UError> {
        let container_type = if self.contains(name, ContainerType::Const) {
            // 同名の定数がある場合はエラー
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::ConstantCantBeAssigned(name.to_string())
            ))
        } else if self.contains(name, ContainerType::Variable) {
            // 同名ローカル変数があれば上書き
            ContainerType::Variable
        } else if self.contains(name, ContainerType::Public) {
            // 同名パブリック変数があれば上書き
            ContainerType::Public
        } else {
            return Ok(());
        };
        match dimension {
            Some(d) => {
                return self.assign_index(name, value, d, container_type)
            },
            None => self.set(name, value, container_type)
        }
        Ok(())
    }

    pub fn assign_public(&mut self, name: &str, value: Object, dimension: Option<Vec<Object>>) -> Result<(), UError> {
        if self.contains(&name, ContainerType::Public) {
            match dimension {
                Some(d) => {
                    return self.assign_index(name, value, d, ContainerType::Public)
                },
                None => self.set(name, value, ContainerType::Public)
            }
        } else {
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::ModuleMemberNotFound(DefinitionType::Public, self.name(),name.to_string())
            ))
        }
        Ok(())
    }

    pub fn is_local_member(&self, name: &str) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, ContainerType::Variable)
    }

    /// 自身のメンバ関数に自身のポインタを渡す
    /// thisとglobalもセットする
    pub fn set_module_reference(&mut self, m: Arc<Mutex<Module>>) {
        for o in self.members.iter_mut() {
            match o.object.as_mut() {
                Object::Function(f) => {
                    f.set_module(m.clone());
                }
                Object::AnonFunc(f) => {
                    f.set_module(m.clone());
                    // 無名関数ならスコープ情報を消す
                    f.outer = None;
                },
                _ => {},
            }
        }
    }

    pub fn is_disposed(&self) -> bool {
        self.members.len() == 0
    }

    pub fn dispose(&mut self) {
        self.members = vec![];
    }

    pub fn get_members_mut(&mut self) -> &mut Vec<NamedObject>{
        self.members.as_mut()
    }
}
