pub mod window_control;
pub mod window_low;
pub mod text_control;
pub mod system_controls;
pub mod math;
pub mod key_codes;

use crate::evaluator::UError;
use crate::winapi_util::{
    get_windows_directory,
    get_system_directory,
    get_special_directory,
    get_screen_width,
    get_screen_height,
    get_color_depth,
};
use crate::evaluator::object::{Object, Version, HashTblEnum, SpecialFuncResultType};
use crate::evaluator::environment::NamedObject;
use crate::ast::Expression;

use std::env;

use cast;
use strum::VariantNames;
use num_traits::ToPrimitive;

pub type BuiltinFunction = fn(BuiltinFuncArgs) -> BuiltinFuncResult;
pub type BuiltinFuncResult = Result<Object, UError>;

#[derive(PartialEq, Debug, Clone)]
pub struct BuiltinFuncArgs {
    func_name: String,
    arg_exprs: Vec<Option<Expression>>,
    args: Vec<Object>,
}

impl BuiltinFuncArgs {
    pub fn new(func_name: String, arguments: Vec<(Option<Expression>, Object)>) -> Self {
        let mut arg_exprs = Vec::new();
        let mut args = Vec::new();
        for (e, o) in arguments {
            arg_exprs.push(e);
            args.push(o);
        }
        BuiltinFuncArgs {
            func_name, arg_exprs, args
        }
    }
    pub fn item(&self, i: usize) -> Option<Object> {
        self.args.get(i).map(|o| o.clone())
    }
    pub fn get(&self) -> Vec<Object> {
        self.args.clone()
    }
    pub fn len(&self) -> usize {
        self.args.len()
    }
    pub fn name(&self) -> &str {
        self.func_name.as_str()
    }
    pub fn get_expr(&self, i: usize) -> Option<Expression> {
        self.arg_exprs.get(i).map_or(None,|e| e.clone())
    }
}

