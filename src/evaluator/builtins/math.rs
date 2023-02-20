use crate::evaluator::builtins::*;
use crate::evaluator::Evaluator;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("isnan", 1, isnan);
    sets
}


pub fn isnan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(n) = args.get_as_num::<f64>(0, None) {
        Ok(n.is_nan().into())
    } else {
        Ok(false.into())
    }
}