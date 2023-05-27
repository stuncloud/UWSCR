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
pub mod clipboard;
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
use crate::evaluator::Evaluator;
use crate::evaluator::object::{UObject,Fopen,Function,browser::{RemoteObject, TabWindow}};
use crate::evaluator::environment::NamedObject;
use crate::evaluator::builtins::key_codes::{SCKeyCode};
use crate::error::evaluator::{UError,UErrorKind,UErrorMessage};
use crate::ast::{Expression, Identifier};

use std::env;
use std::sync::{Mutex, Arc};
use std::string::ToString;

use cast;
use strum::{VariantNames, EnumProperty};
use num_traits::{ToPrimitive, FromPrimitive};
use strum_macros::{Display, EnumVariantNames, EnumProperty};

pub type BuiltinFunction = fn(&mut Evaluator, BuiltinFuncArgs) -> BuiltinFuncResult;
pub type BuiltinFuncResult = Result<Object, BuiltinFuncError>;
pub type BuiltInResult<T> = Result<T, BuiltinFuncError>;

pub enum BuiltinFuncError {
    UError(UError),
    Error(UErrorMessage),
    Kind(UErrorKind, UErrorMessage),
}
impl BuiltinFuncError {
    pub fn new(message: UErrorMessage) -> Self {
        Self::Error(message)
    }
    pub fn new_with_kind(kind: UErrorKind, message: UErrorMessage) -> Self {
        Self::Kind(kind, message)
    }
    pub fn message(&self) -> UErrorMessage {
        match self {
            BuiltinFuncError::UError(e) => e.message.clone(),
            BuiltinFuncError::Error(e) => e.clone(),
            BuiltinFuncError::Kind(_, e) => e.clone(),
        }
    }
    pub fn to_uerror(self, name: String) -> UError {
        match self {
            BuiltinFuncError::UError(e) => e,
            BuiltinFuncError::Error(message) => UError::new(UErrorKind::BuiltinFunctionError(name), message),
            BuiltinFuncError::Kind(kind, message) => UError::new(kind, message),
        }
    }
}

pub fn builtin_func_error(message: UErrorMessage) -> BuiltinFuncError {
    BuiltinFuncError::new(message)
}

impl From<windows::core::Error> for BuiltinFuncError {
    fn from(e: windows::core::Error) -> Self {
        Self::UError(e.into())
    }
}

