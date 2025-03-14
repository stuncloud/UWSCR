mod drop;

use crate::Evaluator;
use crate::builtins::*;
use crate::object::{Object, Fopen, Csv, CsvValue, FopenMode, FGetType, FPutType};
use crate::error::UErrorMessage::FopenError;

use std::io::{Write, Read};
use std::sync::{Arc, Mutex, RwLock};
use std::path::PathBuf;
use std::sync::LazyLock;

use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};


pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("fopen", fopen, get_desc!(fopen));
    sets.add("fclose", fclose, get_desc!(fclose));
    sets.add("fget", fget, get_desc!(fget));
    sets.add("fput", fput, get_desc!(fput));
    sets.add("fdelline", fdelline, get_desc!(fdelline));
    sets.add("readini", readini, get_desc!(readini));
    sets.add("writeini", writeini, get_desc!(writeini));
    sets.add("deleteini", deleteini, get_desc!(deleteini));
    sets.add("deletefile", deletefile, get_desc!(deletefile));
    sets.add("getdir", getdir, get_desc!(getdir));
    sets.add("dropfile", dropfile, get_desc!(dropfile));
    sets.add("zipitems", zipitems, get_desc!(zipitems));
    sets.add("unzip", unzip, get_desc!(unzip));
    sets.add("zip", zip, get_desc!(zip));
    sets.add("csvopen", csvopen, get_desc!(csvopen));
    sets.add("csvclose", csvclose, get_desc!(csvclose));
    sets.add("csvread", csvread, get_desc!(csvread));
    sets.add("csvwrite", csvwrite, get_desc!(csvwrite));
    sets
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum FileConst {
    #[strum[props(desc="ファイルまたはフォルダの有無")]]
    F_EXISTS    = 1,
    #[strum[props(desc="ファイル読み取り権限を追加")]]
    F_READ      = 2,
    #[strum[props(desc="ファイル書き込み権限を追加")]]
    F_WRITE     = 4,
    #[strum[props(desc="ファイル書き込み権限を追加、SJISで書き込む")]]
    F_WRITE1    = 8,
    #[strum[props(desc="ファイル書き込み権限を追加、UTF-8で書き込む")]]
    F_WRITE8    = 16,
    #[strum[props(desc="ファイル書き込み権限を追加、UTF-8Bで書き込む")]]
    F_WRITE8B   = 32,
    #[strum[props(desc="ファイル書き込み権限を追加、UTF-16LEで書き込む")]]
    F_WRITE16   = 64,
    #[strum[props(desc="追記モード")]]
    F_APPEND    = 1024,
    #[strum[props(desc="自動ファイルクローズ")]]
    F_AUTOCLOSE = 2048,
    #[strum[props(desc="文末に改行を加えない")]]
    F_NOCR      = 128,
    #[strum[props(desc="CSVセパレータをタブ文字にする")]]
    F_TAB       = 256,
    #[strum[props(desc="排他モード")]]
    F_EXCLUSIVE = 512,
    #[strum(props(alias="F_INSERT"))]
    F_LINECOUNT = -1,
    F_ALLTEXT   = -2
}

#[builtin_func_desc(
    desc="テキストファイルを開く"
    args=[
        {n="ファイルパス",t="文字列",d="対象ファイルのパス"},
        {o,n="オープンモード",t="定数",d=r#"ファイルの開き方を以下の定数で指定、OR連結可
- F_READ: 読み取りできるようにする (デフォルト)
- F_WRITE: 書き込みできるようにする
- F_WRITE8: UTF-8で書き込む
- F_WRITE8B: BOM付きUTF-8で書き込む
- F_WRITE16: UTF-16LEで書き込む
- F_TAB: CSVセパレータをカンマではなくタブ文字にする
- F_EXCLUSIVE: 排他モードでファイルを開く
- F_NOCR: 文末に改行を入れない
- F_EXISTS: ファイルがあるかどうかを真偽値で返す
- F_APPEND: 文末に追記し即ファイルを閉じる、書き込みバイト数を返す
- F_AUTOCLOSE: ファイルIDが破棄されたときに自動でファイルを閉じる
"#},
        {o,n="追記文字列",t="文字列",d="F_APPEND時のみ有効"},
    ],
    rtype={desc="F_EXISTS: ファイル有無、F_APPEND: 書き込みバイト数、それ以外はファイルID",types="ファイルIDまたは真偽値または数値"}
)]
pub fn fopen(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let flag = args.get_as_int::<u32>(1, Some(FileConst::F_READ as u32))?;

    let mut fopen = Fopen::new(&path, flag);
    if fopen.flag.mode == FopenMode::Append {
        let text = args.get_as_string(2, None)?;
        fopen.append(&text)
            .map_err(|e| builtin_func_error(FopenError(e)))
    } else {
        match fopen.open() {
            Ok(e) => match e {
                Some(b) => Ok(Object::Bool(b)),
                None => {
                    Ok(Object::Fopen(Arc::new(Mutex::new(fopen))))
                }
            },
            Err(e) => Err(builtin_func_error(FopenError(e))),
        }
    }
}

