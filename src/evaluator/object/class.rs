use super::{Object, Module};
use super::super::Evaluator;

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub name: String,
    pub module: Arc<Mutex<Module>>,
    evaluator: Evaluator,
    /// trueならNOTHINGのフリをする
    pub is_dropped: bool,
}

impl Drop for ClassInstance {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl ClassInstance {
    pub fn new(name: String, module: Arc<Mutex<Module>>, evaluator: Evaluator) -> Self {
        Self {
            name,
            module,
            evaluator,
            is_dropped: false,
        }
    }
    pub fn dispose(&mut self) {
        self.is_dropped = true;
        let destructor = {
            let module = self.module.lock().unwrap();
            if let Some(Object::Function(destructor)) = module.get_destructor() {
                Some(destructor)
            } else {
                None
            }
        };
        if let Some(f) = destructor {
            let _ = f.invoke(&mut self.evaluator, vec![], false);
        }
        self.module.lock().unwrap().dispose();
    }
}

impl std::fmt::Display for ClassInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}