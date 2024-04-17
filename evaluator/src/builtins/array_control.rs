mod qsort;

use crate::object::*;
use crate::builtins::*;
use crate::Evaluator;

use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("join", join, get_desc!(join));
    sets.add("qsort", qsort, get_desc!(qsort));
    sets.add("reverse", reverse, get_desc!(reverse));
    sets.add("resize", resize, get_desc!(resize));
    sets.add("slice", slice, get_desc!(slice));
    sets.add("split", split, get_desc!(split));
    sets.add("calcarray", calcarray, get_desc!(calcarray));
    sets.add("setclear", setclear, get_desc!(setclear));
    sets.add("shiftarray", shiftarray, get_desc!(shiftarray));
    sets
}

#[builtin_func_desc(
    desc="配列要素を区切り文字で結合した文字列を返す",
    args=[
        {n="配列",t="配列",d="結合したい配列"},
        {n="区切り文字",t="文字列",d="結合時の区切り文字",o},
        {n="空文字除外",t="真偽値",d="TRUEなら空文字要素を含めない",o},
        {n="開始",t="数値",d="結合したい範囲の開始位置",o},
        {n="終了",t="数値",d="結合したい範囲の終了位置",o},
    ],
    rtype={desc="結合した文字列"}
)]
fn join(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    Ok(joined.into())
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum QsrtConst {
    #[strum[props(desc="昇順")]]
    QSRT_A = 0,
    #[strum[props(desc="降順")]]
    QSRT_D = 1,
    #[strum[props(desc="Unicode昇順")]]
    QSRT_UNICODEA = 2,
    #[strum[props(desc="Unicode降順")]]
    QSRT_UNICODED = 3,
    #[strum[props(desc="自然順ソート昇順")]]
    QSRT_NATURALA = 4,
    #[strum[props(desc="自然順ソート降順")]]
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

#[builtin_func_desc(
    desc="配列要素を並び替える"
    args=[
        {n="配列",t="配列",d="ソートする配列"},
        {n="ソート順",t="定数",d=r#"以下のいずれかを指定
- QSRT_A: 昇順
- QSRT_D: 降順
- QSRT_UNICODEA: UNICODE文字列順 昇順
- QSRT_UNICODED: UNICODE文字列順 降順
- QSRT_NATURALA: 数値順 昇順
- QSRT_NATURALD: 数値順 降順
"#,o},
        {n="連動ソート配列",t="配列",d="ソートする配列に連動してソートされる配列",v=8},
    ],
)]
pub fn qsort(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

    evaluator.invoke_qsort_update(expr, array, exprs, arrays)?;
    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="配列順を反転させる"
    args=[
        {n="arr",t="配列(参照渡し)",d="順序を反転させたい配列"},
    ],
)]
pub fn reverse(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);

    arr.reverse();

    evaluator.update_reference(vec![(expr, Object::Array(arr))])
        .map_err(|err| BuiltinFuncError::UError(err))?;

    Ok(Object::Empty)
}

fn new_empty_array(size: usize) -> Vec<Object> {
    vec![Object::Empty; size]
}
fn get_array_dimension(arr: &Vec<Object>) -> Option<Vec<usize>> {
    arr.iter()
        .map(|o| {
            if let Object::Array(arr2) = o {
                let mut hoge = get_array_dimension(arr2)?;
                hoge.insert(0, arr2.len());
                Some(hoge)
            } else {
                Some(vec![0])
            }
        })
        .reduce(|a, b| {
            match (a, b) {
                (Some(a), Some(b)) => (a == b).then_some(a),
                _ => None
            }
        })
        .flatten()
}
fn get_array_object_from_dimension(dim: Vec<usize>) -> Object {
    let mut dim = dim;
    dim.reverse();
    let mut result = None::<Vec<_>>;
    for size in dim {
        if size > 0 {
            if let Some(default) = &mut result {
                let new = vec![Object::Array(default.clone()); size];
                *default = new;
            } else {
                result = Some(new_empty_array(size));
            }
        }
    }
    result.map(|arr| Object::Array(arr)).unwrap_or(Object::Empty)
}
fn get_resize_default_value(arr: &Vec<Object>, value: Option<Object>) -> Object {
    if let Some(default) = value {
        default
    } else if let Some(dim) = get_array_dimension(arr) {
        get_array_object_from_dimension(dim)
    } else {
        Object::Empty
    }
}

