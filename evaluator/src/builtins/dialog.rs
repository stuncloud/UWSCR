use crate::{LOGPRINTWIN, Evaluator};
use crate::builtins::*;
use crate::object::Object;
use crate::error::UErrorMessage::UWindowError;
use crate::gui::*;
use util::settings::USETTINGS;

use std::sync::Mutex;
use std::rc::Rc;
use std::cell::RefCell;

use strum_macros::{EnumString, VariantNames, EnumProperty};
use num_derive::{ToPrimitive, FromPrimitive};
use once_cell::sync::Lazy;

static MSGBOX_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static INPUT_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static SLCTBOX_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static DIALOG_TITLE: Lazy<String> = Lazy::new(|| {
    let settings = USETTINGS.lock().unwrap();
    match &settings.options.dlg_title {
        Some(title) => title.to_string(),
        None => match std::env::var("GET_UWSC_NAME") {
            Ok(name) => format!("UWSCR - {}", name),
            Err(_) => format!("UWSCR"),
        },
    }
});
static DIALOG_FONT_FAMILY: Lazy<FontFamily> = Lazy::new(|| {
    let s = USETTINGS.lock().unwrap();
    FontFamily::new(&s.options.default_font.name, s.options.default_font.size)
});
thread_local! {
    pub static THREAD_LOCAL_BALLOON: Rc<RefCell<Option<Balloon>>> = Rc::new(RefCell::new(None));
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, VariantNames, EnumProperty)]
pub enum WindowClassName {
    #[strum(props(value="UWSCR.MsgBox"))]
    CLASS_MSGBOX,
    #[strum(props(value="UWSCR.Input"))]
    CLASS_INPUTBOX,
    #[strum(props(value="UWSCR.Slctbox"))]
    CLASS_SLCTBOX,
    #[strum(props(value="UWSCR.Popup"))]
    CLASS_POPUPMENU,
    #[strum(props(value="UWSCR.Balloon"))]
    CLASS_BALLOON,
    #[strum(props(value="UWSCR.LogPrintWin"))]
    CLASS_LOGPRINTWIN,
    #[strum(props(value="UWSCR.Form"))]
    CLASS_FORM,
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum BtnConst {
    BTN_YES    = 4,
    BTN_NO     = 8,
    BTN_OK     = 1,
    BTN_CANCEL = 2,
    BTN_ABORT  = 16,
    BTN_RETRY  = 32,
    BTN_IGNORE = 64,
}

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("msgbox", msgbox, get_desc!(msgbox));
    sets.add("input", input, get_desc!(input));
    sets.add("logprint", logprint, get_desc!(logprint));
    sets.add("slctbox", slctbox, get_desc!(slctbox));
    sets.add("popupmenu", popupmenu, get_desc!(popupmenu));
    sets.add("balloon", balloon, get_desc!(balloon));
    sets.add("fukidasi", balloon, get_desc!(balloon));
    sets.add("createform", createform, get_desc!(createform));
    sets.add("getformdata", getformdata, get_desc!(getformdata));
    sets.add("setformdata", setformdata, get_desc!(setformdata));
    sets
}

#[builtin_func_desc(
    desc="printウィンドウの表示状態を設定"
    args=[
        {n="状態",t="真偽値",d="TRUEならprintウィンドウを表示、FALSEなら非表示にし以後表示されないようにする"},
        {o,n="X",t="数値",d="表示する場合そのX座標"},
        {o,n="Y",t="数値",d="表示する場合そのY座標"},
        {o,n="幅",t="数値",d="表示する場合その幅"},
        {o,n="高さ",t="数値",d="表示する場合その高さ"},
    ],
)]
pub fn logprint(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let flg = args.get_as_bool(0, None)?;
    let x = args.get_as_int_or_empty(1)?;
    let y = args.get_as_int_or_empty(2)?;
    let width = args.get_as_int_or_empty(3)?;
    let height = args.get_as_int_or_empty(4)?;
    if let Some(m) = LOGPRINTWIN.get(){
        let mut guard = m.lock().unwrap();
        let lp = guard.as_mut()
            .map_err(|e| builtin_func_error(e.message.clone()))?;
        lp.set_visibility(flg, flg);
        lp.set_new_pos(x, y, width, height);
    }
    Ok(Object::Empty)
}

