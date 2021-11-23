pub mod window_control;
pub mod window_low;
pub mod text_control;
pub mod system_controls;
pub mod math;
pub mod key_codes;
pub mod com_object;
pub mod browser_control;
pub mod array_control;
pub mod chkimg;

use crate::settings::usettings_singleton;
use crate::winapi::{
    get_windows_directory,
    get_system_directory,
    get_special_directory,
    get_screen_width,
    get_screen_height,
    get_color_depth,
};
use windows::Win32::UI::Shell::CSIDL_APPDATA;
use crate::evaluator::object::{Object, Version, HashTblEnum, SpecialFuncResultType, UTask};
use crate::evaluator::environment::NamedObject;
use crate::error::evaluator::{UError,UErrorKind,UErrorMessage};
use crate::ast::Expression;

use std::env;
use std::sync::{Arc, Mutex};

use cast;
use serde_json::Value;
use strum::VariantNames;
use num_traits::ToPrimitive;
use strum_macros::{ToString, EnumVariantNames};

pub type BuiltinFunction = fn(BuiltinFuncArgs) -> BuiltinFuncResult;
pub type BuiltinFuncResult = Result<Object, UError>;
pub type BuiltInResult<T> = Result<T, UError>;

// pub struct BuiltinFuncError {
//     kind: UErrorKind,
//     message: UErrorMessage
// }
// impl BuiltinFuncError {
//     pub fn new(kind: UErrorKind, message: UErrorMessage) -> Self {
//         Self {kind, message}
//     }
// }

// impl From<BuiltinFuncError> for UError {
//     fn from(e: BuiltinFuncError) -> Self {
//         UError::new(e.kind, e.message)
//     }
// }

#[derive(Debug, Clone)]
pub struct BuiltinFuncArgs {
    func_name: String,
    // arg_exprs: Vec<Option<Expression>>,
    // args: Vec<Object>,
    arguments: Vec<(Option<Expression>, Object)>,
}