#[builtin_func_desc(
    desc="ファイルを閉じて変更を保存する"
    args=[
        {n="ファイルID",t="ファイルID",d="開いたファイルのID"},
        {o,n="エラー抑止",t="真偽値",d="TRUEにすると書き込みエラーを無視する"},
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn fclose(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let ignore_err = args.get_as_bool(1, Some(false))?;
    let closed = fopen.close()
        .map_or_else(
            |e| Err(builtin_func_error(FopenError(e))),
            |b| Ok(Object::Bool(b))
        );
    if ignore_err && closed.is_err() {
        Ok(Object::Bool(false))
    } else {
        closed
    }
}

#[builtin_func_desc(
    desc="テキストファイルから読み出す (要F_READ)"
    args=[
        {n="ファイルID",t="ファイルID",d="開いたファイルのID"},
        {n="行",t="数値",d=r#"読み取る行の番号または以下の定数を指定
- F_LINECOUNT: ファイルの行数を返す
- F_ALLTEXT: ファイル全体のテキストを返す"#},
        {n="列",t="数値",d="CSV読み取りの場合に列番号(1から)、0なら行全体"},
        {o,n="ダブルクォート",t="真偽値または2",d="FALSE: 両端のダブルクォートを削除、TRUE: 削除しない、2: 2連続クォートを1つに"},
    ],
    rtype={desc="読み取った文字列または行数",types="文字列または数値"}
)]
pub fn fget(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let row = args.get_as_int::<i32>(1, None)?;
    let fget_type = FGetType::from(row);
    let column = args.get_as_int(2, Some(0))?;
    let dbl = args.get_as_int(3, Some(0))?;

    fopen.read(fget_type, column, dbl)
        .map_err(|e| builtin_func_error(FopenError(e)))
}

#[builtin_func_desc(
    desc="テキストファイルに書き込む (要F_WRITE)"
    args=[
        {n="ファイルID",t="ファイルID",d="開いたファイルのID"},
        {n="文字列",t="文字列",d="書き込む文字列"},
        {o,n="行",t="数値",d=r#"書き込む行または定数を指定
- 0: 文末に追記
- 1以上: 指定行に書き込み、既存の行は上書き
- F_ALLTEXT: ファイル全体を上書き
"#},
        {o,n="列",t="数値",d=r#"CSV列または定数を指定
- 0: 行全体に書き込み
- 1以上: 該当するCSVカラムに書き込み、既存カラムは上書き
- F_INSERT: 指定行に挿入 (既存の行は一行ずらす)
"#},
    ],
)]
pub fn fput(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    let value = args.get_as_string(1, None)?;
    let row = args.get_as_int(2, Some(0))?;
    let column = args.get_as_int(3, Some(0))?;

    let fput_type = FPutType::from((row, column));

    fopen.write(&value, fput_type)
        .map_or_else(
            |e| Err(builtin_func_error(FopenError(e))),
            |_| Ok(Object::Empty)
        )
}

#[builtin_func_desc(
    desc="行を削除 (要F_READおよびF_WRITE)"
    args=[
        {n="ファイルID",t="ファイルID",d="開いたファイルのID"},
        {n="行",t="数値",d="消したい行の番号"},
    ],
)]
pub fn fdelline(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let row = args.get_as_int(1, None::<usize>)?;
    if row < 1 {
        return Ok(Object::Empty)
    }
    let arc = args.get_as_fopen(0)?;
    let mut fopen = arc.lock().unwrap();
    fopen.remove(row);
    Ok(Object::Empty)
}

