use crate::evaluator::object::*;
use crate::evaluator::builtins::init_builtins;
use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub enum Scope {
    Local,
    Public,
    Const,
    Function,
    Module,
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
                outer: None
            },
            global: init_builtins()
        }
    }

    pub fn new_scope(&mut self) {
        let backup = Some(Box::new(self.current.clone()));
        self.current = Layer {
            local: Vec::new(),
            outer: backup
        }
    }

    pub fn get_local_copy(&mut self) -> Vec<NamedObject> {
        self.current.local.clone()
    }

    pub fn copy_scope(&mut self, outer_local: Vec<NamedObject>) {
        let backup = Some(Box::new(self.current.clone()));
        self.current = Layer {
            local: outer_local,
            outer: backup
        }
    }

    pub fn restore_scope(&mut self) {
        let backup = *self.current.outer.clone().unwrap();
        self.current = backup;
    }

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

    fn get(&mut self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.current.local.clone().into_iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object)
    }

    fn get_from_global(&mut self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.global.clone().into_iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object.clone())
    }

    // 変数評価の際に呼ばれる
    // 参照順は local -> const -> public -> builtin
    pub fn get_variable(&mut self, name: &String) -> Option<Object> {
        match self.get(&name, Scope::Local) {
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

    pub fn get_function(&mut self, name: &String) -> Option<Object> {
        match self.get_from_global(&name, Scope::Function) {
            Some(func) => Some(func),
            None => match self.get_from_global(&name, Scope::BuiltinFunc) {
                Some(func) => Some(func),
                None => None
            }
        }
    }

    pub fn get_module(&mut self, name: &String) -> Option<Object> {
        match self.get_from_global(&name, Scope::Module) {
            Some(module) => Some(module),
            None => None
        }
    }

    fn is_reserved(&mut self, name: &String) -> bool {
        self.global.clone().into_iter().any(|obj| obj.name == *name && obj.scope == Scope::BuiltinConst)
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

    pub fn assign(&mut self, name: String, value: Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if self.is_reserved(&key) {
            // ビルトイン定数には代入できない
            return Err(Object::Error(format!("{} is reserved identifier.", key)))
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

    pub fn set_func_params_to_local(&mut self, name: String, value: &Object) {
        let key = name.to_ascii_uppercase();
        self.add(NamedObject {
            name: key,
            object: value.clone(),
            scope: Scope::Local
        }, false)
    }

    // module関数呼び出し時にメンバをローカル変数としてセット
    pub fn set_module_private_member(&mut self, name: &String) {
        let map = match self.get_module(name) {
            Some(Object::Module(_, h)) => h,
            _ => return
        };
        for (k, v) in map {
            self.add(NamedObject {
                name: k.clone(),
                object: v.clone(),
                scope: Scope::Local
            }, false)
        }
    }

    pub fn has_function(&mut self, name: &String) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, Scope::Function)
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