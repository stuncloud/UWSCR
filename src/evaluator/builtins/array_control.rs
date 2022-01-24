use crate::evaluator::object::*;
use crate::evaluator::builtins::*;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("join", 5, join);
    sets
}

fn join(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arr = args.get_as_array(0, None)?;
    let sep = args.get_as_string(1, Some(" ".into()))?;
    let empty_flg = args.get_as_bool(2, Some(false))?;
    let from = args.get_as_int::<usize>(3, Some(0))?;
    let to = args.get_as_int::<usize>(4, Some(arr.len() - 1))?;
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