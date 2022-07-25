pub mod window_control;
pub mod window_low;
pub mod text_control;
pub mod system_controls;
pub mod math;
pub mod key_codes;
pub mod com_object;
pub mod browser_control;
pub mod array_control;
pub mod dialog;
pub mod file_control;
#[cfg(feature="chkimg")]
pub mod chkimg;

use crate::settings::USETTINGS;
use crate::winapi::{
    get_windows_directory,
    get_system_directory,
    get_special_directory,
    get_screen_width,
    get_screen_height,
    get_color_depth,
};
use windows::Win32::UI::Shell::CSIDL_APPDATA;
use crate::evaluator::object::{
    Object, Version,
    HashTblEnum,
    UTask
};
use crate::evaluator::object::{UObject,Fopen,Function};
use crate::evaluator::environment::NamedObject;
use crate::error::evaluator::{UError,UErrorKind,UErrorMessage};
use crate::ast::Expression;

use std::env;
use std::sync::{Mutex, Arc};
use std::string::ToString;

use cast;
use strum::VariantNames;
use num_traits::ToPrimitive;
use strum_macros::{Display, EnumVariantNames};

pub type BuiltinFunction = fn(BuiltinFuncArgs) -> BuiltinFuncResult;
pub type BuiltinFuncResult = Result<BuiltinFuncReturnValue, UError>;
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

