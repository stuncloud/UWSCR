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

use util::settings::USETTINGS;
use util::winapi::{
    get_windows_directory,
    get_system_directory,
    get_special_directory,
    get_screen_width,
    get_screen_height,
    get_color_depth,
};
use windows::Win32::UI::Shell::CSIDL_APPDATA;
use windows::Win32::System::Threading::GetCurrentThreadId;

use crate::object::{
    Object, Version,
    HashTblEnum,
    UTask,
};
use crate::Evaluator;
use crate::object::{UObject,Fopen,Function,browser::{RemoteObject, TabWindow}, ObjectType, ComObject, StructDef};
use crate::environment::NamedObject;
use crate::builtins::key_codes::SCKeyCode;
use crate::error::{UError,UErrorKind,UErrorMessage};
use parser::ast::{Expression, Identifier};

pub use func_desc::*;
pub use func_desc_macro::*;

use std::env;
use std::sync::{Mutex, Arc};
use std::string::ToString;

use strum::{VariantNames, EnumProperty};
use num_traits::{ToPrimitive, FromPrimitive};
use strum_macros::EnumProperty;

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
        if at > self.len() {
            vec![]
        } else {
            self.arguments.clone().split_off(at)
                .into_iter()
                .map(|(_, o)| o)
                .collect()
        }
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
    pub fn get_as_object_or_empty(&self, i: usize) -> BuiltInResult<Option<Object>> {
        self.get_arg_with_required_flag(i, false, |arg| {
            match arg {
                Object::Empty |
                Object::EmptyParam => Ok(None),
                o => Ok(Some(o))
            }
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
    pub fn get_as_int<T>(&self, i: usize, default: Option<T>) -> BuiltInResult<T>
        where T: FromPrimitive + Clone,
    {
        self.get_arg_with_default2(i, default, |arg, default| {
            match arg {
                Object::Num(n) => T::from_f64(n)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))),
                Object::Bool(b) => T::from_i32(b as i32)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()),)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => T::from_f64(n)
                        .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))),
                    Err(_) => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg)))
                },
                Object::Empty |
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
        where T: FromPrimitive,
    {
        let default = Some(None);
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Num(n) => T::from_f64(n)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())))
                    .map(|t| Some(t)),
                Object::Bool(b) => T::from_i32(b as i32)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())))
                    .map(|t| Some(t)),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => T::from_f64(n)
                        .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())))
                        .map(|t| Some(t)),
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
        where T: FromPrimitive
    {
        self.get_arg_with_default(i, default, |arg| {
            match arg {
                Object::Num(n) => T::from_f64(n)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))),
                Object::Bool(b) => T::from_i32(b as i32)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))),
                Object::String(ref s) => match s.parse::<f64>() {
                    Ok(n) => T::from_f64(n)
                        .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into()))),
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
        where T: FromPrimitive,
    {
        self.get_arg_with_default(i, default, |arg|{
            let type_name = std::any::type_name::<T>().to_string();
            match arg {
                Object::Bool(b) => T::from_i32(b as i32)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, type_name))),
                Object::Num(n) => T::from_f64(n)
                    .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, type_name))),
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
    /// 数値または配列を受けるがEMPTYを許容する引数
    pub fn get_as_int_or_array_or_empty(&self, i: usize) -> BuiltInResult<Option<TwoTypeArg<f64, Vec<Object>>>> {
        self.get_arg_with_required_flag(i, false, |arg|{
            match arg {
                Object::Num(n) => Ok(Some(TwoTypeArg::T(n))),
                Object::Array(arr) => Ok(Some(TwoTypeArg::U(arr))),
                Object::Empty |
                Object::EmptyParam => Ok(None),
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
    /// 文字列、文字列配列または真偽値を受ける
    pub fn get_as_string_array_or_bool(&self, i: usize, default: Option<TwoTypeArg<Vec<String>, bool>>) -> BuiltInResult<TwoTypeArg<Vec<String>, bool>> {
        self.get_arg_with_default(i, default, |arg| {
            let result = match arg {
                Object::Empty |
                Object::EmptyParam => TwoTypeArg::U(false),
                Object::Bool(b) => TwoTypeArg::U(b),
                Object::String(s) => TwoTypeArg::T(vec![s]),
                Object::Array(arr) => {
                    let vec = arr.into_iter().map(|o| o.to_string()).collect();
                    TwoTypeArg::T(vec)
                },
                obj => TwoTypeArg::T(vec![obj.to_string()]),
            };
            Ok(result)
        })
    }

    /// 数値を定数として受ける
    pub fn get_as_const<T: FromPrimitive>(&self, i: usize, required: bool) -> BuiltInResult<Option<T>> {
        self.get_arg_with_required_flag(i, required, |arg| {
            let result = match arg {
                Object::Num(n) => T::from_f64(n),
                Object::Bool(b) => if b {
                    T::from_i32(1)
                } else {
                    T::from_i32(0)
                },
                Object::Empty | Object::EmptyParam => None,
                arg => return Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(arg))),
            };
            Ok(result)
        })
    }

    /// 数値は数値として受けるがそれ以外は文字列として受ける
    pub fn get_as_num_or_string<T>(&self, i: usize) -> BuiltInResult<TwoTypeArg<String, T>>
        where T: FromPrimitive
    {
        self.get_arg( i, |arg| {
            let result = match arg {
                Object::Num(n) => {
                    T::from_f64(n)
                        .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())))
                        .map(|t| TwoTypeArg::U(t))?
                },
                Object::Bool(b) => {
                    T::from_i32(b as i32)
                        .ok_or(BuiltinFuncError::new(UErrorMessage::BuiltinArgCastError(arg, std::any::type_name::<T>().into())))
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
    /// 数値または文字列を受けるが省略時はNone
    pub fn get_as_f64_or_string_or_empty(&self, i: usize) -> BuiltInResult<Option<TwoTypeArg<String, f64>>> {
        self.get_arg_with_required_flag(i, false, |arg| {
            let result = match arg {
                Object::Num(n) => Some(TwoTypeArg::U(n)),
                Object::Bool(b) => {
                    let n = if b {1.0} else {0.0};
                    Some(TwoTypeArg::U(n))
                },
                Object::Empty |
                Object::EmptyParam => None,
                arg => Some(TwoTypeArg::T(arg.to_string())),
            };
            Ok(result)
        })
    }
    pub fn get_as_int_or_string_or_empty<T>(&self, i: usize) -> BuiltInResult<Option<TwoTypeArg<String, T>>>
        where T: FromPrimitive
    {
        self.get_arg_with_required_flag(i, false, |arg| {
            match arg {
                Object::Num(n) => {
                    let t = T::from_f64(n)
                        .map(|t| TwoTypeArg::U(t))
                        .ok_or(BuiltinFuncError::new(UErrorMessage::CastError2(n, std::any::type_name::<T>().into())))?;
                    Ok(Some(t))
                },
                Object::Bool(b) => {
                    let t = T::from_i32(b as i32)
                        .map(|t| TwoTypeArg::U(t))
                        .ok_or(BuiltinFuncError::new(UErrorMessage::CastError2(b as i32 as f64, std::any::type_name::<T>().into())))?;
                    Ok(Some(t))
                },
                Object::Empty |
                Object::EmptyParam => Ok(None),
                obj => Ok(Some(TwoTypeArg::T(obj.to_string())))
            }
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

    pub fn get_as_func_or_num(&self, i: usize) -> BuiltInResult<TwoTypeArg<f64, Function>> {
        self.get_arg(i, |arg| {
            match arg {
                Object::AnonFunc(func) |
                Object::Function(func) => Ok(TwoTypeArg::U(func)),
                obj => {
                    match obj.as_f64(true) {
                        Some(n) => Ok(TwoTypeArg::T(n)),
                        None => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(obj))),
                    }
                },
            }
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
    /// COMオブジェクトを受ける
    pub fn get_as_comobject(&self, i: usize) -> BuiltInResult<ComObject> {
        self.get_arg(i, |obj| {
            match obj {
                Object::ComObject(com) => Ok(com),
                o => Err(BuiltinFuncError::new(UErrorMessage::BuiltinArgInvalid(o))),
            }
        })
    }

    fn get_as_structdef(&self, i: usize) -> BuiltInResult<StructDef> {
        self.get_arg(i, |obj| {
            match obj {
                Object::StructDef(sdef) => Ok(sdef),
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
    func: BuiltinFunction,
    desc: FuncDesc,
}

impl BuiltinFunctionSet {
    pub fn new<S:Into<String>>(name: S, len: i32, func: BuiltinFunction, desc: FuncDesc) -> Self {
        BuiltinFunctionSet {name: name.into(), len, func, desc}
    }
}

pub struct BuiltinFunctionSets {
    sets: Vec<BuiltinFunctionSet>
}

impl BuiltinFunctionSets {
    pub fn new() -> Self {
        BuiltinFunctionSets{sets: vec![]}
    }
    pub fn add(&mut self, name: &str, func: BuiltinFunction, desc: FuncDesc) {
        let len = desc.arg_len();
        self.sets.push(
            BuiltinFunctionSet::new(name, len, func, desc)
        );
    }
    fn append(&mut self, other: &mut Self) {
        self.sets.append(&mut other.sets)
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
impl Into<Vec<BuiltinName>> for BuiltinFunctionSets {
    fn into(self) -> Vec<BuiltinName> {
        self.sets.into_iter()
            .map(|set| BuiltinName::new_func(set.name, set.desc))
            .collect()
    }
}

// pub enum BuiltinNameType {
//     Const,
//     Function,
//     Other,
// }
pub enum BuiltinNameDesc {
    Function(FuncDesc),
    Const(String)
}
pub struct BuiltinName {
    name: String,
    // r#type: BuiltinNameType,
    desc: Option<BuiltinNameDesc>,
    hidden: bool,
}
impl BuiltinName {
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn desc(&self) -> &Option<BuiltinNameDesc> {
        &self.desc
    }
    pub fn is_visible(&self) -> bool {
        ! self.hidden
    }
    fn new_const(name: &str, desc: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            // r#type: BuiltinNameType::Const,
            desc: desc.map(|s| BuiltinNameDesc::Const(s.to_string())),
            hidden: false,
        }
    }
    fn new_func(name: String, desc: FuncDesc) -> Self {
        Self {
            name,
            // r#type: BuiltinNameType::Function,
            desc: Some(BuiltinNameDesc::Function(desc)),
            hidden: false,
        }
    }
}

pub fn get_builtin_names() -> Vec<BuiltinName> {
    // 登録方法が特殊なビルトイン定数
    let mut names: Vec<BuiltinName> = vec![
        // "PARAM_STR",
        // "GET_FUNC_NAME",
        ("TRY_ERRLINE", "エラー発生行情報"),
        ("TRY_ERRMSG", "エラーメッセージ"),
        ("G_TIME_YY", "年"),
        ("G_TIME_MM", "月"),
        ("G_TIME_DD", "日"),
        ("G_TIME_HH", "時"),
        ("G_TIME_NN", "分"),
        ("G_TIME_SS", "秒"),
        ("G_TIME_ZZ", "ミリ秒"),
        ("G_TIME_WW", "曜日 (0:日,1:月,2:火,3:水,4:木,5:金,6:土)"),
        ("G_TIME_YY2", "年 下二桁"),
        ("G_TIME_MM2", "月 二桁"),
        ("G_TIME_DD2", "日 二桁"),
        ("G_TIME_HH2", "時 二桁"),
        ("G_TIME_NN2", "分 二桁"),
        ("G_TIME_SS2", "秒 二桁"),
        ("G_TIME_ZZ2", "ミリ秒 三桁"),
        ("G_TIME_YY4", "年 四桁"),
        ("COM_ERR_FLG", "COMエラー抑制中にCOMエラーが発生したらTRUE"),
    ].into_iter().map(|(name, desc)| BuiltinName::new_const(name, Some(desc))).collect();
    let mut funcs: Vec<BuiltinName> = init_builtin_functions().into();
    names.append(&mut funcs);
    let mut consts: Vec<BuiltinName> = init_builtin_consts().into();
    names.append(&mut consts);
    names
}
pub fn get_builtin_string_names() -> Vec<String> {
    get_builtin_names().into_iter().map(|name| name.name().clone()).collect()
}

fn init_builtin_functions() -> BuiltinFunctionSets {
    let mut sets = builtin_func_sets();
    let mut window_low_sets = window_low::builtin_func_sets();
    let mut window_control_sets = window_control::builtin_func_sets();
    let mut text_control_sets = text_control::builtin_func_sets();
    let mut system_controls_sets = system_controls::builtin_func_sets();
    let mut math_sets = math::builtin_func_sets();
    let mut com_object_sets = com_object::builtin_func_sets();
    let mut browser_control_sets = browser_control::builtin_func_sets();
    let mut array_control_sets = array_control::builtin_func_sets();
    let mut dialog_sets = dialog::builtin_func_sets();
    let mut file_control_sets = file_control::builtin_func_sets();

    sets.append(&mut window_low_sets);
    sets.append(&mut window_control_sets);
    sets.append(&mut text_control_sets);
    sets.append(&mut system_controls_sets);
    sets.append(&mut math_sets);
    sets.append(&mut com_object_sets);
    sets.append(&mut browser_control_sets);
    sets.append(&mut array_control_sets);
    sets.append(&mut dialog_sets);
    sets.append(&mut file_control_sets);

    sets

}
fn init_builtin_consts() -> BuiltinConsts {
    let mut sets = BuiltinConsts {sets: vec![]};

    sets.append(&mut BuiltinConsts::new_str::<ObjectType>());
    // hashtbl
    sets.append(&mut BuiltinConsts::new::<HashTblEnum>());
    // window_low
    sets.append(&mut BuiltinConsts::new::<window_low::MouseButtonEnum>());
    sets.append(&mut BuiltinConsts::new::<window_low::KeyActionEnum>());
    // window_control
    sets.append(&mut BuiltinConsts::new_str::<window_control::SpecialWindowId>());
    sets.append(&mut BuiltinConsts::new::<window_control::CtrlWinCmd>());
    sets.append(&mut BuiltinConsts::new::<window_control::StatusEnum>());
    sets.append(&mut BuiltinConsts::new::<window_control::MonitorEnum>());
    sets.append(&mut BuiltinConsts::new_str::<window_control::GetHndConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::ClkConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::GetItemConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::AccConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::CurConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::ColConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::SldConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::GetStrConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::ImgConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::MorgTargetConst>());
    sets.append(&mut BuiltinConsts::new::<window_control::MorgContextConst>());
    #[cfg(feature="chkimg")]
    sets.append(&mut BuiltinConsts::new::<window_control::ChkImgOption>());

    // text control
    sets.append(&mut BuiltinConsts::new::<text_control::RegexEnum>());
    sets.append(&mut BuiltinConsts::new::<text_control::ErrConst>());
    sets.append(&mut BuiltinConsts::new::<text_control::StrconvConst>());
    sets.append(&mut BuiltinConsts::new::<text_control::FormatConst>());
    sets.append(&mut BuiltinConsts::new::<text_control::CodeConst>());
    // system_constrol
    sets.append(&mut BuiltinConsts::new::<system_controls::OsKind>());
    sets.append(&mut BuiltinConsts::new::<system_controls::KindOfOsResultType>());
    sets.append(&mut BuiltinConsts::new::<system_controls::LockHardExConst>());
    sets.append(&mut BuiltinConsts::new::<system_controls::SensorConst>());
    sets.append(&mut BuiltinConsts::new::<system_controls::ToggleKey>());
    sets.append(&mut BuiltinConsts::new::<system_controls::POFF>());
    sets.append(&mut BuiltinConsts::new::<system_controls::GTimeOffset>());
    sets.append(&mut BuiltinConsts::new::<system_controls::GTimeWeekDay>());
    sets.append(&mut BuiltinConsts::new::<system_controls::SetHotKey>());

    // math

    // key codes
    sets.append(&mut BuiltinConsts::new::<key_codes::VirtualKeyCode>());
    sets.append(&mut BuiltinConsts::new::<key_codes::VirtualMouseButton>());
    // com_object
    sets.append(&mut BuiltinConsts::new::<com_object::VarType>());
    sets.append(&mut BuiltinConsts::new::<com_object::ExcelConst>());
    // browser_control
    sets.append(&mut BuiltinConsts::new::<browser_control::BcEnum>());
    // array_control
    sets.append(&mut BuiltinConsts::new::<array_control::QsrtConst>());
    sets.append(&mut BuiltinConsts::new::<array_control::CalcConst>());
    // dialog.rs
    sets.append(&mut BuiltinConsts::new::<dialog::BtnConst>());
    sets.append(&mut BuiltinConsts::new::<dialog::SlctConst>());
    sets.append(&mut BuiltinConsts::new::<dialog::BalloonFlag>());
    sets.append(&mut BuiltinConsts::new::<dialog::FormOptions>());
    sets.append(&mut BuiltinConsts::new_str::<dialog::WindowClassName>());
    // SLCT_* 定数
    let mut slcts = BuiltinConsts {
        sets: (1..=31_u32).into_iter()
            .map(|n| {
                let val = 2_i32.pow(n - 1);
                let name = format!("SLCT_{n}");
                let desc = Some(format!("{n}番目の選択肢"));
                BuiltinConst::new(name, val.into(), desc)
            })
            .collect()
    };
    sets.append(&mut slcts);
    // file_control
    sets.append(&mut BuiltinConsts::new::<file_control::FileConst>());
    sets.append(&mut BuiltinConsts::new::<file_control::FileOrderConst>());

    // 特殊変数
    let mut special = special_variables();
    sets.append(&mut special);

    sets
}
pub fn init_builtins() -> Vec<NamedObject> {
    let mut vec = Vec::new();
    // ビルトイン関数
    init_builtin_functions().set(&mut vec);
    // ビルトイン定数
    init_builtin_consts().set(&mut vec);

    vec
}

struct BuiltinConst {
    name: String,
    value: Object,
    desc: Option<String>,
    hidden: bool,
}
impl BuiltinConst {
    fn new(name: String, value: Object, desc: Option<String>) -> Self {
        Self { name, value, desc, hidden: false }
    }
    fn new_with_hidden_flg(name: String, value: Object, desc: Option<String>, hidden: bool) -> Self {
        Self { name, value, desc, hidden }
    }
}
struct BuiltinConsts {
    sets: Vec<BuiltinConst>
}
impl BuiltinConsts {
    fn new<E: std::str::FromStr + VariantNames + EnumProperty + ToPrimitive>() -> Self {
        let mut sets = vec![];
        for name in E::VARIANTS {
            if let Ok(value) = E::from_str(name) {
                let hidden = value.get_str("hidden").is_some();
                let num = ToPrimitive::to_f64(&value).unwrap();
                let desc = value.get_str("desc").map(|s| format!("{s} ({num})"));
                sets.push(BuiltinConst::new_with_hidden_flg(name.to_ascii_uppercase(), num.into(), desc, hidden));
                // aliasがあればそれもセットする
                if let Some(alias) = value.get_str("alias") {
                    let desc = value.get_str("desc").map(|s| format!("{s} ({num})"));
                    sets.push(BuiltinConst::new_with_hidden_flg(alias.to_ascii_uppercase(), num.into(), desc, hidden));
                }
            }
        }
        Self {sets}
    }
    fn new_str<E: VariantNames + EnumProperty + std::str::FromStr>() -> Self {
        let mut sets = vec![];
        for name in E::VARIANTS {
            if let Ok(value) = E::from_str(name) {
                let hidden = value.get_str("hidden").is_some();
                let desc = value.get_str("desc").map(|s| s.to_string());
                if let Some(msg) = value.get_str("value") {
                    sets.push(BuiltinConst::new_with_hidden_flg(name.to_ascii_uppercase(), msg.into(), desc, hidden))
                } else {
                    let prefix = value.get_str("prefix").unwrap_or_default();
                    let suffix = value.get_str("suffix").unwrap_or_default();
                    let object = format!("{prefix}{name}{suffix}").into();
                    sets.push(BuiltinConst::new_with_hidden_flg(name.to_ascii_uppercase(), object, desc, hidden))
                }
            }
        }
        Self {sets}
    }
    fn append(&mut self, other: &mut Self) {
        self.sets.append(&mut other.sets)
    }
    fn set(self, vec: &mut Vec<NamedObject>) {
        let mut sets = self.sets.into_iter()
            .map(|set| NamedObject::new_builtin_const(set.name, set.value))
            .collect::<Vec<_>>();
        vec.append(&mut sets)
    }
}

impl Into<Vec<BuiltinName>> for BuiltinConsts {
    fn into(self) -> Vec<BuiltinName> {
        self.sets.into_iter()
            .map(|set| BuiltinName {
                name: set.name,
                desc: set.desc.map(|d| BuiltinNameDesc::Const(d)),
                hidden: set.hidden,
            })
            .collect()
    }
}

fn special_variables() -> BuiltinConsts {
    let mut sets = vec![];
    sets.push(BuiltinConst::new("GET_UWSC_PRO".into(), Object::Empty, Some("常にEMPTY".into())));
    sets.push(BuiltinConst::new("GET_UWSC_VER".into(), Object::Version(
        env!("CARGO_PKG_VERSION").parse::<Version>().unwrap_or(Version::new(0,0,0))
    ), Some("UWSCRのバージョン".into())));
    sets.push(BuiltinConst::new("GET_UWSCR_VER".into(), Object::Version(
        env!("CARGO_PKG_VERSION").parse::<Version>().unwrap_or(Version::new(0,0,0))
    ), Some("UWSCRのバージョン".into())));
    sets.push(BuiltinConst::new("GET_UWSC_DIR".into(),
        env::var("GET_UWSC_DIR").map_or(Object::Empty, |path| Object::String(path))
    , Some("uwscr.exeのあるディレクトリ".into())));
    sets.push(BuiltinConst::new("GET_UWSCR_DIR".into(),
        env::var("GET_UWSC_DIR").map_or(Object::Empty, |path| Object::String(path))
    , Some("uwscr.exeのあるディレクトリ".into())));
    sets.push(BuiltinConst::new("GET_UWSC_NAME".into(),
        env::var("GET_UWSC_NAME").map_or(Object::Empty, |path| Object::String(path))
    , Some("スクリプト名".into())));
    sets.push(BuiltinConst::new("GET_UWSCR_NAME".into(),
        env::var("GET_UWSC_NAME").map_or(Object::Empty, |path| Object::String(path))
    , Some("スクリプト名".into())));
    sets.push(BuiltinConst::new("GET_WIN_DIR".into(), Object::String(
        get_windows_directory()
    ), Some("Windowsディレクトリパス".into())));
    sets.push(BuiltinConst::new("GET_SYS_DIR".into(), Object::String(
        get_system_directory()
    ), Some("システムディレクトリパス".into())));
    sets.push(BuiltinConst::new("GET_APPDATA_DIR".into(), Object::String(
        get_special_directory(CSIDL_APPDATA as i32)
    ), Some("APPDATAパス".into())));

    sets.push(BuiltinConst::new("GET_CUR_DIR".into(), Object::DynamicVar(
        || Object::String(
            match env::current_dir() {
                Ok(p) => p.into_os_string().into_string().unwrap(),
                Err(_) => "".into()
            }
        )
    ), Some("カレントディレクトリ".into())));
    sets.push(BuiltinConst::new("G_MOUSE_X".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos() {
                Ok(p) => p.x as f64,
                Err(_) => -999999.0
            }
        )
    ), Some("マウスX座標".into())));
    sets.push(BuiltinConst::new("G_MOUSE_Y".into(), Object::DynamicVar(
        || Object::Num(
            match window_low::get_current_pos() {
                Ok(p) => p.y as f64,
                Err(_) => -999999.0
            }
        )
    ), Some("マウスY座標".into())));
    sets.push(BuiltinConst::new("G_SCREEN_W".into(), Object::DynamicVar(
        || Object::Num(get_screen_width() as f64)
    ), Some("画面幅".into())));
    sets.push(BuiltinConst::new("G_SCREEN_H".into(), Object::DynamicVar(
        || Object::Num(get_screen_height() as f64)
    ), Some("画面高さ".into())));
    sets.push(BuiltinConst::new("G_SCREEN_C".into(), Object::DynamicVar(
        || Object::Num(get_color_depth() as f64)
    ), Some("色数".into())));
    sets.push(BuiltinConst::new("THREAD_ID".into(), Object::DynamicVar(
        || unsafe { GetCurrentThreadId().into() }
    ), Some("スレッド識別子".into())));
    sets.push(BuiltinConst::new("THREAD_ID2".into(), Object::DynamicVar(
        || format!("{:?}", std::thread::current().id()).into()
    ), Some("スレッド識別子(rust)".into())));
    sets.push(BuiltinConst::new("IS_GUI_BUILD".into(), Object::Bool(
        cfg!(feature="gui")
    ), Some("GUIビルドかどうか".into())));
    sets.push(BuiltinConst::new("HAS_CHKIMG".into(), Object::Bool(
        cfg!(feature="chkimg")
    ), Some("chkimgが含まれるかどうか".into())));
    BuiltinConsts { sets }
}


/// 特殊ビルトイン関数をセット
fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("eval", builtin_eval, get_desc!(builtin_eval));
    sets.add("list_env", list_env, get_desc!(list_env));
    sets.add("list_module_member", list_module_member, get_desc!(list_module_member));
    sets.add("name_of", name_of, get_desc!(name_of));
    sets.add("const_as_string", const_as_string, get_desc!(const_as_string));
    sets.add("assert_equal", assert_equal, get_desc!(assert_equal));
    sets.add("raise", raise, get_desc!(raise));
    sets.add("type_of", type_of, get_desc!(type_of));
    sets.add("get_settings", get_settings, get_desc!(get_settings));
    sets.add("__p_a_n_i_c__", panic, get_desc!(panic));
    sets.add("get_struct_layout", get_struct_layout, get_desc!(get_struct_layout));
    sets
}

// 特殊ビルトイン関数の実体

#[builtin_func_desc(
    desc="文字列をUWSCR構文として評価する",
    rtype={desc="式の評価結果、文の場合EMPTY",types="値"},
    args=[
        {n="UWSCR構文", t="文字列", d="UWSCRの式または文を示す文字列を評価する、式の場合その評価結果を返す"}
    ],
)]
pub fn builtin_eval(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let script = args.get_as_string(0, None)?;
    evaluator.invoke_eval_script(&script)
        .map_err(|err| BuiltinFuncError::UError(err))
}

#[builtin_func_desc(
    desc="EvaluatorのEnvironmentを表示",
    args=[],
)]
pub fn list_env(evaluator: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let env = evaluator.env.get_env();
    Ok(env)
}

#[builtin_func_desc(
    desc="Moduleメンバを表示",
    args=[
        {n="module"}
    ],
)]
pub fn list_module_member(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let name = args.get_as_string(0, None)?;
    let members = evaluator.env.get_module_member(&name);
    Ok(members)
}

#[builtin_func_desc(
    desc="定数名を文字列にする",
    args=[
        {n="定数"}
    ],
)]
pub fn name_of(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let name = if let Some(Expression::Identifier(Identifier(name))) = args.get_expr(0) {
        evaluator.env.get_name_of_builtin_consts(&name)
    } else {
        Object::Empty
    };
    Ok(name)
}

#[builtin_func_desc(
    desc="値から定数名を得る",
    args=[
        {n="値",t="数値",d="定数値"},
        {n="定数名ヒント",t="文字列",d="定数名の一部を指定",o}
    ],
    rtype={desc="定数名"}
)]
pub fn const_as_string(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let value = args.get_as_object(0, None)?;
    let hint = args.get_as_string_or_empty(1)?;
    let name = evaluator.env.find_const(value, hint);
    Ok(name.into())
}

