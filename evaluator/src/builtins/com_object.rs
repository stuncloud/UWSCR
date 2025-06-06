use crate::object::*;
use crate::builtins::*;
use crate::Evaluator;
use util::settings::USETTINGS;

use std::ops::BitOr;
use std::sync::OnceLock;

use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("createoleobj", createoleobj, get_desc!(createoleobj));
    sets.add("getactiveoleobj", getactiveoleobj, get_desc!(getactiveoleobj));
    sets.add("getoleitem", getoleitem, get_desc!(getoleitem));
    sets.add("oleevent", oleevent, get_desc!(oleevent));
    sets.add("vartype", vartype, get_desc!(vartype));
    sets.add("safearray", safearray, get_desc!(safearray));
    sets.add("xlopen", xlopen, get_desc!(xlopen));
    sets.add("xlclose", xlclose, get_desc!(xlclose));
    sets.add("xlactivate", xlactivate, get_desc!(xlactivate));
    sets.add("xlsheet", xlsheet, get_desc!(xlsheet));
    sets.add("xlgetdata", xlgetdata, get_desc!(xlgetdata));
    sets.add("xlsetdata", xlsetdata, get_desc!(xlsetdata));
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum VarType {
    #[strum[props(desc="VT_EMPTY")]]
    VAR_EMPTY    = 0,
    #[strum[props(desc="VT_NULL")]]
    VAR_NULL     = 1,
    #[strum[props(desc="VT_I2")]]
    VAR_SMALLINT = 2,
    #[strum[props(desc="VT_I4")]]
    VAR_INTEGER  = 3,
    #[strum[props(desc="VT_R4")]]
    VAR_SINGLE   = 4,
    #[strum[props(desc="VT_R8")]]
    VAR_DOUBLE   = 5,
    #[strum[props(desc="VT_CY")]]
    VAR_CURRENCY = 6,
    #[strum[props(desc="VT_DATE")]]
    VAR_DATE     = 7,
    #[strum[props(desc="VT_BSTR")]]
    VAR_BSTR     = 8,
    #[strum[props(desc="VT_DISPATCH")]]
    VAR_DISPATCH = 9,
    #[strum[props(desc="VT_ERROR")]]
    VAR_ERROR    = 10,
    #[strum[props(desc="VT_BOOL")]]
    VAR_BOOLEAN  = 11,
    #[strum[props(desc="VT_VARIANT")]]
    VAR_VARIANT  = 12,
    #[strum[props(desc="VT_UNKNOWN")]]
    VAR_UNKNOWN  = 13,
    #[strum[props(desc="VT_I1")]]
    VAR_SBYTE    = 16,
    #[strum[props(desc="VT_UI1")]]
    VAR_BYTE     = 17,
    #[strum[props(desc="VT_UI2")]]
    VAR_WORD     = 18,
    #[strum[props(desc="VT_UI4")]]
    VAR_DWORD    = 19,
    #[strum[props(desc="VT_I8")]]
    VAR_INT64    = 20,
    #[strum[props(desc="VT_LPSTR")]]
    VAR_ASTR     = 256,
    #[strum[props(desc="VT_LPWSTR")]]
    VAR_USTR     = 258,
    #[strum[props(desc="UWSCR値型")]]
    VAR_UWSCR    = 512, // UWSCRデータ型
    #[strum[props(desc="VT_ARRAY")]]
    VAR_ARRAY    = 0x2000,
}
impl PartialEq<VarType> for u16 {
    fn eq(&self, other: &VarType) -> bool {
        match ToPrimitive::to_u16(other) {
            Some(n) => *self == n,
            None => false,
        }
    }
}
impl BitOr<VarType> for u16 {
    type Output = u16;

    fn bitor(self, rhs: VarType) -> Self::Output {
        match ToPrimitive::to_u16(&rhs) {
            Some(n) => n | self,
            None => 0,
        }
    }
}
impl From<VarType> for Object {
    fn from(val: VarType) -> Self {
        ToPrimitive::to_f64(&val).unwrap_or(0.0).into()
    }
}