impl BuiltinFuncArgs {
    pub fn new(func_name: String, arguments: Vec<(Option<Expression>, Object)>) -> Self {
        // let mut arg_exprs = Vec::new();
        // let mut args = Vec::new();
        // for (e, o) in arguments {
        //     arg_exprs.push(e);
        //     args.push(o);
        // }
        BuiltinFuncArgs {
            // func_name, arg_exprs, args
            func_name,
            arguments
        }
    }
    pub fn item(&self, i: usize) -> Option<Object> {
        self.arguments.get(i).map(|o| o.1.clone())
    }
    pub fn len(&self) -> usize {
        self.arguments.len()
    }
    pub fn name(&self) -> String {
        self.func_name.clone()
    }
    pub fn get_expr(&self, i: usize) -> Option<Expression> {
        self.arguments.get(i).map_or(None,|e| e.0.clone())
    }
    pub fn get_objects_from(&mut self, index: usize) -> Vec<Object> {
        let rest = self.arguments.split_off(index);
        rest.into_iter().map(|a| a.1.clone()).collect()
    }
    pub fn get_args_from(&mut self, index: usize) -> Vec<(Option<Expression>, Object)> {
        let rest = self.arguments.split_off(index);
        rest.into_iter().map(|a| a.clone()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionSet {
    name: String,
    len: i32,
    func: BuiltinFunction
}

impl BuiltinFunctionSet {
    pub fn new<S:Into<String>>(name: S, len: i32, func: BuiltinFunction) -> Self {
        BuiltinFunctionSet {name: name.into(), len, func}
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionSets {
    sets: Vec<BuiltinFunctionSet>
}

impl BuiltinFunctionSets {
    pub fn new() -> Self {
        BuiltinFunctionSets{sets: vec![]}
    }
    pub fn add(&mut self, name: &str, len: i32, func: BuiltinFunction) {
        self.sets.push(
            BuiltinFunctionSet::new(name, len, func)
        );
    }
    pub fn set(&self, vec: &mut Vec<NamedObject>) {
        for set in self.sets.clone() {
            let name = set.name.to_ascii_uppercase();
            vec.push(
                NamedObject::new_builtin_func(
                    name.clone(),
                    Object::BuiltinFunction(name, set.len, set.func)
                )
            )
        }
    }
}

pub fn init_builtins() -> Vec<NamedObject> {
    let mut vec = Vec::new();
    // builtin debug functions
    builtin_func_sets().set(&mut vec);
    set_builtin_str_consts::<VariableType>(&mut vec, "", "");
    // hashtbl
    set_builtin_consts::<HashTblEnum>(&mut vec);
    // window_low
    window_low::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<window_low::MouseButtonEnum>(&mut vec);
    set_builtin_consts::<window_low::KeyActionEnum>(&mut vec);
    // window_control
    window_control::builtin_func_sets().set(&mut vec);
    set_builtin_str_consts::<window_control::SpecialWindowId>(&mut vec, "__", "__");
    set_builtin_consts::<window_control::CtrlWinCmd>(&mut vec);
    set_builtin_consts::<window_control::StatusEnum>(&mut vec);
    set_builtin_consts::<window_control::MonitorEnum>(&mut vec);
    set_builtin_consts::<window_control::MonitorEnumAlias>(&mut vec);
    // text control
    text_control::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<text_control::RegexEnum>(&mut vec);
    // system_constrol
    system_controls::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<system_controls::OsKind>(&mut vec);
    set_builtin_consts::<system_controls::KindOfOsResultType>(&mut vec);
    // math
    math::builtin_func_sets().set(&mut vec);
    // key codes
    set_builtin_consts::<key_codes::VirtualKeyCodes>(&mut vec);
    set_builtin_consts::<key_codes::VirtualKeyCodeDups>(&mut vec);
    set_builtin_consts::<key_codes::VirtualMouseButton>(&mut vec);
    // com_object
    com_object::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<com_object::VarType>(&mut vec);
    // browser_control
    browser_control::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<browser_control::BcEnum>(&mut vec);
    // array_control
    array_control::builtin_func_sets().set(&mut vec);
    // 特殊変数
    set_special_variables(&mut vec);

    vec
}

pub fn set_builtin_consts<E: std::str::FromStr + VariantNames + ToPrimitive>(vec: &mut Vec<NamedObject>) {
    for name in E::VARIANTS {
        let value = E::from_str(name).ok().unwrap();
        vec.push(NamedObject::new_builtin_const(
            name.to_ascii_uppercase(),
            Object::Num(ToPrimitive::to_f64(&value).unwrap())
        ));
    }
}

pub fn set_builtin_str_consts<E: VariantNames>(vec: &mut Vec<NamedObject>, prefix: &str, suffix: &str) {
    for name in E::VARIANTS {
        let ucase_name = name.to_ascii_uppercase();
        vec.push(NamedObject::new_builtin_const(ucase_name.clone(), Object::String(format!("{}{}{}", prefix, ucase_name, suffix))));
    }
}

fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("eval", 1, builtin_eval);
    sets.add("list_env", 0, list_env);
    sets.add("list_module_member", 1, list_module_member);
    sets.add("name_of", 1, name_of);
    sets.add("assert_equal", 2, assert_equal);
    sets.add("raise", 2, raise);
    sets.add("type_of", 2, type_of);
    sets.add("get_settings", 0, get_settings);
    sets
}

fn set_special_variables(vec: &mut Vec<NamedObject>) {
    // 特殊変数
    vec.push(NamedObject::new_builtin_const("GET_UWSC_PRO".into(), Object::Bool(false)));
    vec.push(NamedObject::new_builtin_const("GET_UWSC_VER".into(), Object::Version(
        env!("CARGO_PKG_VERSION").parse::<Version>().unwrap_or(Version::new(0,0,0))
    )));
    vec.push(NamedObject::new_builtin_const("GET_UWSCR_VER".into(), Object::Version(
        env!("CARGO_PKG_VERSION").parse::<Version>().unwrap_or(Version::new(0,0,0))
    )));
    vec.push(NamedObject::new_builtin_const("GET_UWSC_DIR".into(),
        env::var("GET_UWSC_DIR").map_or(Object::Empty, |path| Object::String(path))
    ));
    vec.push(NamedObject::new_builtin_const("GET_UWSCR_DIR".into(),
        env::var("GET_UWSC_DIR").map_or(Object::Empty, |path| Object::String(path))
    ));
    vec.push(NamedObject::new_builtin_const("GET_UWSC_NAME".into(),
        env::var("GET_UWSC_NAME").map_or(Object::Empty, |path| Object::String(path))
    ));
    vec.push(NamedObject::new_builtin_const("GET_UWSCR_NAME".into(),
        env::var("GET_UWSC_NAME").map_or(Object::Empty, |path| Object::String(path))
    ));
    vec.push(NamedObject::new_builtin_const("GET_WIN_DIR".into(), Object::String(
        get_windows_directory()
    )));
    vec.push(NamedObject::new_builtin_const("GET_SYS_DIR".into(), Object::String(
        get_system_directory()
    )));
    vec.push(NamedObject::new_builtin_const("GET_APPDATA_DIR".into(), Object::String(
        get_special_directory(CSIDL_APPDATA as i32)
    )));

    vec.push(NamedObject::new_builtin_const("GET_CUR_DIR".into(), Object::DynamicVar(
        || Object::String(
            match env::current_dir() {
                Ok(p) => p.into_os_string().into_string().unwrap(),
                Err(_) => "".into()
            }
        )
    )));
    vec.push(NamedObject::new_builtin_const("G_MOUSE_X".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos("G_MOUSE_X".into()) {
                Ok(p) => p.x as f64,
                Err(_) => -999999.0
            }
        )
    )));
    vec.push(NamedObject::new_builtin_const("G_MOUSE_Y".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos("G_MOUSE_Y".into()) {
                Ok(p) => p.y as f64,
                Err(_) => -999999.0
            }
        )
    )));
    vec.push(NamedObject::new_builtin_const("G_SCREEN_W".into(), Object::DynamicVar(
        || Object::Num(get_screen_width() as f64)
    )));
    vec.push(NamedObject::new_builtin_const("G_SCREEN_H".into(), Object::DynamicVar(
        || Object::Num(get_screen_height() as f64)
    )));
    vec.push(NamedObject::new_builtin_const("G_SCREEN_C".into(), Object::DynamicVar(
        || Object::Num(get_color_depth() as f64)
    )));
}

