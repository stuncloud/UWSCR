use crate::evaluator::builtins::*;
use crate::evaluator::object::Object;
use crate::settings::usettings_singleton;
use crate::error::evaluator::UErrorMessage::UWindowError;
use crate::gui::{
    UWindow,
    FontFamily,
    Msgbox, MsgBoxButton,
};

use std::sync::Mutex;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;

static FONT_FAMILY: Lazy<FontFamily> = Lazy::new(|| {
    let singleton = usettings_singleton(None);
    let usettings = singleton.0.lock().unwrap();
    FontFamily::new(&usettings.options.default_font.name, usettings.options.default_font.size)
});
static MSGBOX_POINT: Lazy<Mutex<(Option<i32>, Option<i32>)>> = Lazy::new(|| Mutex::new((None, None)));

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

pub fn msgbox(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let message = get_string_argument_value(&args, 0, None)?;
    let btns = get_non_float_argument_value::<i32>(&args, 1, Some(BtnConst::BTN_OK as i32))?;

    let x = match get_int_or_empty_argument(&args, 2, Some(None))? {
        Some(-1) => {
            MSGBOX_POINT.lock().unwrap().0
        },
        Some(n) => Some(n),
        None => None,
    };
    let y = match get_int_or_empty_argument(&args, 3, Some(None))? {
        Some(-1) => {
            MSGBOX_POINT.lock().unwrap().1
        },
        Some(n) => Some(n),
        None => None,
    };
    let focus = get_int_or_empty_argument(&args, 4, Some(None))?;
    let _enable_link = get_bool_argument_value(&args, 5, Some(false))?;

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
            let mut mp = MSGBOX_POINT.lock().unwrap();
            mp.0 = Some(x);
            mp.1 = Some(y);
            let pressed = btn.0 as f64;
            Ok(Object::Num(pressed))
        },
        Err(e) => Err(builtin_func_error(UWindowError(e), args.name())),
    }

}