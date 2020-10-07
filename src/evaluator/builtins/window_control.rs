use crate::evaluator::object::*;

pub fn getid(args: Vec<Object>) -> Object {
    Object::Num(args.len() as f64)
}

pub fn clkitem(args: Vec<Object>) -> Object {
    Object::Bool(args.len() > 0)
}