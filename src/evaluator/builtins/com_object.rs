use crate::evaluator::object::*;
use crate::evaluator::builtins::*;
use crate::evaluator::Evaluator;
use crate::settings::USETTINGS;

use std::ops::BitOr;
use std::sync::OnceLock;

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("createoleobj", 1, createoleobj);
    sets.add("getactiveoleobj", 3, getactiveoleobj);
    sets.add("getoleitem", 1, getoleitem);
    sets.add("oleevent", 4, oleevent);
    sets.add("vartype", 2, vartype);
    sets.add("safearray", 4, safearray);
    sets.add("xlopen", 36, xlopen);
    sets.add("xlclose", 2, xlclose);
    sets.add("xlactivate", 3, xlactivate);
    sets.add("xlsheet", 3, xlsheet);
    sets.add("xlgetdata", 4, xlgetdata);
    sets.add("xlsetdata", 7, xlsetdata);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum VarType {
    VAR_EMPTY    = 0,   // VT_EMPTY
    VAR_NULL     = 1,   // VT_NULL
    VAR_SMALLINT = 2,   // VT_I2
    VAR_INTEGER  = 3,   // VT_I4
    VAR_SINGLE   = 4,   // VT_R4
    VAR_DOUBLE   = 5,   // VT_R8
    VAR_CURRENCY = 6,   // VT_CY
    VAR_DATE     = 7,   // VT_DATE
    VAR_BSTR     = 8,   // VT_BSTR
    VAR_DISPATCH = 9,   // VT_DISPATCH
    VAR_ERROR    = 10,  // VT_ERROR
    VAR_BOOLEAN  = 11,  // VT_BOOL
    VAR_VARIANT  = 12,  // VT_VARIANT
    VAR_UNKNOWN  = 13,  // VT_UNKNOWN
    VAR_SBYTE    = 16,  // VT_I1
    VAR_BYTE     = 17,  // VT_UI1
    VAR_WORD     = 18,  // VT_UI2
    VAR_DWORD    = 19,  // VT_UI4
    VAR_INT64    = 20,  // VT_I8
    VAR_ASTR     = 256, // VT_LPSTR
    VAR_USTR     = 258, // VT_LPWSTR
    VAR_UWSCR    = 512, // UWSCRデータ型
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
impl Into<Object> for VarType {
    fn into(self) -> Object {
        ToPrimitive::to_f64(&self).unwrap_or(0.0).into()
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

fn createoleobj(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_string(0, None)?;
    // ignore IE
    let obj = ComObject::new(id, is_ie_allowed())?;
    Ok(Object::ComObject(obj))
}

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

pub fn getoleitem(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let com = args.get_as_comobject(0)?;
    let vec = com.to_object_vec()?;
    Ok(Object::Array(vec))
}

pub fn oleevent(_evaluator: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    // let mut com = args.get_as_comobject(0)?;
    // if args.len() > 1 {
    //     // セット
    //     let interface = args.get_as_string(1, None)?;
    //     let event = args.get_as_string(2, None)?;
    //     let func = match args.get_as_function_or_string(3, true)? {
    //         Some(two) => match two {
    //             TwoTypeArg::T(name) => {
    //                 match evaluator.env.get_function(&name) {
    //                     Some(obj) => match obj {
    //                         Object::Function(func) => Ok(func),
    //                         _ => Err(builtin_func_error(UErrorMessage::IsNotUserFunction(name))),
    //                     },
    //                     None => Err(builtin_func_error(UErrorMessage::FunctionNotFound(name))),
    //                 }
    //             },
    //             TwoTypeArg::U(func) => Ok(func),
    //         },
    //         None => Err(builtin_func_error(UErrorMessage::FunctionRequired)),
    //     }?;
    //     com.set_event_handler(&interface, &event, func, evaluator.clone())?;
    // } else {
    //     // 解除
    //     com.remove_event_handler()?;
    // }
    // Ok(Object::Empty)
    Err(builtin_func_error(UErrorMessage::UnavailableFunction))
}

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
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum ExcelConst {
    XL_DEFAULT = 0,
    XL_NEW     = 1,
    XL_BOOK    = 2,
    XL_OOOC    = 3,
}

pub fn xlopen(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let file = args.get_as_string_or_empty(0)?;
    let flg = args.get_as_const::<ExcelOpenFlag>(1, false)?.unwrap_or_default();
    let params = args.get_rest_as_string_array(2, 0)?;
    let com = Excel::open(file, flg, params)?;
    Ok(Object::ComObject(com))
}

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