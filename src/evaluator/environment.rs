use crate::{
    evaluator::{
        EvalResult,
        object::*,
        builtins::init_builtins,
    },
    settings::USETTINGS,
    error::evaluator::{
        UError, UErrorKind, UErrorMessage, DefinitionType
    }
};

use std::{
    fmt,
    sync::{
        Arc,
        Mutex
    }
};
use std::borrow::Cow;

use regex::Regex;


#[derive(PartialEq, Clone, Debug)]
pub enum ContainerType {
    Variable,
    Public,
    Const,
    Function,
    Module,
    Class,
    Struct,
    BuiltinConst,
    BuiltinFunc,
}

impl fmt::Display for ContainerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ContainerType::Variable => write!(f,"Variable"),
            ContainerType::Public => write!(f,"Public"),
            ContainerType::Const => write!(f,"Const"),
            ContainerType::Function => write!(f,"Function"),
            ContainerType::Module => write!(f,"Module"),
            ContainerType::Class => write!(f,"Class"),
            ContainerType::Struct => write!(f,"Struct"),
            ContainerType::BuiltinConst => write!(f,"BuiltinConst"),
            ContainerType::BuiltinFunc => write!(f,"BuiltinFunc"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedObject {
    pub name: String,
    pub object: Object,
    pub container_type: ContainerType,
}

impl NamedObject {
    pub fn new(name: String, object: Object, container_type: ContainerType) -> Self {
        NamedObject {name, object, container_type}
    }
    pub fn new_builtin_const(name: String, object: Object) -> Self {
        NamedObject {name, object, container_type: ContainerType::BuiltinConst}
    }
    pub fn new_builtin_func(name: String, object: Object) -> Self {
        NamedObject {name, object, container_type: ContainerType::BuiltinFunc}
    }
}

impl fmt::Display for NamedObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}) {} = {}", self.container_type, self.name, self.object)
    }
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub local: Vec<NamedObject>,
    pub outer: Option<Arc<Mutex<Layer>>>,
}

