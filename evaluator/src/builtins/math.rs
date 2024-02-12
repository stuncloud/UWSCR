use crate::builtins::*;
use crate::Evaluator;

use rand::Rng;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("isnan", 1, isnan);
    sets.add("random", 1, random);
    sets.add("abs", 1, abs);
    sets.add("zcut", 1, zcut);
    sets.add("int", 1, int);
    sets.add("ceil", 1, ceil);
    sets.add("round", 2, round);
    sets.add("sqrt", 1, sqrt);
    sets.add("power", 2, power);
    sets.add("exp", 1, exp);
    sets.add("ln", 1, ln);
    sets.add("logn", 2, logn);
    sets.add("sin", 1, sin);
    sets.add("cos", 1, cos);
    sets.add("tan", 1, tan);
    sets.add("arcsin", 1, arcsin);
    sets.add("arccos", 1, arccos);
    sets.add("arctan", 1, arctan);
    sets
}


pub fn isnan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(n) = args.get_as_f64(0, None) {
        Ok(n.is_nan().into())
    } else {
        Ok(false.into())
    }
}

pub fn random(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None)?;
    let r = rand::thread_rng().gen_range(0..n);
    Ok(r.into())
}

pub fn abs(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    let r = n.abs();
    Ok(r.into())
}

pub fn zcut(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    let r = if n < 0 {0} else {n};
    Ok(r.into())
}

pub fn int(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    Ok(n.into())
}

pub fn ceil(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    Ok(n.ceil().into())
}

pub fn round(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let d = args.get_as_int(1, Some(0i32))?;
    let rounded = if d == 0 {
        n.round()
    } else {
        let factor = 10.0_f64.powi(d.abs());
        if d > 0 {
            (n / factor).round() * factor
        } else {
            (n * factor).round() / factor
        }
    };
    Ok(rounded.into())
}

pub fn sqrt(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let s = n.sqrt();
    Ok(s.into())
}

pub fn power(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let e = args.get_as_f64(1, None)?;
    let p = n.powf(e);
    Ok(p.into())
}

pub fn exp(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let e = n.exp();
    Ok(e.into())
}

pub fn ln(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let l = n.ln();
    Ok(l.into())
}

pub fn logn(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let base = args.get_as_f64(0, None)?;
    let n = args.get_as_f64(1, None)?;
    let l = n.log(base);
    Ok(l.into())
}

pub fn sin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.sin();
    Ok(r.into())
}

pub fn cos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.cos();
    Ok(r.into())
}

pub fn tan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.tan();
    Ok(r.into())
}

pub fn arcsin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.asin();
    Ok(r.into())
}

pub fn arccos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.acos();
    Ok(r.into())
}

pub fn arctan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.atan();
    Ok(r.into())
}