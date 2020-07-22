use crate::evaluator::object::*;

pub fn getid(args: Vec<Object>) -> Object {
    Object::Num(1.0)
}

pub fn clkitem(args: Vec<Object>) -> Object {
    Object::Bool(true)
}