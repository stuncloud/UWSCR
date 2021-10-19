use crate::evaluator::object::*;
use crate::evaluator::builtins::*;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("join", 5, join);
    sets
}

fn join(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arr = get_array_argument_value(&args, 0, None)?;
    let sep = get_string_argument_value(&args, 1, Some(" ".into()))?;
    let empty_flg = get_bool_argument_value(&args, 2, Some(false))?;
    let from = get_non_float_argument_value::<usize>(&args, 3, Some(0))?;
    let to = get_non_float_argument_value::<usize>(&args, 4, Some(arr.len() - 1))?;
    if to >= arr.len() {
        return Err(builtin_func_error(
            UErrorMessage::IndexOutOfBounds((to as f64).into()), args.name()));
    }
    let slice = &arr[from..=to];
    let joined = slice.iter()
            .map(|o| o.to_string())
            .filter(|s| if empty_flg {s.len() > 0} else {true})
            .collect::<Vec<String>>()
            .join(&sep);
    Ok(Object::String(joined))
}