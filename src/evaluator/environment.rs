use crate::{
    evaluator::{
        object::*,
        builtins::init_builtins,
    },
    settings::usettings_singleton,
};

use std::{
    fmt,
    sync::{
        Arc,
        Mutex
    }
};
use std::borrow::Cow;

use super::{EvalResult, UError};
use regex::Regex;


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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Layer {
    pub local: Vec<NamedObject>,
    pub outer: Option<Arc<Mutex<Layer>>>,
}

#[derive(Clone, Debug)]
pub struct Environment {
    pub current: Arc<Mutex<Layer>>,
    pub global: Arc<Mutex<Vec<NamedObject>>>
}

impl Environment {
    pub fn new(params: Vec<String>) -> Self {
        let mut env = Environment {
            current: Arc::new(Mutex::new(Layer {
                local: Vec::new(),
                outer: None,
            })),
            global: Arc::new(Mutex::new(init_builtins()))
        };
        let param_str = params.iter().map(|s| Object::String(s.into())).collect::<Vec<Object>>();
        env.define("PARAM_STR".into(), Object::Array(param_str), Scope::Local, false).unwrap();
        env.add(NamedObject::new(
            "TRY_ERRLINE".into(), Object::Empty, Scope::Local
        ), false);
        env.add(NamedObject::new(
            "TRY_ERRMSG".into(), Object::Empty, Scope::Local
        ), false);
        env
    }

    pub fn new_scope(&mut self) {
        let outer = Some(Arc::clone(&self.current));
        self.current = Arc::new(Mutex::new(Layer {
            local: Vec::new(),
            outer,
        }));
        self.add(NamedObject::new(
            "TRY_ERRLINE".into(), Object::Empty, Scope::Local
        ), false);
        self.add(NamedObject::new(
            "TRY_ERRMSG".into(), Object::Empty, Scope::Local
        ), false);
    }

    pub fn get_local_copy(&mut self) -> Vec<NamedObject> {
        self.current.lock().unwrap().local.clone()
    }

    pub fn copy_scope(&mut self, outer_local: Vec<NamedObject>) {
        let outer = Some(Arc::clone(&self.current));
        let mut current = self.current.lock().unwrap();
        *current = Layer {
            local: outer_local,
            outer,
        }
    }

    pub fn restore_scope(&mut self, anon_outer: Option<Arc<Mutex<Vec<NamedObject>>>>) {
        match anon_outer {
            // 無名関数が保持する値を更新する
            Some(r) => {
                let mut anon_outer = r.lock().unwrap();
                for anon_obj in anon_outer.iter_mut() {
                    for local_obj in self.current.lock().unwrap().local.iter() {
                        if local_obj.name == anon_obj.name {
                            anon_obj.object = local_obj.object.clone();
                            break;
                        }
                    }
                }
            },
            None => {}
        }
        // let outer = *self.current.outer.clone().unwrap();
        let outer = self.current.lock().unwrap().outer.clone().unwrap();
        self.current = outer;
    }

    fn add(&mut self, obj: NamedObject, to_global: bool) {
        if to_global {
            self.global.lock().unwrap().push(obj);
        } else {
            self.current.lock().unwrap().local.push(obj);
        }
    }

    pub fn remove_variable(&mut self, name: String) {
        self.current.lock().unwrap().local.retain(|o| o.name != name.to_ascii_uppercase());
    }

    fn set(&mut self, name: &String, scope: Scope, value: Object, to_global: bool) {
        let key = name.to_ascii_uppercase();
        if to_global {
            for obj in self.global.lock().unwrap().iter_mut() {
                if obj.name == key && obj.scope == scope {
                    if check_special_assignment(&obj.object, &value) {
                        obj.object = value;
                    }
                    break;
                }
            }
        } else {
            for obj in self.current.lock().unwrap().local.iter_mut() {
                if obj.name == key && obj.scope == scope {
                    if check_special_assignment(&obj.object, &value) {
                        obj.object = value;
                    }
                    break;
                }
            }
        }
    }