fn get_dlg_point(args: &BuiltinFuncArgs, i: (usize,usize), point: &Lazy<Mutex<(Option<i32>, Option<i32>)>>) -> BuiltInResult<(Option<i32>, Option<i32>)> {
    let x = match args.get_as_int_or_empty(i.0)? {
        Some(-1) => {
            point.lock().unwrap().0
        },
        Some(n) => Some(n),
        None => None,
    };
    let y = match args.get_as_int_or_empty(i.1)? {
        Some(-1) => {
            point.lock().unwrap().1
        },
        Some(n) => Some(n),
        None => None,
    };
    Ok((x, y))
}
fn set_dlg_point(x: i32, y: i32, point: &Lazy<Mutex<(Option<i32>, Option<i32>)>>) {
    let mut m = point.lock().unwrap();
    m.0 = Some(x);
    m.1 = Some(y);
}

#[builtin_func_desc(
    desc="メッセージを表示"
    args=[
        {n="メッセージ",t="文字列",d="表示メッセージ"},
        {o,n="ボタン",t="定数",d=r#"表示するボタンを以下で指定、OR連結可
- BTN_YES: はい
- BTN_NO: いいえ
- BTN_OK: OK
- BTN_CANCEL: キャンセル
- BTN_ABORT: 中止
- BTN_RETRY: 再試行
- BTN_IGNORE: 無視"#},
        {o,n="X",t="数値",d="表示位置のX座標"},
        {o,n="Y",t="数値",d="表示位置のY座標"},
        {o,n="フォーカス",t="定数",d="予めフォーカスしておくボタンを示すBTN定数"},
        {o,n="リンク表示",t="真偽値",d="TRUEならURLをクリック可能なリンクにする"},
    ],
    rtype={desc="押されたボタンを示す定数",types="定数"}
)]
pub fn msgbox(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let message = args.get_as_string(0, None)?;
    let btns = args.get_as_int::<i32>(1, Some(BtnConst::BTN_OK as i32))?;
    let (x, y) = get_dlg_point(&args, (2, 3), &MSGBOX_POINT)?;
    let focus = args.get_as_int_or_empty(4)?;
    let enable_link = args.get_as_bool(5, Some(false))?;

    let font = Some(DIALOG_FONT_FAMILY.clone());
    let defbtn = focus.map(|n| MsgBoxButton(n));
    let title = DIALOG_TITLE.as_str();

    let msgbox = MsgBox::new(title, &message, x, y, MsgBoxButton(btns), defbtn, font, enable_link)
        .map_err(|e| builtin_func_error(UWindowError(e)))?;

    let result = msgbox.message_loop()
        .map_err(|e| builtin_func_error(UWindowError(e)))?;

    let x = result.point.x;
    let y = result.point.y;
    let pressed = result.result.0;

    set_dlg_point(x, y, &MSGBOX_POINT);
    Ok(pressed.into())
}

