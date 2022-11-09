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
    sets.add("resize", 3, resize);
    sets.add("slice", 3, slice);
    sets.add("split", 5, split);
    sets.add("calcarray", 4, calcarray);
    sets.add("setclear", 2, setclear);
    sets.add("shiftarray", 2, shiftarray);
    sets
}

fn join(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arr = args.get_as_array_include_hashtbl(0, None, false)?;
    let sep = args.get_as_string(1, Some(" ".into()))?;
    let empty_flg = args.get_as_bool(2, Some(false))?;
    let from = args.get_as_int::<usize>(3, Some(0))?;
    let to = args.get_as_int::<usize>(4, Some(arr.len() - 1))?;
    if to >= arr.len() {
        return Err(builtin_func_error(
            UErrorMessage::IndexOutOfBounds((to as f64).into())));
    }
    let slice = &arr[from..=to];
    let joined = slice.iter()
            .map(|o| o.to_string())
            .filter(|s| if empty_flg {s.len() > 0} else {true})
            .collect::<Vec<String>>()
            .join(&sep);
    Ok(BuiltinFuncReturnValue::Result(Object::String(joined)))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum QsrtConst {
    QSRT_A = 0,
    QSRT_D = 1,
    QSRT_UNICODEA = 2,
    QSRT_UNICODED = 3,
    QSRT_NATURALA = 4,
    QSRT_NATURALD = 5,
}
impl Default for QsrtConst {
    fn default() -> Self {
        Self::QSRT_A
    }
}
impl Into<qsort::SortOrder> for QsrtConst {
    fn into(self) -> qsort::SortOrder {
        match self {
            QsrtConst::QSRT_A => qsort::SortOrder::Ascending,
            QsrtConst::QSRT_D => qsort::SortOrder::Descending,
            QsrtConst::QSRT_UNICODEA => qsort::SortOrder::UnicodeAsc,
            QsrtConst::QSRT_UNICODED => qsort::SortOrder::UnicodeDsc,
            QsrtConst::QSRT_NATURALA => qsort::SortOrder::NaturalAsc,
            QsrtConst::QSRT_NATURALD => qsort::SortOrder::NaturalDsc,
        }
    }
}

pub fn qsort(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut array = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let order = args.get_as_const::<QsrtConst>(1, false)?.unwrap_or_default();
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
    let qsort = qsort::Qsort::new(order.into());
    qsort.sort(&mut array, &mut arrays);
    Ok(BuiltinFuncReturnValue::Qsort(expr, array, exprs, arrays))
}

pub fn reverse(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);

    arr.reverse();
    Ok(BuiltinFuncReturnValue::Reference { refs: vec![(expr, Object::Array(arr))], result: Object::Empty})
}

pub fn resize(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let size = args.get_as_int_or_empty::<i32>(1)?;
    let value = args.get_as_object(2, Some(Object::Empty))?;
    if let Some(n) = size {
        let new_len = if n < 0 {
            0
        } else {
            n + 1
        } as usize;
        arr.resize(new_len, value);
        let i = arr.len() as isize - 1;
        Ok(BuiltinFuncReturnValue::Reference {
            refs: vec![(expr, Object::Array(arr))],
            result: Object::Num(i as f64)
        })
    } else {
        let i = arr.len() as isize - 1;
        Ok(BuiltinFuncReturnValue::Result(Object::Num(i as f64)))
    }
}

pub fn slice(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut base = args.get_as_array(0, None)?;
    let len = base.len() as i32;
    let from = args.get_as_int(1, Some(0_i32))?
        .min(len)
        .max(0) as usize;
    let to = args.get_as_int(2, Some(len-1))?
        .min(len-1)
        .max(0) as usize;

    let arr = if to >= from {
        base.drain(from..=to).collect::<Vec<_>>()
    } else {
        vec![]
    };
    Ok(BuiltinFuncReturnValue::Result(Object::Array(arr)))
}