    fn get(&self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.current.lock().unwrap().local.iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object.clone())
    }

    fn get_from_global(&self, name: &String, scope: Scope) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.global.lock().unwrap().iter().find(
            |o| o.name == key && o.scope == scope
        ).map(|o| o.object.clone())
    }

    pub fn get_name_of_builtin_consts(&self, name: &String) -> Object {
        let key = name.to_ascii_uppercase();
        self.global.lock().unwrap().iter()
        .find(|o| o.name == key && o.scope == Scope::BuiltinConst)
        .map_or(Object::Empty, |o| Object::String(o.name.to_string()))
    }

    // 変数評価の際に呼ばれる
    pub fn get_variable(&self, name: &String, expand: bool) -> Option<Object> {
        let obj = match self.get(&name, Scope::Local) {
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
                                None => match self.get_from_global(&name, Scope::Local) { // インスタンス自動破棄
                                    Some(value) => Some(value),
                                    None => None
                                }
                            }
                        }
                    }
                }
            }
        };
        match obj {
            Some(Object::DynamicVar(f)) => Some(f()),
            Some(Object::ExpandableTB(text)) => if expand {
                Some(self.expand_string(text))
            } else {
                Some(Object::String(text))
            },
            Some(Object::Instance(ref ins,_)) => if ins.lock().unwrap().is_disposed() {
                Some(Object::Nothing)
            } else {
                obj
            },
            o => o
        }
    }

    pub fn get_tmp_instance(&self, name: &String, from_global: bool) -> Option<Object> {
        if from_global {
            self.get_from_global(&name, Scope::Local)
        } else {
            self.get(name, Scope::Local)
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
    pub fn get_global(&self, name: &String, is_func: bool) -> EvalResult<Object> {
        if is_func {
            match self.get_from_global(name, Scope::Function) {
                Some(o) => Ok(o),
                None => match self.get_from_global(name, Scope::BuiltinFunc) {
                    Some(o) => Ok(o),
                    None => Err(UError::new(
                        "Global".into(),
                        "function not found".into(),
                        None
                    ))
                }
            }
        } else {
            match self.get_from_global(name, Scope::Public) {
                Some(o) => Ok(o),
                None => match self.get_from_global(name, Scope::Const) {
                    Some(o) => Ok(o),
                    None => match self.get_from_global(name, Scope::BuiltinConst) {
                        Some(o) => Ok(o),
                        None => Err(UError::new(
                            "Global".into(),
                            "vaariable not found".into(),
                            None
                        ))
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
        self.global.lock().unwrap().iter().any(|obj| obj.name == *name && obj.scope == Scope::BuiltinConst) ||
        vec![
            "GLOBAL",
            "THIS",
            "TRY_ERRLINE",
            "TRY_ERRMSG"
        ].iter().any(|s| s.to_string() == *name)
    }

    fn contains(&mut self, name: &String, scope: Scope) -> bool {
        if scope == Scope::Local {
            self.current.lock().unwrap().local.iter().any(|obj| obj.name == *name && scope == obj.scope)
        } else {
            self.global.lock().unwrap().iter().any(|obj| obj.name == *name && scope == obj.scope)
        }
    }

    fn define(&mut self, name: String, object: Object, scope: Scope, to_global: bool) -> Result<(), UError> {
        if self.is_reserved(&name) {
            return Err(UError::new(
                "Error on definition",
                &format!("{} is reserved identifier.", name),
                None
            ))
        }
        let obj = NamedObject {name, object, scope};
        self.add(obj, to_global);
        Ok(())
    }

    pub fn define_local(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Local) || self.contains(&key, Scope::Const) {
            return Err(UError::new(
                "Error on definition",
                &format!("{} is already defined.", key),
                None
            ))
        }
        self.define(key, object, Scope::Local, false)
    }

    pub fn define_public(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Const) {
            return Err(UError::new(
                "Error on definition",
                &format!("{} is already defined.", key),
                None
            ))
        }
        self.define(key, object, Scope::Public, true)
    }

    pub fn define_const(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Local) || self.contains(&key, Scope::Public) {
            return Err(UError::new(
                "Error on definition",
                &format!("{} is already defined.", key),
                None
            ))
        } else if self.contains(&key, Scope::Const) {
            // const定義済みで値が異なればエラー、同じなら何もしないでOk返す
            let const_value = self.get_from_global(&key, Scope::Const).unwrap_or(Object::Empty);
            println!("[debug] value: {} new_value: {}", &const_value, &object);

            if ! object.is_equal(&const_value) {
                return Err(UError::new(
                "Error on definition",
                &format!("{} is already defined.", key),
                None
                ));
            }else {
                return Ok(());
            }
        }
        self.define(key, object, Scope::Const, true)
    }

    pub fn define_function(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Function) {
            return Err(UError::new(
                "Function defining error",
                &format!("{} is already defined.", key),
                None
            ));
        }
        self.define(key, object, Scope::Function, true)
    }

    pub fn define_module(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Module) {
            return Err(UError::new(
                "Module defining error",
                &format!("{} is already defined.", key),
                None
            ));
        }
        self.define(key, object, Scope::Module, true)
    }

    pub fn define_class(&mut self, name: String, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains(&key, Scope::Class) {
            return Err(UError::new(
                "Class defining error",
                &format!("{} is already defined.", key),
                None
            ));
        }
        self.define(key, object, Scope::Class, true)
    }

    fn assignment(&mut self, name: String, value: Object, include_local: bool) -> EvalResult<bool> {
        let key = name.to_ascii_uppercase();
        let mut is_public = false;
        if self.is_reserved(&key) {
            // ビルトイン定数には代入できない
            return Err(UError::new(
                "Assignment Error",
                &format!("{} is reserved identifier.", key),
                None
            ))
        }
        if self.contains(&key, Scope::Const) {
            // 同名の定数がある場合エラー
            return Err(UError::new(
                "Assignment Error",
                &format!("you can not assign to constant: {}", key),
                None
            ))
        } else if self.contains(&key, Scope::Local) && include_local {
            // ローカル代入許可の場合のみ
            // 同名のローカル変数が存在する場合は値を上書き
            self.set(&key, Scope::Local, value, false);
        } else if self.contains(&key, Scope::Public) {
            // 同名のグローバル変数が存在する場合は値を上書き
            self.set(&key, Scope::Public, value, true);
            is_public = true;
        } else if include_local {
            // ローカル代入許可の場合のみ
            // 同名の変数が存在しない場合は新たなローカル変数を定義
            // Option Explicitの場合はエラーになる
            let singleton = usettings_singleton(None);
            let usettings = singleton.0.lock().unwrap();
            if usettings.options.explicit {
                return Err(UError::new(
                    "Explicit error",
                    &format!("dim is required for {}", key),
                    None
                ));
            }

            self.define_local(key, value)?;
        } else {
            // ローカル代入不許可
            // 同名のグローバル変数が存在しない場合はエラー
            return Err(UError::new(
                "Assignment Error",
                &format!("public variable not found: {}", key),
                None
            ))
        };
        Ok(is_public)
    }

    pub fn assign(&mut self, name: String, value: Object) -> EvalResult<bool> {
        self.assignment(name, value, true)
    }

    pub fn assign_public(&mut self, name: String, value: Object) -> EvalResult<bool> {
        self.assignment(name, value, false)
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
    pub fn set_module_private_member(&mut self, module: &Arc<Mutex<Module>>) {
        let vec = module.lock().unwrap().get_members();
        for obj in vec {
            self.add(obj, false)
        }
        // thisとglobalも定義
        self.add(NamedObject::new(
            "THIS".into(),
            Object::This(Arc::clone(module)),
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
        for obj in self.current.lock().unwrap().local.iter() {
            arr.push(Object::String(format!("current: {}", obj)));
        }
        for obj in self.global.lock().unwrap().iter() {
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
                    let module = m.lock().unwrap();
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

    fn expand_string(&self, string: String) -> Object {
        let re = Regex::new("<#([^>]+)>").unwrap();
        let mut new_string = string.clone();
        for cap in re.captures_iter(string.as_str()) {
            let expandable = cap.get(1).unwrap().as_str();
            let rep_to: Option<Cow<str>> = match expandable.to_ascii_uppercase().as_str() {
                "CR" => Some("\r\n".into()),
                "TAB" => Some("\t".into()),
                "DBL" => Some("\"".into()),
                text =>  self.get_variable(&text.into(), false).map(|o| format!("{}", o).into()),
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    pub fn set_instances(&mut self, instance: Arc<Mutex<Module>>, id: u32, to_global: bool) {
        let var_name = "@INSTANCES".to_string();
        let ins_name = format!("@INSTANCE{}", id);
        let obj = if to_global {
            self.get_from_global(&var_name, Scope::Local).unwrap_or(Object::Empty)
        } else {
            self.get(&var_name, Scope::Local).unwrap_or(Object::Empty)
        };
        if let Object::Instances(mut v) = obj {
            v.push(ins_name);
            self.set(&var_name, Scope::Local, Object::Instances(v), to_global);
        } else {
            let v = vec![ins_name];
            self.add(NamedObject::new(var_name, Object::Instances(v), Scope::Local), to_global);
        };
        self.add(NamedObject::new(format!("@INSTANCE{}", id), Object::Instance(Arc::clone(&instance), id), Scope::Local), to_global);
    }

    pub fn remove_from_instances(&mut self, id: u32) {
        let name = "@INSTANCES".to_string();
        let ins_name = format!("@INSTANCE{}", id);
        let obj = self.get(&name, Scope::Local).unwrap_or(Object::Empty);
        if let Object::Instances(mut v) = obj {
            v.retain(|n| n != &ins_name);
            self.set(&name, Scope::Local, Object::Instances(v), false);
        }
    }

    pub fn get_instances(&mut self) -> Vec<String> {
        let name = "@INSTANCES".to_string();
        if let Object::Instances(v) = self.get(&name, Scope::Local).unwrap_or(Object::Empty) {
            v
        } else {
            vec![]
        }
    }

    pub fn get_global_instances(&mut self) -> Vec<String> {
        let name = "@INSTANCES".to_string();
        if let Object::Instances(v) = self.get_from_global(&name, Scope::Local).unwrap_or(Object::Empty) {
            v
        } else {
            vec![]
        }
    }

    pub fn set_try_error_messages(&mut self, message: String, line: String) {
        self.set(&"TRY_ERRMSG".into(), Scope::Local, Object::String(message), false);
        self.set(&"TRY_ERRLINE".into(), Scope::Local, Object::String(line), false);
    }
}

// 特殊な代入に対する処理
// falseを返したら代入は行わない
fn check_special_assignment(obj1: &Object, obj2: &Object) -> bool {
    match obj1 {
        // HASH_REMOVEALL
        Object::HashTbl(h) => {
            if let Object::Num(n) = obj2 {
                if n == &109.0 {
                    h.lock().unwrap().clear();
                }
            }
            false
        },
        _ => true
    }
}

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

    pub fn get_constructor(&self) -> Option<Object> {
        self.get(&self.name, Scope::Function)
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
                        "Member not found",
                        &format!("{}.{} is not defined", self.name, name),
                        None
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
                    "Public member not found",
                    &format!("{}.{}() is not defined", self.name, name),
                    None
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
                    "Function not found",
                    &format!("{}.{}() is not defined", self.name, name),
                    None
                );
                match self.get_public_member(name) {
                    Ok(o) => match o {
                        Object::AnonFunc(_,_,_,_) |
                        Object::Function(_,_,_,_,_) |
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
                            "Invalid Index",
                            &format!("index out of bound: {}", i),
                            None
                        ))
                    }
                } else {
                    return Err(UError::new(
                        "Invalid Index",
                        &format!("{} is not a valid index", index),
                        None
                    ))
                }
            },
            Object::HashTbl(h) => {
                let key = match index {
                    Object::Num(n) => n.to_string(),
                    Object::Bool(b) => b.to_string(),
                    Object::String(s) => s,
                    _ => return Err(UError::new(
                        "Invalid key",
                        &format!("{} is not a valid key", index),
                        None
                    ))
                };
                h.lock().unwrap().insert(key, value);
            },
            _ => return Err(UError::new(
                "Invalid index call",
                &format!("{} is neither array nor hashtbl", name),
                None
            ))
        }
        Ok(())
    }

    pub fn assign(&mut self, name: &String, value: Object, index: Option<Object>) -> Result<(), UError> {
        let scope = if self.contains(name, Scope::Const) {
            // 同名の定数がある場合はエラー
            return Err(UError::new(
                "Member already exists",
                &format!("you can not assign to constant: {}.{}", self.name(), name),
                None
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
                "Public member not found",
                &format!("{}.{} is not public", self.name, name),
                None
            ))
        }
        Ok(())
    }

    pub fn is_local_member(&self, name: &String) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains(&key, Scope::Local)
    }

    pub fn set_module_reference_to_member_functions(&mut self, m: Arc<Mutex<Module>>) {
        for  o in self.members.iter_mut() {
            if o.scope == Scope::Function {
                if let Object::Function(n, p, b, i, _) = o.object.clone() {
                    o.object = Object::Function(n, p, b, i, Some(Arc::clone(&m)))
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

#[cfg(test)]
mod tests {
    use crate::evaluator::environment::*;

    fn _env_test() {

    }

    #[test]
    fn test_define_local() {
        let mut env = Environment::new(vec![]);
        assert_eq!(
            env.define_local("hoge".into(),Object::Num(1.1)),
            Ok(())
        )
    }
}