// 特殊ビルトイン関数の実体

pub fn builtin_eval(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let s = get_string_argument_value(&args, 0, None)?;
    Ok(Object::Eval(s))
}

pub fn list_env(_args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::SpecialFuncResult(SpecialFuncResultType::GetEnv))
}

pub fn list_module_member(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let s = get_string_argument_value(&args, 0, None)?;
    Ok(Object::SpecialFuncResult(SpecialFuncResultType::ListModuleMember(s)))
}

pub fn name_of(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::SpecialFuncResult(SpecialFuncResultType::BuiltinConstName(args.get_expr(0))))
}

pub fn get_settings(_args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let singleton = usettings_singleton(None);
    let s = singleton.0.lock().unwrap();
    let json = s.get_current_settings_as_json();
    Ok(Object::String(json))
}

pub fn raise(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = get_string_argument_value(&args, 0, None)?;
    let title = get_string_argument_value(&args, 1, Some(String::new()))?;
    let kind = if title.len() > 0 {
        UErrorKind::Any(title)
    } else {
        UErrorKind::UserDefinedError
    };
    Err(UError::new(kind, UErrorMessage::Any(msg)))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumVariantNames, ToString)]
pub enum VariableType {
    TYPE_NUMBER,
    TYPE_STRING,
    TYPE_BOOL,
    TYPE_ARRAY,
    TYPE_HASHTBL,
    TYPE_ANONYMOUS_FUNCTION,
    TYPE_FUNCTION,
    TYPE_BUILTIN_FUNCTION,
    TYPE_MODULE,
    TYPE_CLASS,
    TYPE_CLASS_INSTANCE,
    TYPE_NULL,
    TYPE_EMPTY,
    TYPE_NOTHING,
    TYPE_HWND,
    TYPE_REGEX,
    TYPE_UOBJECT,
    TYPE_VERSION,
    TYPE_THIS,
    TYPE_GLOBAL,
    TYPE_ENUM,
    TYPE_TASK,
    TYPE_DLL_FUNCTION,
    TYPE_STRUCT,
    TYPE_STRUCT_INSTANCE,
    TYPE_COM_OBJECT,
    TYPE_VARIANT,
    TYPE_OTHER,
}