static ALLOW_IE: OnceLock<bool> = OnceLock::new();
fn is_ie_allowed() -> bool {
    let allow_ie = ALLOW_IE.get_or_init(|| {
        let usettings = USETTINGS.lock().unwrap();
        usettings.options.allow_ie_object
    });
    *allow_ie
}

#[builtin_func_desc(
    desc="COMオブジェクトを返す"
    args=[
        {n="ID",t="文字列",d="COMオブジェクトのProgIDまたはCLSID"},
    ],
    rtype={desc="COMオブジェクト",types="COMオブジェクト"}
)]
fn createoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_string(0, None)?;
    // ignore IE
    let obj = ComObject::new(id, is_ie_allowed())?;
    Ok(Object::ComObject(obj))
}

#[builtin_func_desc(
    desc="動作中のCOMオブジェクトを得る"
    args=[
        {n="ID",t="文字列",d="COMオブジェクトのProgIDまたはCLSID"},
        {o,n="タイトル",t="文字列",d="ExcelやWordのウィンドウタイトル(部分一致)"},
        {o,n="n番目",t="数値",d="同一タイトルが複数ある場合に順番を指定"},
    ],
    rtype={desc="",types=""}
)]
fn getactiveoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_string(0, None)?;
    let title = args.get_as_string_or_empty(1)?;
    let nth = args.get_as_nth(2)?;
    let title = title.map(|title| ObjectTitle::new(title, nth));
    // ignore IE
    match ComObject::get_instance(id, title, is_ie_allowed())? {
        Some(obj) => Ok(Object::ComObject(obj)),
        None => Err(builtin_func_error(UErrorMessage::FailedToGetObject))
    }
}

#[builtin_func_desc(
    desc="コレクションを配列に変換"
    args=[
        {n="コレクション",t="COMオブジェクト",d="変換したいコレクション"},
    ],
    rtype={desc="変換された配列",types="配列"}
)]
pub fn getoleitem(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let vec = com.to_object_vec()?;
    Ok(Object::Array(vec))
}

#[builtin_func_desc(
    desc="COMのイベントを処理する関数を指定"
    args=[
        {n="オブジェクト",t="COMオブジェクト",d="イベントが発生するオブジェクト"},
        {n="インタフェース",t="文字列",d="イベントインタフェース名"},
        {n="イベント",t="文字列",d="イベント名"},
        {n="関数",t="関数または文字列",d="イベント発生時に実行する関数、またはその名前"},
    ],
    rtype={desc="",types=""}
)]
pub fn oleevent(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut com = args.get_as_comobject(0)?;
    if args.len() > 1 {
        // セット
        let interface = args.get_as_string(1, None)?;
        let event = args.get_as_string(2, None)?;
        let func = match args.get_as_function_or_string(3, true)? {
            Some(two) => match two {
                TwoTypeArg::T(name) => {
                    match evaluator.env.get_function(&name) {
                        Some(obj) => match obj {
                            Object::Function(func) => Ok(func),
                            _ => Err(builtin_func_error(UErrorMessage::IsNotUserFunction(name))),
                        },
                        None => Err(builtin_func_error(UErrorMessage::FunctionNotFound(name))),
                    }
                },
                TwoTypeArg::U(func) => Ok(func),
            },
            None => Err(builtin_func_error(UErrorMessage::FunctionRequired)),
        }?;
        com.set_event_handler(&interface, &event, func, evaluator.clone())?;
    } else {
        // 解除
        com.remove_event_handler()?;
    }
    Ok(Object::Empty)
    // Err(builtin_func_error(UErrorMessage::UnavailableFunction))
}

