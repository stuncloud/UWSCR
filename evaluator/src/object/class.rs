use super::{Object, Module, function::This};
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
        let ins = Self {
            name,
            module,
            evaluator,
            is_dropped: false,
        };
        {
            // 無名関数のスコープ情報を消す
            let mut guard = ins.module.lock().unwrap();
            for named_obj in guard.get_members_mut() {
                if let Object::AnonFunc(f) = named_obj.object.as_mut() {
                    f.outer = None;
                }
            }
        }
        ins
    }
    pub fn dispose(&mut self) {
        if ! self.is_dropped {
            self.is_dropped = true;
            let destructor = {
                let module = self.module.try_lock().expect("lock error: ClassInstance::dispose 1");
                if let Some(Object::Function(destructor)) = module.get_destructor() {
                    Some(destructor)
                } else {
                    None
                }
            };
            if let Some(f) = destructor {
                // dispose時はしょうがないのでthisを自身のmoduleにする
                let this = Some(This::Module(self.module.clone()));
                let _ = f.invoke(&mut self.evaluator, vec![], this);
            }
            self.module.try_lock().expect("lock error: ClassInstance::dispose 2").dispose();
        }
    }
    pub fn get_destructor(&self) -> impl FnOnce(Arc<Mutex<Self>>) {
        let evaluator = self.evaluator.clone();
        let destructor = {
            let module = self.module.try_lock().expect("lock error: ClassInstance::get_destructor");
            if let Some(Object::Function(destructor)) = module.get_destructor() {
                Some(destructor)
            } else {
                None
            }
        };
        move |ins: Arc<Mutex<Self>>| {
            let mut evaluator = evaluator;
            if let Some(f) = destructor {
                let this = Some(This::Class(ins));
                let _ = f.invoke(&mut evaluator, vec![], this);
            }
        }
    }
    pub fn dispose2(&mut self) {
        if ! self.is_dropped {
            self.is_dropped = true;
            self.module.try_lock().expect("lock error: ClassInstance::dispose2").dispose();
        }
    }
    // pub fn set_instance_reference(&mut self, ins: Arc<Mutex<Self>>) {
        // let mut mutex = self.module.lock().unwrap();
        // for o in mutex.get_members_mut() {
        //     match o.object.as_mut() {
        //         Object::Function(f) => {
        //             f.set_instance(ins.clone());
        //         }
        //         Object::AnonFunc(f) => {
        //             f.set_instance(ins.clone());
        //             // 無名関数ならスコープ情報を消す
        //             f.outer = None;
        //         },
        //         _ => {},
        //     }
        // }
    // }
}

impl std::fmt::Display for ClassInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}