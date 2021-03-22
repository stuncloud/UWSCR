use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("isnan", 1, isnan);
    sets
}


pub fn isnan(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(n) = get_num_argument_value::<f64>(&args, 0, None) {
        Ok(Object::Bool(n.is_nan()))
    } else {
        Ok(Object::Bool(false))
    }
}