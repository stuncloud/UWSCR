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
    pub options: UOption,
    #[serde(rename(serialize = "$schema"))]
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
            schema: "https://stuncloud.github.io/UWSCR/schema/uwscr-settings-schema.json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UOption {
    // finally部を必ず実行する
    pub opt_finally: bool,
    // 変数宣言必須
    pub explicit: bool,
    // ダイアログタイトル
    pub dlg_title: Option<String>,
    // ログファイルの出力有無
    pub log_file: bool,
    // ログの行数
    pub log_lines: u32,
    // ログファイルの出力先
    pub log_path: Option<String>,
    // メインGUIの座標
    pub position: UPosition,
    // ダイアログなどのフォント
    pub default_font: UFont,
    // 吹き出しを仮想デスクトップにも出すかどうか
    pub fix_balloon: bool,
    // // stopボタン最前面に固定するかどうか (非対応)
    // pub top_stop_form: bool
    // 停止ホットキー無効
    pub no_stop_hot_key: bool,
    // 短絡評価の有無
    pub short_circuit: bool,
    // // 特殊文字を展開しない (非対応)
    // pub special_char: bool
    // publicの重複定義を禁止
    pub opt_public: bool,
    // 大文字小文字を区別する
    pub same_str: bool,
}

impl Default for UOption {
    fn default() -> Self {
        UOption {
            opt_finally: false,
            explicit: false,
            dlg_title: None,
            log_file: true,
            log_lines: 400,
            log_path: None,
            position: UPosition::default(),
            default_font: UFont::default(),
            fix_balloon: false,
            no_stop_hot_key: false,
            short_circuit: true,
            opt_public: false,
            same_str: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UFont {
    font: String,
    size: u32,
}

impl Default for UFont {
    fn default() -> Self {
        UFont {
            font: "gothic".into(),
            size: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPosition {
    left: i32,
    top: i32,
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