static DEFAULT_INI_NAME: LazyLock<String> = LazyLock::new(|| {
    match std::env::var("GET_UWSC_NAME").ok() {
        Some(name) => {
            let mut path = std::path::PathBuf::from(name);
            path.set_extension("ini");
            path.to_string_lossy().to_string()
        },
        None => "uwscr.ini".into(),
    }
});

#[builtin_func_desc(
    desc="iniファイル読み取り"
    args=[
        {o,n="セクション",t="文字列",d="セクション名を指定、省略時セクション一覧取得"},
        {o,n="キー",t="文字列",d="キー名を指定、省略時キー一覧取得"},
        {o,n="ファイル",t="文字列またはファイルID",d="対象ファイルを指定"},
    ],
    rtype={desc="キーの値、またはセクション/キー一覧の配列",types="文字列または配列"}
)]
pub fn readini(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let section = args.get_as_string_or_empty(0)?;
    let key = args.get_as_string_or_empty(1)?;
    let path_or_fopen = args.get_as_string_or_fopen(2)?;
    match path_or_fopen {
        TwoTypeArg::T(path) => {
            let path = path.unwrap_or(DEFAULT_INI_NAME.to_string());
            match (section, key) {
                (Some(section), Some(key)) => {
                    let value = Fopen::ini_read_from_path(&path, &section, &key)
                        .map_err(|e| builtin_func_error(FopenError(e)))?;
                    Ok(value.into())
                },
                (None, _) => {
                    let sections = Fopen::get_sections_from_path(&path)
                        .map_err(|e| builtin_func_error(FopenError(e)))?;
                    Ok(sections.into())
                },
                (Some(section), None) => {
                    let keys = Fopen::get_keys_from_path(&path, &section)
                        .map_err(|e| builtin_func_error(FopenError(e)))?;
                    Ok(keys.into())
                },
            }
        },
        TwoTypeArg::U(arc) => {
            let fopen = arc.lock().unwrap();
            match (section, key) {
                // 該当する値を取得
                (Some(section), Some(key)) => {
                    let value = fopen.ini_read(&section, &key);
                    Ok(value.into())
                },
                // キー一覧を取得
                (Some(section), None) => {
                    let keys = fopen.get_keys(&section);
                    Ok(keys.into())
                },
                // セクション一覧を取得
                (None, _) => {
                    let sections = fopen.get_sections();
                    Ok(sections.into())
                },
            }
        },
    }
}

#[builtin_func_desc(
    desc="iniファイル書き込み"
    args=[
        {n="セクション",t="文字列",d="セクション名"},
        {n="キー",t="文字列",d="キー名"},
        {n="値",t="文字列",d="書き込む値"},
        {o,n="ファイル",t="文字列またはファイルID",d="対象ファイル"},
    ],
)]
pub fn writeini(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let section = args.get_as_string(0, None)?;
    let key = args.get_as_string(1, None)?;
    let value = args.get_as_object(2, None)?.to_string();
    let path_or_fopen = args.get_as_string_or_fopen(3)?;
    match path_or_fopen {
        TwoTypeArg::T(path) => {
            let path = path.unwrap_or(DEFAULT_INI_NAME.to_string());
            Fopen::ini_write_from_path(&path, &section, &key, &value)
                .map_err(|e| builtin_func_error(FopenError(e)))?;
        },
        TwoTypeArg::U(arc) => {
            let mut fopen = arc.lock().unwrap();
            fopen.ini_write(&section, &key, &value);
        },
    }
    Ok(Object::default())
}

#[builtin_func_desc(
    desc="指定キー、またはセクションを削除"
    args=[
        {n="セクション",t="文字列",d="セクション名"},
        {o,n="キー",t="文字列",d="削除するキー、省略時はセクション全体を削除"},
        {o,n="ファイル",t="文字列またはファイルID",d="対象ファイル"},
    ],
)]
pub fn deleteini(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let section = args.get_as_string(0, None)?;
    let key = args.get_as_string_or_empty(1)?;
    let path_or_fopen = args.get_as_string_or_fopen(2)?;
    match path_or_fopen {
        TwoTypeArg::T(path) => {
            let path = path.unwrap_or(DEFAULT_INI_NAME.to_string());
            Fopen::ini_delete_from_path(&path, &section, key.as_deref())
                .map_err(|e| builtin_func_error(FopenError(e)))?;
        },
        TwoTypeArg::U(arc) => {
            let mut fopen = arc.lock().unwrap();
            fopen.ini_delete(&section, key.as_deref());
        },
    }
    Ok(Object::default())
}

