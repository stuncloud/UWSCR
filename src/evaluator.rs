pub mod object;
pub mod env;
pub mod builtins;

use crate::ast::*;
use crate::evaluator::env::*;
use crate::evaluator::object::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct  Evaluator {
    env: Rc<RefCell<Env>>,
}

impl Evaluator {
    pub fn new(env: Rc<RefCell<Env>>) -> Self {
        Evaluator {env}
    }

    fn is_truthy(obj: Object) -> bool {
        match obj {
            Object::Empty | Object::Num(0.0) | Object::Bool(false) => false,
            _ => true
        }
    }

    fn error(msg: String) -> Object {
        Object::Error(msg)
    }

    fn is_error(obj: &Object) -> bool {
        match obj {
            Object::Error(_) => true,
            _ => false
        }
    }

    pub fn eval(&mut self, program: Program) -> Option<Object> {
        let mut result = None;

        for statement in program {
            if statement == Statement::Blank {
                continue;
            }
            match self.eval_statement(statement) {
                Some(Object::Error(msg)) => return Some(Object::Error(msg)),
                obj => result = obj,
            }
        }

        result
    }

    fn eval_block_statement(&mut self, block: BlockStatement) -> Option<Object> {
        let mut result = None;
        for statement in block {
            if statement == Statement::Blank {
                continue;
            }
            match self.eval_statement(statement) {
                Some(Object::Error(msg)) => return Some(Object::Error(msg)),
                obj => result = obj,
            }
        }

        result
    }

    fn eval_statement(&mut self, statement: Statement) -> Option<Object> {
        match statement {
            Statement::Dim(identifier, expression) => {
                let value = match self.eval_expression(expression) {
                    Some(o) => o,
                    None => return None
                };

                if Self::is_error(&value) {
                    Some(value)
                } else {
                    let Identifier(name) = identifier;
                    self.env.borrow_mut().set(name, &value);
                    None
                }
            },
            Statement::Expression(e) => self.eval_expression(e),
            _ => None
        }
    }

    fn eval_if_statement(&mut self) -> Option<Object> {None}

    fn eval_expression(&mut self, expression: Expression) -> Option<Object> {
        match expression {
            Expression::Identifier(i) => Some(self.eval_identifier(i)),
            Expression::Literal(l) => Some(self.eval_literal(l)),
            Expression::Prefix(p, r) => if let Some(right) = self.eval_expression(*r) {
                Some(self.eval_prefix_expression(p, right))
            } else {
                None
            },
            Expression::Infix(i, l, r) => {
                let left = self.eval_expression(*l);
                let right = self.eval_expression(*r);
                if left.is_some() && right.is_some() {
                    Some(self.eval_infix_expression(i, left.unwrap(), right.unwrap()))
                } else {
                    None
                }
            },
            Expression::Index(l, i) => {
                let left = self.eval_expression(*l);
                let index = self.eval_expression(*i);
                if left.is_some() && index.is_some() {
                    Some(self.eval_index_expression(left.unwrap(), index.unwrap()))
                } else {
                    None
                }
            },
            Expression::HashTbl(i, e) => {
                None
            },
            Expression::Function {params, body} => {
                Some(Object::Function(params, body, Rc::clone(&self.env)))
            },
            Expression::FuncCall {func, args} => {
                Some(self.eval_function_call_expression(func, args))
            },
            Expression::Assign(l, r) => {
                let value = match self.eval_expression(*r) {
                    Some(o) => o,
                    None => return None
                };
                if Self::is_error(&value) {
                    Some(value)
                } else {
                    let i = match *l {
                        Expression::Identifier(i) => i,
                        _ => return None
                    };
                    let Identifier(name) = i;
                    self.env.borrow_mut().set(name, &value);
                    None
                }
            },
            Expression::Ternary {condition, consequence, alternative} => {
                self.eval_ternary_expression(*condition, *consequence, *alternative)
            }
        }
    }

