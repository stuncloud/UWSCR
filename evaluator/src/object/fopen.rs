use crate::object::Object;
use util::{
    write_locale,
    error::{CURRENT_LOCALE, Locale},
};

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{PathBuf, Path};
use std::convert::AsRef;
use std::fs::{OpenOptions, File, remove_file};
use std::cmp::Ordering;
use std::os::windows::fs::OpenOptionsExt;
use std::sync::Mutex;
use std::io::BufWriter;
use std::sync::LazyLock;

use windows::core::HSTRING;
use windows::Win32::{
    // System::SystemServices::{GENERIC_READ, GENERIC_WRITE},
    Foundation::FILETIME,
    Storage::FileSystem::{
        FILE_SHARE_NONE,FILE_SHARE_READ,FILE_SHARE_WRITE,FILE_SHARE_DELETE,
        FindFirstFileW, FindNextFileW, FindClose, WIN32_FIND_DATAW,
    },
};
use encoding_rs::{UTF_8, SHIFT_JIS};
use itertools::{Itertools, MultiPeek};
use std::iter::Enumerate;
use std::str::Chars;

static FILE_LIST: LazyLock<Mutex<Vec<(u32, OpenFile)>>> = LazyLock::new(|| Mutex::new(vec![]));
static FILE_ID: LazyLock<Mutex<u32>> = LazyLock::new(|| Mutex::new(0));

type FopenResult<T> = Result<T, FopenError>;

#[derive(Debug, Clone, PartialEq)]
pub enum FopenError {
    UnknownOpenMode(u32),
    IOError(String),
    Utf8Error(String),
    NoOpenFileFound(String),
    UnknownEncoding(String),
    CsvError(String),
    NotReadable,
    Win32Error(String),
    InvalidPath,
}

impl From<std::io::Error> for FopenError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e.to_string())
    }
}
impl From<std::str::Utf8Error> for FopenError {
    fn from(e: std::str::Utf8Error) -> Self {
        Self::Utf8Error(e.to_string())
    }
}
impl From<std::string::FromUtf8Error> for FopenError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error(e.to_string())
    }
}
impl From<csv::Error> for FopenError {
    fn from(e: csv::Error) -> Self {
        Self::CsvError(e.to_string())
    }
}
impl From<csv::IntoInnerError<csv::Writer<std::vec::Vec<u8>>>> for FopenError {
    fn from(e: csv::IntoInnerError<csv::Writer<std::vec::Vec<u8>>>) -> Self {
        Self::CsvError(e.to_string())
    }
}
impl From<windows::core::Error> for FopenError {
    fn from(e: windows::core::Error) -> Self {
        Self::Win32Error(e.to_string())
    }
}