#[builtin_func_desc(
    desc="ファイルを削除",
    args=[
        {n="ファイルパス",t="文字列",d="削除したいファイルのパス、ワイルドカード(`*`, `?`)使用可"},
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn deletefile(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let result = Fopen::delete(&path);
    Ok(Object::Bool(result))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum FileOrderConst {
    #[strum[props(desc="ファイル名順")]]
    ORDERBY_NAME     = 0,
    #[strum[props(desc="ファイルサイズ順")]]
    ORDERBY_SIZE     = 1,
    #[strum[props(desc="ファイル作成日時順")]]
    ORDERBY_CREATED  = 2,
    #[strum[props(desc="ファイル更新日時順")]]
    ORDERBY_MODIFIED = 3,
    #[strum[props(desc="ファイルアクセス日時順")]]
    ORDERBY_ACCESSED = 4,
}

#[builtin_func_desc(
    desc="ファイルまたはディレクトリ一覧の取得",
    args=[
        {n="ディレクトリ",t="文字列",d="ファイル一覧を取得したいディレクトリのパス"},
        {o,n="フィルタ",t="文字列",d="ファイル名のフィルタ、ワイルドカード(`*`, `?`)使用可、`\\`開始でディレクトリ一覧取得"},
        {o,n="非表示フラグ",t="真偽値",d="TRUEなら非表示ファイルも含める"},
        {o,n="取得順",t="定数",d=r#"以下のいずれかを指定
- ORDERBY_NAME: ファイル名順
- ORDERBY_SIZE: サイズ順
- ORDERBY_CREATED: 作成日時順
- ORDERBY_MODIFIED: 更新日時順
- ORDERBY_ACCESSED: 最終アクセス日時順"#},
    ],
    rtype={desc="ファイル名またはディレクトリ名の配列",types="配列"}
)]
pub fn getdir(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let dir = args.get_as_string(0, None)?;
    let mut filter = args.get_as_string_or_empty(1)?.unwrap_or_default();
    let show_hidden = args.get_as_bool(2, Some(false))?;
    let order_by = args.get_as_int(3, Some(0))?;

    let get_dir = filter.starts_with('\\');
    filter = match filter.as_str() {
        "" | "\\" => "*".to_string(),
        f => {
            let f = f.trim_start_matches('\\');
            f.to_string()
        },
    };

    let files = Fopen::list_dir_entries(&dir, &filter, order_by.into(), get_dir, show_hidden, false)
        .map_err(|e| builtin_func_error(FopenError(e)))?
        .iter()
        .map(|s| s.to_string().into())
        .collect();
    Ok(Object::Array(files))
}

#[builtin_func_desc(
    desc="ウィンドウにファイルをドロップする"
    sets=[
        "座標指定なし",
        [
            {n="ID",t="数値",d="ドロップ対象ウィンドウ"},
            {n="ディレクトリ",t="文字列",d="ファイルのあるディレクトリパス"},
            {v=34,n="ファイル名1-34",t="文字列または配列",d="ドロップするファイル名"},
        ],
        "座標指定あり",
        [
            {n="ID",t="数値",d="ドロップ対象ウィンドウ"},
            {n="X",t="数値",d="ドロップするクライアントX座標"},
            {n="Y",t="数値",d="ドロップするクライアントY座標"},
            {n="ディレクトリ",t="文字列",d="ファイルのあるディレクトリパス"},
            {v=32,n="ファイル名1-32",t="文字列または配列",d="ドロップするファイル名"},
        ],
    ],
)]
pub fn dropfile(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = window_control::get_hwnd_from_id(id);
    if hwnd.0 != 0 {
        // 第二引数がいずれも数値なら座標として扱う
        let mut x = args.get_as_i32(1).ok();
        let mut y = args.get_as_i32(2).ok();
        let index = match (x, y) {
            (None, None) => 1,
            (None, Some(_)) => {y = None; 1},
            (Some(_), None) => {x = None; 1},
            (Some(_), Some(_)) => 3,
        };
        let dir = args.get_as_string(index, None)?;
        let files = args.get_rest_as_string_array(index + 1, 1)?;

        let files = drop::get_list_hstring(dir, files);
        let (x, y) = drop::get_point(hwnd, x, y);
        drop::dropfile(hwnd, &files, x, y);
    }
    Ok(Object::Empty)
}

