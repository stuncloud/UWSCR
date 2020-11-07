use crate::evaluator::object::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Clone, Debug)]
pub struct Env {
    local: HashMap<String, Object>,
    constant: HashMap<String, Object>,
    public: HashMap<String, Object>,
    function: HashMap<String, Object>,
    module: HashMap<String, Object>,
    // outer: Option<Rc<RefCell<Env>>>,
}
impl Env {
    pub fn new() -> Self {
        Env {
            local: HashMap::new(),
            constant: HashMap::new(),
            public: HashMap::new(),
            function: HashMap::new(),
            module: HashMap::new(),
        }
    }

    pub fn from_builtin(function: HashMap<String, Object>, constant: HashMap<String, Object>) -> Self {
        Env {
            local: HashMap::new(),
            constant,
            public: HashMap::new(),
            function,
            module: HashMap::new(),
        }
    }

    pub fn new_scope(outer: Rc<RefCell<Env>>) -> Self {
        let outer_scope = outer.borrow_mut();
        Env {
            local: HashMap::new(),
            constant: outer_scope.constant.clone(),
            public: outer_scope.public.clone(),
            function: outer_scope.function.clone(),
            module: outer_scope.module.clone(),
        }
    }

    pub fn copy_scope(outer: Rc<RefCell<Env>>) -> Self {
        let outer_scope = outer.borrow_mut();
        Env {
            local: outer_scope.local.clone(),
            constant: outer_scope.constant.clone(),
            public: outer_scope.public.clone(),
            function: outer_scope.function.clone(),
            module: outer_scope.module.clone(),
        }
    }

    pub fn get_public_scope(&mut self) -> HashMap<String, Object> {
        self.public.clone()
    }

    pub fn set_public_scope(&mut self, public: HashMap<String, Object>) {
        self.public = public;
    }

    pub fn print_env(&mut self, member: String) {
        match member.to_ascii_lowercase().as_str() {
            "local" => Self::print_key_value(self.local.clone()),
            "constant" => Self::print_key_value(self.constant.clone()),
            "public" => Self::print_key_value(self.public.clone()),
            "function" => Self::print_key_value(self.function.clone()),
            _ => ()
        }
    }

    fn print_key_value(map: HashMap<String, Object>) {
        for (k, v) in map {
            match v {
                Object::BuiltinFunction(_, _) |
                Object::BuiltinConst(_) => (),
                _ => println!("{}: {}", k, v)
            }
        }
    }

    // dim定義
    // dim か const で既に定義してたらエラー
    pub fn define_local(&mut self, name: String, value: &Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if ! self.local.contains_key(&key) && ! self.constant.contains_key(&key) {
            self.local.insert(key, value.clone());
            return Ok(());
        }
        Err(Object::Error(format!("{} is already defined.", key)))
    }

    // public
    // const で既に定義してたらエラー
    pub fn define_public(&mut self, name: String, value: &Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if ! self.constant.contains_key(&key) {
            self.public.insert(key, value.clone());
            return Ok(());
        }
        Err(Object::Error(format!("{} is already defined.", key)))
    }

    // const
    // dim か public か constで既に定義してたらエラー
    pub fn define_const(&mut self, name: String, value: &Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if ! self.local.contains_key(&key) && ! self.public.contains_key(&key) && ! self.constant.contains_key(&key) {
            self.constant.insert(key, value.clone());
            return Ok(());
        }
        // 値が同じならOK返す
        if self.constant.contains_key(&key) {
            if self.constant.get(&key).unwrap().clone() == value.clone() {
                return Ok(());
            }
        }
        Err(Object::Error(format!("{} is already defined.", key)))
    }

    // function, procedure
    // function で既に定義してたらエラー
    pub fn define_function(&mut self, name: String, func: &Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if ! self.function.contains_key(&key) {
            self.function.insert(key, func.clone());
            return Ok(());
        }
        Err(Object::Error(format!("{} is already defined.", key)))
    }
    // module
    // module で既に定義してたらエラー
    pub fn define_module(&mut self, name: String, module: &Object) -> Result<(), Object> {
        let key = name.to_ascii_uppercase();
        if ! self.module.contains_key(&key) {
            self.module.insert(key, module.clone());
            return Ok(());
        }
        Err(Object::Error(format!("{} is already defined.", key)))
    }

    // 代入
    pub fn assign(&mut self, name: String, value: &Object) -> Result<(), Object>{
        let key = name.to_ascii_uppercase();
        if self.constant.contains_key(&key) {
            // 定数が存在する場合エラーを返す
            return Err(Object::Error(format!("you can not assign to constant: {}", key)));
        } if self.local.contains_key(&key) {
            // ローカル変数が存在する場合
            // 元の値がGlobalMemberならpublicを上書き
            // それ以外ならlocalを上書き
            match self.local.get(&key) {
                Some(Object::GlobalMember(s)) => {
                    self.public.insert(s.to_ascii_uppercase(), value.clone());
                },
                _ => {
                    self.local.insert(key, value.clone());
                }
            }
        } else if self.public.contains_key(&key) {
            // ローカル変数が存在せず、グローバル変数が存在する場合、グローバル変数を上書き
            self.public.insert(key, value.clone());
        } else {
            // いずれも存在しない場合新たなローカル変数をセット
            self.local.insert(key, value.clone());
        }
        Ok(())
    }

    // 関数呼び出し時に引数をセットする
    pub fn set_function_params(&mut self, name: String, value: &Object) {
        self.local.insert(name.to_ascii_uppercase(), value.clone());
    }

    // module関数呼び出し時にメンバをローカル変数としてセット
    pub fn set_module_private_member(&mut self, name: &String) {
        let map = match self.get_module(name) {
            Some(Object::Module(_, h)) => h,
            _ => return
        };
        for (k, v) in map {
            self.local.insert(k.to_ascii_uppercase(), v.clone());
        }
    }

    // 変数評価の際に呼ばれる
    // 参照順は local -> const -> public -> outer.const -> outer.public
    pub fn get_variable(&mut self, name: &String) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        match self.constant.get(&key) {
            Some(value) => Some(value.clone()),
            None => match self.local.get(&key) {
                Some(value) => Some(value.clone()),
                None => match self.public.get(&key) {
                    Some(value) => Some(value.clone()),
                    None => None
                }
            }
        }
    }

    // 関数呼び出しで呼ばれる
    pub fn get_func(&mut self, name: &String) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        match self.function.get(&key) {
            Some(func) => Some(func.clone()),
            None => None
        }
    }

    // .呼び出し時に呼ばれる
    pub fn get_module(&mut self, name: &String) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        match self.module.get(&key) {
            Some(module) => Some(module.clone()),
            None => None
        }
    }

    pub fn update_module(&mut self, name: &String, new_member: &Object) {
        let key = name.to_ascii_uppercase();
        self.module.insert(key, new_member.clone());
    }

    pub fn does_function_exists(&mut self, name: &String) -> bool {
        self.function.contains_key(&name.to_ascii_uppercase())
    }
}