impl std::fmt::Display for FopenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FopenError::UnknownOpenMode(n) => write_locale!(f,
                "不明なファイルモード: {}",
                "Unknown file open mode: {}",
                n
            ),
            FopenError::IOError(e) => write_locale!(f,
                "ファイルIOエラー ({})",
                "File IO Error: {}",
                e
            ),
            FopenError::Utf8Error(e) => write_locale!(f,
                "UTF8エラー ({})",
                "UTF8 Error: {}",
                e
            ),
            FopenError::CsvError(e) => write_locale!(f,
                "UTF8エラー ({})",
                "UTF8 Error: {}",
                e
            ),
            FopenError::NoOpenFileFound(path) => write_locale!(f,
                "ファイルが開かれていません ({})",
                "File is not opened: {}",
                path
            ),
            FopenError::UnknownEncoding(enc) => write_locale!(f,
                "未対応エンコーディング ({})",
                "Encoding not supported: {}",
                enc
            ),
            FopenError::NotReadable => write_locale!(f,
                "F_READが指定されていないためファイルを読み取れません",
                "Can not read file; f_READ is required",
            ),
            FopenError::Win32Error(e) => write_locale!(f,
                "Win32エラー ({})",
                "Win32 Error: {}",
                e
            ),
            FopenError::InvalidPath => write_locale!(f,
                "パスが不正です",
                "The path is invalid",
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FopenMode {
    Read,
    Write,
    ReadWrite,
    Append,
    Exists,
    Unknown(u32),
}
#[derive(Clone, Debug, PartialEq)]
pub enum FopenEncoding {
    Auto,
    Utf8,
    Utf8B,
    Utf16LE,
    Utf16BE,
    Sjis,
}
impl From<&'static encoding_rs::Encoding> for FopenEncoding {
    fn from(enc: &'static encoding_rs::Encoding) -> Self {
        match enc.name() {
            "UTF-16LE" => FopenEncoding::Utf16LE,
            "UTF-16BE" => FopenEncoding::Utf16BE,
            "Shift_JIS" => FopenEncoding::Sjis,
            "UTF-8" => FopenEncoding::Utf8,
            _ => FopenEncoding::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FopenOption {
    pub exclusive: bool,
    pub tab: bool,
    pub no_cr: bool,
    pub auto_close: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FopenFlag {
    pub mode: FopenMode,
    encoding: FopenEncoding,
    option: FopenOption,
}

impl From<u32> for FopenFlag {
    fn from(n: u32) -> Self {
        // detect file open mode
        let all_write_mode = 0x7C; // F_WRITE or F_WRITE1 or F_WRITE8 or F_WRITE8B or F_WRITE16
        let f_append = 0x400; // F_APPEND
        let f_exists = 0x1;
        let f_read = 0x2;
        let encoding = match n & all_write_mode {
            8 => Some(FopenEncoding::Sjis), // F_WRITE1
            16 => Some(FopenEncoding::Utf8), // F_WRITE8
            32 => Some(FopenEncoding::Utf8B), // F_WRITE8B
            64 => Some(FopenEncoding::Utf16LE), // F_WRITE16
            0 => None,
            _ => Some(FopenEncoding::Auto) // F_WRITE
        };
        let mode = if n & f_exists == f_exists {
            FopenMode::Exists
        } else if n & f_append == f_append {
            FopenMode::Append
        } else if n & f_read == f_read {
            if encoding.is_some() {
                FopenMode::ReadWrite
            } else {
                FopenMode::Read
            }
        } else if encoding.is_some() {
            FopenMode::Write
        } else {
            FopenMode::Unknown(n)
        };
        // detect file open options
        let f_nocr = 0x80;
        let f_tab = 0x100;
        let f_exclusive = 0x200;
        let f_autoclose = 2048;
        let option = FopenOption {
            exclusive: (n & f_exclusive) == f_exclusive,
            tab: (n & f_tab) == f_tab,
            no_cr: (n & f_nocr) == f_nocr,
            auto_close: (n & f_autoclose) == f_autoclose,
        };

        let encoding = encoding.unwrap_or(FopenEncoding::Auto);
        FopenFlag { mode, encoding, option }
    }
}

#[derive(Debug)]
enum OpenFile {
    File(File),
    New(OpenOptions),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fopen {
    pub flag: FopenFlag,
    path: PathBuf,
    id: u32,
    no_cr: bool,
    csv_delimiter: char,
    share: u32,
    // lines: Option<Vec<String>>,
    buf: Option<FopenBuf>,
}

impl Fopen {
    const CRLF: &'static str = "\r\n";
    pub fn new(path: &str, flag: u32) -> Self {
        let flag = FopenFlag::from(flag);
        let no_cr = flag.option.no_cr;
        let use_tab = flag.option.tab;
        let csv_delimiter = if use_tab {'\t'} else {','};
        let share = if flag.option.exclusive {
            FILE_SHARE_NONE.0
        } else {
            FILE_SHARE_READ.0|FILE_SHARE_WRITE.0|FILE_SHARE_DELETE.0
        };
        let path = PathBuf::from(path);
        let id = Self::new_id();
        Self { flag, path, id, no_cr, csv_delimiter, share, buf: None }
    }
    fn new_id() -> u32 {
        let mut m = FILE_ID.lock().unwrap();
        let new = *m + 1;
        *m = new;
        new
    }
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
    pub fn is_closed(&self) -> bool {
        self.id > 0
    }
    pub fn open(&mut self) -> FopenResult<Option<bool>>{
        let exists = self.exists();
        let mut opt = OpenOptions::new();
        opt.share_mode(self.share);
        match self.flag.mode {
            FopenMode::Read => opt.read(true),
            FopenMode::Write => opt.write(true).create(true),
            FopenMode::ReadWrite => opt.read(true).write(true).create(true),
            FopenMode::Append => return Ok(None),
            FopenMode::Exists => return Ok(Some(exists)),
            FopenMode::Unknown(n) => return Err(FopenError::UnknownOpenMode(n)),
        };

        let file = if exists {
            let mut file = opt.open(&self.path)?;
            if self.can_read() {
                let mut buf = vec![];
                file.read_to_end(&mut buf)?;
                let text = self.decode(&buf)?;
                self.buf = Some(FopenBuf::new(text));
            }
            OpenFile::File(file)
        } else {
            OpenFile::New(opt)
        };

        let mut list = FILE_LIST.lock().unwrap();
        list.push((self.id, file));
        Ok(None)
    }
    fn can_read(&self) -> bool {
        self.flag.mode == FopenMode::Read || self.flag.mode == FopenMode::ReadWrite
    }
    fn can_write(&self) -> bool {
        self.flag.mode == FopenMode::Write || self.flag.mode == FopenMode::ReadWrite
    }
    fn to_file(&self, file: &mut File, mut text: String) -> FopenResult<()> {
        // ファイル冒頭から書き込む
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        // F_NOCR かつ 末尾がCRLFであれば除去する
        if self.no_cr && text.ends_with(Self::CRLF){
            if let Some(nocr) = text.strip_suffix(Self::CRLF) {
                text = nocr.into();
            }
        }

        let mut stream = BufWriter::new(file);
        match self.flag.encoding {
            FopenEncoding::Utf16LE => {
                stream.write_all(&[0xFF, 0xFE])?;
                for utf16 in text.encode_utf16() {
                    stream.write_all(&utf16.to_le_bytes())?;
                }
                if ! self.no_cr {
                    for utf16 in Self::CRLF.encode_utf16() {
                        stream.write_all(&utf16.to_le_bytes())?;
                    }
                }
            },
            FopenEncoding::Utf16BE => {
                stream.write_all(&[0xFE, 0xFF])?;
                for utf16 in text.encode_utf16() {
                    stream.write_all(&utf16.to_be_bytes())?;
                }
                if ! self.no_cr {
                    for utf16 in Self::CRLF.encode_utf16() {
                        stream.write_all(&utf16.to_be_bytes())?;
                    }
                }
            },
            FopenEncoding::Sjis => {
                let (cow,_,_) = SHIFT_JIS.encode(&text);
                stream.write_all(cow.as_ref())?;
                if ! self.no_cr {
                    stream.write_all(Self::CRLF.as_bytes())?;
                }
            }
            _ => {
                if self.flag.encoding == FopenEncoding::Utf8B {
                    stream.write_all(&[0xEF, 0xBB, 0xBF])?;
                }
                stream.write_all(text.as_bytes())?;
                if ! self.no_cr {
                    stream.write_all(Self::CRLF.as_bytes())?;
                }
            },
        }
        stream.flush()?;
        Ok(())
    }

    pub fn close(&mut self) -> FopenResult<bool> {
        let mut list = FILE_LIST.lock().unwrap();
        if let Some(index) = list.iter().position(|(id, _)| *id == self.id) {
            if self.can_write() {
                if let Some(buf) = &self.buf {
                    let text = buf.to_string();
                    if let Some((_, openfile)) = list.get_mut(index) {
                        match openfile {
                            OpenFile::File(file) => {
                                self.to_file(file, text)?;
                            },
                            OpenFile::New(opt) => {
                                let mut file = opt.open(&self.path)?;
                                self.to_file(&mut file, text)?;
                            },
                        };
                    }
                }
            }
            list.remove(index);
            self.id = 0;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn append(&self, text: &str) -> FopenResult<Object> {
        let is_new = ! self.exists();
        let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&self.path)?;
        let size = match self.flag.encoding {
            FopenEncoding::Utf16LE => {
                let mut stream = BufWriter::new(file);
                let mut size = 0;
                if is_new {
                    stream.write_all(&[0xFF, 0xFE])?;
                }
                for utf16 in text.encode_utf16() {
                    size += stream.write(&utf16.to_le_bytes())?;
                }
                if ! self.no_cr {
                    for utf16 in Self::CRLF.encode_utf16() {
                        stream.write_all(&utf16.to_le_bytes())?;
                    }
                }
                size
            },
            FopenEncoding::Sjis => {
                let (cow,_,_) = SHIFT_JIS.encode(text);
                let size = file.write(cow.as_ref())?;
                if ! self.no_cr {
                    file.write_all(Self::CRLF.as_bytes())?;
                }
                size
            },
            _ => {
                if is_new && self.flag.encoding == FopenEncoding::Utf8B {
                    file.write_all(&[0xEF, 0xBB, 0xBF])?;
                }
                let size = file.write(text.as_bytes())?;
                if ! self.no_cr {
                    file.write_all(Self::CRLF.as_bytes())?;
                }
                size
            },
        };
        Ok(size.into())
    }
    fn _decode(bytes: &[u8]) -> FopenResult<(FopenEncoding, String)> {
        let (cow, enc, err) = UTF_8.decode(bytes);
        let (txt, enc) = if err {
            let (cow, enc, err) = SHIFT_JIS.decode(bytes);
            if err {
                return Err(FopenError::UnknownEncoding(enc.name().into()));
            } else {
                (cow.to_string(), enc)
            }
        } else {
            (cow.to_string(), enc)
        };
        Ok((FopenEncoding::from(enc), txt))
    }
    fn decode(&mut self, bytes: &[u8]) -> FopenResult<String> {
        let (enc, txt) = Self::_decode(bytes)?;
        if self.flag.encoding == FopenEncoding::Auto {
            self.flag.encoding = enc
        }
        Ok(txt)
    }
    pub fn read(&mut self, fget_type: FGetType, column: u32, dbl: i32) -> FopenResult<Object> {
        let buf = self.buf.as_ref().ok_or(FopenError::NotReadable)?;
        let obj = match fget_type {
            FGetType::Row(row_index) => {
                let text = buf.get_line(row_index);
                if column == 0 {
                    // 行読み出し
                    text.into()
                } else {
                    // csv読み出し
                    match text {
                        Some(text) => {
                            Self::csv_read(text, column, dbl, self.csv_delimiter)
                                .map_or(Object::Empty, |text| text.into())
                        },
                        None => Object::Empty,
                    }
                }
            },
            FGetType::LineCount => {
                let len = buf.get_linecount();
                len.into()
            },
            FGetType::AllText => {
                let text = buf.to_string();
                text.into()
            },
        };
        Ok(obj)
    }
    fn csv_read<D: Into<FgetDbl>>(text: &str, column: u32, flg: D, delimiter: char) -> FopenResult<Option<String>> {
        let csv = UCsv::parse(text, delimiter);

        const DBLDBL: &str = "\"\"";
        const DBL: &str = "\"";

        fn remove_side_quot(s: &str) -> String {
            if s.contains(DBLDBL) {
                // 連続するクォートが含まれていたらそのまま
                s.to_string()
            } else if s.starts_with(DBL) && s.ends_with(DBL) {
                // 前後の単一クォートを消す
                s.strip_prefix(DBL).unwrap()
                    .strip_suffix(DBL).unwrap()
                    .to_string()
            } else {
                // それ以外もそのまま返す
                s.to_string()
            }
        }

        let item = csv.get(column as usize - 1)
            .map(|s| {
                let s = s.trim_matches(' ');
                match flg.into() {
                    FgetDbl::Remove => {
                        remove_side_quot(s)
                    },
                    FgetDbl::AsIs => {
                        s.to_string()
                    },
                    FgetDbl::TwoToOne => {
                        let s = s.replace(DBLDBL, DBL);
                        // ここからはFgetDbl::Removeと同じ処理
                        remove_side_quot(&s)
                    },
                }
            });
        Ok(item)
    }
    pub fn write(&mut self, value: &str, fput_type: FPutType) -> FopenResult<()> {
        match &mut self.buf {
            Some(buf) => {
                match fput_type {
                    FPutType::Row(row_index) => {
                        buf.replace_line(row_index, |s| {
                            *s = value.into();
                        });
                    },
                    FPutType::AllText => {},
                    FPutType::Insert(row_index) => {
                        buf.insert_line(row_index, value);
                    },
                    FPutType::Csv(row_index, col_index) => {
                        buf.replace_line(row_index, |line: &mut String| {
                            let csv = Self::csv_write(line, value, col_index, self.csv_delimiter);
                            *line = csv;
                        })
                    },
                    FPutType::Append(col) => match col {
                        Some(col_index) => {
                            let line = Self::csv_write("", value, col_index, self.csv_delimiter);
                            buf.append_line(&line);
                        },
                        None => {
                            buf.append_line(value);
                        },
                    },
                }
            },
            None => {
                let buf = match fput_type {
                    FPutType::Row(row_index) |
                    FPutType::Insert(row_index) => {
                        let mut buf = FopenBuf::default();
                        buf.insert_line(row_index, value);
                        buf
                    },
                    FPutType::AllText => {
                        FopenBuf::new(value.into())
                    },
                    FPutType::Csv(row_index, col_index) => {
                        let mut buf = FopenBuf::default();
                        buf.replace_line(row_index, |line| {
                            let csv = Self::csv_write(line, value, col_index, self.csv_delimiter);
                            *line = csv;
                        });
                        buf
                    },
                    FPutType::Append(col) => {
                        let mut buf = FopenBuf::default();
                        match col {
                            Some(col_index) => {
                                let line = Self::csv_write("", value, col_index, self.csv_delimiter);
                                buf.append_line(&line);
                            },
                            None => {
                                buf.append_line(value);
                            },
                        }
                        buf
                    },
                };
                self.buf = Some(buf);
            },
        };
        Ok(())
    }
    fn csv_write(line: &str, value: &str, col_index: usize, delimiter: char) -> String {
        let mut csv = UCsv::parse(line, delimiter);
        let column = csv.get_mut(col_index);
        *column = value.into();
        csv.to_string()
    }
    pub fn remove(&mut self, row: usize) {
        if let Some(buf) = &mut self.buf {
            let row_index = row.saturating_sub(1);
            buf.remove_line(row_index);
        }
    }

    /* ini */
    pub fn get_sections(&self) -> Vec<String> {
        match &self.buf {
            Some(buf) => {
                let ini = Ini::parse(buf);
                ini.get_sections()
            },
            None => vec![],
        }
    }
    pub fn get_sections_from_path(path: &str) -> FopenResult<Vec<String>> {
        let f_read = 2;
        let mut fopen = Self::new(path, f_read);
        if let Err(e) = fopen.open() {
            match e {
                // IOエラーは無視して空文字を返す
                FopenError::IOError(_) => return Ok(vec![]),
                e => return Err(e)
            }
        }
        let sections = fopen.get_sections();
        fopen.close()?;
        Ok(sections)
    }
    pub fn get_keys(&self, section: &str) -> Vec<String> {
        match &self.buf {
            Some(buf) => {
                let ini = Ini::parse(buf);
                ini.get_keys(section)
            },
            None => vec![],
        }
    }
    pub fn get_keys_from_path(path: &str, section: &str) -> FopenResult<Vec<String>> {
        let f_read = 2;
        let mut fopen = Self::new(path, f_read);
        if let Err(e) = fopen.open() {
            match e {
                // IOエラーは無視して空文字を返す
                FopenError::IOError(_) => return Ok(vec![]),
                e => return Err(e)
            }
        }
        let keys = fopen.get_keys(section);
        fopen.close()?;
        Ok(keys)
    }
    pub fn ini_read(&self, section: &str, key: &str) -> Option<String> {
        match &self.buf {
            Some(buf) => {
                let ini = Ini::parse(buf);
                ini.get(section, key)
            },
            None => None,
        }
    }
    pub fn ini_read_from_path(path: &str, section: &str, key: &str) -> FopenResult<Option<String>> {
        let f_read = 2;
        let mut fopen = Self::new(path, f_read);
        if let Err(e) = fopen.open() {
            match e {
                // IOエラーは無視して空文字を返す
                FopenError::IOError(_) => return Ok(None),
                e => return Err(e)
            }
        }
        let value = fopen.ini_read(section, key);
        fopen.close()?;
        Ok(value)
    }
    pub fn ini_write(&mut self, section: &str, key: &str, value: &str) {
        let mut ini = match &self.buf {
            Some(buf) => Ini::parse(buf),
            None => Ini::new(),
        };
        if ini.set(section, key, value) {
            self.buf.replace(FopenBuf::new(ini.to_string()));
        }
    }
    pub fn ini_write_from_path(path: &str, section: &str, key: &str, value: &str) -> FopenResult<()> {
        let f_read_or_f_write = 6;
        let mut fopen = Self::new(path, f_read_or_f_write);
        fopen.open()?;
        fopen.ini_write(section, key, value);
        fopen.close()?;
        Ok(())
    }
    pub fn ini_delete(&mut self, section: &str, key: Option<&str>) {
        let mut ini = match &self.buf {
            Some(buf) => Ini::parse(buf),
            None => Ini::new(),
        };
        if match key {
            Some(key) => ini.remove(section, key),
            None => ini.remove_section(section),
        } {
            self.buf.replace(FopenBuf::new(ini.to_string()));
        }
    }
    pub fn ini_delete_from_path(path: &str, section: &str, key: Option<&str>) -> FopenResult<()> {
        let f_read_or_f_write = 6;
        let mut fopen = Self::new(path, f_read_or_f_write);
        fopen.open()?;
        fopen.ini_delete(section, key);
        fopen.close()?;
        Ok(())
    }
    pub fn delete(path: &str) -> bool {
        let mut result = true;
        let buf = PathBuf::from(path);
        let filter = match buf.file_name() {
            Some(f) => Path::new(f),
            None => return false,
        };
        let dir = match buf.parent(){
            Some(p) => p,
            None => return false,
        };
        if let Ok(files) = Self::list_dir_entries(dir, filter, FileOrderBy::Default, false, true, true) {
            if files.is_empty() {
                result = false;
            } else {
                for file in files {
                    if remove_file(&file).is_err() {
                        result = false;
                    }
                }
            }
        } else {
            result = false;
        }
        result
    }
    pub fn list_dir_entries<P: AsRef<Path>>(dir: P, filter: P, order_by: FileOrderBy, get_dir: bool, show_hidden: bool, fullpath: bool) -> FopenResult<Vec<String>> {
        let is_hidden = |attr: u32| attr & 2_u32 > 0;
        let is_dir = |attr: u32| attr & 16_u32 > 0;
        let buf = PathBuf::from(dir.as_ref());
        let path = buf.join(filter);

        if let Ok(data) = Self::get_dir_item(&path) {
            let mut data = data
                .into_iter()
                // 隠しファイル
                .filter(|d| {
                    if is_hidden(d.dwFileAttributes) {
                        show_hidden
                    } else {
                        true
                    }
                })
                // ファイルかフォルダの分岐
                .filter(|d| is_dir(d.dwFileAttributes) == get_dir)
                .collect::<Vec<_>>();
            // ソート
            if order_by != FileOrderBy::Default {
                data.sort_by(|d1, d2| {
                    match order_by {
                        FileOrderBy::Size => {
                            match d1.nFileSizeHigh.cmp(&d2.nFileSizeHigh) {
                                Ordering::Equal => d1.nFileSizeLow.cmp(&d2.nFileSizeLow),
                                order => order,
                            }
                        },
                        FileOrderBy::CreateTime => d1.ftCreationTime.cmp(&d2.ftCreationTime),
                        FileOrderBy::LastWriteTime => d1.ftLastWriteTime.cmp(&d2.ftLastWriteTime),
                        FileOrderBy::LastAccessTime => d1.ftLastAccessTime.cmp(&d2.ftLastAccessTime),
                        _ => Ordering::Equal
                    }
                })
            }
            let items = data
                .into_iter()
                .filter_map(|d| {
                    let name = String::from_utf16_lossy(&d.cFileName);
                    let trimed = name.trim_end_matches('\0');
                    match trimed {
                        "." | ".." => None,
                        _ => if fullpath {
                            let full = buf.join(trimed).to_string_lossy().to_string();
                            Some(full)
                        } else {
                            Some(trimed.to_string())
                        }
                    }
                })
                .collect();
            Ok(items)
        } else {
            Ok(vec![])
        }
    }
    pub fn get_dir_item<P: AsRef<Path>>(path: P) -> FopenResult<Vec<WIN32_FIND_DATAW>> {
        unsafe {
            let mut result = vec![];
            let mut lpfindfiledata = WIN32_FIND_DATAW::default();
            let lpfilename = path.as_ref().to_str()
                .map(HSTRING::from)
                .ok_or(FopenError::InvalidPath)?;

            let hfindfile = FindFirstFileW(&lpfilename, &mut lpfindfiledata)?;
            if ! hfindfile.is_invalid() {
                result.push(lpfindfiledata);
                lpfindfiledata = WIN32_FIND_DATAW::default();
                while FindNextFileW(hfindfile, &mut lpfindfiledata).is_ok() {
                    result.push(lpfindfiledata);
                    lpfindfiledata = WIN32_FIND_DATAW::default();
                }
                let _ = FindClose(hfindfile);
            }
            Ok(result)
        }
    }
}


#[derive(Debug, Clone, PartialEq, Default)]
struct FopenBuf {
    buf: String,
}
impl FopenBuf {
    const LF: char = '\n';
    const CR: &str = "\r";
    const CRLF: &str = "\r\n";
    pub fn new(s: String) -> Self {
        Self { buf: s }
    }
    /// 指定行を取得
    pub fn get_line(&self, row_index: usize) -> Option<&str> {
        let (from, to) = self.get_range(row_index)?;
        match to {
            Some(to) => Some(&self.buf[from..to]),
            None => Some(&self.buf[from..]),
        }
    }
    /// 該当行の開始位置と終了位置を得る
    fn get_range(&self, row_index: usize) -> Option<(usize, Option<usize>)> {
        let mut lfs = self.buf.match_indices(Self::LF).enumerate()
            .filter_map(|(nth, (index, _))| {
                let accept = row_index.eq(&nth) || row_index.saturating_sub(1).eq(&nth);
                accept.then_some(index)
            });
        match (lfs.next(), lfs.next()) {
            (None, None) => {
                if self.buf.contains(Self::LF) {
                    None
                } else {
                    row_index.eq(&0).then_some((0, None))
                }
            },
            (None, Some(_)) => None,
            (Some(index), None) => {
                if row_index > 0 {
                    let from = index + 1;
                    Some((from, None))
                } else {
                    let to = self.fix_to(index);
                    Some((0, Some(to)))
                }
            },
            (Some(from), Some(to)) => {
                let to = self.fix_to(to);
                let from = from + 1;
                Some((from, Some(to)))
            },
        }
    }
    /// `\n` の前に `\r` があった場合は終了位置を修正する
    fn fix_to(&self, to: usize) -> usize {
        let maybe_cr = to.saturating_sub(1);
        match self.buf.get(maybe_cr..to) {
            Some(Self::CR) => maybe_cr,
            _ => to,
        }
    }
    /// 行を置き換える
    pub fn replace_line<F>(&mut self, row_index: usize, replacer: F)
    where F: Fn(&mut String)
    {
        let range = self.get_range(row_index);
        match range {
            Some((from, to)) => {
                let mut row = match to {
                    Some(to) => self.buf.drain(from..to).collect(),
                    None => self.buf.drain(from..).collect(),
                };
                replacer(&mut row);
                self.buf.insert_str(from, &row);
            },
            None => {
                self.add_new_row(row_index);
                let mut row = String::new();
                replacer(&mut row);
                self.buf += &row;
            },
        }
    }
    /// 行を挿入
    pub fn insert_line(&mut self, row_index: usize, line: &str) {
        match self.get_range(row_index) {
            Some((from, _)) => {
                self.buf.insert_str(from, line);
                self.buf.insert_str(from + line.len(), Self::CRLF);
            },
            None => {
                self.add_new_row(row_index);
                self.buf += line;
            },
        }
    }
    fn add_new_row(&mut self, row_index: usize) {
        let rows = self.buf.matches(Self::LF).count() + 1;
        let crlfs = Self::CRLF.repeat(row_index - rows + 1);
        self.buf.push_str(&crlfs);
    }
    /// 行数を得る
    pub fn get_linecount(&self) -> usize {
        self.buf.matches(Self::LF).count() + 1
    }
    /// 追記
    fn append_line(&mut self, line: &str) {
        if self.get_linecount() - 1 > 0 {
            self.buf.push_str(Self::CRLF);
        }
        self.buf.push_str(line);
    }
    /// 削除
    fn remove_line(&mut self, row_index: usize) {
        if let Some((from, to)) = self.get_range(row_index) {
            match to {
                Some(to) => {
                    if row_index > 1 {
                        // 2行目以降は直前の改行から取り除く
                        if self.buf.get(from.saturating_sub(2)..from).is_some_and(|s| Self::CRLF.eq(s)) {
                            // 直前のcrlfからを取り除く
                            let _ = self.buf.drain(from-2..to);
                        } else {
                            // 直前のlfから取り除く
                            let _ = self.buf.drain(from-1..to);
                        }
                    } else {
                        // 1行目は直後の改行まで取り除く
                        if self.buf.get(to..to+2).is_some_and(|s| Self::CRLF.eq(s)) {
                            // 改行がCRLF
                            let _ = self.buf.drain(from..to+2);
                        } else {
                            // 改行がLF
                            let _ = self.buf.drain(from..to+1);
                        }
                    }
                },
                None => {
                    if row_index > 1 {
                        // 2行目以降かつ文末まで
                        if self.buf.get(from.saturating_sub(2)..from).is_some_and(|s| Self::CRLF.eq(s)) {
                            // 直前のcrlfからを取り除く
                            let _ = self.buf.drain(from-2..);
                        } else {
                            // 直前のlfから取り除く
                            let _ = self.buf.drain(from-1..);
                        }
                    } else {
                        // 1行目かつ文末まで
                        self.buf.clear();
                    }
                },
            }
        }
    }
    pub fn to_vec(&self) -> Vec<String> {
        self.buf.lines().map(|s| s.into()).collect()
    }
}

impl std::fmt::Display for FopenBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}
impl From<&FopenBuf> for Vec<String> {
    fn from(buf: &FopenBuf) -> Self {
        buf.to_vec()
    }
}

enum FgetDbl {
    Remove,
    AsIs,
    TwoToOne,
}
impl From<i32> for FgetDbl {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Remove,
            2 => Self::TwoToOne,
            _ => Self::AsIs
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FileOrderBy {
    Default,
    Size,
    CreateTime,
    LastWriteTime,
    LastAccessTime
}

impl From<i32> for FileOrderBy {
    fn from(n: i32) -> Self {
        match n {
            1 => Self::Size,
            2 => Self::CreateTime,
            3 => Self::LastWriteTime,
            4 => Self::LastAccessTime,
            _ => Self::Default,
        }
    }
}

impl Default for FileOrderBy {
    fn default() -> Self {
        Self::Default
    }
}

pub enum FGetType {
    Row(usize),
    LineCount,
    AllText,
}
impl From<i32> for FGetType {
    fn from(n: i32) -> Self {
        match n {
            1.. => Self::Row(n as usize - 1),
            0 | -1 => Self::LineCount,
            _ => Self::AllText,
        }
    }
}
pub enum FPutType {
    Row(usize),
    AllText,
    Insert(usize),
    Csv(usize, usize),
    Append(Option<usize>),
}
impl From<(i32, i32)> for FPutType {
    fn from((row, col): (i32, i32)) -> Self {
        match row {
            -2 => Self::AllText, // F_ALLTEXT
            1.. => {
                let row_index = row as usize - 1;
                match col {
                    -1 => Self::Insert(row_index), // F_INSERT
                    1.. => {
                        let col_index = col as usize - 1;
                        Self::Csv(row_index, col_index)
                    },
                    _ => Self::Row(row_index), // 列0
                }
            },
            _ => if col > 0 {
                Self::Append(Some(col as usize - 1)) // 行0 + 列指定
            } else {
                Self::Append(None) // 行0 + 列0
            }
        }
    }
}

impl Drop for Fopen {
    fn drop(&mut self) {
        if self.flag.option.auto_close {
            // 自動保存フラグがあればクローズ処理を行う
            let _ = self.close();
        }
    }
}

impl std::fmt::Display for Fopen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{} (mode: {:?}, encoding: {:?}, options: {:?})",
            self.path.to_str().unwrap_or_default(),
            self.flag.mode,
            self.flag.encoding,
            self.flag.option,
        )
    }
}

struct UCsv {
    items: Vec<String>,
    delimiter: char,
}
impl UCsv {
    fn _new(delimiter: char) -> Self {
        Self { items: Vec::new(), delimiter}
    }
    fn parse(row: &str, delimiter: char) -> Self {
        let mut items = Vec::new();
        let mut chars = row.chars().enumerate().multipeek();
        while let Some((index, char)) = chars.peek() {
            match char {
                // 区切り文字
                ch if delimiter.eq(ch) => {
                    let index = *index;
                    let item = Self::take(&mut chars, Some(index));
                    items.push(item);
                },
                '"' => {
                    let item = Self::parse_quoted_item(&mut chars, delimiter);
                    items.push(item);
                },
                _ => {},
            }
        }
        let item = Self::take(&mut chars, None);
        if !item.is_empty() {
            items.push(item);
        }
        Self { items, delimiter }
    }
    fn parse_quoted_item(chars: &mut MultiPeekableCharIndices, delimiter: char) -> String {
        chars.reset_peek();
        let mut cnt = 0;
        while let Some((index, char)) = chars.peek() {
            match char {
                '"' => {
                    cnt += 1;
                },
                ch if delimiter.eq(ch) => if cnt % 2 == 0 {
                    let index = *index;
                    return Self::take(chars, Some(index));
                },
                _ => {},
            }
        }
        Self::take(chars, None)
    }
    fn take(chars: &mut MultiPeekableCharIndices, index: Option<usize>) -> String {
        let item = match index {
            Some(index) => chars.peeking_take_while(|(i, _)| *i < index)
                .map(|(_, ch)| ch)
                .collect(),
            None => chars.peeking_take_while(|_| true)
                .map(|(_, ch)| ch)
                .collect(),
        };
        // 区切り文字なので消費
        chars.next();
        item
    }

    fn resize(&mut self, new_len: usize) {
        self.items.resize_with(new_len, Default::default);
    }
    fn get(&self, index: usize) -> Option<&String> {
        self.items.get(index)
    }
    fn get_mut(&mut self, index: usize) -> &mut String {
        if self.items.get(index).is_none() {
            self.resize(index+1);
        }
        self.items.get_mut(index).unwrap()
    }
    // fn splice(&mut self, index: usize, item: String) {
    //     if self.items.get(index).is_none() {
    //         self.resize(index+1);
    //     }
    //     self.items.splice(index..=index, [item]);
    // }
}
type MultiPeekableCharIndices<'a> = MultiPeek<Enumerate<Chars<'a>>>;
impl std::fmt::Display for UCsv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sep = self.delimiter.to_string();
        let row = self.items.join(&sep);
        write!(f, "{row}")
    }
}


#[derive(Debug)]
struct Ini {
    lines: Vec<IniLine>,
}

#[derive(Debug)]
enum IniLine {
    Section(String),
    Key(IniKey),
    Other(String)
}

impl IniLine {
    fn get_inikey_if_match(&self, section: &str, key: &str) -> Option<IniKey> {
        if let Self::Key(inikey) = self {
            if inikey.section.to_ascii_uppercase() == section.to_ascii_uppercase() &&
            inikey.key.to_ascii_uppercase() == key.to_ascii_uppercase() {
                Some(inikey.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
    fn get_value_if_match(&self, section: &str, key: &str) -> Option<String> {
        self.get_inikey_if_match(section, key)
            .map(|inikey| inikey.value.clone())
    }
    fn is_in_section(&self, section: &str) -> bool {
        match self {
            IniLine::Section(section2) => {
                section2.to_ascii_uppercase() == section.to_ascii_uppercase()
            },
            IniLine::Key(inikey) => inikey.is_in(section),
            IniLine::Other(_) => false,
        }
    }
}

#[derive(Debug, Clone)]
struct IniKey {
    section: String,
    key: String,
    value: String,
}

impl IniKey {
    fn is_in(&self, section: &str) -> bool {
        self.section.to_ascii_uppercase() == section.to_ascii_uppercase()
    }
}

impl std::fmt::Display for IniLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IniLine::Section(sec) => write!(f, "[{sec}]"),
            IniLine::Key(IniKey { section: _, key, value }) => write!(f, "{key}={value}"),
            IniLine::Other(line) => write!(f, "{line}"),
        }
    }
}

impl Ini {
    fn new() -> Self {
        Self {lines: vec![]}
    }
    // fn parse(text: &str) -> Self {
    fn parse<V: Into<Vec<String>>>(lines: V) -> Self {
        let mut current_section = None::<String>;
        let lines: Vec<String> = lines.into();
        let lines = lines.iter()
                .map(|s| {
                    let trim = s.trim();
                    if trim.starts_with("[") && trim.ends_with("]") {
                        let section = trim.trim_start_matches('[').trim_end_matches(']');
                        current_section = Some(section.to_string());
                        IniLine::Section(section.to_string())
                    } else if current_section.is_some() {
                        match trim.split_once('=') {
                            Some((key, val)) => IniLine::Key(IniKey {
                                section: current_section.as_ref().unwrap().to_string(),
                                key: key.trim().to_string(),
                                value: val.trim().to_string(),
                            }),
                            None => {
                                if trim.len() > 0 {
                                    // 空行以外のOtherだったらセクションから外す
                                    current_section = None;
                                }
                                IniLine::Other(s.to_string())
                            },
                        }
                    } else {
                        if trim.len() > 0 {
                            // 空行以外のOtherだったらセクションから外す
                            current_section = None;
                        }
                        IniLine::Other(s.to_string())
                    }
                })
                .collect();
        Self {lines}
    }

    fn insert(&mut self, inikey: IniKey) {
        let section = inikey.section.clone();
        let index = self.lines.iter()
            .rposition(|line| line.is_in_section(&section));
        match index {
            Some(index) => {
                self.lines.insert(index + 1, IniLine::Key(inikey));
            },
            None => {
                self.lines.push(IniLine::Section(section));
                self.lines.push(IniLine::Key(inikey));
            },
        }
    }

    fn get(&self, section: &str, key: &str) -> Option<String> {
        self.lines.iter()
            .find_map(|l| l.get_value_if_match(section, key))
    }
    fn set(&mut self, section: &str, key: &str, value: &str) -> bool {
        let maybe_inikey = self.lines.iter_mut()
                .find(|l| l.get_inikey_if_match(section, key).is_some());
        match maybe_inikey {
            Some(line) => if let IniLine::Key(inikey) = line {
                inikey.value = value.to_string();
                true
            } else {
                false
            },
            None => {
                let inikey = IniKey {
                    section: section.to_string(),
                    key: key.to_string(),
                    value: value.to_string(),
                };
                self.insert(inikey);
                true
            },
        }
    }
    fn remove(&mut self, section: &str, key: &str) -> bool {
        let maybe = self.lines.iter()
                .position(|line| line.get_inikey_if_match(section, key).is_some());
        match maybe {
            Some(index) => {
                self.lines.remove(index);
                true
            },
            None => false,
        }
    }
    fn remove_section(&mut self, section: &str) -> bool {
        let len = self.lines.len();
        self.lines.retain(|line| match line {
            IniLine::Section(s) => {
                s.to_ascii_uppercase() != section.to_ascii_uppercase()
            },
            IniLine::Key(key) => {
                key.section.to_ascii_uppercase() != section.to_ascii_uppercase()
            },
            IniLine::Other(_) => true,
        });
        self.lines.len() != len
    }

    fn get_sections(&self) -> Vec<String> {
        self.lines.iter()
            .filter_map(|line| match line {
                IniLine::Section(s) => Some(s.to_string()),
                _ => None,
            })
            .collect()
    }
    fn get_keys(&self, section: &str) -> Vec<String> {
        self.lines.iter()
            .filter_map(|line| match line {
                IniLine::Key(key) => {
                    if key.is_in(section) {
                        Some(key.key.to_string())
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .collect()
    }

    fn to_lines(&self) -> Vec<String> {
        let lines = self.lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();
        lines
    }
}
impl std::fmt::Display for Ini {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.to_lines().join("\r\n");
        write!(f, "{s}")
    }
}

trait FileTimeExt {
    fn cmp(&self, other: &Self) -> Ordering;
}

impl FileTimeExt for FILETIME {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.dwHighDateTime.cmp(&other.dwHighDateTime) {
            Ordering::Equal => self.dwLowDateTime.cmp(&other.dwLowDateTime),
            order => order,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
struct CsvRow(Vec<String>);
impl CsvRow {
    fn as_vec(&self) -> Vec<String> {
        self.0.clone()
    }
    fn get(&self, index: usize) -> Option<&String> {
        self.0.get(index)
    }
    fn resize(&mut self, new_len: usize) {
        self.0.resize_with(new_len, Default::default);
    }
    fn get_mut(&mut self, index: usize) -> &mut String {
        if self.0.get(index).is_none() {
            self.resize(index+1);
        }
        self.0.get_mut(index).unwrap()
    }
    fn splice(&mut self, index: usize, vec: Vec<String>) {
        if self.0.get(index).is_none() {
            self.resize(index+1);
        }
        self.0.splice(index..=index, vec);
    }
    fn find(&self, target: &str) -> Option<usize> {
        self.0.iter().enumerate()
            .find(|(_, item)| item.eq_ignore_ascii_case(target))
            .map(|(i, _)| i)
    }
}
#[derive(Debug, Clone)]
struct CsvBuffer {
    rows: Vec<CsvRow>,
    headers: Option<CsvRow>,
}
impl CsvBuffer {
    fn get_row(&self, index: usize) -> Option<&CsvRow> {
        self.rows.get(index)
    }
    fn get_row_mut(&mut self, index: usize) -> &mut CsvRow {
        if self.rows.get(index).is_some() {
            // インデックスが範囲内ならミュータブル参照を返す
            self.rows.get_mut(index).unwrap()
        } else {
            // インデックスが範囲外なら配列を拡張する
            self.rows.resize_with(index + 1, Default::default);
            self.get_row_mut(index)
        }
    }
    fn get_column(&self, index: usize) -> Vec<Option<&String>> {
        let cols = self.rows.iter()
            .map(|row| row.get(index))
            .collect::<Vec<_>>();
        cols
    }
    fn get_header(&self) -> Option<&CsvRow> {
        self.headers.as_ref()
    }
    fn get_header_mut(&mut self) -> Option<&mut CsvRow> {
        self.headers.as_mut()
    }
    fn clear(&mut self) {
        self.rows.clear();
        self.headers = None;
    }
    fn new_header_if_none(&mut self) {
        if self.headers.is_none() {
            self.headers.replace(Default::default());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Csv {
    path: Option<PathBuf>,
    opt: Option<OpenOptions>,
    // buf: Vec<u8>,
    buf: CsvBuffer,
    encoding: FopenEncoding,
    header: bool,
    delimiter: u8,
    changed: bool,
}
impl std::fmt::Display for Csv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}", path.to_string_lossy()),
            None => write_locale!(f,
                "閉じられたファイル",
                "Closed file"
            ),
        }
    }
}

impl Csv {
    fn should_drop(&self) -> bool {
        self.path.is_some() && self.opt.is_some()
    }
    pub fn open(file: &str, header: bool, delimiter: u8) -> FopenResult<Self> {
        let path = PathBuf::from(file);
        let mut opt = OpenOptions::new();
        opt.write(true);
        opt.create(true);
        opt.read(true);
        let (encoding, csv) = if path.exists() {
            let mut file = opt.open(&path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Fopen::_decode(&buf)?
        } else {
            (FopenEncoding::Utf8, String::new())
        };

        let buf = Self::csv_to_buffer(csv, header, delimiter)?;

        let csv = Csv {
            path: Some(path),
            opt: Some(opt),
            buf,
            encoding,
            header,
            delimiter,
            changed: false
        };
        Ok(csv)
    }
    pub fn close(&mut self) -> FopenResult<()> {
        if self.changed {
            // 変更があった場合のみ書き込みを行う
            if let (Some(path), Some(opt)) = (&self.path, &self.opt) {
                let file = opt.open(path)?;

                let csv = self.buffer_to_csv()?;
                let mut stream = BufWriter::new(file);
                match self.encoding {
                    FopenEncoding::Utf16LE => {
                        stream.write_all(&[0xFF, 0xFE])?;
                        for utf16 in csv.encode_utf16() {
                            stream.write_all(&utf16.to_le_bytes())?;
                        }
                    },
                    FopenEncoding::Utf16BE => {
                        stream.write_all(&[0xFE, 0xFF])?;
                        for utf16 in csv.encode_utf16() {
                            stream.write_all(&utf16.to_be_bytes())?;
                        }
                    },
                    FopenEncoding::Sjis => {
                        let (cow,_,_) = SHIFT_JIS.encode(&csv);
                        stream.write_all(cow.as_ref())?;
                    },
                    _ => {
                        stream.write_all(csv.as_bytes())?;
                    }
                };
                stream.flush()?;
            }
        }
        self.path = None;
        self.opt = None;
        self.buf.clear();
        Ok(())
    }

    fn csv_to_buffer(csv: String, header: bool, delimiter: u8) -> FopenResult<CsvBuffer> {

        let rdr = csv.as_bytes();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(header)
            .delimiter(delimiter)
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(rdr);
        let headers = if header {
            let headers = reader.headers()?.deserialize::<CsvRow>(None)?;
            Some(headers)
        } else {
            None
        };
        let rows = reader.deserialize::<CsvRow>()
            .collect::<csv::Result<Vec<_>>>()?;
        let buf = CsvBuffer { rows, headers };
        Ok(buf)
    }

    fn buffer_to_csv(&self) -> FopenResult<String> {
        let mut writer = csv::WriterBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.header)
            .flexible(true)
            .from_writer(vec![]);

        if let Some(headers) = &self.buf.headers {
            writer.serialize(headers)?;
        }
        for row in &self.buf.rows {
            writer.serialize(row)?;
        }
        let csv = String::from_utf8(writer.into_inner()?)?;
        Ok(csv)
    }
    /// ヘッダ名に合わせた列番号を返す
    fn get_col_index_by_name(&self, name: &str) -> Option<usize> {
        let headers = self.buf.headers.as_ref()?;
        headers.find(name).map(|i| i+1)
    }

    pub fn read_all(&self) -> FopenResult<CsvValue> {
        self.buffer_to_csv().map(|csv| CsvValue::All(csv))
    }
    pub fn read(&self, row: usize, column: Option<usize>) -> CsvValue {
        match column {
            // 列指定あり
            Some(col) => match row {
                // 0行目はヘッダ行
                0 => {
                    let index = col.saturating_sub(1);
                    let headers = self.buf.get_header();
                    CsvValue::from((headers, index))
                },
                // 1以降は該当行
                r => {
                    let row_index = r.saturating_sub(1);
                    let row = self.buf.get_row(row_index);
                    let col_index = col.saturating_sub(1);
                    CsvValue::from((row, col_index))
                }
            },
            // 列指定なし: 行全体を取得
            None => match row {
                // 0行目はヘッダ行を返す
                0 => {
                    let header = self.buf.get_header();
                    CsvValue::from(header)
                },
                // 1以降は該当行
                r => {
                    let index = r.saturating_sub(1);
                    let row = self.buf.get_row(index);
                    CsvValue::from(row)
                }
            },
        }
    }
    pub fn read_by_name(&self, row: usize, name: &str) -> CsvValue {
        match self.get_col_index_by_name(name) {
            Some(index) => self.read(row, Some(index)),
            None => CsvValue::NotFound,
        }
    }
    pub fn read_col(&self, column: usize) -> CsvValue {
        let col_index = column.saturating_sub(1);
        let cols = self.buf.get_column(col_index);
        cols.into()
    }
    pub fn read_col_by_name(&self, column: &str) -> CsvValue {
        match self.get_col_index_by_name(column) {
            Some(index) => self.read_col(index),
            None => CsvValue::NotFound,
        }
    }

    pub fn write(&mut self, value: CsvValue, row: usize, column: usize) -> bool {
        let row = match row {
            0 => {
                self.buf.new_header_if_none();
                self.buf.get_header_mut().unwrap()
            },
            row => {
                let row_index = row.saturating_sub(1);
                self.buf.get_row_mut(row_index)
            },
        };
        let col_index = column.saturating_sub(1);
        match value {
            CsvValue::Row(items) => {
                        row.splice(col_index, items);
                        self.changed = true;
                        true
                    },
            CsvValue::Column(item) => {
                        let column = row.get_mut(col_index);
                        *column = item;
                        self.changed = true;
                        true
                    },
            CsvValue::All(_) => false,
            CsvValue::NotFound => false,
        }
    }
    pub fn write_by_name(&mut self, value: CsvValue, row: usize, column: &str) -> bool {
        match self.get_col_index_by_name(column) {
            Some(column) => self.write(value, row, column),
            None => false,
        }
    }
}

impl Drop for Csv {
    fn drop(&mut self) {
        if self.should_drop() {
            let _ = self.close();
        }
    }
}

pub enum CsvValue {
    Row(Vec<String>),
    Column(String),
    All(String),
    NotFound,
}
impl From<Option<&CsvRow>> for CsvValue {
    /// 行を返す
    fn from(row: Option<&CsvRow>) -> Self {
        match row {
            Some(row) => Self::Row(row.as_vec()),
            None => Self::NotFound,
        }
    }
}
impl From<(Option<&CsvRow>, usize)> for CsvValue {
    /// 行内の指定列を返す
    fn from((row, index): (Option<&CsvRow>, usize)) -> Self {
        match row {
            Some(row) => match row.get(index) {
                Some(s) => Self::Column(s.into()),
                None => Self::NotFound,
            },
            None => Self::NotFound,
        }
    }
}
impl From<Vec<String>> for CsvValue {
    fn from(vec: Vec<String>) -> Self {
        if vec.len() == 1 {
            let s = vec.first().unwrap();
            CsvValue::Column(s.into())
        } else {
            CsvValue::Row(vec)
        }
    }
}
impl From<Vec<Option<&String>>> for CsvValue {
    fn from(vec: Vec<Option<&String>>) -> Self {
        let cols = vec.iter()
            .map(|col| col.map(|s| s.to_string()).unwrap_or_default())
            .collect();
        CsvValue::Row(cols)
    }
}