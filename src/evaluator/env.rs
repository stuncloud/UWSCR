use crate::evaluator::object::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Clone, Debug)]
pub struct Env {
    store: HashMap<String, Object>,
    outer: Option<Rc<RefCell<Env>>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            store: HashMap::new(),
            outer: None,
        }
    }

    pub fn from(store: HashMap<String, Object>) -> Self {
        Env {
            store,
            outer: None
        }
    }

    pub fn new_with_outer(outer: Rc<RefCell<Env>>) -> Self {
        Env {
            store: HashMap::new(),
            outer: Some(outer)
        }
    }

    pub fn is_defined(&mut self, name: &String) -> bool {
        self.store.contains_key(&name.to_ascii_uppercase())
    }

    pub fn is_same_type(&mut self, name: String, object: Object) -> bool {
        match self.get(name) {
            Some(o) => o == object,
            None => false
        }
    }

    pub fn get(&mut self, name: String) -> Option<Object> {
        match self.store.get(&name.to_ascii_uppercase()) {
            Some(value) => Some(value.clone()),
            None => match self.outer {
                Some(ref outer) => outer.borrow_mut().get(name),
                None => None
            }
        }
    }

    pub fn set(&mut self, name: String, value: &Object) {
        self.store.insert(name.to_ascii_uppercase(), value.clone());
    }
}