#[builtin_func_desc(
    desc="配列サイズを変更する"
    args=[
        {n="arr",t="配列(参照渡し)",d="サイズ変更を行う配列"},
        {n="変更後最大インデックス",t="数値",d="この値+1のサイズにリサイズする",o},
        {n="初期値",t="値",d="追加される要素の初期値",o},
    ],
    rtype={desc="最大インデックス値 (配列サイズ-1)",types="数値"}
)]
pub fn resize(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let size = args.get_as_int_or_empty::<i32>(1)?;
    if let Some(n) = size {
        let value = args.get_as_object_or_empty(2)?;
        let default = get_resize_default_value(&arr, value);
        let new_len = if n < 0 {
            0
        } else {
            n + 1
        } as usize;
        arr.resize(new_len, default);
        let i = arr.len() as isize - 1;

        evaluator.update_reference(vec![(expr, Object::Array(arr))])
            .map_err(|err| BuiltinFuncError::UError(err))?;

        Ok(Object::Num(i as f64))
    } else {
        let i = arr.len() as isize - 1;
        Ok(Object::Num(i as f64))
    }
}

#[builtin_func_desc(
    desc="配列の一部をコピー"
    args=[
        {n="コピー元",t="配列",d="コピー元配列"},
        {n="開始",t="数値",d="開始位置、省略時は0",o},
        {n="終了",t="数値",d="終了位置、省略時は配列末尾まで",o},
    ],
    rtype={desc="コピーされた配列",types="配列"}
)]
pub fn slice(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
    Ok(Object::Array(arr))
}

#[builtin_func_desc(
    desc="文字列を区切り文字で分割して配列にする"
    args=[
        {n="文字列",t="文字列",d="分割する文字列"},
        {n="区切り文字",t="文字列",d="デフォルトは半角スペース",o},
        {n="空文字除外",t="真偽値",d="TRUEなら区切り文字間が空の場合に無視、デフォルトFALSE",o},
        {n="数値変換",t="真偽値",d="TRUEなら文字列を数値変換する、デフォルトFALSE",o},
        {n="CSV変換",t="真偽値",d="TRUEならCSVとして変換する、デフォルトFALSE",o},
    ],
    rtype={desc="分割された配列",types="配列"}
)]
pub fn split(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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

        let record = reader.records()
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
            });
        record
            .map(|r| r.map(|arr| Object::Array(arr)))
            .unwrap_or(Err(builtin_func_error(UErrorMessage::Any("CSV conversion error".into()))))
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
        Ok(Object::Array(arr))
    }

}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, PartialEq)]
pub enum CalcConst {
    #[strum[props(desc="合計値")]]
    CALC_ADD = 1,
    #[strum[props(desc="最小値")]]
    CALC_MIN = 2,
    #[strum[props(desc="最大値")]]
    CALC_MAX = 3,
    #[strum[props(desc="平均値")]]
    CALC_AVR = 4,
}

#[builtin_func_desc(
    desc="配列内の数値で計算を行う"
    args=[
        {n="配列",t="配列",d="数値配列"},
        {n="計算方法",t="定数",d=r#"以下のいずれかを指定
- CALC_ADD: 合計値を得る
- CALC_MIN: 最小値を得る
- CALC_MAX: 最大値を得る
- CALC_AVR: 平均値を得る
"#},
        {n="開始",t="数値",d="開始位置",o},
        {n="終了",t="数値",d="終了位置",o},
    ],
    rtype={desc="計算後の値",types="数値"}
)]
pub fn calcarray(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
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
        return Ok(Object::Empty);
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
            Ok(Object::Num(n / len))
        } else {
            Ok(Object::Num(n))
        },
        None => Ok(Object::Empty),
    }
}

#[builtin_func_desc(
    desc="配列を指定値で埋める"
    args=[
        {n="配列",t="配列(参照渡し)",d="値を埋める配列"},
        {n="値",t="値",d="各要素を埋める値、省略時はEMPTY",o},
    ],
)]
pub fn setclear(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let value = args.get_as_object(1, Some(Object::Empty))?;

    arr.fill(value);

    evaluator.update_reference(vec![(expr, Object::Array(arr))])
        .map_err(|err| BuiltinFuncError::UError(err))?;

    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="配列要素をシフトさせる"
    args=[
        {n="配列",t="配列(参照渡し)",d="対象の配列"},
        {n="シフト値",t="数値",d="正の値なら後方、負なら前方に各要素をずらす"},
    ],
)]
pub fn shiftarray(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut arr = args.get_as_array(0, None)?;
    let expr = args.get_expr(0);
    let shift = args.get_as_int(1, None::<i32>)?;
    if shift == 0 {
        return Ok(Object::Empty)
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

    evaluator.update_reference(vec![(expr, Object::Array(arr))])
        .map_err(|err| BuiltinFuncError::UError(err))?;

    Ok(Object::Empty)
}