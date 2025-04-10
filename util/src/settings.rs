use crate::winapi::{get_special_directory, shell_execute};

use std::{
    fs::{
        OpenOptions,
        create_dir_all,
        read,
    },
    fmt,
    io::Write,
    path::PathBuf,
    sync::Mutex,
    str::FromStr,
    marker::PhantomData,
};

use windows::Win32::UI::Shell::CSIDL_APPDATA;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{self, Visitor, MapAccess};
use serde_json;
use schemars::{schema_for, JsonSchema};
use std::sync::LazyLock;

/// %APPDATA%\UWSCR\settings.json
/// %APPDATA%\UWSCR\uwscr-settings-schema.json
static SETTING_FILES: LazyLock<Result<SettingFiles, Error>> = LazyLock::new(|| {
    // AppData
    let mut path = PathBuf::from(get_special_directory(CSIDL_APPDATA as i32));
    // UWSCRフォルダがなければ作る
    path.push("UWSCR");
    if ! path.exists() {
        create_dir_all(&path)?
    }
    let mut schema = path.clone();
    let mut settings = path;
    schema.push("uwscr-settings-schema.json");
    settings.push("settings.json");
    Ok(SettingFiles { settings, schema })
});
struct SettingFiles {
    settings: PathBuf,
    schema: PathBuf,
}

pub static USETTINGS: LazyLock<Mutex<USettings>> = LazyLock::new(|| {
    let settings = if let Ok(files) = SETTING_FILES.as_ref() {
        USettings::from_file(&files.settings).unwrap_or_default()
    } else {
        USettings::default()
    };
    Mutex::new(settings)
});

// #[derive(Debug, Clone)]
// pub struct SingletonSettings(pub Arc<Mutex<USettings>>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct USettings {
    /// OPTION設定
    #[serde(default)]
    pub options: UOption,
    /// BrowserControl設定
    #[serde(default)]
    pub browser: Browser,
    /// chkimg設定
    #[serde(default)]
    pub chkimg: Chkimg,
    /// print窓のフォント設定
    #[serde(default, deserialize_with = "string_or_struct")]
    pub logfont: LogFont,
    /// この設定ファイルのschemaファイルのパス
    #[serde(default = "get_schema_url", skip_deserializing, rename(serialize = "$schema"))]
    pub schema: String,
}

fn get_schema_url() -> String {
    SETTING_FILES.as_ref().ok()
        .and_then(|f| {
            url::Url::from_file_path(&f.schema)
                .map(|url| url.to_string())
                .ok()
        })
        .unwrap_or_default()
}

impl USettings {
    pub fn get_current_settings_as_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_default()
    }
    pub fn from_file(path: &PathBuf) -> Result<Self, Error> {
        let json = read(path)?;
        let usettings = serde_json::from_slice::<USettings>(&json)?;
        Ok(usettings)
    }
    pub fn to_file(&self, path: &PathBuf) -> Result<(), Error> {
        let json = serde_json::to_string_pretty::<USettings>(self)?;
        let mut file = OpenOptions::new()
                            .create(true)
                            .truncate(true)
                            .write(true)
                            .open::<&PathBuf>(path)?;
        write!(file, "{}", json)?;
        Ok(())
    }
}

