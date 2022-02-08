use crate::{
    evaluator::builtins::system_controls::shell_execute,
    winapi::get_special_directory,
};
use windows::Win32::UI::Shell::CSIDL_APPDATA;

use core::fmt;
use std::{
    fs::{
        OpenOptions,
        create_dir_all,
        read
    },
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex, Once}
};

use serde::{Serialize, Deserialize};
use serde_json;
use schemars::{schema_for, JsonSchema};

#[derive(Debug, Clone)]
pub struct SingletonSettings(pub Arc<Mutex<USettings>>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct USettings {
    #[serde(default)]
    pub options: UOption,
    #[serde(default)]
    pub browser: Browser,
    #[serde(default)]
    pub chkimg: Chkimg,
    #[serde(skip_deserializing, rename(serialize = "$schema"))]
    pub schema: String,
}

impl USettings {
    pub fn get_current_settings_as_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or(String::new())
    }
}

impl Default for USettings {
    fn default() -> Self {
        let uri = "https://github.com/stuncloud/UWSCR/releases/download/0.3.0/uwscr-settings-schema.json".to_string();
        let schema = if cfg!(debug_assertions) {
            match std::env::current_dir() {
                Ok(mut p) => {
                    p.push("schema");
                    p.push("uwscr-settings-schema.json");
                    match url::Url::from_file_path(p) {
                        Ok(u) => u.as_str().to_string(),
                        Err(_) => uri
                    }
                },
                Err(_) => uri
            }
        } else {
            uri
        };
        USettings {
            options: UOption::default(),
            browser: Browser::default(),
            chkimg: Chkimg::default(),
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
    #[serde(default)]
    pub default_font: DefaultFont,
    /// 吹き出しを仮想デスクトップにも出すかどうか
    #[serde(default)]
    pub fix_balloon: bool,
    // /// stopボタン最前面に固定するかどうか (非対応)
    // #[serde(default)]
    // pub top_stop_form: bool
    // 停止ホットキー無効
    #[serde(default)]
    pub no_stop_hot_key: bool,
    /// 短絡評価の有無
    #[serde(default)]
    pub short_circuit: bool,
    // /// 特殊文字を展開しない (非対応)
    // #[serde(default)]
    // pub special_char: bool
    // publicの重複定義を禁止
    #[serde(default)]
    pub opt_public: bool,
    /// 大文字小文字を区別する
    #[serde(default)]
    pub same_str: bool,
    /// print窓を非表示
    #[serde(default)]
    pub disable_logprintwin: bool,
    /// 標準出力を有効にする
    #[serde(default)]
    pub enable_stdout: bool,
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
            opt_public: false,
            same_str: false,
            allow_ie_object: false,
            disable_logprintwin: false,
            enable_stdout: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UPosition {
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub top: i32,
}

impl Default for UPosition {
    fn default() -> Self {
        UPosition {
            left: 0,
            top: 0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DefaultFont {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: i32
}
impl Default for DefaultFont {
    fn default() -> Self {
        Self { name: "Yu Gothic UI".into(), size: 15 }
    }
}
impl DefaultFont {
    pub fn new(name: &str, size: i32) -> Self {
        Self {name: name.into(), size}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Browser {
    // Chromeのパス
    #[serde(default)]
    pub chrome: Option<String>,
    // MSEdgeのパス
    #[serde(default)]
    pub msedge: Option<String>,
}

impl Default for Browser {
    fn default() -> Self {
        Self {
            chrome: None,
            msedge: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Chkimg {
    // chkimg()実行時の画面を保存するかどうか
    #[serde(default)]
    pub save_ss: bool,
}
impl Default for Chkimg {
    fn default() -> Self {
        Self {
            save_ss: false,
        }
    }
}

pub fn usettings_singleton(usettings: Option<USettings>) -> Box<SingletonSettings> {
    static mut SINGLETON: Option<Box<SingletonSettings>> = None;
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            let s = match usettings {
                Some(s) => s,
                None => USettings::default()
            };
            let singlton = SingletonSettings(
                Arc::new(Mutex::new(s))
            );
            SINGLETON = Some(Box::new(singlton));
        });
        SINGLETON.clone().unwrap()
    }
}

impl USettings {
    // fn load_from_file() {}
}

#[derive(Debug)]
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

pub fn load_settings() -> Result<Box<SingletonSettings>, Error> {
    let mut path = PathBuf::from(
        get_special_directory(CSIDL_APPDATA as i32)
    );
    path.push("UWSCR");
    path.push("settings.json");

    let usettings = if path.exists() {
        // jsonから設定を読み取る
        let json = read(&path)?;
        let from_json = serde_json::from_slice::<USettings>(&json)?;
        Some(from_json)
    } else {
        None
    };
    let singleton = usettings_singleton(usettings);
    Ok(singleton)
}

pub fn out_default_setting_file() -> Result<String, Error> {
    let mut path = PathBuf::from(
        get_special_directory(CSIDL_APPDATA as i32)
    );
    path.push("UWSCR");
    if ! path.exists() {
        create_dir_all(&path)?
    }
    path.push("settings.json");
    if ! path.exists() {
        let s = USettings::default();
        let json = serde_json::to_string_pretty::<USettings>(&s)?;
        let mut file = OpenOptions::new().create(true).write(true).open::<&PathBuf>(&path)?;
        write!(file, "{}", json)?;
    }
    shell_execute(path.to_str().unwrap().to_string(), None);
    Ok(format!("Opening {}", path.to_str().unwrap()))
}

pub fn out_json_schema_file(mut path: PathBuf) -> Result<String, Error> {
    if ! path.exists() {
        create_dir_all(&path)?
    }
    path.push("uwscr-settings-schema.json");

    let schema = schema_for!(USettings);
    let json = serde_json::to_string_pretty(&schema)?;
    let mut file = OpenOptions::new().create(true).write(true).open::<&PathBuf>(&path)?;
    write!(file, "{}", json)?;

    Ok(format!("Created {}", path.to_str().unwrap()))
}
