mod qsort;

use crate::evaluator::object::*;
use crate::evaluator::builtins::*;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("join", 5, join);
    sets.add("qsort", 10, qsort);
    sets.add("reverse", 1, reverse);
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

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum QsrtConst {
    QSRT_A = 0,
    QSRT_D = 1,
    QSRT_UNICODEA = 2,
    QSRT_UNICODED = 3,
    QSRT_NATURALA = 4,
    QSRT_NATURALD = 5,
}

pub fn qsort(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut array = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let order = args.get_as_const(1, Some(qsort::SortOrder::Ascending))?;
    let mut arrays = [
        args.get_as_array_or_empty(2)?,
        args.get_as_array_or_empty(3)?,
        args.get_as_array_or_empty(4)?,
        args.get_as_array_or_empty(5)?,
        args.get_as_array_or_empty(6)?,
        args.get_as_array_or_empty(7)?,
        args.get_as_array_or_empty(8)?,
        args.get_as_array_or_empty(9)?,
    ];
    let exprs = [
        args.get_expr(2),
        args.get_expr(3),
        args.get_expr(4),
        args.get_expr(5),
        args.get_expr(6),
        args.get_expr(7),
        args.get_expr(8),
        args.get_expr(9),
    ];
    let qsort = qsort::Qsort::new(order);
    qsort.sort(&mut array, &mut arrays);
    Ok(Object::SpecialFuncResult(SpecialFuncResultType::Qsort(expr, array, exprs, arrays)))
}

pub fn reverse(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);

    arr.reverse();
    Ok(Object::SpecialFuncResult(SpecialFuncResultType::Reference(vec![
        (expr, Object::Array(arr))
    ])))
}