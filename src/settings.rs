use crate::{
    evaluator::builtins::system_controls::shell_execute,
    winapi::{
        get_special_directory,
        bindings::Windows::Win32::UI::Shell::CSIDL_APPDATA,
    }
};

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

#[derive(Debug, Clone)]
pub struct SingletonSettings(pub Arc<Mutex<USettings>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USettings {
    #[serde(default)]
    pub options: UOption,
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
        USettings {
            options: UOption::default(),
            schema: "https://github.com/stuncloud/UWSCR/releases/download/0.1.7/uwscr-settings-schema.json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UOption {
    // finally部を必ず実行する
    #[serde(default)]
    pub opt_finally: bool,
    // 変数宣言必須
    #[serde(default)]
    pub explicit: bool,
    // ダイアログタイトル
    #[serde(default)]
    pub dlg_title: Option<String>,
    // ログファイルの出力有無など
    #[serde(default)]
    pub log_file: u8,
    // ログの行数
    #[serde(default)]
    pub log_lines: u32,
    // ログファイルの出力先
    #[serde(default)]
    pub log_path: Option<String>,
    // メインGUIの座標
    #[serde(default)]
    pub position: UPosition,
    // ダイアログなどのフォント
    #[serde(default)]
    pub default_font: String,
    // 吹き出しを仮想デスクトップにも出すかどうか
    #[serde(default)]
    pub fix_balloon: bool,
    // // stopボタン最前面に固定するかどうか (非対応)
    // #[serde(default)]
    // pub top_stop_form: bool
    // 停止ホットキー無効
    #[serde(default)]
    pub no_stop_hot_key: bool,
    // 短絡評価の有無
    #[serde(default)]
    pub short_circuit: bool,
    // // 特殊文字を展開しない (非対応)
    // #[serde(default)]
    // pub special_char: bool
    // publicの重複定義を禁止
    #[serde(default)]
    pub opt_public: bool,
    // 大文字小文字を区別する
    #[serde(default)]
    pub same_str: bool,
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
            default_font: "MS Gothic,12".into(),
            fix_balloon: false,
            no_stop_hot_key: false,
            short_circuit: true,
            opt_public: false,
            same_str: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            msg: format!("{:?}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error {
            msg: format!("{:?}", e),
        }
    }
}

pub fn load_settings() -> Result<(), Error> {
    let mut path = PathBuf::from(
        get_special_directory(CSIDL_APPDATA as i32)
    );
    path.push("UWSCR");
    path.push("settings.json");

    if path.exists() {
        let json = read(&path)?;
        let from_json = serde_json::from_slice::<USettings>(&json)?;
        // jsonから読み取った設定をセット
        usettings_singleton(Some(from_json));
    }
    Ok(())
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
