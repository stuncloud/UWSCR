use crate::ast::{Expression, BlockStatement, FuncParam, ParamType, ParamKind, Literal};
use crate::evaluator::environment::{NamedObject};
use crate::error::evaluator::{UError, UErrorKind, UErrorMessage, ParamTypeDetail};
use super::{Object, Module};
use super::super::{EvalResult, Evaluator};


use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>, // Noneなら無名関数
    pub params: Vec<FuncParam>,
    pub body: BlockStatement,
    pub is_proc: bool,
    pub module: Option<Arc<Mutex<Module>>>, // module, classの実装
    pub outer: Option<Arc<Mutex<Vec<NamedObject>>>>, // 無名関数にコピーするスコープ情報
}

impl Default for Function {
    fn default() -> Self {
        Self {
            name: None,
            params: vec![],
            body: vec![],
            is_proc: true,
            module: None,
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
    pub fn invoke(&self, evaluator: &mut Evaluator, mut arguments: Vec<(Option<Expression>, Object)>, is_instance: bool) -> EvalResult<Object> {
        let param_len = self.params.len();
        let mut params = self.params.clone();
        if param_len > arguments.len() {
            // デフォルト引数が渡された場合
            arguments.resize(params.len(), (None, Object::EmptyParam));
        } else if param_len < arguments.len() {
            // 可変長引数が渡された場合
            params.resize(arguments.len(), FuncParam::new_dummy());
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

        /* 引数の処理 */

        // 引数定義と渡された引数をくっつける
        let list = params.into_iter().zip(arguments.into_iter());
        // 可変長引数
        let mut variadic = vec![];
        // 可変長引数の変数名
        let mut variadic_name = None;
        // 参照渡し引数
        let mut reference = vec![];

        for (_, (param, (arg_expr, obj))) in list.enumerate() {
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
                    match arg_expr.unwrap() {
                        Expression::Array(_, _) |
                        Expression::Assign(_, _) |
                        Expression::CompoundAssign(_, _, _) => return Err(UError::new(
                            UErrorKind::FuncCallError,
                            UErrorMessage::FuncInvalidArgument(name),
                        )),
                        e => reference.push((name.clone(), e))
                    }
                    obj
                },
                ParamKind::Array(b) => {
                    let e = arg_expr.unwrap();
                    match e {
                        Expression::Identifier(_) |
                        Expression::Index(_, _, _) |
                        Expression::DotCall(_, _) => if b {
                            reference.push((name.clone(), e))
                        },
                        Expression::Literal(Literal::Array(_)) => {},
                        _ => return Err(UError::new(
                            UErrorKind::FuncCallError,
                            UErrorMessage::FuncInvalidArgument(name),
                        )),
                    }
                    obj
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
                    if variadic.len() < 1 {
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
            if variadic.len() < 1 {
                evaluator.env.define_local(&name, value)?;
            }
        }
        // 可変長引数のローカル変数を定義
        if variadic_name.is_some() && variadic.len() > 0 {
            evaluator.env.define_local(&variadic_name.unwrap(), Object::Array(variadic))?;
        }

        // モジュール・クラスインスタンスのプライメートメンバをセット
        if let Some(ref m) = self.module {
            evaluator.env.set_module_private_member(m);
        }

        // functionならresult変数を初期化
        if ! self.is_proc {
            evaluator.env.assign("result".into(), Object::Empty)?;
        }

        /* 関数を実行 */
        let block = self.body.clone();
        if let Err(e) = evaluator.eval_block_statement(block) {
            // 関数ブロックでエラーが発生した場合は、関数の実行事態ががなかったことになる
            // - 戻り値を返さない
            // - 参照渡しされた変数は更新されない
            // - 関数内で作られたインスタンスを自動破棄しない
            evaluator.env.restore_scope(&None);
            return Err(e);
        }

        /* 戻り値 */
        let result = if is_instance {
            // この戻り値は使われない
            Object::Empty
        } else if self.is_proc {
            Object::Empty
        } else {
            evaluator.env.get_variable(&"result".into(), true).unwrap_or_default()
        };

        /* 参照渡しの処理 */
        let mut ref_values = vec![];

        // 参照渡しされた変数の値を得ておく
        for (name, expr) in reference {
            let obj = evaluator.env.get_variable(&name, true).unwrap();
            ref_values.push((expr, obj));
        }

        // 関数スコープを抜ける
        evaluator.env.restore_scope(&self.outer);

        // 呼び出し元スコープで参照渡しした変数の値を更新する
        for (expr, value) in ref_values {
            match expr {
                Expression::Identifier(_) |
                Expression::Index(_, _, _) |
                Expression::DotCall(_, _) => {
                    evaluator.eval_assign_expression(expr, value)?;
                },
                _ => {}
            }
        }


        /* 結果を返す */
        Ok(result)
    }

    pub fn set_module(&mut self, m: Arc<Mutex<Module>>) {
        self.module = Some(m)
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
            ParamType::Number => match obj {
                Object::Num(_) => return Ok(()),
                _ => {}
            },
            ParamType::Bool => match obj {
                Object::Bool(_) => return Ok(()),
                _ => {}
            },
            ParamType::Array => match obj {
                Object::Array(_) => return Ok(()),
                _ => {}
            },
            ParamType::HashTbl => match obj {
                Object::HashTbl(_) => return Ok(()),
                _ => {}
            },
            ParamType::Function => match obj {
                Object::Function(_) |
                Object::AnonFunc(_) => return Ok(()),
                _ => {}
            },
            ParamType::UObject => match obj {
                Object::UObject(_) => return Ok(()),
                _ => {}
            },
            ParamType::UserDefinition(ref name) => match obj {
                Object::Instance(ref arc) => {
                    let m = arc.lock().unwrap();
                    if m.name.to_ascii_lowercase() == name.to_ascii_lowercase() {
                        return Ok(());
                    }
                },
                Object::Num(n) => {
                    if let Some(Object::Enum(e)) = self.env.get_variable(name, false) {
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