pub fn type_of(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg = get_any_argument_value(&args, 0, None)?;
    let t = match arg {
        Object::Num(_) => VariableType::TYPE_NUMBER,
        Object::String(_) => VariableType::TYPE_STRING,
        Object::Bool(_) => VariableType::TYPE_BOOL,
        Object::Array(_) => VariableType::TYPE_ARRAY,
        Object::HashTbl(_) => VariableType::TYPE_HASHTBL,
        Object::AnonFunc(_,_,_,_) => VariableType::TYPE_ANONYMOUS_FUNCTION,
        Object::Function(_,_,_,_,_) => VariableType::TYPE_FUNCTION,
        Object::BuiltinFunction(_,_,_) => VariableType::TYPE_BUILTIN_FUNCTION,
        Object::Module(_) => VariableType::TYPE_MODULE,
        Object::Class(_,_) => VariableType::TYPE_CLASS,
        Object::Instance(ref m,_) => {
            let ins = m.lock().unwrap();
            if ins.is_disposed() {
                VariableType::TYPE_NOTHING
            } else {
                VariableType::TYPE_CLASS_INSTANCE
            }
        },
        Object::Null => VariableType::TYPE_NULL,
        Object::Empty => VariableType::TYPE_EMPTY,
        Object::Nothing => VariableType::TYPE_NOTHING,
        Object::Handle(_) => VariableType::TYPE_HWND,
        Object::RegEx(_) => VariableType::TYPE_REGEX,
        Object::This(_) => VariableType::TYPE_THIS,
        Object::Global => VariableType::TYPE_GLOBAL,
        Object::UObject(_) |
        Object::UChild(_, _) => VariableType::TYPE_UOBJECT,
        Object::Version(_) => VariableType::TYPE_VERSION,
        Object::ExpandableTB(_) => VariableType::TYPE_STRING,
        Object::Enum(_) => VariableType::TYPE_ENUM,
        Object::Task(_) => VariableType::TYPE_TASK,
        Object::DefDllFunction(_,_,_,_) => VariableType::TYPE_DLL_FUNCTION,
        Object::Struct(_,_,_) => VariableType::TYPE_STRUCT,
        Object::UStruct(_,_,_) => VariableType::TYPE_STRUCT_INSTANCE,
        Object::ComObject(_) => VariableType::TYPE_COM_OBJECT,
        Object::Variant(_) => VariableType::TYPE_VARIANT,
        _ => VariableType::TYPE_OTHER
    };
    Ok(Object::String(t.to_string()))
}

pub fn assert_equal(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg1 = get_any_argument_value(&args,0, None)?;
    let arg2 = get_any_argument_value(&args,1, None)?;
    if arg1.is_equal(&arg2) {
        Ok(Object::Empty)
    } else {
        Err(UError::new(
            UErrorKind::AssertEqError,
            UErrorMessage::AssertEqLeftAndRight(arg1, arg2)
        ))
    }
}

// エラー出力用関数

pub fn builtin_func_error(msg: UErrorMessage, name: String) -> UError {
    UError::new(UErrorKind::BuiltinFunctionError(name), msg)
}

// ビルトイン関数の引数を受け取るための関数群
// i: 引数のインデックス
// default: 省略可能な引数のデフォルト値、必須引数ならNoneを渡す
// 引数が省略されていた場合はdefaultの値を返す
// 引数が必須なのになかったらエラーを返す