struct Zip {
    path: PathBuf,
}

impl Zip {
    fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path)
        }
    }

    fn list(&self) -> std::io::Result<Vec<String>> {
        use std::fs::OpenOptions;
        use std::cmp::Ordering::{Greater,Less};

        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)?;
        let zip = zip::ZipArchive::new(file)?;
        let mut names = zip.file_names()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
        names.sort_by(|a,b| {
            match (a.contains('/'), b.contains('/')) {
                (true, true) => a.cmp(b),
                (true, false) => Less,
                (false, true) => Greater,
                (false, false) => a.cmp(b),
            }
        });
        Ok(names)
    }

    fn extract(&self, out: &str) -> zip::result::ZipResult<()> {
        use std::fs::OpenOptions;

        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        zip.extract(out)
    }

    fn compress(&self, files: Vec<String>) -> zip::result::ZipResult<()> {
        use std::fs::OpenOptions;
        let append = PathBuf::from(&self.path).exists();
        let file = OpenOptions::new()
                .write(true)
                .read(append)
                .create(true)
                .open(&self.path)?;
        let mut zip = if append {
            zip::ZipWriter::new_append(file)?
        } else {
            zip::ZipWriter::new(file)
        };
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let paths = files.into_iter().map(|p| PathBuf::from(p));
        for path in paths {
            if path.is_dir() {

                let mut list = Vec::new();
                Self::list_children(&path, &mut list);
                if ! list.is_empty() {
                    let parent = path.parent()
                        .map(|p| p.to_string_lossy())
                        .filter(|p| ! p.is_empty())
                        .map(|p| p.to_string() + "\\");

                    for path in list.into_iter() {
                        let name = if let Some(parent) = &parent {
                            path.to_string_lossy().replace(parent, "")
                        } else {
                            path.to_string_lossy().to_string()
                        };
                        zip.start_file(name, options)?;
                        let mut item = std::fs::File::open(&path)?;
                        let mut buf = vec![];
                        item.read_to_end(&mut buf)?;
                        zip.write_all(&buf)?;
                    }
                }
            } else if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                zip.start_file(name, options)?;
                let mut item = std::fs::File::open(&path)?;
                let mut buf = vec![];
                item.read_to_end(&mut buf)?;
                zip.write_all(&buf)?;
            }
        }
        zip.finish()?;
        Ok(())
    }
    fn list_children(path: &PathBuf, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = path.read_dir() {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            Self::list_children(&entry.path(), list)
                        } else if file_type.is_file() {
                            list.push(entry.path());
                        }
                    }
                }
            }
        }
    }
}

#[builtin_func_desc(
    desc="zipファイル内のファイル一覧を得る",
    args=[
        {n="zip",t="文字列",d="zipファイルのパス"},
    ],
    rtype={desc="ファイル名の配列",types="配列"}
)]
pub fn zipitems(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let zip = Zip::new(&path);
    let list = zip.list()?;
    let array = list.into_iter()
        .map(|s| s.into())
        .collect();
    Ok(Object::Array(array))
}

#[builtin_func_desc(
    desc="zipファイルを展開",
    args=[
        {n="zip",t="文字列",d="zipファイルのパス"},
        {n="展開先",t="文字列",d="展開先フォルダのパス"},
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn unzip(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let out = args.get_as_string(1, None)?;
    let zip = Zip::new(&path);
    let result = zip.extract(&out).is_ok();
    Ok(Object::Bool(result))
}

#[builtin_func_desc(
    desc="zipファイルを作成",
    args=[
        {n="zip",t="文字列",d="作成するzipファイルのパス"},
        {v=10,n="ファイル1-10",t="文字列または配列",d="zipに含めるファイルパス (配列可)"},
    ],
    rtype={desc="",types=""}
)]
pub fn zip(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let path = args.get_as_string(0, None)?;
    let files = args.get_rest_as_string_array(1, 1)?;
    let zip = Zip::new(&path);
    let result = zip.compress(files).is_ok();
    Ok(Object::Bool(result))
}