impl From<UError> for BuiltinFuncError {
    fn from(e: UError) -> Self {
        Self::UError(e)
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinFuncArgs {
    arguments: Vec<(Option<Expression>, Object)>,
    is_await: bool,
}

impl BuiltinFuncArgs {
    pub fn new(arguments: Vec<(Option<Expression>, Object)>, is_await: bool) -> Self {
        BuiltinFuncArgs {
            arguments,
            is_await
        }
    }
    pub fn is_await(&self) -> bool {
        self.is_await
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
    fn get_arg<T, F: Fn(Object)-> BuiltInResult<T>>(&self, i: usize, f: F) -> BuiltInResult<T> {
        if self.len() >= i+ 1 {
            let obj = self.item(i);
            f(obj)
        } else {
            Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1)))
        }
    }
    fn get_arg_with_default<T, F: Fn(Object)-> BuiltInResult<T>>(&self, i: usize, default: Option<T>, f: F) -> BuiltInResult<T> {
        if self.len() >= i+ 1 {
            let obj = self.item(i);
            if obj == Object::EmptyParam {
                // 引数が省略されていた場合、デフォルト値を返すか必須引数ならエラーにする
                default.ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1)))
            } else {
                f(obj)
            }
        } else {
            default.ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1)))
        }
    }
    fn get_arg_with_default2<T, F: Fn(Object, Option<T>)-> BuiltInResult<T>>(&self, i: usize, default: Option<T>, f: F) -> BuiltInResult<T> {
        if self.len() >= i+ 1 {
            let obj = self.item(i);
            f(obj, default)
        } else {
            let err = BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1));
            default.ok_or(err)
        }
    }
    fn get_arg_with_required_flag<T, F: Fn(Object)-> BuiltInResult<Option<T>>>(&self, i: usize, required: bool, f: F) -> BuiltInResult<Option<T>> {
        if self.len() >= i+ 1 {
            let obj = self.item(i);
            f(obj)
        } else {
            if required {
                let err = BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1));
                Err(err)
            } else {
                Ok(None)
            }
        }
    }

    /// 受けた引数をObjectのまま受ける
    pub fn get_as_object(&self, i: usize, default: Option<Object>) -> BuiltInResult<Object> {
        self.get_arg_with_default(i, default, |arg|{
            Ok(arg)
        })
    }
    pub fn get_as_f64(&self, i: usize, default: Option<f64>) -> BuiltInResult<f64> {
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Num(n) => Ok(n),
                Object::Bool(b) => Ok(b as i32 as f64),
                Object::String(ref s) => {
                    s.parse().map_err(|_| BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
                },
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 引数を任意の整数型として受ける
    pub fn get_as_int<T: Clone>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        self.get_arg_with_default2(i, default, |arg, default| {
            match arg {
                Object::Num(n) => T::cast(n).or(Err(BuiltinFuncError::new(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                ))),
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(BuiltinFuncError::new(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                ))),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => T::cast(n).or(Err(BuiltinFuncError::new(
                        UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                    ))),
                    Err(_) => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
                },
                Object::EmptyParam => {
                    default.ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1)))
                },
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// n番目を得る、1以下は1にする
    pub fn get_as_nth(&self, i: usize) -> BuiltInResult<u32>
    {
        let nth = self.get_as_int::<u32>(i, Some(1))?;
        Ok(1.max(nth))
    }
    /// 整数として受けるがEMPTYの場合は引数が省略されたとみなす
    pub fn get_as_int_or_empty<T>(&self, i: usize) -> BuiltInResult<Option<T>>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        let default = Some(None);
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Num(n) => match T::cast(n) {
                    Ok(t) => Ok(Some(t)),
                    Err(_) => Err(BuiltinFuncError::new(
                        UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),

                    ))
                },
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(BuiltinFuncError::new(
                    UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),
                ))).map(|t| Some(t)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => match T::cast(n) {
                        Ok(t) => Ok(Some(t)),
                        Err(_) => Err(BuiltinFuncError::new(
                            UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())
                        ))
                    },
                    Err(_) => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
                },
                Object::Empty |
                Object::EmptyParam => Ok(None),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 引数を任意の数値型として受ける
    pub fn get_as_num<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=T>,
    {
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Num(n) => Ok(T::cast(n)),
                Object::Bool(b) => Ok(T::cast(b as i32 as f64)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => Ok(T::cast(n)),
                    Err(_) => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
                },
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 引数を文字列として受ける
    /// あらゆる型を文字列にする
    /// 引数省略(EmptyParam)はエラーになる
    pub fn get_as_string(&self, i: usize, default: Option<String>) -> BuiltInResult<String> {
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::String(s) => Ok(s),
                Object::RegEx(re) => Ok(re),
                Object::EmptyParam => Err(
                    BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + 1))
                ),
                o => Ok(o.to_string()),
            }
        })
    }
    /// 文字列として受けるがEMPTYの場合は引数省略とみなす
    pub fn get_as_string_or_empty(&self, i: usize) -> BuiltInResult<Option<String>> {
        let default = Some(None);
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::String(s) => Ok(Some(s)),
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
        self.get_arg_with_default(i, default, |arg| {
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
        self.get_arg_with_default(i, default, |arg| {
            let vec = match arg {
                Object::Array(vec) => vec.into_iter().map(|o|o.to_string()).collect(),
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
        self.get_arg_with_default(i, default, |arg| {
            Ok(arg.is_truthy())
        })
    }
    /// 真偽値または真偽値の配列を受ける引数
    /// EMPTYの場合はNoneを返す
    pub fn get_as_bool_array(&self, i: usize, default: Option<Option<Vec<bool>>>) -> BuiltInResult<Option<Vec<bool>>> {
        self.get_arg_with_default(i ,default, |arg| {
            match arg {
                Object::Array(vec) => Ok(Some(vec.iter().map(|o|o.is_truthy()).collect())),
                Object::Empty |
                Object::EmptyParam => Ok(None),
                o => Ok(Some(vec![o.is_truthy()]))
            }
        })
    }
    /// bool及び数値を数値として受ける
    pub fn get_as_bool_or_int<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: cast::From<f64, Output=Result<T, cast::Error>>,
    {
        self.get_arg_with_default(i, default, |arg|{
            let type_name = std::any::type_name::<T>().to_string();
            match arg {
                Object::Bool(b) => T::cast(b as i32 as f64).or(Err(BuiltinFuncError::new(
                    UErrorMessage::BuiltinArgCastError(arg, type_name),
                ))),
                Object::Num(n) => T::cast(n).or(Err(BuiltinFuncError::new(
                    UErrorMessage::BuiltinArgCastError(arg, type_name),
                ))),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 3状態引数 (TRUE, FALSE, 2のやつ)
    pub fn get_as_three_state(&self, i: usize, default: Option<ThreeState>) -> BuiltInResult<ThreeState> {
        self.get_arg_with_default(i, default, |arg|{
            match arg {
                Object::Bool(b) => Ok(b.into()),
                Object::Num(n) => Ok(n.into()),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// UObjectを受ける引数
    pub fn get_as_uobject(&self, i: usize) -> BuiltInResult<UObject> {
        self.get_arg(i, |arg| {
            match arg {
                Object::UObject(ref u) => Ok(u.clone()),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 配列を受ける引数
    pub fn get_as_array(&self, i: usize, default: Option<Vec<Object>>) -> BuiltInResult<Vec<Object>> {
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Array(arr) => Ok(arr),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 配列として受ける引数、連想配列も含む
    pub fn get_as_array_include_hashtbl(&self, i: usize, default: Option<Vec<Object>>, get_hash_key: bool) -> BuiltInResult<Vec<Object>> {
        self.get_arg_with_default(i, default, |arg| {
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
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    pub fn get_as_array_or_empty(&self, i: usize) -> BuiltInResult<Option<Vec<Object>>> {
        let default = Some(None::<Vec<Object>>);
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Array(arr) => Ok(Some(arr)),
                Object::Empty |
                Object::EmptyParam => Ok(None),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 数値または配列を受ける引数
    pub fn get_as_int_or_array(&self, i: usize, default: Option<TwoTypeArg<f64, Vec<Object>>>) -> BuiltInResult<TwoTypeArg<f64, Vec<Object>>> {
        self.get_arg_with_default(i, default, |arg|{
            match arg {
                Object::Num(n) => Ok(TwoTypeArg::T(n)),
                Object::Array(arr) => Ok(TwoTypeArg::U(arr)),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// タスクを受ける引数
    pub fn get_as_task(&self, i: usize) -> BuiltInResult<TwoTypeArg<UTask, RemoteObject>> {
        self.get_arg(i, |arg| {
            match arg {
                Object::Task(utask) => Ok(TwoTypeArg::T(utask)),
                Object::RemoteObject(remote) => {
                    if remote.is_promise() {
                        Ok(TwoTypeArg::U(remote))
                    } else {
                        Err(BuiltinFuncError::new(UErrorMessage::RemoteObjectIsNotPromise))
                    }
                }
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// i32として受ける、文字列をパースしない
    pub fn get_as_i32(&self, i: usize) -> BuiltInResult<i32> {
        self.get_arg(i, |arg| {
            match arg {
                Object::Num(n) => Ok(n as i32),
                _ => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
            }
        })
    }
    /// 残りの引数を文字列の配列として受ける
    /// (空文字は無視される)
    /// - requires: 最低限必要なアイテム数
    pub fn get_rest_as_string_array(&self, i: usize, requires: usize) -> BuiltInResult<Vec<String>> {
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
            .filter(|s| ! s.is_empty())
            .collect::<Vec<_>>();
        if vec.len() < requires {
            Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgRequiredAt(i + requires)))
        } else {
            Ok(vec)
        }
    }
    /// 残りの引数をsckey用のキーコードとして得る
    pub fn get_sckey_codes(&self, i: usize) -> BuiltInResult<Vec<SCKeyCode>> {
        self.get_arg(i, |_| {
            let vec = self.split_off(i)
                .into_iter()
                .filter_map(|o| match o {
                    Object::Num(n) => {
                        FromPrimitive::from_f64(n)
                            .map(|key| SCKeyCode::VirtualKeyCode(key))
                    },
                    Object::String(s) => {
                        s.chars().next()
                            .map(|char| SCKeyCode::Unicode(char as u16))
                    },
                    _ => None,
                })
                .collect();
            Ok(vec)
        })
    }

    pub fn get_as_fopen(&self, i: usize) -> BuiltInResult<Arc<Mutex<Fopen>>> {
        self.get_arg(i, |arg| {
            match arg {
                Object::Fopen(arc) => Ok(Arc::clone(&arc)),
                arg => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg))),
            }
        })
    }
    pub fn get_as_string_or_fopen(&self, i: usize) -> BuiltInResult<TwoTypeArg<Option<String>, Arc<Mutex<Fopen>>>> {
        let default = Some(TwoTypeArg::T(None));
        self.get_arg_with_default(i, default, |arg| {
            let result = match arg {
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
        self.get_arg_with_default(i, default, |arg| {
            let result = match arg {
                Object::Empty |
                Object::EmptyParam => TwoTypeArg::U(false),
                Object::Bool(b) => TwoTypeArg::U(b),
                Object::String(s) => TwoTypeArg::T(s),
                obj => TwoTypeArg::T(obj.to_string()),
            };
            Ok(result)
        })
    }

    /// 数値を定数として受ける
    pub fn get_as_const<T: FromPrimitive>(&self, i: usize, required: bool) -> BuiltInResult<Option<T>> {
        self.get_arg_with_required_flag(i, required, |arg| {
            let result = match arg {
                Object::Num(n) => T::from_f64(n),
                Object::Empty | Object::EmptyParam => None,
                arg => return Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg))),
            };
            Ok(result)
        })
    }

    /// 数値は数値として受けるがそれ以外は文字列として受ける
    pub fn get_as_num_or_string<T>(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, T>>
        where T: cast::From<f64, Output=Result<T, cast::Error>>
    {
        self.get_arg( i, |arg| {
            let result = match arg {
                Object::Num(n) => {
                    T::cast(n)
                        .or(Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))))
                        .map(|t| TwoTypeArg::U(t))?
                },
                Object::Bool(b) => {
                    let n = if b {1.0} else {0.0};
                    T::cast(n)
                        .or(Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))))
                        .map(|t| TwoTypeArg::U(t))?
                },
                arg => TwoTypeArg::T(arg.to_string()),
            };
            Ok(result)
        })
    }
    /// 数値はf64として受けるがそれ以外は文字列として受ける
    pub fn get_as_f64_or_string(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, f64>> {
        self.get_arg( i, |arg| {
            let result = match arg {
                Object::Num(n) => TwoTypeArg::U(n),
                Object::Bool(b) => {
                    let n = if b {1.0} else {0.0};
                    TwoTypeArg::U(n)
                },
                arg => TwoTypeArg::T(arg.to_string()),
            };
            Ok(result)
        })
    }

    pub fn get_as_string_or_bytearray(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, Vec<u8>>> {
        self.get_arg(i, |arg| {
            let result = match arg {
                Object::String(s) => TwoTypeArg::T(s),
                Object::ByteArray(a) => TwoTypeArg::U(a),
                arg => return Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg))),
            };
            Ok(result)
        })
    }

    /// ユーザー定義関数を受ける
    /// 省略時や空文字の場合はNoneを返す
    pub fn get_as_function_or_string(&self, i: usize, required: bool) -> BuiltInResult<Option<TwoTypeArg<String, Function>>> {
        self.get_arg_with_required_flag(i, required, |arg| {
            let value = match arg {
                Object::Empty |
                Object::EmptyParam => None,
                Object::AnonFunc(func) |
                Object::Function(func) => Some(TwoTypeArg::U(func)),
                Object::String(str) => {
                    let name = str.clone();
                    if name.is_empty() {
                        None
                    } else {
                        Some(TwoTypeArg::T(name))
                    }
                },
                arg => return Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg))),
            };
            Ok(value)
        })
    }

    /// RemoteObjectを受ける
    pub fn get_as_remoteobject(&self, i: usize) -> BuiltInResult<RemoteObject> {
        self.get_arg(i, |obj| {
            match obj {
                Object::RemoteObject(remote) => Ok(remote),
                o => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(o))),
            }
        })
    }
    /// TabWindowを受ける
    pub fn get_as_tabwindow(&self, i: usize) -> BuiltInResult<TabWindow> {
        self.get_arg(i, |obj| {
            match obj {
                Object::TabWindow(tab) => Ok(tab),
                o => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(o))),
            }
        })
    }

}

pub enum TwoTypeArg<T, U> {
    T(T),
    U(U),
}

pub enum ThreeState {
    True,
    False,
    Other,
}
impl From<f64> for ThreeState {
    fn from(n: f64) -> Self {
        match n as i64 {
            0 => Self::False,
            2 => Self::Other,
            _ => Self::True
        }
    }
}
impl From<bool> for ThreeState {
    fn from(b: bool) -> Self {
        if b { Self::True } else { Self::False }
    }
}
impl ThreeState {
    pub fn as_bool(&self) -> bool {
        match self {
            Self::False => false,
            _ => true
        }
    }
}

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
    pub fn set(self, vec: &mut Vec<NamedObject>) {
        for set in self.sets {
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
    set_builtin_str_consts::<window_control::GetHndConst>(&mut vec, "__", "__");
    set_builtin_consts::<window_control::ClkConst>(&mut vec);
    set_builtin_consts::<window_control::GetItemConst>(&mut vec);
    set_builtin_consts::<window_control::AccConst>(&mut vec);
    set_builtin_consts::<window_control::CurConst>(&mut vec);
    set_builtin_consts::<window_control::ColConst>(&mut vec);
    set_builtin_consts::<window_control::SldConst>(&mut vec);
    set_builtin_consts::<window_control::GetStrConst>(&mut vec);
    set_builtin_consts::<window_control::ImgConst>(&mut vec);
    set_builtin_consts::<window_control::MorgTargetConst>(&mut vec);
    set_builtin_consts::<window_control::MorgContextConst>(&mut vec);

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
    set_builtin_consts::<system_controls::LockHardExConst>(&mut vec);
    set_builtin_consts::<system_controls::SensorConst>(&mut vec);
    set_builtin_consts::<system_controls::ToggleKey>(&mut vec);
    set_builtin_consts::<system_controls::POFF>(&mut vec);
    set_builtin_consts::<system_controls::GTimeOffset>(&mut vec);
    set_builtin_consts::<system_controls::GTimeWeekDay>(&mut vec);
    // math
    math::builtin_func_sets().set(&mut vec);
    // key codes
    set_builtin_consts::<key_codes::VirtualKeyCode>(&mut vec);
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
    set_builtin_consts::<file_control::FileOrderConst>(&mut vec);
    file_control::builtin_func_sets().set(&mut vec);
    // 特殊変数
    set_special_variables(&mut vec);

    vec
}

pub fn set_builtin_consts<E: std::str::FromStr + VariantNames + EnumProperty + ToPrimitive>(vec: &mut Vec<NamedObject>) {
    for name in E::VARIANTS {
        if let Ok(value) = E::from_str(name) {
            // props(hidden="true") であればスキップ
            if value.get_str("hidden").is_none() {
                let num = ToPrimitive::to_f64(&value).unwrap();
                vec.push(NamedObject::new_builtin_const(
                    name.to_ascii_uppercase(),
                    Object::Num(num)
                ));
                // aliasがあればそれもセットする
                if let Some(alias) = value.get_str("alias") {
                    vec.push(NamedObject::new_builtin_const(
                        alias.to_ascii_uppercase(),
                        Object::Num(num)
                    ));
                }
            }
        }
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
    vec.push(NamedObject::new_builtin_const("GET_UWSC_PRO".into(), Object::Empty));
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
            match window_low::get_current_pos() {
                Ok(p) => p.x as f64,
                Err(_) => -999999.0
            }
        )
    )));
    vec.push(NamedObject::new_builtin_const("G_MOUSE_Y".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos() {
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

pub fn builtin_eval(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let script = args.get_as_string(0, None)?;
    evaluator.invoke_eval_script(&script)
        .map_err(|err| BuiltinFuncError::UError(err))
}

pub fn list_env(evaluator: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let env = evaluator.env.get_env();
    Ok(env)
}

pub fn list_module_member(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let name = args.get_as_string(0, None)?;
    let members = evaluator.env.get_module_member(&name);
    Ok(members)
}

pub fn name_of(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let name = if let Some(Expression::Identifier(Identifier(name))) = args.get_expr(0) {
        evaluator.env.get_name_of_builtin_consts(&name)
    } else {
        Object::Empty
    };
    Ok(name)
}

pub fn get_settings(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let json = {
        let s = USETTINGS.lock().unwrap();
        s.get_current_settings_as_json()
    };
    Ok(Object::String(json))
}

pub fn raise(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = args.get_as_string(0, None)?;
    let title = args.get_as_string(1, Some(String::new()))?;
    let kind = if title.len() > 0 {
        UErrorKind::Any(title)
    } else {
        UErrorKind::UserDefinedError
    };
    Err(BuiltinFuncError::new_with_kind(kind, UErrorMessage::Any(msg)))
}

pub fn panic(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    TYPE_ASYNC_FUNCTION,
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
    TYPE_SAFEARRAY,
    TYPE_BROWSERBUILDER_OBJECT,
    TYPE_BROWSER_OBJECT,
    TYPE_TABWINDOW_OBJECT,
    TYPE_REMOTE_OBJECT,
    TYPE_BROWSER_FUNCTION,
    // TYPE_ELEMENT_OBJECT,
    TYPE_FILE_ID,
    TYPE_BYTE_ARRAY,
    TYPE_REFERENCE,
    TYPE_WEB_REQUEST,
    TYPE_WEB_RESPONSE,
    TYPE_WEB_FUNCTION,
    TYPE_HTML_NODE,

    TYPE_NOT_VALUE_TYPE,
}

pub fn type_of(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
        Object::AsyncFunction(_) => VariableType::TYPE_ASYNC_FUNCTION,
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
        Object::ComMember(_, _) |
        Object::ComObject(_) => VariableType::TYPE_COM_OBJECT,
        Object::Variant(_) => VariableType::TYPE_VARIANT,
        Object::SafeArray(_) => VariableType::TYPE_SAFEARRAY,
        Object::BrowserBuilder(_) => VariableType::TYPE_BROWSERBUILDER_OBJECT,
        Object::Browser(_) => VariableType::TYPE_BROWSER_OBJECT,
        Object::TabWindow(_) => VariableType::TYPE_TABWINDOW_OBJECT,
        Object::RemoteObject(_) => VariableType::TYPE_REMOTE_OBJECT,
        Object::BrowserFunction(_) => VariableType::TYPE_BROWSER_FUNCTION,
        Object::Fopen(_) => VariableType::TYPE_FILE_ID,
        Object::ByteArray(_) => VariableType::TYPE_BYTE_ARRAY,
        Object::Reference(_, _) => VariableType::TYPE_REFERENCE,
        Object::WebRequest(_) => VariableType::TYPE_WEB_REQUEST,
        Object::WebResponse(_) => VariableType::TYPE_WEB_RESPONSE,
        Object::WebFunction(_) => VariableType::TYPE_WEB_FUNCTION,
        Object::HtmlNode(_) => VariableType::TYPE_HTML_NODE,

        Object::EmptyParam |
        Object::VarArgument(_) |
        Object::DynamicVar(_) |
        Object::Continue(_) |
        Object::Break(_) |
        Object::Exit => VariableType::TYPE_NOT_VALUE_TYPE,
    };
    Ok(Object::String(t.to_string()))
}

pub fn assert_equal(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg1 = args.get_as_object(0, None)?;
    let arg2 = args.get_as_object(1, None)?;
    if arg1.is_equal(&arg2) {
        Ok(Object::Empty)
    } else {
        Err(BuiltinFuncError::new_with_kind(UErrorKind::AssertEqError, UErrorMessage::AssertEqLeftAndRight(arg1, arg2)))
    }
}