pub fn get_any_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<Object>) -> BuiltInResult<Object> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        Ok(arg)
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_non_float_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> BuiltInResult<T>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(n) => T::cast(n).or(Err(builtin_func_error(
                UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                args.name()
            ))),
            Object::String(ref s) => match s.parse::<f64>() {
                Ok(n) => T::cast(n).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                    args.name()
                ))),
                Err(_) => Err(builtin_func_error(
                    UErrorMessage::BuiltinArgInvalid(arg),
                    args.name())
                )
            },
            Object::EmptyParam => {
                default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
            },
            _ => Err(builtin_func_error(
                UErrorMessage::BuiltinArgInvalid(arg),
                args.name())
            )
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

// 数値またはNoneを返す省略時に可変なデフォルト値を取る引数に使う
pub fn get_int_or_empty_argument(args: &BuiltinFuncArgs, i: usize, default: Option<Option<i32>>) -> BuiltInResult<Option<i32>> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(n) => Ok(Some(n as i32)),
            Object::String(ref s) => match s.parse::<f64>() {
                Ok(n) => Ok(Some(n as i32)),
                Err(_) => Err(builtin_func_error(
                    UErrorMessage::BuiltinArgInvalid(arg), args.name())
                )
            },
            Object::Empty |
            Object::EmptyParam => Ok(None),
            _ => Err(builtin_func_error(
                UErrorMessage::BuiltinArgInvalid(arg), args.name())
            )
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_num_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> BuiltInResult<T>
    where T: cast::From<f64, Output=T>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(n) => Ok(T::cast(n)),
            Object::String(ref s) => match s.parse::<f64>() {
                Ok(n) => Ok(T::cast(n)),
                Err(_) => Err(builtin_func_error(
                    UErrorMessage::BuiltinArgInvalid(arg), args.name())
                )
            },
            _ => Err(builtin_func_error(
                UErrorMessage::BuiltinArgInvalid(arg), args.name())
            )
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_string_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<String>) -> BuiltInResult<String> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match &arg {
            Object::String(s) => Ok(s.clone()),
            Object::RegEx(re) => Ok(re.clone()),
            o => Ok(format!("{}", o)),
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_string_or_empty_argument(args: &BuiltinFuncArgs, i: usize, default: Option<Option<String>>) -> BuiltInResult<Option<String>> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match &arg {
            Object::String(s) => Ok(Some(s.to_string())),
            Object::Empty |
            Object::EmptyParam => Ok(None),
            o => Ok(Some(format!("{}", o))),
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_bool_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<bool>) -> BuiltInResult<bool> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        Ok(arg.is_truthy())
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_bool_or_int_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> BuiltInResult<T>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        let type_name = std::any::type_name::<T>().to_string();
        match arg {
            Object::Bool(b) => T::cast(b as i32 as f64).or(Err(builtin_func_error(
                UErrorMessage::BuiltinArgCastError(arg, type_name),
                args.name()
            ))),
            Object::Num(n) => T::cast(n).or(Err(builtin_func_error(
                UErrorMessage::BuiltinArgCastError(arg, type_name),
                args.name())
            )),
            _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), args.name()))
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_uobject_argument_value(args: &BuiltinFuncArgs, i: usize) -> BuiltInResult<(Arc<Mutex<Value>>, Option<String>)> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::UObject(ref u) => Ok((Arc::clone(u), None)),
            Object::UChild(ref u, p) => Ok((Arc::clone(u), Some(p))),
            _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), args.name()))
        }
    } else {
        Err(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_array_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<Vec<Object>>) -> BuiltInResult<Vec<Object>> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Array(arr) => Ok(arr),
            _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), args.name()))
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_num_or_array_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<Object>) -> BuiltInResult<Object> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(_) |
            Object::Array(_) => Ok(arg),
            _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), args.name()))
        }
    } else {
        default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}

pub fn get_task_argument_value(args: &BuiltinFuncArgs, i: usize) -> BuiltInResult<UTask> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Task(utask) => Ok(utask),
            _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), args.name()))
        }
    } else {
        Err(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), args.name()))
    }
}