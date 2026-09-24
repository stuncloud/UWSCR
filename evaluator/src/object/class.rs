use super::{Object, Module, function::This};
use super::super::{Evaluator, ContainerType};

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ClassInstance {
    name: String,
    module: Module,
    evaluator: Evaluator,
    // /// trueならNOTHINGのフリをする
    // is_dropped: bool,
}

impl PartialEq for ClassInstance {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
        && self.module == other.module
    }
}

impl Drop for ClassInstance {
    fn drop(&mut self) {
        if self.module.is_last_copy() {
            self.dispose();
        }
    }
}

impl ClassInstance {
    pub fn new(name: String, module: Module, evaluator: Evaluator) -> Self {
        let ins = Self {
            name,
            module,
            evaluator,
        };
        // // thisを追加
        // ins.module.add("this".into(), Object::Instance(ins.clone()), ContainerType::Variable);
        // 無名関数のスコープ情報を消す
        ins.module.remove_outer_from_private_func();
        ins
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn dropped(&self) -> bool {
        self.inner().is_disposed()
    }
    pub fn inner(&self) -> &Module {
        &self.module
    }
    pub fn dispose(&self) {
        if ! self.inner().is_disposed() {
            let destructor = self.module.get_destructor();
            if let Some(f) = destructor {
                // dispose時はしょうがないのでthisを自身のmoduleにする
                let this = Some(This::Module(self.module.clone()));
                let mut evaluator = self.evaluator.clone();
                let _ = f.invoke(&mut evaluator, vec![], this);
            }
            self.module.dispose();
        }
    }
    // pub fn get_destructor(&self) -> impl FnOnce(Arc<Mutex<Self>>) {
    //     let evaluator = self.evaluator.clone();
    //     let destructor = {
    //         if let Some(Object::Function(destructor)) = self.module.get_destructor() {
    //             Some(destructor)
    //         } else {
    //             None
    //         }
    //     };
    //     move |ins: Arc<Mutex<Self>>| {
    //         let mut evaluator = evaluator;
    //         if let Some(f) = destructor {
    //             let this = Some(This::Class(ins));
    //             let _ = f.invoke(&mut evaluator, vec![], this);
    //         }
    //     }
    // }
    // pub fn dispose2(&mut self) {
    //     if ! self.is_dropped {
    //         self.is_dropped = true;
    //         self.module.dispose();
    //     }
    // }
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