#[builtin_func_desc(
    desc="インプットボックスを表示"
    args=[
        {n="メッセージ",t="文字列または配列",d="表示メッセージ、配列の場合は表示メッセージとラベル"},
        {o,n="デフォルト値",t="文字列または配列",d="デフォルト値、配列の場合はラベル毎のデフォルト値"},
        {o,n="マスク表示",t="真偽値または配列",d="TRUEなら入力値をマスクする、配列の場合はラベル毎のマスク設定"},
        {o,n="X",t="数値",d="表示位置のX座標"},
        {o,n="Y",t="数値",d="表示位置のY座標"},
    ],
    rtype={desc="入力値",types="文字列または配列"}
)]
pub fn input(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut msg = args.get_as_string_array(0)?;
    let mut label = match msg.len() {
        0 => return Err(builtin_func_error(UErrorMessage::EmptyArrayNotAllowed)),
        1 => vec![None],
        _ => msg.drain(1..).map(|s| Some(s)).collect::<Vec<_>>(),
    };
    if label.len() > 5 {
        label.resize(5, None);
    }
    let mut default_values = match args.get_as_string_array_or_empty(1)? {
        Some(vec) => vec.into_iter().map(|s| Some(s)).collect(),
        None => vec![None],
    };
    default_values.resize(label.len(), None);
    let mut mask_flags = args.get_as_bool_array(2, Some(None))?.unwrap_or(vec![]);
    mask_flags.resize(label.len(), false);
    let (x, y) = get_dlg_point(&args, (3, 4), &INPUT_POINT)?;

    let fields = label.into_iter()
        .zip(default_values.into_iter())
        .zip(mask_flags.into_iter())
        .map(|((label, default), mask)| {
            InputField::new(label, default, mask)
        })
        .collect::<Vec<_>>();
    let count = fields.len();
    let title = DIALOG_TITLE.as_str();
    let font = DIALOG_FONT_FAMILY.clone();
    let caption = msg.pop().unwrap_or_default();

    let input = InputBox::new(title, Some(font), caption, fields, x, y)
            .map_err(|e| builtin_func_error(UWindowError(e)))?;

    let result = input.message_loop()
        .map_err(|e| builtin_func_error(UWindowError(e)))?;

    let x = result.point.x;
    let y = result.point.y;
    set_dlg_point(x, y, &INPUT_POINT);
    match result.result {
        Some(mut vec) => if vec.len() == 1 {
            let s = vec.pop().unwrap_or_default();
            Ok(Object::String(s))
        } else {
            let arr = vec.into_iter().map(|s| Object::String(s)).collect();
            Ok(Object::Array(arr))
        },
        None => if count > 1 {
            Ok(Object::Array(vec![]))
        } else {
            Ok(Object::Empty)
        },
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum SlctConst {
    SLCT_BTN = 1,
    SLCT_CHK = 2,
    SLCT_RDO = 4,
    SLCT_CMB = 8,
    SLCT_LST = 16,
    SLCT_STR = 64,
    SLCT_NUM = 128,
}

#[builtin_func_desc(
    desc="セレクトボックスを表示"
    sets=[
        [
            {n="表示方法",t="定数",d=r#"以下の定数のいずれかを指定
- SLCT_BTN: ボタン
- SLCT_CHK: チェックボックス
- SLCT_RDO: ラジオボタン
- SLCT_CMB: コンボボックス
- SLCT_LST: リストボックス

さらに以下をOR連結可
- SLCT_STR: 戻り値を選択項目名にする
- SLCT_NUM: 戻り値を選択項目のインデックス値にする"#},
            {n="タイムアウト秒",t="数値",d="0より大きければその秒数経過でセレクトボックスをキャンセル扱いで閉じる"},
            {o,n="X",t="数値",d="表示位置のX座標"},
            {o,n="Y",t="数値",d="表示位置のY座標"},
            {o,n="メッセージ",t="文字列",d="表示メッセージ"},
            {n="表示項目1",t="文字列",d="1つ目の項目 (必須)"},
            {v=28,n="表示項目2-29",t="文字列",d="表示メッセージ2つ目以降の表示項目"},
        ],
        [
            {n="表示方法",t="定数",d=r#"以下の定数のいずれかを指定
- SLCT_BTN: ボタン
- SLCT_CHK: チェックボックス
- SLCT_RDO: ラジオボタン
- SLCT_CMB: コンボボックス
- SLCT_LST: リストボックス

さらに以下をOR連結可
- SLCT_STR: 戻り値を選択項目名にする
- SLCT_NUM: 戻り値を選択項目のインデックス値にする"#},
            {n="タイムアウト秒",t="数値",d="0より大きければその秒数経過でセレクトボックスをキャンセル扱いで閉じる"},
            {o,n="メッセージ",t="文字列",d="表示メッセージ"},
            {n="表示項目1",t="文字列",d="1つ目の項目 (必須)"},
            {v=30,n="表示項目2-31",t="文字列",d="表示メッセージ2つ目以降の表示項目"},
        ],
    ],
    rtype={desc="選択した項目に該当する値、複数選択の場合配列",types="値または配列"}
)]
pub fn slctbox(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    // 第一引数: 種別と戻り値型
    let n = args.get_as_int(0, None)?;
    let r#type = SlctType::new(n);
    // 第二引数: タイマー秒数
    let wait = args.get_as_int(1, None)?;
    let progress = if wait > 0.0 {Some(wait)} else {None};
    // 第三引数・第四引数: いずれも数値なら座標
    let mut x = args.get_as_i32(2).ok();
    let mut y = args.get_as_i32(3).ok();
    let msg_index = match (x, y) {
        (None, None) => 2,
        (None, Some(_)) => {y = None; 2},
        (Some(_), None) => {x = None; 2},
        (Some(_), Some(_)) => 4,
    };
    let message = args.get_as_string_or_empty(msg_index)?;
    // 残りの引数を文字列の配列として受ける
    let items = args.get_rest_as_string_array(msg_index + 1, 0)?;

    // 表示位置の決定
    let pos_x = match x {
        Some(-1) => SLCTBOX_POINT.lock().unwrap().0,
        Some(n) => Some(n),
        None => None
    };
    let pos_y = match y {
        Some(-1) => SLCTBOX_POINT.lock().unwrap().1,
        Some(n) => Some(n),
        None => None
    };

    let font = Some(DIALOG_FONT_FAMILY.clone());
    let title = DIALOG_TITLE.as_str();
    let slct = Slctbox::new(title, message, r#type, items, progress, font, pos_x, pos_y)
            .map_err(|e| builtin_func_error(UWindowError(e)))?;
    let result = slct.message_loop()
        .map_err(|e| builtin_func_error(UWindowError(e)))?;

    set_dlg_point(result.point.x, result.point.y, &SLCTBOX_POINT);
    let obj = match result.result {
        SlctReturnValue::Const(n) |
        SlctReturnValue::Index(n) => n.into(),
        SlctReturnValue::String(s) => s.into(),
        SlctReturnValue::Multi(vec) => {
            let vec = vec.into_iter()
                .map(|v| match v {
                    SlctReturnValue::Const(n) |
                    SlctReturnValue::Index(n) => n.into(),
                    SlctReturnValue::String(s) => s.into(),
                    _ => (-1).into()
                })
                .collect();
            Object::Array(vec)
        },
        SlctReturnValue::Cancel => (-1).into(),
    };
    Ok(obj)
}

#[builtin_func_desc(
    desc="ポップアップメニューを表示"
    args=[
        {n="メニュー項目",t="配列",d="表示項目を示す配列"},
        {o,n="X",t="数値",d="表示位置のX座標、省略時はマウス位置"},
        {o,n="Y",t="数値",d="表示位置のY座標、省略時はマウス位置"},
    ],
    rtype={desc="選択項目",types="文字列"}
)]
pub fn popupmenu(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let list = args.get_as_array_include_hashtbl(0, None, true)?;
    let x = args.get_as_int_or_empty(1)?;
    let y = args.get_as_int_or_empty(2)?;
    let items = list.into_iter()
        .map(|o| o.into())
        .collect();
    let popup = PopupMenu::new(items)
        .map_err(|e| builtin_func_error(UErrorMessage::UWindowError(e)))?;
    let selected = popup.show(x, y)
        .map_err(|e| builtin_func_error(UWindowError(e)))?;

    Ok(selected.into())
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum BalloonFlag {
    #[default]
    FUKI_DEFAULT = 0,
    FUKI_UP      = 1,
    FUKI_DOWN    = 2,
    FUKI_LEFT    = 3,
    FUKI_RIGHT   = 4,
    FUKI_ROUND   = 9,
    FUKI_POINT   = 0xF0,
}

#[builtin_func_desc(
    desc="吹き出しを表示"
    args=[
        {n="メッセージ",t="文字列",d="表示メッセージ"},
        {o,n="X",t="数値",d="表示位置のX座標"},
        {o,n="Y",t="数値",d="表示位置のY座標"},
        {o,n="変形",t="定数",d=r#"以下の定数のいずれかを指定
- FUKI_DEFAULT: 変形しない (デフォルト)
- FUKI_UP: 吹き出しに上向きの嘴を付ける
- FUKI_DOWN: 吹き出しに下向きの嘴を付ける
- FUKI_LEFT: 吹き出しに左向きの嘴を付ける
- FUKI_RIGHT: 吹き出しに右向きの嘴を付ける
- FUKI_ROUND: 吹き出しの角を丸くする

嘴定数に対して以下をOR連結可能
- FUKI_POINT: 表示位置の基準を吹き出し左上ではなく嘴の先にする
"#},
        {o,n="フォントサイズ",t="数値",d="表示される文字のサイズ"},
        {o,n="フォント名",t="文字列",d="表示される文字のフォント名"},
        {o,n="文字色",t="数値",d="文字色をBGR値で指定"},
        {o,n="背景色",t="数値",d="背景色をBGR値で指定"},
        {o,n="透過",t="数値",d="0: 透過しない、1-255: 透過度、-1: 背景透明枠あり、-2: 背景透明枠なし"},
    ],
)]
pub fn balloon(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let balloon = if args.len() == 0 {
        // balloon消す
        None
    } else {
        let message = args.get_as_string(0, None)?;
        let x = args.get_as_int(1, Some(0_i32))?;
        let y = args.get_as_int(2, Some(0_i32))?;
        let shape = args.get_as_int(3, Some(0))?;
        let font_size = args.get_as_int_or_empty::<i32>(4)?;
        let font_name = args.get_as_string_or_empty(5)?;
        let font = Some(FontFamily::from((font_name, font_size)));
        let fore_color = args.get_as_int_or_empty::<u32>(6)?;
        let back_color = args.get_as_int_or_empty::<u32>(7)?;
        let transparency = args.get_as_int(8, Some(0))?;
        let balloon = Balloon::new(&message, x, y, font, fore_color, back_color, shape, transparency)
            .map_err(|e| builtin_func_error(UWindowError(e)))?;
        Some(balloon)
    };

    let cell = THREAD_LOCAL_BALLOON.with(|b| b.clone());
    let mut local_balloon = cell.borrow_mut();
    *local_balloon = balloon;

    Ok(Object::Empty)
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum FormOptions {
    FOM_NOICON    = 1,
    FOM_MINIMIZE  = 256,
    FOM_MAXIMIZE  = 512,
    FOM_NOHIDE    = 2,
    FOM_NOSUBMIT  = 4,
    FOM_NORESIZE  = 8,
    FOM_BROWSER   = 128,
    FOM_FORMHIDE  = 4096,
    FOM_TOPMOST   = 16,
    FOM_NOTASKBAR = 16384,
    FOM_FORM2     = 8192,
    FOM_DEFAULT   = 0,
}
impl Into<u32> for FormOptions {
    fn into(self) -> u32 {
        ToPrimitive::to_u32(&self).unwrap_or_default()
    }
}

#[builtin_func_desc(
    desc="フォームウィンドウを表示"
    args=[
        {n="HTMLファイル",t="文字列",d="表示するHTMLファイルのパス"},
        {n="タイトル",t="文字列",d="ウィンドウタイトル"},
        {o,n="非同期フラグ",t="真偽値",d="TRUEならフォーム表示後に制御を返す、FALSEならフォームが処理されるまで待機"},
        {o,n="表示オプション",t="定数",d=r#"以下の定数をOR連結で指定
- FOM_DEFAULT: オプションなし (デフォルト)
- FOM_NOICON: 閉じるボタンを非表示
- FOM_MINIMIZE: 最小化ボタンを表示
- FOM_MAXIMIZE: 最大化ボタンを表示
- FOM_NOHIDE: submitボタンが押されてもウィンドウを閉じない
- FOM_NOSUBMIT: submitボタンが押されてもsubmitに割り当てられた処理(action)を行わない
- FOM_NORESIZE: ウィンドウのサイズ変更不可
- FOM_FORMHIDE: ウィンドウを非表示で起動
- FOM_TOPMOST: ウィンドウを最前面に固定
- FOM_NOTASKBAR: タスクバーにアイコンを表示しない
"#},
        {o,n="幅",t="数値",d="ウィンドウ幅"},
        {o,n="高さ",t="数値",d="ウィンドウ高さ"},
        {o,n="X",t="数値",d="ウィンドウ表示位置X座標"},
        {o,n="Y",t="数値",d="ウィンドウ表示位置Y座標"},
    ],
    rtype={desc="非同期フラグによりFormオブジェクトかForm情報オブジェクトを返す",types="FormまたはForm情報"}
)]
pub fn createform(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let file = args.get_as_string(0, None)?;
    let title = args.get_as_string(1, None)?;
    let sync = args.get_as_bool(2, Some(false))?;
    let opt = args.get_as_int::<u32>(3, Some(0))?;
    let w = args.get_as_int_or_empty::<i32>(4)?;
    let h = args.get_as_int_or_empty::<i32>(5)?;
    let x = args.get_as_int_or_empty::<i32>(6)?;
    let y = args.get_as_int_or_empty::<i32>(7)?;
    let size = FormSize::new(x, y, w, h);

    let form = WebViewForm::new(&title, size, opt)?;
    form.run(&file)?;
    if sync {
        Ok(Object::WebViewForm(form))
    } else {
        let value = form.message_loop()?;
        Ok(Object::UObject(UObject::new(value)))
    }
}

#[builtin_func_desc(
    desc="使用不可"
)]
pub fn getformdata(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Err(builtin_func_error(UErrorMessage::UnavailableFunction))
}

#[builtin_func_desc(
    desc="使用不可"
)]
pub fn setformdata(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Err(builtin_func_error(UErrorMessage::UnavailableFunction))
}