impl Layer {
    pub fn clear(&mut self) {
        self.local.clear();
        if let Some(m) = &self.outer {
            let mut layer = m.lock().unwrap();
            layer.clear();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Environment {
    pub current: Arc<Mutex<Layer>>,
    pub global: Arc<Mutex<Vec<NamedObject>>>
}

impl Environment {
    pub fn clear(&mut self) {
        {
            let mut layer = self.current.lock().unwrap();
            layer.clear();
        }
        {
            let mut vec = self.global.lock().unwrap();
            vec.clear();
        }
    }

    pub fn new(params: Vec<String>) -> Self {
        let mut env = Environment {
            current: Arc::new(Mutex::new(Layer {
                local: Vec::new(),
                outer: None,
            })),
            global: Arc::new(Mutex::new(init_builtins()))
        };
        let param_str = params.iter().map(|s| Object::String(s.into())).collect::<Vec<Object>>();
        env.define("PARAM_STR".into(), Object::Array(param_str), ContainerType::Variable, false).unwrap();
        env.add(NamedObject::new(
            "TRY_ERRLINE".into(), Object::Empty, ContainerType::Variable
        ), false);
        env.add(NamedObject::new(
            "TRY_ERRMSG".into(), Object::Empty, ContainerType::Variable
        ), false);
        env.init_g_time_const(1899, 12, 30, 0, 0, 0, 0, 6);
        env
    }

    pub fn new_scope(&mut self) {
        let outer = Some(Arc::clone(&self.current));
        self.current = Arc::new(Mutex::new(Layer {
            local: Vec::new(),
            outer,
        }));
        self.add(NamedObject::new(
            "TRY_ERRLINE".into(), Object::Empty, ContainerType::Variable
        ), false);
        self.add(NamedObject::new(
            "TRY_ERRMSG".into(), Object::Empty, ContainerType::Variable
        ), false);
        self.init_g_time_const(1899, 12, 30, 0, 0, 0, 0, 6);
    }

    pub fn get_local_copy(&mut self) -> Vec<NamedObject> {
        self.current.lock().unwrap().local.clone()
    }

    pub fn copy_scope(&mut self, outer_local: Vec<NamedObject>) {
        let outer = Some(Arc::clone(&self.current));
        self.current = Arc::new(Mutex::new(Layer {
            local: outer_local,
            outer,
        }));
    }

    pub fn restore_scope(&mut self, anon_outer: &Option<Arc<Mutex<Vec<NamedObject>>>>) {
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
        let outer = {
            let mut layer = self.current.lock().unwrap();
            layer.local = vec![];
            layer.outer.clone().unwrap()
        };
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

    fn set(&mut self, name: &str, container_type: ContainerType, value: Object, to_global: bool) {
        let key = name.to_ascii_uppercase();
        if to_global {
            for obj in self.global.lock().unwrap().iter_mut() {
                if obj.name == key && obj.container_type == container_type {
                    if check_special_assignment(&obj.object, &value) {
                        obj.object = value;
                    }
                    break;
                }
            }
        } else {
            for obj in self.current.lock().unwrap().local.iter_mut() {
                if obj.name == key && obj.container_type == container_type {
                    if check_special_assignment(&obj.object, &value) {
                        obj.object = value;
                    }
                    break;
                }
            }
        }
    }

    fn get(&self, name: &str, container_type: ContainerType) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.current.lock().unwrap().local.iter().find(
            |o| o.name == key && o.container_type == container_type
        ).map(|o| o.object.clone())
    }

    fn get_from_global(&self, name: &str, container_type: ContainerType) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        self.global.lock().unwrap().iter().find(
            |o| o.name == key && o.container_type == container_type
        ).map(|o| o.object.clone())
    }

    pub fn get_name_of_builtin_consts(&self, name: &str) -> Object {
        let key = name.to_ascii_uppercase();
        self.global.lock().unwrap().iter()
        .find(|o| o.name == key && o.container_type == ContainerType::BuiltinConst)
        .map_or(Object::Empty, |o| Object::String(o.name.to_string()))
    }

    // 変数評価の際に呼ばれる
    pub fn get_variable(&self, name: &str, expand: bool) -> Option<Object> {
        let obj = match self.get(&name, ContainerType::Variable) {
            Some(value) => Some(value),
            None => match self.get(&name, ContainerType::Const) { // module関数から呼ばれた場合のみ
                Some(value) => Some(value),
                None => match self.get(&name, ContainerType::Public) { // module関数から呼ばれた場合のみ
                    Some(value) => Some(value),
                    None => match self.get_from_global(&name, ContainerType::Const) {
                        Some(value) => Some(value),
                        None => match self.get_from_global(&name, ContainerType::Public) {
                            Some(value) => Some(value),
                            None => match self.get_from_global(&name, ContainerType::BuiltinConst) {
                                Some(value) => Some(value),
                                None => match self.get_from_global(&name, ContainerType::Variable) { // インスタンス自動破棄
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
            Some(Object::Instance(ref ins)) => if ins.lock().unwrap().is_dropped {
                Some(Object::Nothing)
            } else {
                obj
            },
            o => o
        }
    }

    pub fn get_tmp_instance(&self, name: &str, from_global: bool) -> Option<Object> {
        if from_global {
            self.get_from_global(&name, ContainerType::Variable)
        } else {
            self.get(name, ContainerType::Variable)
        }
    }

    pub fn get_function(&self, name: &str) -> Option<Object> {
        match self.get(&name, ContainerType::Function) { // module関数から呼ばれた場合のみ
            Some(func) => Some(func),
            None =>  match self.get_from_global(&name, ContainerType::Function) {
                Some(func) => Some(func),
                None => match self.get_from_global(&name, ContainerType::BuiltinFunc) {
                    Some(func) => Some(func),
                    None => None
                }
            }
        }
    }

    // global.hoge
    pub fn get_global(&self, name: &str, is_func: bool) -> EvalResult<Object> {
        if is_func {
            match self.get_from_global(name, ContainerType::Function) {
                Some(o) => Ok(o),
                None => match self.get_from_global(name, ContainerType::BuiltinFunc) {
                    Some(o) => Ok(o),
                    None => Err(UError::new(
                        UErrorKind::EvaluatorError,
                        UErrorMessage::FunctionNotFound(name.to_string())
                    ))
                }
            }
        } else {
            match self.get_from_global(name, ContainerType::Public) {
                Some(o) => Ok(o),
                None => match self.get_from_global(name, ContainerType::Const) {
                    Some(o) => Ok(o),
                    None => match self.get_from_global(name, ContainerType::BuiltinConst) {
                        Some(o) => Ok(o),
                        None => Err(UError::new(
                            UErrorKind::EvaluatorError,
                            UErrorMessage::VariableNotFound(name.to_string())
                        ))
                    }
                }
            }
        }
    }

    pub fn get_module(&self, name: &str) -> Option<Object> {
        self.get_from_global(&name, ContainerType::Module)
    }

    pub fn get_class(&self, name: &str) -> Option<Object> {
        self.get_from_global(&name, ContainerType::Class)
    }

    pub fn get_struct(&self, name: &str) -> Option<Object> {
        self.get_from_global(&name, ContainerType::Struct)
    }

    // 予約語チェック
    fn is_reserved(&mut self, name: &str) -> bool {
        self.global.lock().unwrap().iter().any(|obj| obj.name == *name && obj.container_type == ContainerType::BuiltinConst) ||
        vec![
            "GLOBAL",
            "THIS",
            "TRY_ERRLINE",
            "TRY_ERRMSG"
        ].iter().any(|s| s.to_string() == *name)
    }

    fn contains_in_local(&mut self, name: &str, container_types: &[ContainerType]) -> bool {
        self.current.lock().unwrap().local.iter().any(|obj| obj.name == *name && container_types.contains(&obj.container_type))
    }
    fn contains_in_global(&mut self, name: &str, container_types: &[ContainerType]) -> bool {
        self.global.lock().unwrap().iter().any(|obj| obj.name == *name && container_types.contains(&obj.container_type))
    }

    fn define(&mut self, name: String, object: Object, container_type: ContainerType, to_global: bool) -> Result<(), UError> {
        if self.is_reserved(&name) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Variable),
                UErrorMessage::Reserved(name)
            ))
        }
        let obj = NamedObject {name, object, container_type};
        self.add(obj, to_global);
        Ok(())
    }

    pub fn define_local(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Variable, ContainerType::Const]) || self.contains_in_global(&key, &[ContainerType::Const]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Variable),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Variable, false)
    }

    pub fn _define_local_const(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Variable, ContainerType::Const]) || self.contains_in_global(&key, &[ContainerType::Const]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Const),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Const, false)
    }

    pub fn define_public(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Const]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Public),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Public, true)
    }

    pub fn define_const(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Public]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Const),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        } else if self.contains_in_global(&key, &[ContainerType::Const]) {
            // const定義済みで値が異なればエラー、同じなら何もしないでOk返す
            let const_value = self.get_from_global(&key, ContainerType::Const).unwrap_or(Object::Empty);

            if ! object.is_equal(&const_value) {
                return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Const),
                UErrorMessage::AlreadyDefined(name.into())
                ));
            }else {
                return Ok(());
            }
        }
        self.define(key, object, ContainerType::Const, true)
    }

    pub fn define_function(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Function]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Function),
                UErrorMessage::AlreadyDefined(name.into())
            ));
        }
        self.define(key, object, ContainerType::Function, true)
    }

    /// モジュール関数
    pub fn define_module_function(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Function]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Function),
                UErrorMessage::AlreadyDefined(name.into())
            ));
        }
        self.define(key, object, ContainerType::Function, false)
    }
    /// モジュールの定数
    pub fn define_module_const(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Variable,ContainerType::Const,ContainerType::Public]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Variable),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Const, false)
    }
    /// モジュールのパブリック変数
    pub fn define_module_public(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Variable, ContainerType::Const]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Variable),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Public, false)
    }
    /// モジュールの変数
    pub fn define_module_variable(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_local(&key, &[ContainerType::Variable,ContainerType::Const,ContainerType::Public]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Variable),
                UErrorMessage::AlreadyDefined(name.into())
            ))
        }
        self.define(key, object, ContainerType::Variable, false)
    }

    pub fn define_dll_function(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        self.define(key, object, ContainerType::Function, true)
    }

    pub fn define_module(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Module]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Module),
                UErrorMessage::AlreadyDefined(name.into())
            ));
        }
        self.define(key, object, ContainerType::Module, true)
    }

    pub fn define_class(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Class]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Class),
                UErrorMessage::AlreadyDefined(name.into())
            ));
        }
        self.define(key, object, ContainerType::Class, true)
    }

    pub fn define_struct(&mut self, name: &str, object: Object) -> Result<(), UError> {
        let key = name.to_ascii_uppercase();
        if self.contains_in_global(&key, &[ContainerType::Struct]) {
            return Err(UError::new(
                UErrorKind::DefinitionError(DefinitionType::Struct),
                UErrorMessage::AlreadyDefined(name.into())
            ));
        }
        self.define(key, object, ContainerType::Struct, true)
    }

    fn assignment(&mut self, name: String, value: Object, include_local: bool) -> EvalResult<bool> {
        let key = name.to_ascii_uppercase();
        let mut is_public = false;
        if self.is_reserved(&key) {
            // ビルトイン定数には代入できない
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::Reserved(key),
            ))
        }
        if self.contains_in_global(&key, &[ContainerType::Const]) {
            // 同名の定数がある場合エラー
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::ConstantCantBeAssigned(key)
            ))
        } else if self.contains_in_local(&key, &[ContainerType::Variable]) && include_local {
            // ローカル代入許可の場合のみ
            // 同名のローカル変数が存在する場合は値を上書き
            self.set(&key, ContainerType::Variable, value, false);
        } else if self.contains_in_global(&key, &[ContainerType::Public]) {
            // 同名のグローバル変数が存在する場合は値を上書き
            self.set(&key, ContainerType::Public, value, true);
            is_public = true;
        } else if include_local {
            // ローカル代入許可の場合のみ
            // 同名の変数が存在しない場合は新たなローカル変数を定義
            // Option Explicitの場合はエラーになる
            let usettings = USETTINGS.lock().unwrap();
            if usettings.options.explicit {
                return Err(UError::new(
                    UErrorKind::DefinitionError(DefinitionType::Variable),
                    UErrorMessage::ExplicitError(key)
                ));
            }

            self.define_local(&key, value)?;
        } else {
            // ローカル代入不許可
            // 同名のグローバル変数が存在しない場合はエラー
            return Err(UError::new(
                UErrorKind::AssignError,
                UErrorMessage::GlobalVariableNotFound(Some(key))
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
            container_type: ContainerType::Variable
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
            ContainerType::Variable
        ), false);
        self.add(NamedObject::new(
            "GLOBAL".into(),
            Object::Global,
            ContainerType::Variable
        ), false);
    }

    pub fn has_function(&mut self, name: &str) -> bool {
        let key = name.to_ascii_uppercase();
        self.contains_in_global(&key, &[ContainerType::Function])
    }

    // for builtin debug fungtions

    pub fn get_env(&self) -> Object {
        let mut arr = Vec::new();
        for obj in self.current.lock().unwrap().local.iter() {
            arr.push(Object::String(format!("current: {}", obj)));
        }
        for obj in self.global.lock().unwrap().iter() {
            if obj.container_type != ContainerType::BuiltinConst && obj.container_type != ContainerType::BuiltinFunc {
                arr.push(Object::String(format!("global: {}", obj)));
            }
        }
        Object::Array(arr)
    }

    pub fn get_module_member(&self, name: &str) -> Object {
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
                text =>  self.get_variable(text, false).map(|o| format!("{}", o).into()),
            };
            new_string = rep_to.map_or(new_string.clone(), |to| new_string.replace(format!("<#{}>", expandable).as_str(), to.as_ref()));
        }
        Object::String(new_string)
    }

    pub fn set_try_error_messages(&mut self, message: String, line: String) {
        self.set("TRY_ERRMSG", ContainerType::Variable, Object::String(message), false);
        self.set("TRY_ERRLINE", ContainerType::Variable, Object::String(line), false);
    }

    pub fn init_g_time_const(&mut self, year: i32, month: i32, date: i32, hour: i32, minute: i32, second: i32, millisec: i32, day: i32) {
        self.add(NamedObject::new("G_TIME_YY".into(), year.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_MM".into(), month.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_DD".into(), date.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_HH".into(), hour.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_NN".into(), minute.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_SS".into(), second.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_ZZ".into(), millisec.into(), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_WW".into(), day.into(), ContainerType::Const), false);
        let to_str_obj = |n: i32, len: usize| {
            let str = format!("{:0>1$}", n, len);
            str.into()
        };
        self.add(NamedObject::new("G_TIME_YY2".into(), to_str_obj(year%100, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_MM2".into(), to_str_obj(month, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_DD2".into(), to_str_obj(date, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_HH2".into(), to_str_obj(hour, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_NN2".into(), to_str_obj(minute, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_SS2".into(), to_str_obj(second, 2), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_ZZ2".into(), to_str_obj(millisec, 3), ContainerType::Const), false);
        self.add(NamedObject::new("G_TIME_YY4".into(), to_str_obj(year, 4), ContainerType::Const), false);
    }
    pub fn set_g_time_const(&mut self, year: i32, month: i32, date: i32, hour: i32, minute: i32, second: i32, millisec: i32, day: i32) {
        self.set("G_TIME_YY", ContainerType::Const, year.into(), false);
        self.set("G_TIME_MM", ContainerType::Const, month.into(), false);
        self.set("G_TIME_DD", ContainerType::Const, date.into(), false);
        self.set("G_TIME_HH", ContainerType::Const, hour.into(), false);
        self.set("G_TIME_NN", ContainerType::Const, minute.into(), false);
        self.set("G_TIME_SS", ContainerType::Const, second.into(), false);
        self.set("G_TIME_ZZ", ContainerType::Const, millisec.into(), false);
        self.set("G_TIME_WW", ContainerType::Const, day.into(), false);
        let to_str_obj = |n: i32, len: usize| {
            let str = format!("{:0>1$}", n, len);
            str.into()
        };
        self.set("G_TIME_YY2", ContainerType::Const, to_str_obj(year%100, 2), false);
        self.set("G_TIME_MM2", ContainerType::Const, to_str_obj(month, 2), false);
        self.set("G_TIME_DD2", ContainerType::Const, to_str_obj(date, 2), false);
        self.set("G_TIME_HH2", ContainerType::Const, to_str_obj(hour, 2), false);
        self.set("G_TIME_NN2", ContainerType::Const, to_str_obj(minute, 2), false);
        self.set("G_TIME_SS2", ContainerType::Const, to_str_obj(second, 2), false);
        self.set("G_TIME_ZZ2", ContainerType::Const, to_str_obj(millisec, 3), false);
        self.set("G_TIME_YY4", ContainerType::Const, to_str_obj(year, 4), false);
    }

    pub fn clone_outer(&self) -> Option<Arc<Mutex<Layer>>> {
        let current = self.current.lock().unwrap();
        current.outer.clone()
    }
    pub fn get_from_reference(&self, name: &str, outer: &Arc<Mutex<Layer>>) -> Option<Object> {
        let key = name.to_ascii_uppercase();
        let value = {
            let layer = outer.lock().unwrap();
            layer.local.iter()
                .find(|no| no.name == key)
                .map(|no| no.object.clone())
        };
        if value.is_none() {
            let global = self.global.lock().unwrap();
            global.iter()
                .find(|no| no.name == key)
                .map(|no| no.object.clone())
        } else {
            value
        }
    }

    pub fn set_get_func_name(&mut self, value: Option<String>) {
        let name = "GET_FUNC_NAME";
        if self.contains_in_local(name, &[ContainerType::Const]) {
            self.set(name, ContainerType::Const, value.into(), false);
        } else {
            self.add(NamedObject::new(name.into(), value.into(), ContainerType::Const), false);
        }
    }

}

// 特殊な代入に対する処理
// falseを返したら代入は行わない
pub fn check_special_assignment(obj1: &Object, obj2: &Object) -> bool {
    match obj1 {
        // HASH_REMOVEALL
        Object::HashTbl(h) => {
            if let Object::Num(n) = obj2 {
                let hash_remove_all = super::object::hashtbl::HashTblEnum::HASH_REMOVEALL as i32;
                if *n as i32 == hash_remove_all {
                    h.lock().unwrap().clear();
                    return false;
                }
            }
            true
        },
        Object::Instance(m) => {
            // クラスインスタンスにNothingが代入される場合はdisposeする
            if let Object::Nothing = obj2 {
                let mut ins = m.lock().unwrap();
                ins.dispose();
            }
            true
        },
        _ => true
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