#[builtin_func_desc(
    desc="VARIANT型の取得や変換"
    sets=[
        "型取得",
        [
            {n="値",t="VARIANT",d="型を得たいVARIANT値"},
        ],
        "型変換",
        [
            {n="値",t="値",d="VARIANT値に変換したい値"},
            {n="VAR定数",t="定数",d="変換したい型を示す定数を指定"},
        ],
        "プロパティ型取得",
        [
            {n="COMオブジェクト",t="COMオブジェクト",d="プロパティの型を調べたいオブジェクト"},
            {n="プロパティ名",t="文字列",d="型を調べたいプロパティの名前"},
        ],
    ],
    rtype={desc="VARIANT型を示す定数、または変換後のVARIANT値",types="定数またはVARIANT"}
)]
fn vartype(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let obj = args.get_as_object(0, None)?;
    match args.get_as_int_or_string_or_empty::<u16>(1)? {
        Some(two) => match two {
            TwoTypeArg::T(prop) => {
                match obj {
                    Object::ComObject(com) => {
                        match com.get_prop_vt(&prop) {
                            Ok(vt) => Ok(vt.into()),
                            Err(_) => Ok(Object::Empty),
                        }
                    },
                    _ => Ok(Object::Empty)
                }
            },
            TwoTypeArg::U(vt) => {
                match obj {
                    Object::Variant(v) => {
                        let new = v.change_type(vt)?;
                        Ok(new.into())
                    }
                    obj => {
                        let v = Variant::try_from(obj)?;
                        let new = v.change_type(vt)?;
                        Ok(new.into())
                    }
                }
            },
        },
        None => match obj {
            Object::Variant(v) => {
                let vt = v.get_type();
                Ok(vt.into())
            },
            _ => Ok(VarType::VAR_UWSCR.into())
        },
    }
    // match (obj, vt) {
    //     (Object::Variant(variant), Some(vt)) => {
    //         let new = variant.change_type(vt)?;
    //         Ok(new.into())
    //     },
    //     (Object::Variant(variant), None) => {
    //         let vt = variant.get_type();
    //         Ok(vt.into())
    //     }
    //     (obj, Some(vt)) => {
    //         let variant = Variant::try_from(obj)?;
    //         let new = variant.change_type(vt)?;
    //         Ok(new.into())
    //     },
    //     (_, None) => Ok(VarType::VAR_UWSCR.into())
    // }
}

#[builtin_func_desc(
    desc="使用不可"
)]
fn safearray(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::Empty)
}

impl From<ComError> for BuiltinFuncError {
    fn from(e: ComError) -> Self {
        BuiltinFuncError::UError(e.into())
    }
}

// Excel

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum ExcelConst {
    #[strum[props(desc="実行中のExcelを使う、なければ新しく起動")]]
    XL_DEFAULT = 0,
    #[strum[props(desc="常に新しいExcelを起動")]]
    XL_NEW     = 1,
    #[strum[props(desc="Workbookオブジェクトを得る")]]
    XL_BOOK    = 2,
    XL_OOOC    = 3,
}

#[builtin_func_desc(
    desc="EXCELを起動しそのオブジェクトを返す"
    args=[
        {n="ファイル名",t="文字列",d="Excelファイル名、省略時は新規作成",o},
        {n="起動フラグ",t="定数",d=r#"以下から指定
- XL_DEFAULT: 起動済みのExcelがあればそれを使い、なければ新規起動
- XL_NEW: 常にExcelを新規に起動
- XL_BOOK: ApplicationではなくWorkbookオブジェクトを返す
"#,o},
        {n="パラメータ",t="文字列",d="'パラメータ名:=値'形式で指定",v=34},
    ],
    rtype={desc="Excelオブジェクト",types="COMオブジェクト"}
)]
pub fn xlopen(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let file = args.get_as_string_or_empty(0)?;
    let flg = args.get_as_const::<ExcelOpenFlag>(1, false)?.unwrap_or_default();
    let params = args.get_rest_as_string_array(2, 0)?;
    let com = Excel::open(file, flg, params)?;
    Ok(Object::ComObject(com))
}