pub fn split(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let str = args.get_as_string(0, None)?;
    let delimiter = args.get_as_string(1, Some(" ".to_string()))?;
    let empty_flg = args.get_as_bool(2, Some(false))?;
    let num_flg = args.get_as_bool(3, Some(false))?;
    let csv_flg = args.get_as_bool(4, Some(false))?;

    if csv_flg {
        let delimiter_byte = delimiter.bytes().next().unwrap_or(b',');
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(delimiter_byte)
            .quote(b'"')
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(str.as_bytes());
        match reader.records()
            .next()
            .map(|record| {
                match record {
                    Ok(r) => {
                        let vec = r.into_iter()
                            .map(|s| Object::String(s.into()))
                            .collect::<Vec<_>>();
                        Ok(vec)
                    },
                    Err(e) => Err(builtin_func_error(UErrorMessage::Any(e.to_string()))),
                }
            }) {
            Some(r) => r.map(|v| BuiltinFuncReturnValue::Result(Object::Array(v))),
            None => Err(builtin_func_error(UErrorMessage::Any("CSV conversion error".into()))),
        }
    } else {
        let split = str.split(delimiter.as_str());
        let mut arr = if num_flg {
            split.map(|s| {
                match s.parse::<f64>() {
                    Ok(n) => Object::Num(n),
                    Err(_) => Object::String("".into()),
                }
            })
            .collect::<Vec<_>>()
        } else {
            split.map(|s| Object::String(s.into())).collect::<Vec<_>>()
        };
        if empty_flg {
            arr.retain(|o| {
                if let Object::String(s) = o {
                    s.len() > 0
                } else {
                    true
                }
            })
        }
        Ok(BuiltinFuncReturnValue::Result(Object::Array(arr)))
    }

}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive, PartialEq)]
pub enum CalcConst {
    CALC_ADD = 1,
    CALC_MIN = 2,
    CALC_MAX = 3,
    CALC_AVR = 4,
}

pub fn calcarray(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut base = args.get_as_array(0, None)?;
    let len = base.len() as i32;
    let maybe_const = args.get_as_const(1, true)?;
    let from = args.get_as_int(2, Some(0_i32))?
        .min(len)
        .max(0) as usize;
    let to = args.get_as_int(3, Some(len-1))?
        .min(len-1)
        .max(0) as usize;

    let arr = if to >= from {
        base.drain(from..=to).collect::<Vec<_>>()
    } else {
        vec![]
    };

    let Some(calc_const) = maybe_const else {
        return Ok(BuiltinFuncReturnValue::Result(Object::Empty));
    };
    let calc_func = match calc_const {
        CalcConst::CALC_ADD |
        CalcConst::CALC_AVR => |a: f64, b: f64| a + b,
        CalcConst::CALC_MIN => |a: f64, b: f64| a.min(b),
        CalcConst::CALC_MAX => |a: f64, b: f64| a.max(b),
    };

    let nums = arr.into_iter()
        .filter_map(|o| if let Object::Num(n) = o {Some(n)} else {None});
    let len = nums.clone().count() as f64;
    let result = nums.reduce(calc_func);

    match result {
        Some(n) => if calc_const == CalcConst::CALC_AVR {
            Ok(BuiltinFuncReturnValue::Result(Object::Num(n / len)))
        } else {
            Ok(BuiltinFuncReturnValue::Result(Object::Num(n)))
        },
        None => Ok(BuiltinFuncReturnValue::Result(Object::Empty)),
    }
}

pub fn setclear(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let value = args.get_as_object(1, Some(Object::Empty))?;

    arr.fill(value);

    Ok(BuiltinFuncReturnValue::Reference {
        refs: vec![(expr, Object::Array(arr))],
        result: Object::Empty
    })
}

pub fn shiftarray(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let shift = args.get_as_int(1, None::<i32>)?;
    if shift == 0 {
        return Ok(BuiltinFuncReturnValue::Result(Object::Empty))
    }

    let len = arr.len();
    let rotate = shift.abs() as usize;
    arr.resize(len + rotate, Object::Empty);
    if shift > 0 {
        arr.rotate_right(rotate);
    } else if shift < 0 {
        arr.rotate_left(rotate);
    }
    arr.resize(len, Object::Empty);

    Ok(BuiltinFuncReturnValue::Reference {
        refs: vec![(expr, Object::Array(arr))],
        result: Object::Empty
    })
}