impl Default for USettings {
    fn default() -> Self {
        let schema = get_schema_url();
        USettings {
            options: UOption::default(),
            browser: Browser::default(),
            chkimg: Chkimg::default(),
            logfont: LogFont::default(),
            schema
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UOption {
    /// finally部を必ず実行する
    #[serde(default)]
    pub opt_finally: bool,
    /// 変数宣言必須
    #[serde(default)]
    pub explicit: bool,
    /// ダイアログタイトル
    #[serde(default)]
    pub dlg_title: Option<String>,
    /// ログファイルの出力有無など
    #[serde(default)]
    pub log_file: u8,
    /// ログの行数
    #[serde(default)]
    pub log_lines: u32,
    /// ログファイルの出力先
    #[serde(default)]
    pub log_path: Option<String>,
    /// メインGUIの座標
    #[serde(default)]
    pub position: UPosition,
    /// ダイアログなどのフォント
    #[serde(default, deserialize_with = "string_or_struct")]
    pub default_font: DefaultFont,
    /// 吹き出しを仮想デスクトップにも出すかどうか
    #[serde(default)]
    pub fix_balloon: bool,
    // /// stopボタン最前面に固定するかどうか (非対応)
    // #[serde(default)]
    // pub top_stop_form: bool
    /// 停止ホットキー無効
    #[serde(default)]
    pub no_stop_hot_key: bool,
    /// 短絡評価の有無
    #[serde(default)]
    pub short_circuit: bool,
    /// 特殊文字を展開しない
    #[serde(default)]
    pub special_char: bool,
    /// publicの重複定義を禁止
    #[serde(default)]
    pub opt_public: bool,
    /// 大文字小文字を区別する
    #[serde(default)]
    pub same_str: bool,
    /// print文でGUI出力するかどうか
    #[serde(default)]
    pub gui_print: bool,
    /// 条件式が真偽値を返さなければならないかどうか
    #[serde(default)]
    pub force_bool: bool,
    /// 条件式の判定をUWSCと同等にする
    #[serde(default)]
    pub cond_uwsc: bool,
    /// IEオブジェクトを許可 (非公開)
    #[serde(skip_serializing, default)]
    #[schemars(skip)]
    pub allow_ie_object: bool,
}

impl Default for UOption {
    fn default() -> Self {
        UOption {
            opt_finally: false,
            explicit: false,
            dlg_title: None,
            log_file: 1,
            log_lines: 400,
            log_path: None,
            position: UPosition::default(),
            default_font: DefaultFont::default(),
            fix_balloon: false,
            no_stop_hot_key: false,
            short_circuit: true,
            special_char: false,
            opt_public: false,
            same_str: false,
            gui_print: false,
            force_bool: false,
            cond_uwsc: false,
            allow_ie_object: false,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UPosition {
    /// x座標
    #[serde(default)]
    pub left: i32,
    /// y座標
    #[serde(default)]
    pub top: i32,
}

const DEFAULT_FONT_SIZE: i32 = 12;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DefaultFont {
    /// フォント名
    #[serde(default)]
    pub name: String,
    /// フォントサイズ
    #[serde(default)]
    pub size: i32
}
impl Default for DefaultFont {
    fn default() -> Self {
        Self { name: "Yu Gothic UI".into(), size: DEFAULT_FONT_SIZE }
    }
}
impl DefaultFont {
    pub fn new(name: &str, size: i32) -> Self {
        Self {name: name.into(), size}
    }
}
impl FromStr for DefaultFont {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = s.split(",").collect::<Vec<_>>();
        let name = f[0];
        let size = f[1].parse().unwrap_or(DEFAULT_FONT_SIZE);
        Ok(DefaultFont::new(name, size))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogFont {
    /// フォント名
    #[serde(default)]
    pub name: String,
    /// フォントサイズ
    #[serde(default)]
    pub size: i32
}
impl Default for LogFont {
    fn default() -> Self {
        Self { name: "MS Gothic".into(), size: 15 }
    }
}
impl LogFont {
    pub fn new(name: &str, size: i32) -> Self {
        Self {name: name.into(), size}
    }
}
impl FromStr for LogFont {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = s.split(",").collect::<Vec<_>>();
        let name = f[0];
        let size = f[1].parse().unwrap_or(15);
        Ok(LogFont::new(name, size))
    }
}

fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = ()>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = ()>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Browser {
    /// Chromeのパス
    #[serde(default)]
    pub chrome: Option<String>,
    /// MSEdgeのパス
    #[serde(default)]
    pub msedge: Option<String>,
    #[serde(skip_serializing, default)]
    #[schemars(skip)]
    pub vivaldi: Option<String>,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Chkimg {
    /// chkimg()実行時の画面を保存するかどうか
    #[serde(default)]
    pub save_ss: bool,
}

// pub fn usettings_singleton(usettings: Option<USettings>) -> Box<SingletonSettings> {
//     static mut SINGLETON: Option<Box<SingletonSettings>> = None;
//     static ONCE: Once = Once::new();
//     unsafe {
//         ONCE.call_once(|| {
//             let s = match usettings {
//                 Some(s) => s,
//                 None => USettings::default()
//             };
//             let singlton = SingletonSettings(
//                 Arc::new(Mutex::new(s))
//             );
//             SINGLETON = Some(Box::new(singlton));
//         });
//         SINGLETON.clone().unwrap()
//     }
// }

#[derive(Debug, Clone)]
pub struct Error {
    msg: String
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Error {
            msg: e.to_string(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error {
            msg: e.to_string(),
        }
    }
}
impl From<&Error> for Error {
    fn from(e: &Error) -> Self {
        e.clone()
    }
}

// pub fn load_settings() -> Result<Box<SingletonSettings>, Error> {
//     let path = SETTING_FILE_PATH.as_ref()?;

//     let usettings = if path.exists() {
//         Some(USettings::from_file(path)?)
//     } else {
//         None
//     };
//     let singleton = usettings_singleton(usettings);
//     Ok(singleton)
// }

pub enum FileMode {
    Open,
    Init,
    Merge
}
impl From<&String> for FileMode {
    fn from(s: &String) -> Self {
        let mode = match s.to_ascii_lowercase().as_str() {
            "init" => Self::Init,
            "merge" => Self::Merge,
            _ => Self::Open
        };
        mode
    }
}
pub fn out_default_setting_file(mode: FileMode) -> Result<String, Error> {
    let files = SETTING_FILES.as_ref()?;
    // ファイルが無ければ必ず新規作成
    let mode = if ! files.settings.exists() {FileMode::Init} else {mode};
    match mode {
        FileMode::Open => {},
        FileMode::Init => {
            let s = USettings::default();
            s.to_file(&files.settings)?;
            // ローカルjson schemaも更新
            save_json_schema(&files.schema)?;
        },
        FileMode::Merge => {
            let s = USettings::from_file(&files.settings)?;
            s.to_file(&files.settings)?;
            // ローカルjson schemaも更新
            save_json_schema(&files.schema)?;
        },
    }
    shell_execute(files.settings.to_str().unwrap().to_string(), None);
    Ok(format!("Opening {}", files.settings.to_str().unwrap()))
}
fn save_json_schema(path: &std::path::Path) -> Result<(), Error> {
    let parent = path.parent().unwrap().to_path_buf();
    out_json_schema_file(parent)?;
    Ok(())
}

pub fn out_json_schema_file(mut path: PathBuf) -> Result<String, Error> {
    if ! path.exists() {
        create_dir_all(&path)?
    }
    path.push("uwscr-settings-schema.json");

    let schema = schema_for!(USettings);
    let json = serde_json::to_string_pretty(&schema)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open::<&PathBuf>(&path)?;
    write!(file, "{}", json)?;

    Ok(format!("Created {}", path.to_str().unwrap()))
}
