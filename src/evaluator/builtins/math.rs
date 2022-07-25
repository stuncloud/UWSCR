use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("isnan", 1, isnan);
    sets
}


pub fn isnan(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(n) = args.get_as_num::<f64>(0, None) {
        Ok(BuiltinFuncReturnValue::Result(Object::Bool(n.is_nan())))
    } else {
        Ok(BuiltinFuncReturnValue::Result(Object::Bool(false)))
    }
}