macro_rules! get_arg_value {
    ($args:expr, $i:expr, $default:expr, $expr:expr) => {
        {
            if $args.len() >= $i + 1 {
                $expr
            } else {
                $default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt($i + 1), $args.name()))
            }
        }
    };
    ($args:expr, $i:expr, $expr:expr) => {
        {
            if $args.len() >= $i + 1 {
                $expr
            } else {
                Err(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt($i + 1), $args.name()))
            }
        }
    };
}

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
    pub fn item(&self, i: usize) -> Object {
        self.arguments.get(i).map(|o| o.1.clone()).unwrap_or_default()
    }
    fn split_off(&self, at: usize) -> Vec<Object> {
        self.arguments.clone().split_off(at)
            .into_iter()
            .map(|(_, o)| o)
            .collect()
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
    // pub fn get_objects_from(&mut self, index: usize) -> Vec<Object> {
    //     let rest = self.arguments.split_off(index);
    //     rest.into_iter().map(|a| a.1.clone()).collect()
    // }
    pub fn take_argument(&mut self, index: usize) -> Vec<(Option<Expression>, Object)> {
        let rest = self.arguments.split_off(index);
        rest.into_iter().map(|a| a.clone()).collect()
    }

    // ビルトイン関数の引数を受け取るための関数群
    // i: 引数のインデックス
    // default: 省略可能な引数のデフォルト値、必須引数ならNoneを渡す
    // 引数が省略されていた場合はdefaultの値を返す
    // 引数が必須なのになかったらエラーを返す

    /// 受けた引数をObjectのまま受ける
    pub fn get_as_object(&self, i: usize, default: Option<Object>) -> BuiltInResult<Object> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            Ok(arg)
        })
    }
    /// 引数を任意の整数型として受ける
    pub fn get_as_int<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Num(n) => T::cast(n).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                    self.name()
                ))),
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                    self.name()
                ))),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => T::cast(n).or(Err(builtin_func_error(
                        UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                        self.name()
                    ))),
                    Err(_) => Err(builtin_func_error(
                        UErrorMessage::BuiltinArgInvalid(arg),
                        self.name())
                    )
                },
                Object::EmptyParam => {
                    default.ok_or(builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), self.name()))
                },
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())
                )
            }
        })
    }
    /// 整数として受けるがEMPTYの場合は引数が省略されたとみなす
    pub fn get_as_int_or_empty<T>(&self, i: usize) -> BuiltInResult<Option<T>>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        let default = Some(None);
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Num(n) => match T::cast(n) {
                    Ok(t) => Ok(Some(t)),
                    Err(_) => Err(builtin_func_error(
                        UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                        self.name()
                    ))
                },
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                    self.name()
                ))).map(|t| Some(t)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => match T::cast(n) {
                        Ok(t) => Ok(Some(t)),
                        Err(_) => Err(builtin_func_error(
                            UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                            self.name()
                        ))
                    },
                    Err(_) => Err(builtin_func_error(
                        UErrorMessage::BuiltinArgInvalid(arg), self.name())
                    )
                },
                Object::Empty |
                Object::EmptyParam => Ok(None),
                _ => Err(builtin_func_error(
                    UErrorMessage::BuiltinArgInvalid(arg), self.name())
                )
            }
        })
    }
    /// 引数を任意の数値型として受ける
    pub fn get_as_num<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=T>,
    {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Num(n) => Ok(T::cast(n)),
                Object::Bool(b) => Ok(T::cast(b as i32 as f64)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => Ok(T::cast(n)),
                    Err(_) => Err(builtin_func_error(
                        UErrorMessage::BuiltinArgInvalid(arg), self.name())
                    )
                },
                _ => Err(builtin_func_error(
                    UErrorMessage::BuiltinArgInvalid(arg), self.name())
                )
            }
        })
    }
    /// 引数を文字列として受ける
    /// あらゆる型を文字列にする
    /// 引数省略(EmptyParam)はエラーになる
    pub fn get_as_string(&self, i: usize, default: Option<String>) -> BuiltInResult<String> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match &arg {
                Object::String(s) => Ok(s.clone()),
                Object::RegEx(re) => Ok(re.clone()),
                Object::EmptyParam => Err(
                    builtin_func_error(UErrorMessage::BuiltinArgRequiredAt(i + 1), self.name())
                ),
                o => Ok(o.to_string()),
            }
        })
    }
    /// 文字列として受けるがEMPTYの場合は引数省略とみなす
    pub fn get_as_string_or_empty(&self, i: usize) -> BuiltInResult<Option<String>> {
        let default = Some(None);
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match &arg {
                Object::String(s) => Ok(Some(s.to_string())),
                Object::Empty |
                Object::EmptyParam => Ok(None),
                o => Ok(Some(o.to_string())),
            }
        })
    }
    /// 文字列または文字列の配列を受ける引数
    /// EMPTYの場合はNoneを返す
    pub fn get_as_string_array_or_empty(&self, i: usize) -> BuiltInResult<Option<Vec<String>>> {
        let default = Some(None);
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            let vec = match &arg {
                Object::Array(vec) => Some(vec.iter().map(|o|o.to_string()).collect()),
                Object::HashTbl(arc) => {
                    let hash = arc.lock().unwrap();
                    let keys = hash.keys()
                        .into_iter()
                        .map(|o|o.to_string())
                        .collect();
                    Some(keys)
                },
                Object::Empty |
                Object::EmptyParam => None,
                o => Some(vec![o.to_string()])
            };
            Ok(vec)
        })
    }
    /// 文字列または文字列の配列を受ける引数(必須)
    pub fn get_as_string_array(&self, i: usize) -> BuiltInResult<Vec<String>> {
        let default = None;
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            let vec = match &arg {
                Object::Array(vec) => vec.iter().map(|o|o.to_string()).collect(),
                Object::HashTbl(arc) => {
                    let hash = arc.lock().unwrap();
                    hash.keys()
                        .into_iter()
                        .map(|o|o.to_string())
                        .collect()
                }
                o => vec![o.to_string()]
            };
            Ok(vec)
        })
    }
    /// 引数を真偽値として受ける
    pub fn get_as_bool(&self, i: usize, default: Option<bool>) -> BuiltInResult<bool> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            Ok(arg.is_truthy())
        })
    }
    /// 真偽値または真偽値の配列を受ける引数
    /// EMPTYの場合はNoneを返す
    pub fn get_as_bool_array(&self, i: usize, default: Option<Option<Vec<bool>>>) -> BuiltInResult<Option<Vec<bool>>> {
        get_arg_value!(self,i,default, {
            let arg = self.item(i);
            match &arg {
                Object::Array(vec) => Ok(Some(vec.iter().map(|o|o.is_truthy()).collect())),
                Object::Empty |
                Object::EmptyParam => Ok(None),
                o => Ok(Some(vec![o.is_truthy()]))
            }
        })
    }
    /// 引数をboolまたは数値として受ける (TRUE, FALSE, 2 のやつ)
    pub fn get_as_bool_or_int<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            let type_name = std::any::type_name::<T>().to_string();
            match arg {
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, type_name),
                    self.name()
                ))),
                Object::Num(n) => T::cast(n).or(Err(builtin_func_error(
                    UErrorMessage::BuiltinArgCastError(arg, type_name),
                    self.name())
                )),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// UObjectを受ける引数
    pub fn get_as_uobject(&self, i: usize) -> BuiltInResult<UObject> {
        get_arg_value!(self, i, {
            let arg = self.item(i);
            match arg {
                Object::UObject(ref u) => Ok(u.clone()),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// 配列を受ける引数
    pub fn get_as_array(&self, i: usize, default: Option<Vec<Object>>) -> BuiltInResult<Vec<Object>> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Array(arr) => Ok(arr),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// 配列として受ける引数、連想配列も含む
    pub fn get_as_array_include_hashtbl(&self, i: usize, default: Option<Vec<Object>>, get_hash_key: bool) -> BuiltInResult<Vec<Object>> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Array(arr) => Ok(arr),
                Object::HashTbl(m) => {
                    let hash = m.lock().unwrap();
                    if get_hash_key {
                        Ok(hash.keys())
                    } else {
                        Ok(hash.values())
                    }
                },
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    pub fn get_as_array_or_empty(&self, i: usize) -> BuiltInResult<Option<Vec<Object>>> {
        let default = Some(None::<Vec<Object>>);
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Array(arr) => Ok(Some(arr)),
                Object::Empty |
                Object::EmptyParam => Ok(None),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// 数値または配列を受ける引数
    pub fn get_as_int_or_array(&self, i: usize, default: Option<Object>) -> BuiltInResult<Object> {
        get_arg_value!(self, i, default, {
            let arg = self.item(i);
            match arg {
                Object::Num(_) |
                Object::Array(_) => Ok(arg),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// タスクを受ける引数
    pub fn get_as_task(&self, i: usize) -> BuiltInResult<UTask> {
        get_arg_value!(self, i, {
            let arg = self.item(i);
            match arg {
                Object::Task(utask) => Ok(utask),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// i32として受ける、文字列をパースしない
    pub fn get_as_i32(&self, i: usize) -> BuiltInResult<i32> {
        get_arg_value!(self,i, {
            let arg = self.item(i);
            match arg {
                Object::Num(n) => Ok(n as i32),
                _ => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name()))
            }
        })
    }
    /// 残りの引数を文字列の配列として受ける
    pub fn get_rest_as_string_array(&self, i: usize) -> BuiltInResult<Vec<String>> {
        get_arg_value!(self, i, {
            let vec = self.split_off(i)
                .into_iter()
                .map(|o| match o {
                    Object::Array(vec) => vec.into_iter()
                            .map(|o| o.to_string())
                            .collect(),
                    Object::HashTbl(arc) => {
                        let hash = arc.lock().unwrap();
                        hash.keys().into_iter()
                            .map(|o|o.to_string())
                            .collect()
                    }
                    Object::Empty|
                    Object::EmptyParam => {vec![]},
                    o => vec![o.to_string()]
                })
                .flatten()
                .collect();
            Ok(vec)
        })
    }

    pub fn get_as_fopen(&self, i: usize) -> BuiltInResult<Arc<Mutex<Fopen>>> {
        get_arg_value!(self, i, {
            match self.item(i) {
                Object::Fopen(arc) => Ok(Arc::clone(&arc)),
                arg => Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())),
            }
        })
    }
    pub fn get_as_string_or_fopen(&self, i: usize) -> BuiltInResult<TwoTypeArg<Option<String>, Arc<Mutex<Fopen>>>> {
        let default = Some(TwoTypeArg::T(None));
        get_arg_value!(self, i, default, {
            let result = match self.item(i) {
                Object::Empty |
                Object::EmptyParam => TwoTypeArg::T(None),
                Object::Fopen(arc) => TwoTypeArg::U(Arc::clone(&arc)),
                o => TwoTypeArg::T(Some(o.to_string()))
            };
            Ok(result)
        })
    }
    /// 文字列または真偽値を受ける
    pub fn get_as_string_or_bool(&self, i: usize, default: Option<TwoTypeArg<String, bool>>) -> BuiltInResult<TwoTypeArg<String, bool>> {
        get_arg_value!(self, i, default, {
            let result = match self.item(i) {
                Object::Empty |
                Object::EmptyParam => TwoTypeArg::U(false),
                Object::Bool(b) => TwoTypeArg::U(b),
                Object::String(s) => TwoTypeArg::T(s),
                arg => return Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())),
            };
            Ok(result)
        })
    }

    pub fn get_as_const<T: From<f64>>(&self, i: usize, default: Option<T>) -> BuiltInResult<T> {
        get_arg_value!(self, i, default, {
            let result = match self.item(i) {
                Object::Num(n) => T::from(n),
                arg => return Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())),
            };
            Ok(result)
        })
    }

    pub fn get_as_num_or_string(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, f64>> {
        get_arg_value!(self, i, {
            let result = match self.item(i) {
                Object::String(s) => TwoTypeArg::T(s),
                Object::Num(n) => TwoTypeArg::U(n),
                arg => return Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())),
            };
            Ok(result)
        })
    }

    pub fn get_as_string_or_bytearray(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, Vec<u8>>> {
        get_arg_value!(self, i, {
            let result = match self.item(i) {
                Object::String(s) => TwoTypeArg::T(s),
                Object::ByteArray(a) => TwoTypeArg::U(a),
                arg => return Err(builtin_func_error(UErrorMessage::BuiltinArgInvalid(arg), self.name())),
            };
            Ok(result)
        })
    }
}

pub enum TwoTypeArg<T, U> {
    T(T),
    U(U),
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
    set_builtin_consts::<text_control::ErrConst>(&mut vec);
    set_builtin_consts::<text_control::StrconvConst>(&mut vec);
    set_builtin_consts::<text_control::FormatConst>(&mut vec);
    set_builtin_consts::<text_control::CodeConst>(&mut vec);
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
    set_builtin_consts::<array_control::QsrtConst>(&mut vec);
    set_builtin_consts::<array_control::CalcConst>(&mut vec);
    // dialog.rs
    set_builtin_consts::<dialog::BtnConst>(&mut vec);
    set_builtin_consts::<dialog::SlctConst>(&mut vec);
    // SLCT_* 定数
    for n in 1..=31_u32 {
        let val = 2_i32.pow(n-1);
        let object = val.into();
        let name = format!("SLCT_{}",n);
        vec.push(NamedObject::new_builtin_const(name, object))
    }
    dialog::builtin_func_sets().set(&mut vec);
    // file_control
    set_builtin_consts::<file_control::FileConst>(&mut vec);
    set_builtin_consts::<file_control::FileConstDup>(&mut vec);
    set_builtin_consts::<file_control::FileOrderConst>(&mut vec);
    file_control::builtin_func_sets().set(&mut vec);
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
    sets.add("__p_a_n_i_c__", 1, panic);
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
    let s = args.get_as_string(0, None)?;
    Ok(BuiltinFuncReturnValue::Eval(s))
}

pub fn list_env(_args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(BuiltinFuncReturnValue ::GetEnv)
}

pub fn list_module_member(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let s = args.get_as_string(0, None)?;
    Ok(BuiltinFuncReturnValue ::ListModuleMember(s))
}

pub fn name_of(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(BuiltinFuncReturnValue ::BuiltinConstName(args.get_expr(0)))
}

pub fn get_settings(_args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let json = {
        let s = USETTINGS.lock().unwrap();
        s.get_current_settings_as_json()
    };
    Ok(BuiltinFuncReturnValue::Result(Object::String(json)))
}

pub fn raise(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = args.get_as_string(0, None)?;
    let title = args.get_as_string(1, Some(String::new()))?;
    let kind = if title.len() > 0 {
        UErrorKind::Any(title)
    } else {
        UErrorKind::UserDefinedError
    };
    Err(UError::new(kind, UErrorMessage::Any(msg)))
}

pub fn panic(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = args.get_as_string(0, None)?;
    panic!("{msg}");
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumVariantNames, Display)]
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
    let arg = args.get_as_object(0, None)?;
    let t = match arg {
        Object::Num(_) => VariableType::TYPE_NUMBER,
        Object::String(_) => VariableType::TYPE_STRING,
        Object::Bool(_) => VariableType::TYPE_BOOL,
        Object::Array(_) => VariableType::TYPE_ARRAY,
        Object::HashTbl(_) => VariableType::TYPE_HASHTBL,
        Object::AnonFunc(_) => VariableType::TYPE_ANONYMOUS_FUNCTION,
        Object::Function(_) => VariableType::TYPE_FUNCTION,
        Object::BuiltinFunction(_,_,_) => VariableType::TYPE_BUILTIN_FUNCTION,
        Object::Module(_) => VariableType::TYPE_MODULE,
        Object::Class(_,_) => VariableType::TYPE_CLASS,
        Object::Instance(ref m) => {
            let ins = m.lock().unwrap();
            if ins.is_dropped {
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
        Object::UObject(_) => VariableType::TYPE_UOBJECT,
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
    Ok(BuiltinFuncReturnValue::Result(Object::String(t.to_string())))
}

pub fn assert_equal(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg1 = args.get_as_object(0, None)?;
    let arg2 = args.get_as_object(1, None)?;
    if arg1.is_equal(&arg2) {
        Ok(BuiltinFuncReturnValue::Result(Object::Empty))
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

pub enum BuiltinFuncReturnValue {
    Result(Object),
    Reference {
        refs: Vec<(Option<Expression>, Object)>,
        result: Object
    },
    GetEnv,
    ListModuleMember(String),
    BuiltinConstName(Option<Expression>),
    Task(Function, Vec<(Option<Expression>, Object)>),
    GetLogPrintWinId,
    Balloon(Option<crate::gui::Balloon>),
    BalloonID,
    Token {token: String, remained: String, expression: Option<Expression>},
    Qsort(Option<Expression>, Vec<Object>, [Option<Expression>; 8], [Option<Vec<Object>>; 8]),
    Eval(String),
}