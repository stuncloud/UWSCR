use crate::error::evaluator::{write_locale, CURRENT_LOCALE, Locale,};
use crate::evaluator::object::Object;
use crate::winapi::{
    // to_wide_string,
    WString, PcwstrExt,
    to_ansi_bytes,
};

use std::io::{Write, Read, Seek, SeekFrom};
use std::path::{PathBuf, Path};
use std::convert::AsRef;
use std::fs::{OpenOptions, File, remove_file};
use std::cmp::Ordering;
use std::os::windows::fs::OpenOptionsExt;
use std::sync::Mutex;
use std::io::BufWriter;

use windows::Win32::{
    // System::SystemServices::{GENERIC_READ, GENERIC_WRITE},
    Foundation::{FILETIME, HWND, RECT, WPARAM, LPARAM, POINT},
    Storage::FileSystem::{
        FILE_SHARE_NONE,FILE_SHARE_READ,FILE_SHARE_WRITE,FILE_SHARE_DELETE,
        FindFirstFileW, FindNextFileW, FindClose, WIN32_FIND_DATAW,
    },
    UI::{
        WindowsAndMessaging::{
            WM_DROPFILES,
            GetWindowRect, GetClientRect,
            PostMessageA,
            // PostMessageW, WindowFromPoint,
        },
        Shell::{
            DROPFILES,
        }
    },
    System::Memory::{
        GHND,
        GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock,
    }
};
use once_cell::sync::Lazy;
use encoding_rs::{UTF_8, SHIFT_JIS};

