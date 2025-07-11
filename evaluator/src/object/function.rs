use crate::environment::NamedObject;
use crate::error::{UError, UErrorKind, UErrorMessage, ParamTypeDetail};
use super::{Object, Module, ClassInstance};
use super::super::{EvalResult, Evaluator};

use parser::ast::{Expression, BlockStatement, FuncParam, ParamType, ParamKind};

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>, // Noneなら無名関数
    pub params: Vec<FuncParam>,
    pub body: BlockStatement,
    pub is_proc: bool,
    pub outer: Option<Arc<Mutex<Vec<NamedObject>>>>, // 無名関数にコピーするスコープ情報
}

impl Default for Function {
    fn default() -> Self {
        Self {
            name: None,
            params: vec![],
            body: vec![],
            is_proc: true,
            outer: None,
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
        self.params == other.params &&
        self.body == other.body &&
        self.is_proc == other.is_proc
    }
}

impl Function {
    pub fn new_named(name: String, params: Vec<FuncParam>, body: BlockStatement, is_proc: bool) -> Self {
        Self {
            name: Some(name),
            params,
            body,
            is_proc,
            outer: None,
        }
    }
    pub fn new_anon(params: Vec<FuncParam>, body: BlockStatement, is_proc: bool, outer: Arc<Mutex<Vec<NamedObject>>>) -> Self {
        Self {
            name: None,
            params,
            body,
            is_proc,
            outer: Some(outer),
        }
    }
    pub fn new_call(params: Vec<FuncParam>, body: BlockStatement) -> Self {
        Self {
            name: None,
            params,
            body,
            is_proc: true,
            outer: None,
        }
    }
    pub fn invoke(&self, evaluator: &mut Evaluator, mut arguments: Vec<(Option<Expression>, Object)>, this: Option<This>) -> EvalResult<Object> {
        let param_len = self.params.len();
        let mut params = self.params.clone();
        // if param_len > arguments.len() {
        //     // デフォルト引数が渡された場合
        //     arguments.resize(params.len(), (None, Object::EmptyParam));
        // } else if param_len < arguments.len() {
        //     // 可変長引数が渡された場合
        //     params.resize(arguments.len(), FuncParam::new_dummy());
        // }
        match arguments.len().cmp(&param_len) {
            std::cmp::Ordering::Less => {
                // デフォルト引数が渡された場合
                arguments.resize(params.len(), (None, Object::EmptyParam));
            },
            std::cmp::Ordering::Equal => {},
            std::cmp::Ordering::Greater => {
                // 可変長引数が渡された場合
                params.resize(arguments.len(), FuncParam::new_dummy());
            },
        }

        // 無名関数ならローカルコープをコピーする
        if self.outer.is_some() && self.name.is_none() {
            let outer_clone = self.outer.clone().unwrap();
            let outer_local = outer_clone.lock().unwrap();
            evaluator.env.copy_scope(outer_local.clone());
        } else {
            // 通常の関数なら新しいスコープを作る
            evaluator.env.new_scope();
        }
        /* GET_FUNC_NAME */
        evaluator.env.set_get_func_name(self.name.clone());

        /* 引数の処理 */

        // 引数定義と渡された引数をくっつける
        let list = params.into_iter().zip(arguments);
        // 可変長引数
        let mut variadic = vec![];
        // 可変長引数の変数名
        let mut variadic_name = None;

        // for (_i, (param, (arg_expr, obj))) in list.enumerate() {
        for (param, (arg_expr, obj)) in list {
            let name = param.name();
            // 引数種別チェック
            // デフォルト値の評価もここでやる
            let value = match param.kind {
                ParamKind::Identifier => {
                    if arg_expr.is_none() {
                        return Err(UError::new(
                            UErrorKind::FuncCallError,
                            UErrorMessage::FuncArgRequired(name)
                        ))
                    }
                    obj
                },
                ParamKind::Reference => {
                    match arg_expr {
                        Some(expr) => match expr {
                            Expression::Array(_, _) |
                            Expression::Assign(_, _) |
                            Expression::CompoundAssign(_, _, _) => return Err(UError::new(
                                UErrorKind::FuncCallError,
                                UErrorMessage::FuncInvalidArgument(name),
                            )),
                            e => {
                                // 型チェック
                                evaluator.is_valid_type(&param, &obj)?;
                                // パラメータ変数に参照を代入
                                if let Some(outer) = evaluator.env.clone_outer() {
                                    evaluator.env.define_param_to_local(&name, Object::Reference(e, outer))?;
                                } else {
                                    Err(UError::new(UErrorKind::EvaluatorError, UErrorMessage::NoOuterScopeFound))?;
                                }
                            }
                        },
                        None => {
                            Err(UError::new(
                                UErrorKind::FuncCallError,
                                UErrorMessage::FuncArgRequired(name)
                            ))?;
                        }
                    }
                    // 通常のパラメータ変数への代入は行わないためcontinueする
                    continue;
                },
                ParamKind::Default(ref e) => {
                    if Object::EmptyParam.is_equal(&obj) {
                        evaluator.eval_expression(e.clone())?
                    } else {
                        obj
                    }
                },
                ParamKind::Variadic => {
                    variadic_name = Some(name);
                    variadic.push(obj);
                    continue;
                },
                ParamKind::Dummy => {
                    if variadic.is_empty() {
                        return Err(UError::new(
                            UErrorKind::FuncCallError,
                            UErrorMessage::FuncTooManyArguments(param_len)
                        ))
                    }
                    variadic.push(obj);
                    continue;
                },
            };
            // 型チェック
            evaluator.is_valid_type(&param, &value)?;

            // 可変長引数でなければローカル変数を定義
            if variadic.is_empty() {
                evaluator.env.define_param_to_local(&name, value)?;
            }
        }
        // 可変長引数のローカル変数を定義
        if variadic_name.is_some() && !variadic.is_empty() {
            evaluator.env.define_param_to_local(&variadic_name.unwrap(), Object::Array(variadic))?;
        }

        // モジュール・クラスインスタンスであればthisとglobalをセットする
        if let Some(this) = this {
            evaluator.env.set_this_and_global(this);
        }

        // functionならresult変数を初期化
        if ! self.is_proc {
            evaluator.env.set_result();
        }

        /* 関数を実行 */
        let block = self.body.clone();
        if let Err(e) = evaluator.eval_block_statement(block) {
            // 関数ブロックでエラーが発生した場合は、関数の実行事態ががなかったことになる
            // - 戻り値を返さない
            // - 参照渡しされた変数は更新されない
            evaluator.env.restore_scope(&None);
            return Err(e);
        }

        /* 戻り値 */
        let result = if self.is_proc {
            Object::Empty
        } else {
            evaluator.get_variable("result").unwrap_or_default()
        };

        // 関数スコープを抜ける
        evaluator.env.restore_scope(&self.outer);

        /* 結果を返す */
        Ok(result)
    }
}

impl Evaluator {
    fn is_valid_type(&self, param: &FuncParam, obj: &Object) -> EvalResult<()> {
        match param.param_type {
            ParamType::Any => return Ok(()),
            ParamType::String => match obj {
                Object::String(_) |
                Object::ExpandableTB(_) => return Ok(()),
                _ => {}
            },
            ParamType::Number => if let Object::Num(_) = obj { return Ok(()) },
            ParamType::Bool => if let Object::Bool(_) = obj { return Ok(()) },
            ParamType::Array => if let Object::Array(_) = obj { return Ok(()) },
            ParamType::HashTbl => if let Object::HashTbl(_) = obj { return Ok(()) },
            ParamType::Function => match obj {
                Object::MemberCaller(_, _) |
                Object::Function(_) |
                Object::AnonFunc(_) => return Ok(()),
                _ => {}
            },
            ParamType::UObject => if let Object::UObject(_) = obj { return Ok(()) },
            ParamType::UserDefinition(ref name) => match obj {
                Object::Instance(arc) => {
                    let m = arc.lock().unwrap();
                    if m.name.eq_ignore_ascii_case(name) {
                        return Ok(());
                    }
                },
                Object::Num(n) => {
                    if let Some(Object::Enum(e)) = self.get_variable(name) {
                        if e.include(*n) {
                            return Ok(());
                        }
                    }
                },
                _ => {}
            },
        }
        Err(UError::new(
            UErrorKind::FuncCallError,
            UErrorMessage::InvalidParamType(param.name(), param.param_type.clone().into())
        ))
    }
}

impl From<ParamType> for ParamTypeDetail {
    fn from(p: ParamType) -> Self {
        match p {
            ParamType::Any => ParamTypeDetail::Any,
            ParamType::String => ParamTypeDetail::String,
            ParamType::Number => ParamTypeDetail::Number,
            ParamType::Bool => ParamTypeDetail::Bool,
            ParamType::Array => ParamTypeDetail::Array,
            ParamType::HashTbl => ParamTypeDetail::HashTbl,
            ParamType::Function => ParamTypeDetail::Function,
            ParamType::UObject => ParamTypeDetail::UObject,
            ParamType::UserDefinition(s) => ParamTypeDetail::UserDefinition(s),
        }
    }
}

pub enum This {
    Module(Arc<Mutex<Module>>),
    Class(Arc<Mutex<ClassInstance>>),
}