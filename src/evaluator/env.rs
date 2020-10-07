use crate::evaluator::object::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Clone, Debug)]
pub struct Env {
    store: HashMap<String, Object>,
    global: HashMap<String, Object>,
    outer: Option<Rc<RefCell<Env>>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            store: HashMap::new(),
            global: HashMap::new(),
            outer: None,
        }
    }

    pub fn from(store: HashMap<String, Object>) -> Self {
        Env {
            store,
            global: HashMap::new(),
            outer: None
        }
    }

    pub fn new_with_outer(outer: Rc<RefCell<Env>>) -> Self {
        Env {
            store: HashMap::new(),
            global: HashMap::new(),
            outer: Some(outer)
        }
    }

    pub fn get(&mut self, name: String) -> Option<Object> {
        match self.store.get(&name.to_ascii_uppercase()) {
            Some(value) => Some(value.clone()),
            None => match self.outer {
                Some(ref outer) => outer.borrow_mut().get(name),
                None => match self.global.get(&name) {
                    Some(value) => Some(value.clone()),
                    None => None
                }
            }
        }
    }

    pub fn set(&mut self, name: String, value: &Object) {
        self.store.insert(name.to_ascii_uppercase(), value.clone());
    }

    pub fn set_global(&mut self, name: String, value: &Object) {
        self.global.insert(name.to_ascii_uppercase(), value.clone());
    }
}