static FILE_LIST: Lazy<Mutex<Vec<(u32, File)>>> = Lazy::new(|| Mutex::new(vec![]));
static FILE_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

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
#[derive(Clone, Debug, PartialEq)]
pub enum FopenOption {
    Exclusive,
    Tab,
    NoCR,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FopenFlag {
    pub mode: FopenMode,
    encoding: FopenEncoding,
    option: Vec<FopenOption>,
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
        let mut option = vec![];
        let f_nocr = 0x80;
        let f_tab = 0x100;
        let f_exclusive = 0x200;
        if n & f_nocr == f_nocr {option.push(FopenOption::NoCR)}
        if n & f_tab == f_tab {option.push(FopenOption::Tab)}
        if n & f_exclusive == f_exclusive {option.push(FopenOption::Exclusive)}

        let encoding = encoding.unwrap_or(FopenEncoding::Auto);
        FopenFlag { mode, encoding, option }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fopen {
    pub flag: FopenFlag,
    path: PathBuf,
    id: u32,
    no_cr: bool,
    use_tab: bool,
    share: u32,
    text: Option<String>,
}

impl Fopen {
    pub fn new(path: &str, flag: u32) -> Self {
        let flag = FopenFlag::from(flag);
        let no_cr = flag.option.contains(&FopenOption::NoCR);
        let use_tab = flag.option.contains(&FopenOption::Tab);
        let share = if flag.option.contains(&FopenOption::Exclusive) {
            FILE_SHARE_NONE.0
        } else {
            FILE_SHARE_READ.0|FILE_SHARE_WRITE.0|FILE_SHARE_DELETE.0
        };
        let path = PathBuf::from(path);
        let id = Self::new_id();
        Self { flag, path, id, no_cr, use_tab, share, text: None }
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
        let mut opt = OpenOptions::new();
        opt.share_mode(self.share);
        match self.flag.mode {
            FopenMode::Read => opt.read(true),
            FopenMode::Write => opt.write(true).create(true),
            FopenMode::ReadWrite => opt.read(true).write(true).create(true),
            FopenMode::Append => return Ok(None),
            FopenMode::Exists => return Ok(Some(self.exists())),
            FopenMode::Unknown(n) => return Err(FopenError::UnknownOpenMode(n)),
        };

        let mut file = opt.open(&self.path)?;
        if self.can_read() {
            let mut buf = vec![];
            file.read_to_end(&mut buf)?;
            let text = self.decode(&buf)?;
            self.text = Some(text);
        }
        if self.can_write() {
            file.seek(SeekFrom::Start(0))?;
            file.set_len(0)?;
        }
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
    pub fn close(&mut self) -> FopenResult<bool> {
        let mut list = FILE_LIST.lock().unwrap();
        if let Some(index) = list.iter().position(|(id, _)| *id == self.id) {
            if self.can_write() {
                if let Some(ref text) = self.text {
                    if let Some((_, file)) = list.get_mut(index) {
                        let mut stream = BufWriter::new(file);
                        match self.flag.encoding {
                            FopenEncoding::Utf16LE => {
                                stream.write(&[0xFF, 0xFE])?;
                                for utf16 in text.encode_utf16() {
                                    stream.write(&utf16.to_be_bytes())?;
                                }
                                if ! self.no_cr {
                                    for utf16 in "\r\n".encode_utf16() {
                                        stream.write(&utf16.to_be_bytes())?;
                                    }
                                }
                            },
                            FopenEncoding::Utf16BE => {
                                stream.write(&[0xFE, 0xFF])?;
                                for utf16 in text.encode_utf16() {
                                    stream.write(&utf16.to_le_bytes())?;
                                }
                                if ! self.no_cr {
                                    for utf16 in "\r\n".encode_utf16() {
                                        stream.write(&utf16.to_le_bytes())?;
                                    }
                                }
                            },
                            FopenEncoding::Sjis => {
                                let (cow,_,_) = SHIFT_JIS.encode(text);
                                stream.write(cow.as_ref())?;
                                if ! self.no_cr {
                                    stream.write("\r\n".as_bytes())?;
                                }
                            }
                            _ => {
                                if self.flag.encoding == FopenEncoding::Utf8B {
                                    stream.write(&[0xEF, 0xBB, 0xBF])?;
                                }
                                stream.write(text.as_bytes())?;
                                if ! self.no_cr {
                                    stream.write("\r\n".as_bytes())?;
                                }
                            },
                        }
                        stream.flush()?;
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
                    stream.write(&[0xFF, 0xFE])?;
                }
                for utf16 in text.encode_utf16() {
                    size += stream.write(&utf16.to_le_bytes())?;
                }
                if ! self.no_cr {
                    for utf16 in "\r\n".encode_utf16() {
                        stream.write(&utf16.to_le_bytes())?;
                    }
                }
                size
            },
            FopenEncoding::Sjis => {
                let (cow,_,_) = SHIFT_JIS.encode(text);
                let size = file.write(cow.as_ref())?;
                if ! self.no_cr {
                    file.write("\r\n".as_bytes())?;
                }
                size
            },
            _ => {
                if is_new && self.flag.encoding == FopenEncoding::Utf8B {
                    file.write(&[0xEF, 0xBB, 0xBF])?;
                }
                let size = file.write(text.as_bytes())?;
                if ! self.no_cr {
                    file.write("\r\n".as_bytes())?;
                }
                size
            },
        };
        Ok(size.into())
    }
    fn decode(&mut self, bytes: &[u8]) -> FopenResult<String> {
        let (cow, enc, err) = UTF_8.decode(bytes);
        let (txt, enc) = if err {
            let (cow, enc, err) = SHIFT_JIS.decode(bytes);
            if err {
                return Err(FopenError::UnknownEncoding(enc.name().into()));
            } else {
                (cow.to_string(), enc.name())
            }
        } else {
            (cow.to_string(), enc.name())
        };
        if self.flag.encoding == FopenEncoding::Auto {
            self.flag.encoding = match enc {
                "UTF-16LE" => FopenEncoding::Utf16LE,
                "UTF-16BE" => FopenEncoding::Utf16BE,
                "Shift_JIS" => FopenEncoding::Sjis,
                "UTF-8" => FopenEncoding::Utf8,
                _ => FopenEncoding::Auto,
            }
        }
        Ok(txt)
    }
    pub fn read(&mut self, fget_type: FGetType, column: u32, as_is: bool) -> FopenResult<Object> {
        let text = self.text.as_ref().ok_or(FopenError::NotReadable)?;
        let obj = match fget_type {
            FGetType::Row(n) => {
                let n = n as usize - 1;
                if column == 0 {
                    // 行読み出し
                    text.lines()
                        .nth(n)
                        .map_or(Object::Empty, |row| row.to_string().into())
                } else {
                    // csv読み出し
                    self.csv_read(text, n, column, as_is)?
                        .map_or(Object::Empty, |text| text.into())
                }
            },
            FGetType::LineCount => {
                let len = text.lines().count();
                len.into()
            },
            FGetType::AllText => Object::String(text.to_string()),
        };
        Ok(obj)
    }
    fn csv_read(&self, text: &str, n: usize, column: u32, as_is: bool) -> FopenResult<Option<String>> {
        let delimiter = if self.use_tab {b'\t'} else {b','};
        let quote = if as_is {0} else {b'"'};
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(delimiter)
            // .quoting(quoting)
            .quote(quote)
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(text.as_bytes());
        let result = match reader.records().nth(n) {
            Some(record) => {
                let i = column as usize - 1;
                record?.get(i).map(|s| s.to_string())
            },
            None => None,
        };
        Ok(result)
    }
    pub fn write(&mut self, value: &str, fput_type: FPutType) -> FopenResult<()> {
        let written = match &self.text {
            Some(text) => {
                let mut lines: Vec<String> = text.lines().map(|l|l.to_string()).collect();
                match fput_type {
                    FPutType::Row(row) => {
                        let row = row as usize;
                        if row > lines.len() {
                            lines.resize(row, String::new());
                        }
                        let index = row - 1;
                        lines[index] = value.to_string();
                    },
                    FPutType::AllText => lines = vec![value.to_string()],
                    FPutType::Insert(row) => {
                        let row = row as usize;
                        if row > lines.len() {
                            lines.resize(row, String::new());
                        }
                        let index = row - 1;
                        lines.insert(index, value.to_string());
                    },
                    FPutType::Csv(row, col) => self.csv_write(&mut lines, value, Some(row), col)?,
                    FPutType::Append(col) => match col {
                        Some(col) => self.csv_write(&mut lines, value, None, col)?,
                        None => lines.push(value.to_string()),
                    },
                }
                lines
            },
            None => {
                let mut lines = vec![];
                match fput_type {
                    FPutType::Row(row) |
                    FPutType::Insert(row) => {
                        lines.resize((row - 1) as usize, String::new());
                        lines.push(value.to_string());
                    },
                    FPutType::AllText => lines.push(value.to_string()),
                    FPutType::Csv(row, col) => self.csv_write(&mut lines, value, Some(row), col)?,
                    FPutType::Append(col) => match col {
                        Some(col) => self.csv_write(&mut lines, value, None, col)?,
                        None => lines.push(value.to_string()),
                    },
                }
                lines
            },
        };
        let new_text = written.join("\r\n");
        self.text = Some(new_text);
        Ok(())
    }
    fn csv_write(&self, lines: &mut Vec<String>, value: &str, row: Option<i32>, col: i32) -> FopenResult<()> {
        let delimiter = if self.use_tab {b'\t'} else {b','};
        let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(vec![]);

        if let Some(row) = row {
            let row = row as usize;
            let index = row - 1;
            if row > lines.len() {
                lines.resize(row, String::new());
            }
            let maybe_record = if let Some(text) = lines.get(index) {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .delimiter(delimiter)
                    .flexible(true)
                    .from_reader(text.as_bytes());
                match reader.records().next() {
                    Some(record) => Some(record?),
                    None => None,
                }
            } else {
                None
            };
            let mut c = 0;
            match maybe_record {
                Some(record) => for v in record.iter() {
                    c += 1;
                    if c == col {
                        writer.write_field(value)?;
                    } else {
                        writer.write_field(v)?;
                    }
                },
                None => {
                    for _ in 1..col {
                        writer.write_field("")?;
                    }
                    writer.write_field(value)?;
                },
            }
            writer.write_record(None::<&[u8]>)?;
            writer.flush()?;
            let csv = String::from_utf8(writer.into_inner()?)?;
            lines[index] = csv.trim_end_matches("\n").to_string();
        } else {
            for _ in 1..col {
                writer.write_field("")?;
            }
            writer.write_field(value)?;
            writer.write_record(None::<&[u8]>)?;
            writer.flush()?;
            let csv = String::from_utf8(writer.into_inner()?)?;
            lines.push(csv.trim_end_matches("\n").to_string());
        }
        Ok(())
    }
    pub fn remove(&mut self, row: usize) {
        if let Some(text) = &self.text {
            let index = row - 1;
            let mut lines = text.lines().collect::<Vec<_>>();
            if row <= lines.len() {
                lines.remove(index);
                let new = lines.join("\r\n");
                self.text = Some(new);
            }
        }
    }

    /* ini */
    pub fn get_sections(&self) -> Vec<String> {
        match &self.text {
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
        match &self.text {
            Some(text) => {
                let ini = Ini::parse(text);
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
        match &self.text {
            Some(text) => {
                let ini = Ini::parse(text);
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
        let mut ini = match &self.text {
            Some(text) => Ini::parse(text),
            None => Ini::new(),
        };
        if ini.set(section, key, value) {
            self.text = Some(ini.to_string());
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
        let mut ini = match &self.text {
            Some(text) => Ini::parse(text),
            None => Ini::new(),
        };
        if match key {
            Some(key) => ini.remove(section, key),
            None => ini.remove_section(section),
        } {
            self.text = Some(ini.to_string());
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
            for file in files {
                if remove_file(&file).is_err() {
                    result = false;
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
        let mut data = Self::get_dir_item(&path)?
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
    }
    pub fn get_dir_item<P: AsRef<Path>>(path: P) -> FopenResult<Vec<WIN32_FIND_DATAW>> {
        unsafe {
            let mut result = vec![];
            let mut lpfindfiledata = WIN32_FIND_DATAW::default();
            let lpfilename = path.as_ref().to_string_lossy().to_string().to_wide_null_terminated().to_pcwstr();
            let hfindfile = FindFirstFileW(lpfilename, &mut lpfindfiledata)?;
            if ! hfindfile.is_invalid() {
                result.push(lpfindfiledata.clone());
                lpfindfiledata = WIN32_FIND_DATAW::default();
                while FindNextFileW(hfindfile, &mut lpfindfiledata).as_bool() {
                    result.push(lpfindfiledata.clone());
                    lpfindfiledata = WIN32_FIND_DATAW::default();
                }
                FindClose(hfindfile);
            }
            Ok(result)
        }
    }

    pub fn drop_file(hwnd: HWND, files: Vec<String>, x: Option<i32>, y: Option<i32>) {
        unsafe {
            let mut rect = RECT::default();
            let use_screen = true;
            let (x, y) = if use_screen {
                // スクリーン座標
                GetWindowRect(hwnd, &mut rect);
                let center = |max, min| (max-min) / 2;
                let x = x.unwrap_or(center(rect.right, rect.left)) + rect.left;
                let y = y.unwrap_or(center(rect.bottom, rect.top)) + rect.top;
                (x, y)
            } else {
                // クライアント座標
                GetClientRect(hwnd, &mut rect);
                let center = |max, min| (max-min) / 2 + min;
                let x = x.unwrap_or(center(rect.right, rect.left));
                let y = y.unwrap_or(center(rect.bottom, rect.top));
                (x, y)
            };

            let point = POINT { x, y };
            println!("[debug] point: {:?}", &point);
            println!("[debug] hwnd: {:?}", &hwnd);
            // let target = match WindowFromPoint(point) {
            //     HWND(0) => hwnd,
            //     h => h
            // };
            let target = hwnd;
            println!("[debug] target: {:?}", &target);

            let joined = format!("{}\0", files.join("\0"));
            let files = to_ansi_bytes(&joined);
            println!("[debug] files: {:?}", &files);

            // DROPFILES構造体
            let dropfiles_size = std::mem::size_of::<DROPFILES>();
            let size = dropfiles_size + files.len();
            let hmem = GlobalAlloc(GHND, size);
            let ptr = GlobalLock(hmem);
            let df = &mut *(ptr as *mut DROPFILES);
            df.pFiles = dropfiles_size as u32;
            df.pt.x = x;
            df.pt.y = y;
            df.fNC = use_screen.into(); // trueでスクリーン座標, falseでクライアント座標
            df.fWide = false.into();

            // ファイル名リストを構造体にコピー
            let dest = (ptr as usize + dropfiles_size) as *mut libc::c_void;
            let src = files.as_ptr() as *const libc::c_void;
            let n = files.len();
            libc::memcpy(dest, src, n);

            GlobalUnlock(hmem);

            let result = PostMessageA(target, WM_DROPFILES, WPARAM(hmem as usize), LPARAM(0));
            println!("[debug] result: {:?}", &result);
            if ! result.as_bool() {
                GlobalFree(hmem);
                println!("[debug] 失敗…");
            } else {
                println!("[debug] 成功！");
            }
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
    Row(i32),
    LineCount,
    AllText,
}
impl From<i32> for FGetType {
    fn from(n: i32) -> Self {
        match n {
            1.. => Self::Row(n),
            0 | -1 => Self::LineCount,
            _ => Self::AllText,
        }
    }
}
pub enum FPutType {
    Row(i32),
    AllText,
    Insert(i32),
    Csv(i32, i32),
    Append(Option<i32>),
}
impl From<(i32, i32)> for FPutType {
    fn from((row, col): (i32, i32)) -> Self {
        match row {
            -2 => Self::AllText, // F_ALLTEXT
            1.. => match col {
                -1 => Self::Insert(row), // F_INSERT
                1.. => Self::Csv(row, col),
                _ => Self::Row(row), // 列0
            },
            _ => if col > 0 {
                Self::Append(Some(col)) // 行0 + 列指定
            } else {
                Self::Append(None) // 行0 + 列0
            }
        }
    }
}

impl Drop for Fopen {
    fn drop(&mut self) {
        let _ = self.close();
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
        if let Self::Key(inikey) = self {
            inikey.is_in(section)
        } else {
            false
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
    fn parse(text: &str) -> Self {
        let mut current_section = None::<String>;
        let lines = text.lines()
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

    fn to_string(&self) -> String {
        let lines = self.lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>();
        lines.join("\r\n").to_string()
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