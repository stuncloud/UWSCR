use crate::builtins::*;
use crate::Evaluator;

use rand::Rng;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("isnan", isnan, get_desc!(isnan));
    sets.add("random", random, get_desc!(random));
    sets.add("abs", abs, get_desc!(abs));
    sets.add("zcut", zcut, get_desc!(zcut));
    sets.add("int", int, get_desc!(int));
    sets.add("ceil", ceil, get_desc!(ceil));
    sets.add("round", round, get_desc!(round));
    sets.add("sqrt", sqrt, get_desc!(sqrt));
    sets.add("power", power, get_desc!(power));
    sets.add("exp", exp, get_desc!(exp));
    sets.add("ln", ln, get_desc!(ln));
    sets.add("logn", logn, get_desc!(logn));
    sets.add("sin", sin, get_desc!(sin));
    sets.add("cos", cos, get_desc!(cos));
    sets.add("tan", tan, get_desc!(tan));
    sets.add("arcsin", arcsin, get_desc!(arcsin));
    sets.add("arccos", arccos, get_desc!(arccos));
    sets.add("arctan", arctan, get_desc!(arctan));
    sets
}

#[builtin_func_desc(
    desc="値がNaNかどうか",
    args=[
        {n="値",t="数値",d="調べたい値"},
    ],
    rtype={desc="NaNであればTRUE",types="真偽値"}
)]
pub fn isnan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(n) = args.get_as_f64(0, None) {
        Ok(n.is_nan().into())
    } else {
        Ok(false.into())
    }
}

#[builtin_func_desc(
    desc="0以上n未満の整数をランダムに返す",
    args=[
        {n="n",t="数値",d="ランダム値の上限+1の値を指定"},
    ],
    rtype={desc="ランダム値",types="数値"}
)]
pub fn random(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None)?;
    let r = rand::thread_rng().gen_range(0..n);
    Ok(r.into())
}

#[builtin_func_desc(
    desc="絶対値を得る",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="絶対値",types="数値"}
)]
pub fn abs(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    let r = n.abs();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="マイナス値は0にする",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="0以上の整数",types="数値"}
)]
pub fn zcut(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    let r = if n < 0 {0} else {n};
    Ok(r.into())
}

#[builtin_func_desc(
    desc="小数点以下切り落とし",
    args=[
        {n="n",t="数値",d="小数値"},
    ],
    rtype={desc="整数値",types="数値"}
)]
pub fn int(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_int(0, None::<i64>)?;
    Ok(n.into())
}


#[builtin_func_desc(
    desc="小数点以下切り上げ",
    args=[
        {n="n",t="数値",d="小数値"},
    ],
    rtype={desc="整数",types="数値"}
)]
pub fn ceil(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    Ok(n.ceil().into())
}

#[builtin_func_desc(
    desc="指定桁数で丸め",
    args=[
        {n="n",t="数値",d="入力値"},
        {o,n="桁",t="数値",d="丸め桁、マイナスなら小数点以下の桁数"},
    ],
    rtype={desc="丸め値",types="数値"}
)]
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

#[builtin_func_desc(
    desc="平方根",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="平方根、場合によりNaN",types="数値"}
)]
pub fn sqrt(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let s = n.sqrt();
    Ok(s.into())
}

#[builtin_func_desc(
    desc="累乗",
    args=[
        {n="n",t="数値",d="基数"},
        {n="e",t="数値",d="指数"},
    ],
    rtype={desc="累乗",types="数値"}
)]
pub fn power(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let e = args.get_as_f64(1, None)?;
    let p = n.powf(e);
    Ok(p.into())
}

#[builtin_func_desc(
    desc="指数関数",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="数値",types="数値"}
)]
pub fn exp(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let e = n.exp();
    Ok(e.into())
}

#[builtin_func_desc(
    desc="自然対数",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="数値",types="数値"}
)]
pub fn ln(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let l = n.ln();
    Ok(l.into())
}

#[builtin_func_desc(
    desc="bを底としたnの対数",
    args=[
        {n="b",t="数値",d="底"},
        {n="n",t="数値",d="数値"},
    ],
    rtype={desc="対数",types="数値"}
)]
pub fn logn(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let base = args.get_as_f64(0, None)?;
    let n = args.get_as_f64(1, None)?;
    let l = n.log(base);
    Ok(l.into())
}

#[builtin_func_desc(
    desc="サイン",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn sin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.sin();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="コサイン",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn cos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.cos();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="タンジェント",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn tan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.tan();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="アークサイン",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn arcsin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.asin();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="アークコサイン",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn arccos(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.acos();
    Ok(r.into())
}

#[builtin_func_desc(
    desc="アークタンジェント",
    args=[
        {n="n",t="数値",d="入力値"},
    ],
    rtype={desc="ラジアン",types="数値"}
)]
pub fn arctan(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let n = args.get_as_f64(0, None)?;
    let r = n.atan();
    Ok(r.into())
}