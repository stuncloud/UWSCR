use crate::evaluator::object::*;

pub fn copy(args: Vec<Object>) -> Object {
    Object::String(format!("{}", args.len()))
}