use crate::evaluator::object::*;
use crate::evaluator::builtins::init_builtins;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;


#[derive(PartialEq, Clone, Debug)]
pub enum Scope {
    Local,
    Public,
    Const,
    Function,
    Module,
    Class,
    BuiltinConst,
    BuiltinFunc,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Scope::Local => write!(f,"Local"),
            Scope::Public => write!(f,"Public"),
            Scope::Const => write!(f,"Const"),
            Scope::Function => write!(f,"Function"),
            Scope::Module => write!(f,"Module"),
            Scope::Class => write!(f,"Class"),
            Scope::BuiltinConst => write!(f,"BuiltinConst"),
            Scope::BuiltinFunc => write!(f,"BuiltinFunc"),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct NamedObject {
    name: String,
    object: Object,
    scope: Scope,
}

impl NamedObject {
    pub fn new(name: String, object: Object, scope: Scope) -> Self {
        NamedObject {name, object, scope}
    }
    pub fn new_builtin_const(name: String, object: Object) -> Self {
        NamedObject {name, object, scope: Scope::BuiltinConst}
    }
    pub fn new_builtin_func(name: String, object: Object) -> Self {
        NamedObject {name, object, scope: Scope::BuiltinFunc}
    }
}

impl fmt::Display for NamedObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}) {} = {}", self.scope, self.name, self.object)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Layer {
    local: Vec<NamedObject>,
    outer: Option<Box<Layer>>,
    module_name: Option<String>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Environment {
    current: Layer,
    global: Vec<NamedObject>
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            current: Layer {
                local: Vec::new(),
                outer: None,
                module_name: None,
            },
            global: init_builtins()
        }
    }

    pub fn new_scope(&mut self, module_name: Option<String>) {
        let outer = Some(Box::new(self.current.clone()));
        self.current = Layer {
            local: Vec::new(),
            outer,
            module_name,
        }
    }

    pub fn get_local_copy(&mut self) -> Vec<NamedObject> {
        self.current.local.clone()
    }

    pub fn copy_scope(&mut self, outer_local: Vec<NamedObject>, module_name: Option<String>) {
        let outer = Some(Box::new(self.current.clone()));
        self.current = Layer {
            local: outer_local,
            outer,
            module_name,
        }
    }

    pub fn restore_scope(&mut self) {
        let outer = *self.current.outer.clone().unwrap();
        self.current = outer;
    }

    pub fn get_current_module_name(&self) -> Option<String> {
        self.current.module_name.clone()
    }

    fn add(&mut self, obj: NamedObject, to_global: bool) {
        if to_global {
            self.global.push(obj);
        } else {
            self.current.local.push(obj);
        }
    }

    fn set(&mut self, name: &String, scope: Scope, value: Object, to_global: bool) {
        let key = name.to_ascii_uppercase();
        if to_global {
            for obj in self.global.iter_mut() {
                if obj.name == key && obj.scope == scope {
                    obj.object = value;
                    break;
                }
            }
        } else {
            for obj in self.current.local.iter_mut() {
                if obj.name == key && obj.scope == scope {
                    obj.object = value;
                    break;
                }
            }
        }
    }

    fn get(&self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.current.local.clone().into_iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object)
    }

    fn get_from_global(&self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.global.clone().into_iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object.clone())
    }

    pub fn get_name_of_builtin_consts(&self, name: &String) -> Object {
        let key = name.to_ascii_uppercase();
        self.global.clone().into_iter()
        .find(|o| o.name == key && o.scope == Scope::BuiltinConst)
        .map_or(Object::Empty, |o| Object::String(o.name))
    }

    // 変数評価の際に呼ばれる
    pub fn get_variable(&self, name: &String) -> Option<Object> {
        match self.get(&name, Scope::Local) {
            Some(value) => Some(value),
            None => match self.get(&name, Scope::Const) { // module関数から呼ばれた場合のみ
                Some(value) => Some(value),
                None => match self.get(&name, Scope::Public) { // module関数から呼ばれた場合のみ
                    Some(value) => Some(value),
                    None => match self.get_from_global(&name, Scope::Const) {
                        Some(value) => Some(value),
                        None => match self.get_from_global(&name, Scope::Public) {
                            Some(value) => Some(value),
                            None => match self.get_from_global(&name, Scope::BuiltinConst) {
                                Some(value) => Some(value),
                                None => None
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_function(&self, name: &String) -> Option<Object> {
        match self.get(&name, Scope::Function) { // module関数から呼ばれた場合のみ
            Some(func) => Some(func),
            None =>  match self.get_from_global(&name, Scope::Function) {
                Some(func) => Some(func),
                None => match self.get_from_global(&name, Scope::BuiltinFunc) {
                    Some(func) => Some(func),
                    None => None
                }
            }
        }
    }

    // global.hoge
    pub fn get_global(&self, name: &String, is_func: bool) -> Object {
        if is_func {
            match self.get_from_global(name, Scope::Function) {
                Some(o) => o,
                None => match self.get_from_global(name, Scope::BuiltinFunc) {
                    Some(o) => o,
                    None => Object::Error(format!("global: function not found"))
                }
            }
        } else {
            match self.get_from_global(name, Scope::Public) {
                Some(o) => o,
                None => match self.get_from_global(name, Scope::Const) {
                    Some(o) => o,
                    None => match self.get_from_global(name, Scope::BuiltinConst) {
                        Some(o) => o,
                        None => Object::Error(format!("global: vaariable not found"))
                    }
                }
            }
        }
    }

    pub fn get_module(&self, name: &String) -> Option<Object> {
        self.get_from_global(&name, Scope::Module)
    }

    pub fn get_class(&self, name: &String) -> Option<Object> {
        self.get_from_global(&name, Scope::Class)
    }

    // 予約語チェック
    fn is_reserved(&mut self, name: &String) -> bool {
        self.global.clone().into_iter().any(|obj| obj.name == *name && obj.scope == Scope::BuiltinConst) ||
        vec!["GLOBAL","THIS"].iter().any(|s| s.to_string() == *name)
    }

    fn contains(&mut self, name: &String, scope: Scope) -> bool {
        let store = if scope == Scope::Local {
            self.current.local.clone()
        } else {
            self.global.clone()
        };
        store.into_iter().any(|obj| obj.name == *name && scope == obj.scope)
    }

    fn define(&mut self, name: String, object: Object, scope: Scope, to_global: bool) -> Result<(), Object> {
        if self.is_reserved(&name) {
            return Err(Object::Error(format!("{} is reserved identifier.", name)))
        }
        let obj = NamedObject {name, object, scope};
        self.add(obj, to_global);
        Ok(())
    }

    pub fn define_local(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Local) || self.contains(&key, Scope::Const) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        }
        self.define(key, object, Scope::Local, false)
    }

    pub fn define_public(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Const) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        }
        self.define(key, object, Scope::Public, true)
    }

    pub fn define_const(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Local) || self.contains(&key, Scope::Public) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        } else if self.contains(&key, Scope::Const) {
            // const定義済みで値が異なればエラー、同じなら何もしないでOk返す
            if self.get(&key, Scope::Const).unwrap_or(Object::Empty) != object {
                return Err(Object::Error(format!("{} is already defined.", key)))
            }else {
                return Ok(())
            }
        }
        self.define(key, object, Scope::Const, true)
    }

    pub fn define_function(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Function) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        }
        self.define(key, object, Scope::Function, true)
    }

    pub fn define_module(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Module) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        }
        self.define(key, object, Scope::Module, true)
    }

    pub fn define_class(&mut self, name: String, object: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Class) {
            return Err(Object::Error(format!("{} is already defined.", key)))
        }
        self.define(key, object, Scope::Class, true)
    }

    fn hash_remove_all(&mut self, name: &String) -> bool {
        if let Object::HashTbl(h) = self.get_variable(name).unwrap_or(Object::Empty) {
            h.borrow_mut().clear();
            return true;
        }
        false
    }

    pub fn assign(&mut self, name: String, value: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.is_reserved(&key) {
            // ビルトイン定数には代入できない
            return Err(Object::Error(format!("{} is reserved identifier.", key)))
        }
        // HASH_REMOVEALL
        if let Object::Num(n) = value {
            if n == -109.0 {
                if self.hash_remove_all(&key) {
                    return Ok(())
                }
            }
        }
        if self.contains(&key, Scope::Const) {
            // 同名の定数がある場合エラー
            return Err(Object::Error(format!("you can not assign to constant: {}", key)));
        } else if self.contains(&key, Scope::Local) {
            // 同名のローカル変数が存在する場合は値を上書き
            self.set(&key, Scope::Local, value, false);
        } else if self.contains(&key, Scope::Public) {
            // 同名のグローバル変数が存在する場合は値を上書き
            self.set(&key, Scope::Public, value, true);
        } else {
            // 同名の変数が存在しない場合は新たなローカル変数を定義
            // Option Explicitの場合は無効 (未実装)
            return self.define_local(key, value);
        }
        Ok(())
    }

    pub fn assign_public(&mut self, name: String, value: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.is_reserved(&key) {
            // ビルトイン定数には代入できない
            return Err(Object::Error(format!("{} is reserved identifier.", key)))
        }
        // HASH_REMOVEALL
        if let Object::Num(n) = value {
            if n == -109.0 {
                if self.hash_remove_all(&key) {
                    return Ok(())
                }
            }
        }
        if self.contains(&key, Scope::Const) {
            // 同名の定数がある場合エラー
            return Err(Object::Error(format!("you can not assign to constant: {}", key)));
        } else if self.contains(&key, Scope::Public) {
            // 同名のグローバル変数が存在する場合は値を上書き
            self.set(&key, Scope::Public, value, true);
        } else {
            // 同名のグローバル変数が存在しない場合はエラー
            return Err(Object::Error(format!("public variable not found: {}", key)));
        }
        Ok(())
    }

    pub fn set_func_params_to_local(&mut self, name: String, value: &Object) {
        let key = name.to_ascii_uppercase();
        self.add(NamedObject {
            name: key,
            object: value.clone(),
            scope: Scope::Local
        }, false)
    }

    // module関数呼び出し時にメンバをローカル変数としてセット
    pub fn set_module_private_member(&mut self, module: &Rc<RefCell<Module>>) {
        let vec = module.borrow().get_members();
        for obj in vec {
            self.add(obj, false)
        }
        // thisとglobalも定義
        self.add(NamedObject::new(
            "THIS".into(),
            Object::This(Rc::clone(module)),
            Scope::Local
        ), false);
        self.add(NamedObject::new(
            "GLOBAL".into(),
            Object::Global,
            Scope::Local
        ), false);
    }

    pub fn has_function(&mut self, name: &String) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, Scope::Function)
    }

    // for builtin debug fungtions

    pub fn get_env(&self) -> Object {
        let mut arr = Vec::new();
        for obj in self.current.local.clone().into_iter() {
            arr.push(Object::String(format!("current: {}", obj)));
        }
        for obj in self.global.clone().into_iter() {
            if obj.scope != Scope::BuiltinConst && obj.scope != Scope::BuiltinFunc {
                arr.push(Object::String(format!("global: {}", obj)));
            }
        }
        Object::Array(arr)
    }

    pub fn get_module_member(&self, name: &String) -> Object {
        let mut arr = Vec::new();
        match self.get_module(name) {
            Some(o) => match o {
                Object::Module(m) => {
                    let module = m.borrow();
                    for obj in module.get_members().into_iter() {
                        arr.push(Object::String(format!("{}: {}", module.name(), obj)))
                    }
                },
                _ => ()
            },
            None => ()
        }
        Object::Array(arr)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Module {
    name: String,
    members: Vec<NamedObject>
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

    pub fn has_constructor(&self) -> bool {
        let name = self.name().to_ascii_uppercase();
        self.contains(&name, Scope::Function)
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
                obj.object = value;
                break;
            }
        }
    }

    pub fn get_member(&self, name: &String) -> Object {
        match self.get(name, Scope::Local) {
            Some(o) => o,
            None => match self.get(name, Scope::Public) {
                Some(o) => o,
                None => match self.get(name, Scope::Const) {
                    Some(o) => o,
                    None => Object::Error(format!("{}.{} is not defined", self.name, name))
                }
            }
        }
    }

    pub fn get_public_member(&self, name: &String) -> Object {
        match self.get(name, Scope::Public) {
            Some(o) => o,
            None => match self.get(name, Scope::Const) {
                Some(o) => o,
                None => Object::Error(format!("{}.{} is not defined", self.name, name))
            }
        }
    }

    pub fn get_function(&self, name: &String) -> Object {
        self.get(name, Scope::Function).unwrap_or(Object::Error(format!("{}.{}() is not defined", self.name, name)))
    }

    pub fn assign(&mut self, name: &String, value: Object) -> Result<(), Object> {
        if self.contains(name, Scope::Const) {
            // 同名の定数がある場合はエラー
            return Err(Object::Error(format!("you can not assign to constant: {}.{}", self.name(), name)));
        } else if self.contains(name, Scope::Local) {
            // 同名ローカル変数があれば上書き
            self.set(name, value, Scope::Local)
        } else if self.contains(name, Scope::Public) {
            // 同名パブリック変数があれば上書き
            self.set(name, value, Scope::Public)
        }
        Ok(())
    }

    pub fn assign_public(&mut self, name: &String, value: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Public) {
            self.set(&name, value, Scope::Public);
            Ok(())
        } else {
            Err(Object::Error(format!("{}.{} is not defined or not public", self.name, name)))
        }
    }

    pub fn is_local_member(&self, name: &String) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, Scope::Local)
    }

    pub fn set_rc_to_functions(&mut self, rc: Rc<RefCell<Module>>) {
        for  o in self.members.iter_mut() {
            if o.scope == Scope::Function {
                if let Object::Function(n, p, b, i, _) = o.object.clone() {
                    o.object = Object::Function(n, p, b, i, Some(Rc::clone(&rc)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluator::environment::*;

    fn _env_test() {

    }

    #[test]
    fn test_define_local() {
        let mut env = Environment::new();
        assert_eq!(
            env.define_local("hoge".into(),Object::Num(1.1)),
            Ok(())
        )
    }
}