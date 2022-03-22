use crate::evaluator::LOGPRINTWIN;
use crate::evaluator::builtins::*;
use crate::evaluator::object::Object;
use crate::settings::usettings_singleton;
use crate::error::evaluator::UErrorMessage::UWindowError;
use crate::gui::{
    UWindow,
    FontFamily,
    Msgbox, MsgBoxButton,
    InputBox, InputField,
    Slctbox, SlctReturnValue,
    PopupMenu,
};

use std::sync::Mutex;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use once_cell::sync::Lazy;

static FONT_FAMILY: Lazy<FontFamily> = Lazy::new(|| {
    let singleton = usettings_singleton(None);
    let usettings = singleton.0.lock().unwrap();
    FontFamily::new(&usettings.options.default_font.name, usettings.options.default_font.size)
});
static MSGBOX_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static INPUT_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static SLCTBOX_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));
static DIALOG_TITLE: Lazy<String> = Lazy::new(|| {
    let singleton = usettings_singleton(None);
    let settings = singleton.0.lock().unwrap();
    match &settings.options.dlg_title {
        Some(title) => title.to_string(),
        None => match std::env::var("GET_UWSC_NAME") {
            Ok(name) => format!("UWSCR - {}", name),
            Err(_) => format!("UWSCR"),
        },
    }
});

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
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
    sets.add("msgbox", 6, msgbox);
    sets.add("input", 5, input);
    sets.add("logprint", 5, logprint);
    sets.add("slctbox", 34, slctbox);
    sets.add("popupmenu", 3, popupmenu);
    sets
}

pub fn logprint(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let flg = args.get_as_bool(0, None)?;
    let left = args.get_as_int_or_empty(1, Some(None))?;
    let top = args.get_as_int_or_empty(2, Some(None))?;
    let width = args.get_as_int_or_empty(3, Some(None))?;
    let height = args.get_as_int_or_empty(4, Some(None))?;
    if let Some(m) = LOGPRINTWIN.get(){
        let mut lp = m.lock().unwrap();
        lp.set_visibility(flg);
        lp.move_to(left, top, width, height);
    }
    Ok(Object::Empty)
}

fn get_dlg_point(args: &BuiltinFuncArgs, i: (usize,usize), point: &Lazy<Mutex<(Option<i32>, Option<i32>)>>) -> BuiltInResult<(Option<i32>, Option<i32>)> {
    let x = match args.get_as_int_or_empty(i.0, Some(None))? {
        Some(-1) => {
            point.lock().unwrap().0
        },
        Some(n) => Some(n),
        None => None,
    };
    let y = match args.get_as_int_or_empty(i.1, Some(None))? {
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

pub fn msgbox(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let message = args.get_as_string(0, None)?;
    let btns = args.get_as_int::<i32>(1, Some(BtnConst::BTN_OK as i32))?;
    let (x, y) = get_dlg_point(&args, (2, 3), &MSGBOX_POINT)?;
    let focus = args.get_as_int_or_empty(4, Some(None))?;
    let _enable_link = args.get_as_bool(5, Some(false))?;

    let font_family = FONT_FAMILY.clone();
    let selected = focus.map(|n| MsgBoxButton(n));

    let title = DIALOG_TITLE.as_str();
    let msgbox = match Msgbox::new(
        title,
        &message,
        MsgBoxButton(btns),
        Some(font_family),
        selected,
        x, y,
    ) {
        Ok(m) => m,
        Err(e) => return Err(builtin_func_error(UWindowError(e), args.name())),
    };
    msgbox.show();
    match msgbox.message_loop() {
        Ok((btn, x, y)) => {
            set_dlg_point(x, y, &MSGBOX_POINT);
            let pressed = btn.0 as f64;
            Ok(Object::Num(pressed))
        },
        Err(e) => Err(builtin_func_error(UWindowError(e), args.name())),
    }
}

pub fn input(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let mut msg = args.get_as_string_array(0, None)?.unwrap_or(vec![]);
    let mut label = match msg.len() {
        0 => return Err(builtin_func_error(UErrorMessage::EmptyArrayNotAllowed, args.name())),
        1 => vec![None],
        _ => msg.drain(1..).map(|s| Some(s)).collect::<Vec<_>>(),
    };
    if label.len() > 5 {
        label.resize(5, None);
    }
    let mut default_values = match args.get_as_string_array(1, Some(None))? {
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
    let font = FONT_FAMILY.clone();
    let caption = msg.pop().unwrap_or_default();

    let input = match InputBox::new(title, Some(font), &caption, fields, x, y) {
        Ok(input) => input,
        Err(e) => return Err(builtin_func_error(UWindowError(e), args.name())),
    };
    input.show();
    match input.message_loop() {
        Ok((result, x, y)) => {
            set_dlg_point(x, y, &INPUT_POINT);
            match result {
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
        },
        Err(e) => Err(builtin_func_error(UWindowError(e), args.name())),
    }

}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum SlctConst {
    SLCT_BTN = 1,
    SLCT_CHK = 2,
    SLCT_RDO = 4,
    SLCT_CMB = 8,
    SLCT_LST = 16,
    SLCT_STR = 64,
    SLCT_NUM = 128,
}

pub fn slctbox(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    // 第一引数: 種別と戻り値型
    let n = args.get_as_int(0, None)?;
    let (r#type, option) = Slctbox::convert_to_type_and_option(n);
    // 第二引数: タイマー秒数
    let wait = args.get_as_int(1, None)?;
    // 第三引数・第四引数: いずれも数値なら座標
    let mut x = args.get_as_i32(2).ok();
    let mut y = args.get_as_i32(3).ok();
    let msg_index = match (x, y) {
        (None, None) => 2,
        (None, Some(_)) => {y = None; 2},
        (Some(_), None) => {x = None; 2},
        (Some(_), Some(_)) => 4,
    };
    let message = args.get_as_string_or_empty(msg_index, Some(None))?;
    // 残りの引数を文字列の配列として受ける
    let items = args.get_rest_as_string_array(msg_index + 1)?;

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

    let font = FONT_FAMILY.clone();
    let title = DIALOG_TITLE.as_str();
    let slct = match Slctbox::new(title, message, r#type, option, items, font, wait, pos_x, pos_y) {
        Ok(slct) => slct,
        Err(e) => return Err(builtin_func_error(UWindowError(e), args.name())),
    };
    slct.show();
    let (ret, left, top) = match slct.message_loop() {
        Ok(ret) => ret,
        Err(e) => return Err(builtin_func_error(UWindowError(e), args.name())),
    };
    set_dlg_point(left, top, &SLCTBOX_POINT);
    let obj = match ret {
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

pub fn popupmenu(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let list = args.get_as_array(0, None)?;
    let x = args.get_as_int_or_empty(1, Some(None))?;
    let y = args.get_as_int_or_empty(2, Some(None))?;
    let popup = PopupMenu::new(list);
    let selected = match popup.show(x, y) {
        Ok(s) => s,
        Err(e) => return Err(builtin_func_error(UWindowError(e), args.name())),
    };
    let obj = match selected {
        Some(s) => Object::String(s),
        None => Object::Empty,
    };
    Ok(obj)
}