    fn eval_identifier(&mut self, identifier: Identifier) -> Object {
        let Identifier(name) = identifier;

        match self.env.borrow_mut().get(name.clone()) {
            Some(o) => o,
            None => Object::Error(String::from(format!("identifier not found: {}", name)))
        }
    }

    fn eval_prefix_expression(&mut self, prefix: Prefix, right: Object) -> Object {
        match prefix {
            Prefix::Not => self.eval_not_operator_expression(right),
            Prefix::Minus => self.eval_minus_operator_expression(right),
            Prefix::Plus => self.eval_plus_operator_expression(right),
        }
    }

    fn eval_not_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Bool(true) => Object::Bool(false),
            Object::Bool(false) => Object::Bool(true),
            Object::Empty => Object::Bool(true),
            Object::Num(0.0) => Object::Bool(true),
            _ => Object::Bool(false)
        }
    }

    fn eval_minus_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Num(n) => Object::Num(-n),
            _ => Self::error(format!("unknown operator: -{}", right))
        }
    }

    fn eval_plus_operator_expression(&mut self, right: Object) -> Object {
        match right {
            Object::Num(n) => Object::Num(n),
            _ => Self::error(format!("unknown operator: +{}", right))
        }
    }

    fn eval_index_expression(&mut self, left: Object, index: Object) -> Object {
        match left {
            Object::Array(ref a) => if let Object::Num(i) = index {
                self.eval_array_index_expression(a.clone(), i as i64)
            } else {
                Self::error(format!("imvalid index: {}[{}]", left, index))
            },
            Object::Hash(ref h) => match index {
                Object::Num(n) => {
                    match h.get(&Object::String(n.to_string())) {
                        Some(o) => o.clone(),
                        None => Object::Empty
                    }
                },
                Object::Bool(_) |
                Object::String(_) => match h.get(&index) {
                    Some(o) => o.clone(),
                    None => Object::Empty
                },
                Object::Error(_) => index,
                _ => Self::error(format!("invalid key:{}", index))
            },
            _ => Self::error(format!("unknown operator: {} {}", left, index))
        }
    }

    fn eval_array_index_expression(&mut self, array: Vec<Object>, index: i64) -> Object {
        let max = (array.len() as i64) - 1;
        if index < 0 || index > max {
            return Self::error(format!("index out of bounds: {}", index));
        }

        match array.get(index as usize) {
            Some(o) => o.clone(),
            None => Object::Empty
        }
    }

    fn eval_infix_expression(&mut self, infix: Infix, left: Object, right: Object) -> Object {
        match left {
            Object::Num(l) => {
                match right {
                    Object::Num(n) => {
                        self.eval_infix_number_expression(infix, l, n)
                    },
                    Object::String(s) => {
                        self.eval_infix_string_expression(infix, l.to_string(), s)
                    },
                    _ => {
                        Self::error(format!("mismatched type: {} {} {}", left, infix, right))
                    }
                }
            },
            Object::String(l) => {
                let r = match right {
                    Object::Num(n) => n.to_string(),
                    Object::String(s) => s,
                    _ => return Self::error(format!("mismatched type: {} {} {}", l, infix, right))
                };
                self.eval_infix_string_expression(infix, l, r)
            },
            Object::Bool(l) => if let Object::Bool(r) = right {
                self.eval_infix_logical_operator_expression(infix, l, r)
            } else {
                Self::error(format!("mismatched type: {} {} {}", left, infix, right))
            }
            _ => Self::error(format!("bad operator: {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_number_expression(&mut self, infix: Infix, left: f64, right: f64) -> Object {
        match infix {
            Infix::Plus => Object::Num(left + right),
            Infix::Minus => Object::Num(left - right),
            Infix::Multiply => Object::Num(left * right),
            Infix::Divide => Object::Num(left / right),
            Infix::LessThan => Object::Bool(left < right),
            Infix::LessThanEqual => Object::Bool(left <= right),
            Infix::GreaterThan => Object::Bool(left > right),
            Infix::GreaterThanEqual => Object::Bool(left >= right),
            Infix::Equal => Object::Bool(left == right),
            Infix::NotEqual => Object::Bool(left != right),
            Infix::And => Object::Num((left as i64 & right as i64) as f64),
            Infix::Or => Object::Num((left as i64 | right as i64) as f64),
            Infix::Xor => Object::Num((left as i64 ^ right as i64) as f64),
            _ => Self::error(format!("bad operator {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_string_expression(&mut self, infix: Infix, left: String, right: String) -> Object {
        match infix {
            Infix::Plus => Object::String(format!("{}{}", left, right)),
            _ => Self::error(format!("bad operator: {} {} {}", left, infix, right))
        }
    }

    fn eval_infix_logical_operator_expression(&mut self, infix: Infix, left: bool, right: bool) -> Object {
        match infix {
            Infix::And => Object::Bool(left && right),
            Infix::Or => Object::Bool(left || right),
            _ => Self::error(format!("bad operator {} {} {}", left, infix, right))
        }
    }

    fn eval_literal(&mut self, literal: Literal) -> Object {
        match literal {
            Literal::Num(value) => Object::Num(value),
            Literal::String(value) => Object::String(value),
            Literal::Bool(value) => Object::Bool(value),
            Literal::Array(objects) => self.eval_array_literal(objects),
            Literal::Hash(pairs) => self.eval_hash_literal(pairs),
            Literal::Empty => Object::Empty,
            Literal::Null => Object::Null,
            Literal::Nothing => Object::Nothing,
        }
    }

    fn eval_array_literal(&mut self, objects: Vec<Expression>) -> Object {
        Object::Array(
            objects.iter().map(
                |e| self.eval_expression(e.clone()).unwrap()
            ).collect::<Vec<_>>()
        )
    }

    fn eval_hash_literal(&mut self, pairs: Vec<(Expression, Expression)>) -> Object {
        Object::Empty
    }

    fn eval_function_call_expression(&mut self, func: Box<Expression>, args: Vec<Expression>) -> Object {
        let args = args.iter().map(
            |e| self.eval_expression(e.clone()).unwrap()
        ).collect::<Vec<_>>();

        let (params, body, env) = match self.eval_expression(*func) {
            Some(Object::Function(p, b, e)) => (p, b, e),
            Some(Object::Builtin(expected_param_len, f)) => {
                if expected_param_len < 0 || expected_param_len > args.len() as i32 {
                    return f(args);
                } else {
                    return Self::error(format!(
                        "too much arguments ({}). max count of arguments should be {}",
                        args.len(), expected_param_len
                    ));
                }
            },
            Some(o) => return Self::error(format!(
                "{} is not a function", o
            )),
            None => return Object::Empty,
        };

        if params.len() != args.len() {
            return Self::error(format!(
                "length of arguments should be {}, not {}",
                params.len(),
                args.len()
            ));
        }

        let current_env = Rc::clone(&self.env);
        let mut scoped_env = Env::new_with_outer(Rc::clone(&env));
        let list = params.iter().zip(args.iter());
        for (_, (identifier, o)) in list.enumerate() {
            let Identifier(name) = identifier.clone();
            scoped_env.set(name, o);
        }

        self.env = Rc::new(RefCell::new(scoped_env));
        let object = self.eval_block_statement(body);
        self.env = current_env;

        match object {
            Some(o) => o,
            None => Object::Empty
        }
    }

    fn eval_ternary_expression(&mut self, condition: Expression, consequence: Expression, alternative: Expression) -> Option<Object> {
        let condition = match self.eval_expression(condition) {
            Some(c) => c,
            None => return None
        };
        if Self::is_truthy(condition) {
            self.eval_expression(consequence)
        } else {
            self.eval_expression(alternative)
        }
    }

}