#[builtin_func_desc(
    desc="現在の設定をjson文字列で得る",
    args=[],
)]
pub fn get_settings(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let json = {
        let s = USETTINGS.lock().unwrap();
        s.get_current_settings_as_json()
    };
    Ok(Object::String(json))
}

#[builtin_func_desc(
    desc="実行時エラーを発生させる",
    args=[
        {n="msg",t="文字列",d="エラーメッセージ"},
        {n="title",t="文字列",d="エラータイトル",o}
    ],
)]
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

#[builtin_func_desc(
    desc="故意にpanicさせる",
    args=[
        {n="msg"}
    ],
)]
pub fn panic(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let msg = args.get_as_string(0, None)?;
    panic!("{msg}");
}

#[builtin_func_desc(
    desc="値の型を得る",
    args=[
        {n="値"}
    ],
)]
pub fn type_of(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg = args.get_as_object(0, None)?;
    let t = arg.get_type();
    Ok(Object::String(t.to_string()))
}

#[builtin_func_desc(
    desc="2つの引数が一致しない場合実行時エラーになる",
    args=[
        {n="arg1",t="すべて",d="比較される値"},
        {n="arg2",t="すべて",d="比較する値"},
    ],
)]
pub fn assert_equal(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arg1 = args.get_as_object(0, None)?;
    let arg2 = args.get_as_object(1, None)?;
    if arg1.is_equal(&arg2) {
        Ok(Object::Empty)
    } else {
        Err(BuiltinFuncError::new_with_kind(UErrorKind::AssertEqError, UErrorMessage::AssertEqLeftAndRight(arg1, arg2)))
    }
}

#[builtin_func_desc(
    desc="構造体レイアウトを表示する",
    args=[
        {n="構造体定義"}
    ],
)]
pub fn get_struct_layout(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let sdef = args.get_as_structdef(0)?;
    let layout = sdef.layout(None);
    Ok(layout.into())
}