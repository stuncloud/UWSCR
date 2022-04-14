
use crate::evaluator::builtins::*;
use crate::evaluator::object::Object;
use crate::evaluator::object::{Fopen, FopenMode, FGetType, FPutType};
use crate::error::evaluator::UErrorMessage::FopenError;

use std::sync::{Arc, Mutex};

use strum_macros::{EnumString, EnumVariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use once_cell::sync::Lazy;

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("fopen", 3, fopen);
    sets.add("fclose", 2, fclose);
    sets.add("fget", 4, fget);
    sets.add("fput", 4, fput);
    sets.add("fdelline", 2, fdelline);
    sets.add("readini", 3, readini);
    sets.add("writeini", 4, writeini);
    sets.add("deleteini", 3, deleteini);
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum FileConst {
    F_EXISTS    = 1,
    F_READ      = 2,
    F_WRITE     = 4,
    F_WRITE1    = 8,
    F_WRITE8    = 16,
    F_WRITE8B   = 32,
    F_WRITE16   = 64,
    F_APPEND    = 1024,
    F_NOCR      = 128,
    F_TAB       = 256,
    F_EXCLUSIVE = 512,
    F_LINECOUNT = -1,
    F_ALLTEXT   = -2
}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumVariantNames, ToPrimitive, FromPrimitive)]
pub enum FileConstDup {
    F_INSERT = -1,
}

pub fn fopen(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let flag = args.get_as_int::<u32>(1, Some(FileConst::F_READ as u32))?;
    // let text = args.get_as_string_or_empty(2)?;

    let mut fopen = Fopen::new(&path, flag);
    if fopen.flag.mode == FopenMode::Append {
        let text = args.get_as_string(2, None)?;
        fopen.append(&text)
            .map_err(|e| builtin_func_error(FopenError(e), args.name()))
    } else {
        match fopen.open() {
            Ok(e) => match e {
                Some(b) => Ok(Object::Bool(b)),
                None => {
                    Ok(Object::Fopen(Arc::new(Mutex::new(fopen))))
                }
            },
            Err(e) => Err(builtin_func_error(FopenError(e), args.name())),
        }
    }
}

pub fn fclose(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let ignore_err = args.get_as_bool(1, Some(false))?;
    let closed = fopen.close()
        .map_or_else(
            |e| Err(builtin_func_error(FopenError(e), args.name())),
            |b| Ok(Object::Bool(b))
        );
    if ignore_err && closed.is_err() {
        Ok(Object::Bool(false))
    } else {
        closed
    }
}

pub fn fget(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let row = args.get_as_int::<i32>(1, None)?;
    let fget_type = FGetType::from(row);
    let column = args.get_as_int(2, Some(0))?;
    let dbl = args.get_as_bool(3, Some(false))?;

    fopen.read(fget_type, column, dbl)
        .map_err(|e| builtin_func_error(FopenError(e), args.name()))
}

pub fn fput(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let value = args.get_as_string(1, None)?;
    let row = args.get_as_int(2, Some(0))?;
    let column = args.get_as_int(3, Some(0))?;

    let fput_type = FPutType::from((row, column));

    fopen.write(&value, fput_type)
        .map_or_else(
            |e| Err(builtin_func_error(FopenError(e), args.name())),
            |_| Ok(Object::Empty)
        )
}

pub fn fdelline(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let row = args.get_as_int(1, None::<usize>)?;
    if row < 1 {
        return Ok(Object::Empty)
    }
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    fopen.remove(row);
    Ok(Object::Empty)
}

static DEFAULT_INI_NAME: Lazy<String> = Lazy::new(|| {
    match std::env::var("GET_UWSC_NAME").ok() {
        Some(name) => {
            let mut path = std::path::PathBuf::from(name);
            path.set_extension("ini");
            path.to_string_lossy().to_string()
        },
        None => "uwscr.ini".into(),
    }
});


pub fn readini(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let section = args.get_as_string_or_empty(0)?;
    let key = args.get_as_string_or_empty(1)?;
    let (path, arc) = args.get_as_string_or_fopen(2)?;
    match arc {
        // fidが渡された場合
        Some(arc) => {
            let mut fopen = arc.lock().unwrap();
            match (section, key) {
                // 該当する値を取得
                (Some(section), Some(key)) => {
                    let value = fopen.ini_read(Some(&section), &key)
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(value.into())
                },
                // セクションなしの該当キーの値を取得
                (None, Some(key)) => {
                    let value = fopen.ini_read(None, &key)
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(value.into())
                },
                // キー一覧を取得
                (Some(section), None) => {
                    let keys = fopen.get_keys(Some(&section))
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(keys.into())
                },
                // セクション一覧を取得
                (None, None) => {
                    let sections = fopen.get_sections()
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(sections.into())
                },
            }
        },
        // ファイルパスの場合
        None => {
            let path = path.unwrap_or(DEFAULT_INI_NAME.to_string());
            match (section, key) {
                (Some(section), Some(key)) => {
                    let value = Fopen::ini_read_from_path(&path, Some(&section), &key)
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(value.into())
                },
                (None, Some(key)) => {
                    let value = Fopen::ini_read_from_path(&path, None, &key)
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(value.into())
                },
                (None, None) => {
                    let sections = Fopen::get_sections_from_path(&path)
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(sections.into())
                },
                (Some(section), None) => {
                    let keys = Fopen::get_keys_from_path(&path, Some(&section))
                        .map_err(|e| builtin_func_error(FopenError(e), args.name()))?;
                    Ok(keys.into())
                },
            }
        },
    }
}

pub fn writeini(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::default())
}

pub fn deleteini(args: BuiltinFuncArgs) -> BuiltinFuncResult {
    Ok(Object::default())
}