#[builtin_func_desc(
    desc="Excelを終了する"
    sets=[
        "保存して終了",
        [
            {n="Excel",t="COMオブジェクト",d="Excel.ApplicationまたはWorkbookオブジェクト"},
            {n="ファイル名",t="文字列",d="保存先ファイル名、省略時は上書き保存",o},
        ],
        "保存せず終了",
        [
            {n="Excel",t="COMオブジェクト",d="Excel.ApplicationまたはWorkbookオブジェクト"},
            {n="TRUE",t="真偽値",d="保存せず終了する場合TRUE",o},
        ],
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn xlclose(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let Ok(excel) = Excel::new(com) else {
        return Ok(false.into());
    };
    let path = match args.get_as_string_or_bool(1, Some(TwoTypeArg::U(false)))? {
        TwoTypeArg::T(path) => Some(Some(path)),
        TwoTypeArg::U(b) => if b {
            None
        } else {
            Some(None)
        },
    };
    let result = excel.close(path).is_some();
    Ok(result.into())
}

#[builtin_func_desc(
    desc="シートをアクティブにする"
    args=[
        {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
        {n="シート識別子",t="文字列または数値",d="シート名または順番を示す数値(1から)"},
        {n="ブック識別子",t="文字列または数値",d="ブック名または順番を示す数値(1から)",o},
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn xlactivate(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let sheet_id = match args.get_as_f64_or_string(1)? {
        TwoTypeArg::T(s) => s.into(),
        TwoTypeArg::U(n) => n.into(),
    };
    let book_id = args.get_as_f64_or_string_or_empty(2)?
        .map(|two| match two {
            TwoTypeArg::T(s) => s.into(),
            TwoTypeArg::U(n) => n.into(),
        });
    let Ok(excel) = Excel::new(com) else {
        return Ok(false.into());
    };
    let result = excel.activate_sheet(sheet_id, book_id).is_some();
    Ok(result.into())
}

#[builtin_func_desc(
    desc="シートの追加・削除"
    args=[
        {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
        {n="シート識別子",t="文字列または数値",d="シート名、削除時のみ順番を示す数値(1から)が有効"},
        {n="削除フラグ",t="真偽値",d="TRUEなら該当シートを削除、FALSEならシート名を追加",o},
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn xlsheet(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let Ok(excel) = Excel::new(com) else {
        return Ok(false.into());
    };
    let sheet_id = args.get_as_f64_or_string(1)?;
    let delete = args.get_as_bool(2, Some(false))?;
    let result = if delete {
        let sheet_id = match args.get_as_f64_or_string(1)? {
            TwoTypeArg::T(s) => s.into(),
            TwoTypeArg::U(n) => n.into(),
        };
        excel.delete_sheet(sheet_id).is_some()
    } else {
        let sheet_id = match sheet_id {
            TwoTypeArg::T(s) => s,
            TwoTypeArg::U(n) => n.to_string(),
        };
        excel.add_sheet(&sheet_id).is_some()
    };
    Ok(result.into())
}

#[builtin_func_desc(
    desc="セルの値を取得"
    sets=[
        "範囲指定",
        [
            {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
            {n="範囲",t="文字列",d="A1形式で指定",o},
            {n="シート識別子",t="文字列または数値",d="シート名または順番を示す数値(1から)、省略時はアクティブシート",o},
        ],
        "行列指定",
        [
            {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
            {n="行",t="数値",d="行番号を指定"},
            {n="列",t="数値",d="列番号を指定"},
            {n="シート識別子",t="文字列または数値",d="シート名または順番を示す数値(1から)、省略時はアクティブシート",o},
        ],
    ],
    rtype={desc="範囲指定の場合配列、単一セル指定なら値を返す",types="配列または該当する値型"}
)]
pub fn xlgetdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let excel = Excel::new(com)?;
    let get_sheet = |index: usize| {
        args.get_as_f64_or_string_or_empty(index).map(|opt| {
            match opt {
                Some(tts) => match tts {
                    TwoTypeArg::T(s) => Some(s.into()),
                    TwoTypeArg::U(n) => Some(n.into()),
                },
                None => None
            }
        })
    };
    let value = match args.get_as_f64_or_string_or_empty(1)? {
        Some(tta) => match tta {
            TwoTypeArg::T(a1) => {
                let sheet = match get_sheet(2)? {
                    Some(obj) => Some(obj),
                    None => get_sheet(3)?,
                };
                excel.get_range_value(Some(a1), sheet)?
            },
            TwoTypeArg::U(row) => {
                let column = args.get_as_f64(2, None)?;
                let sheet = get_sheet(3)?;
                excel.get_cell_value(row, column, sheet)?
            },
        },
        None => {
            let sheet = match get_sheet(2)? {
                Some(obj) => Some(obj),
                None => get_sheet(3)?,
            };
            excel.get_range_value(None, sheet)?
        },
    };
    Ok(value)
}

#[builtin_func_desc(
    desc="セルに値をセット"
    sets=[
        "範囲指定",
        [
            {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
            {n="値",t="値または配列",d="入力値"},
            {n="範囲",t="文字列",d="A1形式で指定",o},
            {n="シート識別子",t="文字列または数値",d="シート名または順番を示す数値(1から)、省略時はアクティブシート",o},
            {n="文字色",t="数値",d="文字色を変更する場合にBGR値を指定",o},
            {n="背景色",t="数値",d="背景色を変更する場合にBGR値を指定",o},
        ],
        "行列指定",
        [
            {n="Excel",t="COMオブジェクト",d="ApplicationまたはWorkbookオブジェクト"},
            {n="値",t="値または配列",d="入力値"},
            {n="行",t="数値",d="行番号を指定"},
            {n="列",t="数値",d="列番号を指定"},
            {n="シート識別子",t="文字列または数値",d="シート名または順番を示す数値(1から)、省略時はアクティブシート",o},
            {n="文字色",t="数値",d="文字色を変更する場合にBGR値を指定",o},
            {n="背景色",t="数値",d="背景色を変更する場合にBGR値を指定",o},
        ],
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn xlsetdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let Ok(excel) = Excel::new(com) else {
        return Ok(false.into());
    };
    let value = args.get_as_object(1, None)?;

    let get_sheet = |index: usize| {
        args.get_as_f64_or_string_or_empty(index).map(|opt| {
            match opt {
                Some(tts) => match tts {
                    TwoTypeArg::T(s) => Some(s.into()),
                    TwoTypeArg::U(n) => Some(n.into()),
                },
                None => None
            }
        })
    };

    let range = match args.get_as_f64_or_string_or_empty(2)? {
        Some(tta) => match tta {
            TwoTypeArg::T(a1) => {
                let sheet_id = match get_sheet(3)? {
                    Some(obj) => Some(obj),
                    None => get_sheet(4)?,
                };
                excel.get_a1_range(Some(a1), sheet_id)
            },
            TwoTypeArg::U(row) => {
                let column = args.get_as_f64(3, None)?;
                let sheet_id = get_sheet(4)?;
                excel.get_cell_range(row, column, sheet_id)
            },
        },
        None => {
            let sheet_id = match get_sheet(3)? {
                Some(obj) => Some(obj),
                None => get_sheet(4)?,
            };
            excel.get_a1_range(None, sheet_id)
        },
    };

    if let Ok(range) = range {
        let color = args.get_as_int_or_empty(5)?;
        let bg_color = args.get_as_int_or_empty(6)?;

        let result = excel.set_range(value, range, color, bg_color).is_some();

        Ok(result.into())
    } else {
        Ok(false.into())
    }

}