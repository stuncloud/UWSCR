use crate::evaluator::builtins::*;
use crate::evaluator::object::Object;
use crate::settings::usettings_singleton;
use crate::error::evaluator::UErrorMessage::UWindowError;
use crate::gui::{
    UWindow,
    FontFamily,
    Msgbox, MsgBoxButton,
    InputBox, InputField,
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
    sets
}

fn get_dlg_title() -> String {
    let singleton = usettings_singleton(None);
    let settings = singleton.0.lock().unwrap();
    match &settings.options.dlg_title {
        Some(title) => title.to_string(),
        None => match std::env::var("GET_UWSC_NAME") {
            Ok(name) => format!("UWSCR - {}", name),
            Err(_) => format!("UWSCR"),
        },
    }
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

    let title = get_dlg_title();
    let msgbox = match Msgbox::new(
        &title,
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
    let title = get_dlg_title();
    let font = FONT_FAMILY.clone();
    let caption = msg.pop().unwrap_or_default();

    let input = match InputBox::new(&title, Some(font), &caption, fields, x, y) {
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