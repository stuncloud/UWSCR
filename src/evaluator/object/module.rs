use super::{Object, Function};
use crate::evaluator::{EvalResult};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage, DefinitionType};
use crate::evaluator::environment::{
    NamedObject, Scope,
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
        match self.get(&self.name, Scope::Function)? {
            Object::Function(f) |
            Object::AnonFunc(f) => {
                Some(f)
            },
            _ => None
        }
    }

    pub fn has_destructor(&self) -> bool {
        let name = format!("_{}_", self.name());
        self.contains(&name, Scope::Function)
    }

    pub fn is_destructor_name(&self, name: &String) -> bool {
        name.to_string() == format!("_{}_", self.name())
    }

    pub fn get_destructor(&self) -> Option<Object> {
        let name = format!("_{}_", self.name());
        self.get(&name, Scope::Function)
    }

    pub fn add(&mut self, name: String, object: Object, scope: Scope) {
        self.members.push(NamedObject::new(name.to_ascii_uppercase(), object, scope))
    }

    fn contains(&self, name: &String, scope: Scope) -> bool {
        let key = name.to_ascii_uppercase();
        self.members.clone().into_iter().any(|obj| obj.name == key && scope == obj.scope)
    }

    fn get(&self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.members.clone().into_iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object)
    }

    fn set(&mut self, name: &String, value: Object, scope: Scope) {
        let key = name.to_ascii_uppercase();
        for obj in self.members.iter_mut() {
            if obj.name == key && obj.scope == scope {
                if check_special_assignment(&obj.object, &value) {
                    obj.object = value;
                }
                break;
            }
        }
    }

    pub fn get_member(&self, name: &String) -> EvalResult<Object> {
        match self.get(name, Scope::Local) {
            Some(o) => Ok(o),
            None => match self.get(name, Scope::Public) {
                Some(o) => Ok(o),
                None => match self.get(name, Scope::Const) {
                    Some(o) => Ok(o),
                    None => Err(UError::new(
                        UErrorKind::ModuleError,
                        UErrorMessage::ModuleMemberNotFound(DefinitionType::Any, self.name.to_string(), name.to_string())
                    ))
                }
            }
        }
    }

    pub fn get_public_member(&self, name: &String) -> EvalResult<Object> {
        match self.get(name, Scope::Public) {
            Some(o) => Ok(o),
            None => match self.get(name, Scope::Const) {
                Some(o) => Ok(o),
                None => Err(UError::new(
                    UErrorKind::ModuleError,
                    UErrorMessage::ModuleMemberNotFound(DefinitionType::Public, self.name.to_string(), name.to_string())
                ))
            }
        }
    }

    pub fn get_function(&self, name: &String) -> EvalResult<Object> {
        match self.get(name, Scope::Function) {
            Some(o) => Ok(o),
            None => if self.is_destructor_name(name) && ! self.has_destructor() {
                Ok(Object::DestructorNotFound)
            } else {
                let e = UError::new(
                    UErrorKind::ModuleError,
                    UErrorMessage::ModuleMemberNotFound(DefinitionType::Function, self.name.to_string(), name.to_string())
                );
                match self.get_public_member(name) {
                    Ok(o) => match o {
                        Object::AnonFunc(_) |
                        Object::Function(_) |
                        Object::BuiltinFunction(_,_,_) => Ok(o),
                        _ => Err(e)
                    },
                    Err(_) => Err(e)
                }
            },
        }
    }

    fn assign_index(&mut self, name: &String, value: Object, index: Object, scope: Scope) -> Result<(), UError> {
        match self.get_member(name)? {
            Object::Array(mut a) => {
                if let Object::Num(n) = index {
                    let i = n as usize;
                    if i < a.len() {
                        a[i] = value;
                        self.set(name, Object::Array(a), scope);
                    } else {
                        return Err(UError::new(
                            UErrorKind::ArrayError,
                            UErrorMessage::IndexOutOfBounds(index)
                        ))
                    }
                } else {
                    return Err(UError::new(
                        UErrorKind::ArrayError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            Object::HashTbl(h) => {
                let key = match index {
                    Object::Num(n) => n.to_string(),
                    Object::Bool(b) => b.to_string(),
                    Object::String(s) => s,
                    _ => return Err(UError::new(
                        UErrorKind::HashtblError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                };
                h.lock().unwrap().insert(key, value);
            },
            Object::ByteArray(mut arr) => {
                if let Object::Num(i) = index {
                    if let Some(val) = arr.get_mut(i as usize) {
                        if let Object::Num(n) = value {
                            if let Ok(new_val) = u8::try_from(n as i64) {
                                *val = new_val;
                            } else {
                                return Err(UError::new(
                                    UErrorKind::AssignError,
                                    UErrorMessage::NotAnByte(value)
                                ));
                            }
                            self.set(name, Object::ByteArray(arr), scope);
                        } else {
                            return Err(UError::new(
                                UErrorKind::AssignError,
                                UErrorMessage::NotAnByte(value)
                            ));
                        }
                    } else {
                        return Err(UError::new(
                            UErrorKind::AssignError,
                            UErrorMessage::InvalidIndex(index)
                        ))
                    }
                } else {
                    return Err(UError::new(
                        UErrorKind::AssignError,
                        UErrorMessage::InvalidIndex(index)
                    ))
                }
            },
            o => return Err(UError::new(
                UErrorKind::ArrayError,
                UErrorMessage::NotAnArray(o)
            ))
        }
        Ok(())
    }

    pub fn assign(&mut self, name: &String, value: Object, index: Option<Object>) -> Result<(), UError> {
        let scope = if self.contains(name, Scope::Const) {
            // 同名の定数がある場合はエラー
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::ConstantCantBeAssigned(name.to_string())
            ))
        } else if self.contains(name, Scope::Local) {
            // 同名ローカル変数があれば上書き
            Scope::Local
        } else if self.contains(name, Scope::Public) {
            // 同名パブリック変数があれば上書き
            Scope::Public
        } else {
            return Ok(());
        };
        match index {
            Some(i) => {
                return self.assign_index(name, value, i, scope)
            },
            None => self.set(name, value, scope)
        }
        Ok(())
    }

    pub fn assign_public(&mut self, name: &String, value: Object, index: Option<Object>) -> Result<(), UError> {
        if self.contains(&name, Scope::Public) {
            match index {
                Some(i) => {
                    return self.assign_index(name, value, i, Scope::Public)
                },
                None => self.set(name, value, Scope::Public)
            }
        } else {
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::ModuleMemberNotFound(DefinitionType::Public, self.name(),name.to_string())
            ))
        }
        Ok(())
    }

    pub fn is_local_member(&self, name: &String) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, Scope::Local)
    }

    pub fn set_module_reference_to_member_functions(&mut self, m: Arc<Mutex<Module>>) {
        for o in self.members.iter_mut() {
            if o.scope == Scope::Function {
                if let Object::Function(mut f) = o.object.clone() {
                    f.set_module(Arc::clone(&m));
                    o.object = Object::Function(f);
                }
            }
        }
    }

    pub fn is_disposed(&self) -> bool {
        self.members.len() == 0
    }

    pub fn dispose(&mut self) {
        self.members = vec![];
    }
}