#[builtin_func_desc(
    desc="csvファイルを開く",
    args=[
        {n="csv",t="文字列",d="csvファイルのパス"},
        {o, n="ヘッダ",t="真偽値",d="csvファイルにヘッダ行があるかどうか"},
        {o, n="TSVモード",t="真偽値またはASCII文字",d="TRUEでセパレータがTSVになる"},
    ]
    rtype={desc="開いたCSVを示すオブジェクト", types="CSVオブジェクト"}
)]
pub fn csvopen(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let file = args.get_as_string(0, None)?;
    let header = args.get_as_bool(1, Some(false))?;
    let delimiter = match args.get_as_string_or_bool(2, Some(TwoTypeArg::U(false)))? {
        TwoTypeArg::T(d) => match d.len() {
            1 => {
                let ch = d.chars().next().unwrap_or(',');
                if ch.is_ascii() {
                    ch as u8
                } else {
                    return Err(builtin_func_error(UErrorMessage::ShouldBeAsciiCharacter(d)));
                }
            },
            _ => {
                return Err(builtin_func_error(UErrorMessage::ShouldBeAsciiCharacter(d)));
            }
        },
        TwoTypeArg::U(b) => if b {b'\t'} else {b','},
    };
    let csv = Csv::open(&file, header, delimiter)
        .map_err(|e| builtin_func_error(UErrorMessage::FopenError(e)))?;
    Ok(Object::Csv(Arc::new(RwLock::new(csv))))
}

#[builtin_func_desc(
    desc="CSVをファイルに書き込んで閉じる",
    args=[
        {n="csv",t="CSVオブジェクト",d="csvopenの戻り値"},
    ],
)]
pub fn csvclose(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let csv = _args.get_as_csv(0)?;
    let mut w = csv.write().unwrap();
    w.close()
        .map(|_| Object::Empty)
        .map_err(|e| builtin_func_error(UErrorMessage::FopenError(e)))
}

#[builtin_func_desc(
    desc="CSVから読み出す",
    rtype={desc="読み出した文字列",types="配列または文字列"}
    args=[
        {n="csv",t="CSVオブジェクト",d="csvopenの戻り値"},
        {o, n="row",t="数値",d="行番号"},
        {o, n="column",t="数値または文字列",d="列番号またはカラム名"},
    ],
)]
pub fn csvread(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let csv = _args.get_as_csv(0)?;
    let row = _args.get_as_int_or_empty(1)?;
    let column = _args.get_as_int_or_string_or_empty(2)?;

    let r = csv.read().unwrap();

    let value = match (row, column) {
        // 行および列が未指定ならCSV全体
        (None, None) => {
            r.read_all()
                .map_err(|e| builtin_func_error(UErrorMessage::FopenError(e)))?
        },
        // 列のみ指定なら該当列の配列
        (None, Some(two)) => match two {
            TwoTypeArg::T(column) => r.read_col_by_name(&column),
            TwoTypeArg::U(column) => r.read_col(column),
        },
        // 行指定のみ
        (Some(row), None) => {
            r.read(row, None)
        },
        // 行列指定
        (Some(row), Some(two)) => match two {
            // ヘッダ名
            TwoTypeArg::T(column) => r.read_by_name(row, &column),
            // インデックス
            TwoTypeArg::U(column) => r.read(row, Some(column)),
        },
    };
    let obj = match value {
        CsvValue::Row(items) => {
            let vec = items.into_iter().map(|s| s.into()).collect();
            Object::Array(vec)
        },
        CsvValue::Column(item) => item.into(),
        CsvValue::All(csv) => csv.into(),
        CsvValue::NotFound => Object::Empty,
    };
    Ok(obj)
}

#[builtin_func_desc(
    desc="CSVに値を書き込む",
    args=[
        {n="csv",t="CSVオブジェクト",d="csvopenの戻り値"},
        {n="row",t="数値",d="行番号"},
        {n="column",t="数値または文字列",d="列番号またはカラム名"},
        {n="値",t="文字列または配列",d="書き込む値、配列の場合は指定位置に挿入"},
    ],
)]
pub fn csvwrite(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {

    let csv = _args.get_as_csv(0)?;
    let row = _args.get_as_int(1, None)?;
    let column = _args.get_as_num_or_string(2)?;
    let value = _args.get_as_string_array(3)?;
    let value = CsvValue::from(value);

    let mut w = csv.write().unwrap();

    let succeed = match column {
        TwoTypeArg::T(column) => w.write_by_name(value, row, &column),
        TwoTypeArg::U(column) => w.write(value, row, column),
    };
    Ok(succeed.into())
}