pub enum BuiltinError {
    FunctionError(String, Option<String>),
    ArgumentError(String, Option<String>),
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
    // hashtbl
    set_builtin_consts::<HashTblEnum>(&mut vec);
    // window_low
    window_low::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<window_low::MouseButtonEnum>(&mut vec);
    set_builtin_consts::<window_low::KeyActionEnum>(&mut vec);
    // window_control
    window_control::builtin_func_sets().set(&mut vec);
    set_builtin_str_consts::<window_control::SpecialWindowId>(&mut vec);
    set_builtin_consts::<window_control::CtrlWinCmd>(&mut vec);
    set_builtin_consts::<window_control::StatusEnum>(&mut vec);
    set_builtin_consts::<window_control::MonitorEnum>(&mut vec);
    // text control
    text_control::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<text_control::RegexEnum>(&mut vec);
    // system_constrol
    system_controls::builtin_func_sets().set(&mut vec);
    set_builtin_consts::<system_controls::OsNumber>(&mut vec);
    set_builtin_consts::<system_controls::KindOfOsResultType>(&mut vec);
    // math
    math::builtin_func_sets().set(&mut vec);
    // key codes
    set_builtin_consts::<key_codes::VirtualKeyCodes>(&mut vec);
    set_builtin_consts::<key_codes::VirtualKeyCodeDups>(&mut vec);
    set_builtin_consts::<key_codes::VirtualMouseButton>(&mut vec);

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

pub fn set_builtin_str_consts<E: VariantNames>(vec: &mut Vec<NamedObject>) {
    for name in E::VARIANTS {
        let ucase_name = name.to_ascii_uppercase();
        vec.push(NamedObject::new_builtin_const(ucase_name.clone(), Object::String(format!("__{}__", ucase_name))));
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
    let uwscr_dir = match env::var("GET_UWSC_DIR") {
        Ok(s) => s,
        Err(_) => match env::current_dir() {
            Ok(p) => p.into_os_string().into_string().unwrap(),
            Err(_) => "".to_string()
        }
    };
    vec.push(NamedObject::new_builtin_const("GET_UWSC_DIR".into(), Object::String(uwscr_dir.clone())));
    vec.push(NamedObject::new_builtin_const("GET_UWSCR_DIR".into(), Object::String(uwscr_dir)));
    vec.push(NamedObject::new_builtin_const("GET_UWSC_NAME".into(), Object::String(
        env::var("GET_UWSC_NAME").unwrap_or("".into())
    )));
    vec.push(NamedObject::new_builtin_const("GET_UWSCR_NAME".into(), Object::String(
        env::var("GET_UWSC_NAME").unwrap_or("".into())
    )));
    vec.push(NamedObject::new_builtin_const("GET_WIN_DIR".into(), Object::String(
        get_windows_directory()
    )));
    vec.push(NamedObject::new_builtin_const("GET_SYS_DIR".into(), Object::String(
        get_system_directory()
    )));
    vec.push(NamedObject::new_builtin_const("GET_APPDATA_DIR".into(), Object::String(
        get_special_directory(winapi::um::shlobj::CSIDL_APPDATA)
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
            match window_low::get_current_pos("") {
                Ok(p) => p.x as f64,
                Err(_) => -999999.0
            }
        )
    )));
    vec.push(NamedObject::new_builtin_const("G_MOUSE_Y".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos("") {
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

pub fn raise(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = get_string_argument_value(&args, 0, None)?;
    let title = get_string_argument_value(&args, 1, Some("User defined error".into()))?;
    Err(UError::new(title, msg, None))
}

pub fn assert_equal(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg1 = get_any_argument_value(&args,0, None)?;
    let arg2 = get_any_argument_value(&args,1, None)?;
    if arg1 == arg2 {
        Ok(Object::Empty)
    } else {
        Err(UError::new("assertion error".into(), format!("left: {}, right: {}", arg1, arg2), None))
    }
}

// エラー出力用関数

pub fn builtin_func_error<S: Into<String>>(name: &str, msg: S)-> UError {
    UError::new("builtin function error".into(), msg.into(), Some(name.into()))
}

pub fn builtin_arg_error(msg: String, name: &str) -> BuiltinError {
    BuiltinError::ArgumentError(msg, Some(name.into()))
}

// ビルトイン関数の引数を受け取るための関数群
// i: 引数のインデックス
// default: 省略可能な引数のデフォルト値、必須引数ならNoneを渡す
// 引数が省略されていた場合はdefaultの値を返す
// 引数が必須なのになかったらエラーを返す

pub fn get_any_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<Object>) -> Result<Object, BuiltinError> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        Ok(arg)
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_non_float_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> Result<T, BuiltinError>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(n) => T::cast(n).or(Err(builtin_arg_error(
                format!("unable to cast {} to {}", n, std::any::type_name::<T>()), args.name())
            )),
            _ => Err(builtin_arg_error(
                format!("bad argument: {}", arg), args.name())
            )
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_num_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> Result<T, BuiltinError>
    where T: cast::From<f64, Output=T>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Num(n) => Ok(T::cast(n)),
            _ => Err(builtin_arg_error(
                format!("bad argument: {}", arg), args.name())
            )
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_string_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<String>) -> Result<String, BuiltinError> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match &arg {
            Object::String(s) => Ok(s.clone()),
            Object::RegEx(re) => Ok(re.clone()),
            _ => Err(builtin_arg_error(format!("bad argument: {}", arg), args.name()))
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_bool_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<bool>) -> Result<bool, BuiltinError> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::Bool(b) => Ok(b),
            _ => Err(builtin_arg_error(format!("bad argument: {}", arg), args.name()))
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_bool_or_int_argument_value<T>(args: &BuiltinFuncArgs, i: usize, default: Option<T>) -> Result<T, BuiltinError>
    where T: cast::From<f64, Output=Result<T, cast::Error>>,
{
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        let err = "cast error".to_string();
        match arg {
            Object::Bool(b) => if b {
                T::cast(1.0).or(Err(builtin_arg_error(err, args.name())))
            } else {
                T::cast(0.0).or(Err(builtin_arg_error(err, args.name())))
            },
            Object::Num(n) => T::cast(n).or(Err(builtin_arg_error(
                format!("unable to cast {} to {}", n, std::any::type_name::<T>()), args.name())
            )),
            _ => Err(builtin_arg_error(format!("bad argument: {}", arg), args.name()))
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}

pub fn get_uobject_argument_value(args: &BuiltinFuncArgs, i: usize, default: Option<Object>) -> Result<Object, BuiltinError> {
    if args.len() >= i + 1 {
        let arg = args.item(i).unwrap();
        match arg {
            Object::UObject(_) |
            Object::UChild(_, _) => Ok(arg),
            _ => Err(builtin_arg_error(format!("bad argument: {}", arg), args.name()))
        }
    } else {
        default.ok_or(builtin_arg_error(format!("argument {} required", i + 1), args.name()))
    }
}
