use super::Object;
use crate::builtins::window_control::get_id_from_hwnd;
use crate::error::{UError, UErrorKind, UErrorMessage};
use util::settings::USETTINGS;

use std::sync::{Arc, Mutex};
use std::net::TcpStream;
use std::collections::HashMap;
use std::fmt;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wmi::{WMIConnection, FilterValue, COMLibrary, WMIResult, WMIError};

use windows::Win32::{
        Foundation::{LPARAM, HWND, BOOL},
        UI::WindowsAndMessaging::{
            GW_OWNER,
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible, GetWindow,
        }
    };

type BrowserResult<T> = Result<T, UError>;

impl From<tungstenite::error::Error> for UError {
    fn from(e: tungstenite::error::Error) -> Self {
        Self::new(
            UErrorKind::WebSocketError,
            UErrorMessage::Any(e.to_string())
        )
    }
}
impl From<serde_json::Error> for UError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(
            UErrorKind::ConversionError,
            UErrorMessage::JsonParseError(e.to_string())
        )
    }
}
impl From<WMIError> for UError {
    fn from(e: WMIError) -> Self {
        Self::new(
            UErrorKind::WmiError,
            UErrorMessage::Any(e.to_string())
        )
    }
}
impl From<reqwest::Error> for UError {
    fn from(e: reqwest::Error) -> Self {
        Self::new(
            UErrorKind::WebRequestError,
            UErrorMessage::Any(e.to_string())
        )
    }
}

/// ブラウザ種別
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrowserType {
    Chrome,
    MSEdge,
    Vivaldi,
}

impl fmt::Display for BrowserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserType::Chrome => write!(f, "chrome.exe"),
            BrowserType::MSEdge => write!(f, "msedge.exe"),
            BrowserType::Vivaldi => write!(f, "vivaldi.exe"),
        }
    }
}

/// BrowserBuilderオブジェクト
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserBuilder {
    pub port: u16,
    pub r#type: BrowserType,
    pub headless: bool,
    pub private: bool,
    pub profile: Option<String>,
    args: Vec<String>,
}
impl fmt::Display for BrowserBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.r#type, self.port)
    }
}
impl BrowserBuilder {
    pub fn new(r#type: BrowserType, port: u16) -> Self {
        Self { port, r#type, headless: false, private: false, profile: None, args: vec![] }
    }
    pub fn port(&mut self, port: u16) {
        self.port = port;
    }
    fn headless(&mut self, headless: bool) {
        self.headless = headless;
    }
    fn private(&mut self, private: bool) {
        self.private = private;
    }
    fn profile(&mut self, profile: Option<String>) {
        self.profile = profile;
    }
    fn add_arg(&mut self, arg: String) {
        self.args.push(arg);
    }
    pub fn invoke_method(&mut self, name: &str, args: Vec<Object>) -> BrowserResult<Option<Browser>> {
        match name.to_ascii_lowercase().as_str() {
            "port" => {
                let port = args.as_f64(0)? as u16;
                self.port(port);
                Ok(None)
            },
            "headless" => {
                let headless = args.as_bool(0).unwrap_or(true);
                self.headless(headless);
                Ok(None)
            },
            "private" => {
                let private = args.as_bool(0).unwrap_or(true);
                self.private(private);
                Ok(None)
            },
            "profile" => {
                let profile = args.as_string(0).unwrap_or_default();
                let profile = if profile.is_empty() {None} else {Some(profile)};
                self.profile(profile);
                Ok(None)
            },
            "argument" => {
                let arg = args.as_string(0)?;
                self.add_arg(arg);
                Ok(None)
            },
            "start" => {
                let browser = self.start()?;
                Ok(Some(browser))
            },
            member => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(member.into())
            ))
        }
    }

    /// 以下の順にパスを確認し、いずれも得られなかった場合はエラーを返す
    /// 1. 設定ファイル
    /// 2. レジストリ (HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\)
    /// 3. レジストリ (HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\)
    fn get_browser_path(&self) -> BrowserResult<String> {
        let path = {
            let usettings = USETTINGS.lock().unwrap();
            match self.r#type {
                BrowserType::Chrome => usettings.browser.chrome.clone(),
                BrowserType::MSEdge => usettings.browser.msedge.clone(),
                BrowserType::Vivaldi => usettings.browser.vivaldi.clone(),
            }
        };
        match path {
            Some(path) => Ok(path),
            None => {
                let key = format!(r#"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{}"#, self.r#type);
                let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
                let subkey = match hklm.open_subkey(&key) {
                    Ok(subkey) => subkey,
                    Err(_) => {
                        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
                        hkcu.open_subkey(&key)?
                    },
                };
                Ok(subkey.get_value("")?)
            }
        }
    }
    fn run_browser(&self) -> BrowserResult<()> {
        let mut args = match self.r#type {
            BrowserType::Chrome |
            BrowserType::MSEdge |
            BrowserType::Vivaldi => {
                vec![
                    "--enable-automation".into(),
                    format!("--remote-debugging-port={}", self.port),
                ]
            },
        };
        if self.headless {
            args.push("--headless".into());
            args.push("--disable-gpu".into());
        }
        if self.private {
            let arg = match self.r#type {
                BrowserType::Chrome |
                BrowserType::Vivaldi => "-incognito",
                BrowserType::MSEdge => "-inprivate",
            }.into();
            args.push(arg)
        }
        if let Some(profile) = &self.profile {
            let arg = format!("--user-data-dir={profile}");
            args.push(arg);
        }
        if ! self.args.is_empty() {
            let mut user_args = self.args.clone();
            args.append(&mut user_args);
        }

        let path = self.get_browser_path()?;
        Command::new(&path)
            .args(args)
            .spawn()?;

        if Browser::wait_for_connection(self.port) {
            Ok(())
        } else {
            Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::FailedToOpenPort(self.port)
            ))
        }
    }
    fn test_connection(&self) -> BrowserResult<bool> {
        let name = self.r#type.to_string();
        match BrowserProcess::is_process_available(self.port, &name)? {
            ProcessFound::None => Ok(false),
            ProcessFound::Found => Ok(true),
            ProcessFound::NoPort => {
                if self.profile.is_some() {
                    // プロファイルが指定されている場合はエラーにしない
                    Ok(false)
                } else {
                    Err(UError::new(
                        UErrorKind::BrowserControlError,
                        UErrorMessage::BrowserHasNoDebugPort(name, self.port)
                    ))
                }
            },
            ProcessFound::UnMatch => {
                    Err(UError::new(
                        UErrorKind::BrowserControlError,
                        UErrorMessage::BrowserDebuggingPortUnmatch(name, self.port)
                    ))
            },
        }
    }
    pub fn start(&self) -> BrowserResult<Browser> {
        if ! self.test_connection()? {
            self.run_browser()?;
        }
        let version = Browser::get_request_t::<BrowserVersion>(self.port, "/json/version")?;
        let ws = WebSocket::new(&version.web_socket_debugger_url)?;

        Ok(Browser::new(self.port, self.r#type, version, Arc::new(Mutex::new(ws))))
    }
}


#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
struct BrowserVersion {
    #[serde(rename="Browser")]
    browser: String,
    #[serde(rename="Protocol-Version")]
    protocol_version: String,
    #[serde(rename="User-Agent")]
    user_agent: String,
    #[serde(rename="V8-Version")]
    v8_version: String,
    #[serde(rename="WebKit-Version")]
    webkit_version: String,
    #[serde(rename="webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

/// Browserオブジェクト
#[derive(Clone)]
pub struct Browser {
    pub port: u16,
    pub r#type: BrowserType,
    version: BrowserVersion,
    ws: Arc<Mutex<WebSocket>>,
}

impl fmt::Display for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "port: {}, browser: {}, protocol version: {}", self.port, self.version.browser, self.version.protocol_version)
    }
}
impl fmt::Debug for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Browser")
        .field("port", &self.port)
        .field("r#type", &self.r#type)
        .field("version", &self.version)
        .finish()
    }
}
impl PartialEq for Browser {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port &&
        self.r#type == other.r#type &&
        self.version == other.version
    }
}

impl Browser {
    fn new(port: u16, r#type: BrowserType, version: BrowserVersion, ws: Arc<Mutex<WebSocket>>) -> Self {
        Self { port, r#type, version, ws }
    }
    fn wait_for_connection(port: u16) -> bool {
        let addr = format!("localhost:{port}");
        let timeout = std::time::Duration::from_secs(5);
        let from = std::time::Instant::now();
        loop {
            if std::net::TcpStream::connect(&addr).is_ok() {
                break true;
            }
            if from.elapsed() >= timeout {
                break false;
            }
        }
    }
    fn request(port: u16, path: &str, put: bool) -> BrowserResult<String> {
        let uri = format!("http://localhost:{}{}", port, path);
        let client = reqwest::blocking::ClientBuilder::new()
            .no_proxy()
            .build()?;
        let response = if put {
            client.put(uri).send()?
        } else {
            client.get(uri).send()?
        };
        if response.status().is_success() {
            let text = response.text()?;
            Ok(text)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::WebResponseWasNotOk(response.status().to_string())
            ))
        }
    }
    fn request_t<T: DeserializeOwned>(port: u16, path: &str, put: bool) -> BrowserResult<T> {
        let response = Self::request(port, path, put)?;
        let t = serde_json::from_str(&response)?;
        Ok(t)
    }
    fn get_request(port: u16, path: &str) -> BrowserResult<String> {
        Self::request(port, path, false)
    }
    fn get_request_t<T: DeserializeOwned>(port: u16, path: &str) -> BrowserResult<T> {
        Self::request_t::<T>(port, path, false)
    }
    fn _put_request<T: DeserializeOwned>(port: u16, path: &str) -> BrowserResult<T> {
        Self::request_t::<T>(port, path, true)
    }
    fn send(&self, method: &str, params: Value) -> BrowserResult<Option<Value>> {
        let mut ws = self.ws.lock().unwrap();
        match ws.send(method, params)? {
            CDPReceived::Result(result) => Ok(Some(result)),
            CDPReceived::Error(err) => Err(UError::new(UErrorKind::DevtoolsProtocolError, UErrorMessage::DTPError(err.code, err.message.unwrap_or_default()))),
            CDPReceived::Dialog => Ok(None),
        }
    }
    fn tabs(&self) -> BrowserResult<Vec<TargetInfo>> {
        let value = self.send("Target.getTargets", json!({}))?
            .ok_or(UError::new(UErrorKind::BrowserControlError, UErrorMessage::DetectedDialogOpening))?;
        let infos = serde_json::from_value::<TargetInfos>(value)?;
        let tabs = infos.target_infos.into_iter()
            .filter(|target| {
                target.r#type == "page"
                && ! target.url.starts_with("devtools://")
                && ! target.url.starts_with("chrome-extension://")
            })
            .collect();
        Ok(tabs)
    }
    fn count(&self) -> BrowserResult<usize> {
        let count = self.tabs()?.len();
        Ok(count)
    }
    fn gen_ws_uri(&self, target_id: &str) -> String {
        format!("ws://localhost:{}/devtools/page/{}", self.port, target_id)
    }
    pub fn get_tabs(&self) -> BrowserResult<Vec<TabWindow>> {
        let tabs = self.tabs()?;
        tabs.iter()
            .map(|target| {
                let uri = self.gen_ws_uri(&target.target_id);
                TabWindow::new(self.port, target.target_id.to_string(), uri)
            })
            .collect()
    }
    pub fn get_tab(&self, index: usize) -> BrowserResult<TabWindow> {
        let tabs = self.get_tabs()?;
        let nth = tabs.into_iter().nth(index);
        if let Some(tab) = nth {
            Ok(tab)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::IndexOutOfBounds(index.into())
            ))
        }
    }
    fn close(&self) -> BrowserResult<()> {
        self.send("Browser.close", json!({}))?;
        Ok(())
    }
    fn new_tab(&self, uri: &str) -> BrowserResult<TabWindow> {
        let value = self.send("Target.createTarget", json!({
            "url": uri
        }))?.ok_or(UError::new(UErrorKind::BrowserControlError, UErrorMessage::DetectedDialogOpening))?;
        if let Value::String(target_id) = &value["targetId"] {
            let uri = self.gen_ws_uri(target_id);
            TabWindow::new(self.port, target_id.to_string(), uri)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidTabPage(uri.into())
            ))
        }
    }
    fn get_window_id(&self) -> BrowserResult<Object> {
        let pid = BrowserProcess::get_pid_from_port(self.port)?;
        let hwnd = BrowserProcess::get_hwnd_from_pid(pid);
        let id = get_id_from_hwnd(hwnd);
        Ok(id.into())
    }
    fn set_download_dir(&self, dir: String) -> BrowserResult<()> {
        self.send("Browser.setDownloadBehavior", json!({
            "behavior": "allow",
            "downloadPath": dir
        }))?;
        Ok(())
    }
    pub fn get_property(&self, name: &str) -> BrowserResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "protocol" => {
                let vertion = self.version.protocol_version.clone();
                Ok(vertion.into())
            },
            "count" => {
                let count = self.count()?;
                Ok(count.into())
            },
            "tabs" => {
                let tabs = self.get_tabs()?
                    .into_iter()
                    .map(Object::TabWindow)
                    .collect();
                Ok(Object::Array(tabs))
            },
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    pub fn invoke_method(&self, name: &str, args: Vec<Object>) -> BrowserResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "id" => {
                self.get_window_id()
            },
            "new" => {
                let uri = args.as_string(0)?;
                let tab = self.new_tab(&uri)?;
                Ok(Object::TabWindow(tab))
            },
            "close" => {
                self.close()?;
                Ok(Object::Empty)
            },
            "download" => {
                let dir = args.as_string(0)?;
                self.set_download_dir(dir)?;
                Ok(Object::Empty)
            }
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }

}

/// タブオブジェクト
#[derive(Debug, Clone)]
pub struct TabWindow {
    port: u16,
    id: String,
    dp: DevtoolsProtocol,
}
impl PartialEq for TabWindow {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port && self.id == other.id
    }
}
impl fmt::Display for TabWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl TabWindow {
    fn new(port: u16, id: String, uri: String) -> BrowserResult<Self> {
        let dp = DevtoolsProtocol::new(uri)?;
        Ok(Self { port, id, dp })
    }
    pub fn document(&self) -> BrowserResult<RemoteObject> {
        self.dp.runtime_evaluate("document")
    }
    pub fn close(&self) -> BrowserResult<()> {
        self.dp.send("Target.closeTarget", json!({
            "targetId": &self.id
        }))?;
        Ok(())
    }
    fn is_navigate_completed(&self) -> bool {
        // エラーは握りつぶしてfalseを返す
        if let Ok(document) = self.document() {
            if let Ok(state) = document.get_property("readyState") {
                match state.into_value() {
                    Some(v) => v.as_str().unwrap_or_default() == "complete",
                    None => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn wait_for_page_load(&self, limit: f64) -> BrowserResult<bool> {
        let from = Instant::now();
        loop {
            if self.is_navigate_completed() {
                sleep(Duration::from_millis(100));
                return Ok(true)
            } else if from.elapsed().as_secs_f64() >= limit {
                break;
            } else {
                sleep(Duration::from_millis(100));
            }
        }
        Ok(false)
    }
    pub fn navigate(&self, uri: &str) -> BrowserResult<bool> {
        self.dp.send("Page.navigate", json!({"url": uri}))?;
        self.wait_for_page_load(10.0)
    }
    pub fn reload(&self, ignore_cache: bool) -> BrowserResult<bool> {
        self.dp.send("Page.reload", json!({
            "ignoreCache": ignore_cache
        }))?;
        self.wait_for_page_load(10.0)
    }
    pub fn activate(&self) -> BrowserResult<()> {
        let path = format!("/json/activate/{}", &self.id);
        Browser::get_request(self.port, &path)?;
        Ok(())
    }
    fn dialog(&self, accept: bool, prompt: Option<String>) -> BrowserResult<()> {
        let params = match prompt {
            Some(text) => json!({
                "accept": accept,
                "promptText": text
            }),
            None => json!({
                "accept": accept
            }),
        };
        self.dp.send("Page.handleJavaScriptDialog", params)?;
        Ok(())
    }
    fn dialog_message(&self) -> Option<String> {
        let ws = self.dp.ws.lock().unwrap();
        ws.dlg_message.clone()
    }
    fn dialog_type(&self) -> Option<String> {
        let ws = self.dp.ws.lock().unwrap();
        ws.dlg_type.clone()
    }
    fn click(&self, button: &str, x: f64, y: f64) -> BrowserResult<()> {
        self.dp.send("Input.dispatchMouseEvent", json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button
        }))?;
        self.dp.send("Input.dispatchMouseEvent", json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button
        }))?;
        Ok(())
    }
    fn left_click(&self, x: f64, y: f64) -> BrowserResult<()> {
        self.click("left", x, y)
    }
    fn right_click(&self, x: f64, y: f64) -> BrowserResult<()> {
        self.click("right", x, y)
    }
    fn middle_click(&self, x: f64, y: f64) -> BrowserResult<()> {
        self.click("middle", x, y)
    }
    pub fn invoke_method(&self, name: &str, args: Vec<Object>) -> BrowserResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "navigate" => {
                let uri = args.as_string(0)?;
                self.navigate(&uri)
                    .map(|b| b.into())
            },
            "reload" => {
                let ignore_cache = args.as_bool(0)?;
                self.reload(ignore_cache)
                    .map(|b| b.into())
            },
            "wait" => {
                let limit = args.as_f64(0).unwrap_or(10.0);
                self.wait_for_page_load(limit)
                    .map(|b| b.into())
            },
            "activate" => {
                self.activate()?;
                Ok(Object::Empty)
            },
            "close" => {
                self.close()?;
                Ok(Object::Empty)
            },
            "dialog" => {
                let accept = args.as_bool(0).unwrap_or(true);
                let prompt = args.as_string(1).ok();
                self.dialog(accept, prompt)?;
                Ok(Object::Empty)
            },
            "dlgmsg" => {
                let msg = self.dialog_message();
                Ok(msg.into())
            }
            "dlgtype" => {
                let msg = self.dialog_type();
                Ok(msg.into())
            }
            "leftclick" => {
                let x = args.as_f64(0)?;
                let y = args.as_f64(1)?;
                self.left_click(x, y)?;
                Ok(Object::Empty)
            },
            "rightclick" => {
                let x = args.as_f64(0)?;
                let y = args.as_f64(1)?;
                self.right_click(x, y)?;
                Ok(Object::Empty)
            },
            "middleclick" => {
                let x = args.as_f64(0)?;
                let y = args.as_f64(1)?;
                self.middle_click(x, y)?;
                Ok(Object::Empty)
            },
            "eval" => {
                let expression = args.as_string(0)?;
                let remote = self.dp.runtime_evaluate(&expression)?;
                Ok(remote.into())
            },
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    pub fn get_property(&self, name: &str) -> BrowserResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "document" => {
                let document = self.document()?;
                Ok(document.into())
            },
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    fn query_selector_all(&self, selector: String) -> BrowserResult<impl Iterator<Item = RemoteObject>> {
        let document = self.document()?;
        let args = vec![
            RemoteFuncArg::Value(Value::String(selector))
        ];
        let id = document.remote.object_id.as_ref().unwrap();
        let declaration = "function(selector) {return this.querySelectorAll(selector);}";
        let elements = document.dp.invoke_function(id, declaration, args, false, false)?;
        elements.into_iter()
    }
    fn get_nth_element_by_name_value(&self, name: String, value: Option<String>, nth: usize) -> BrowserResult<Option<RemoteObject>> {
        let selector = match value {
            Some(value) => format!("*[name=\"{name}\"][value=\"{value}\"]"),
            None => format!("*[name=\"{name}\"]"),
        };
        let mut elements = self.query_selector_all(selector)?;
        Ok(elements.nth(nth - 1))
    }
    fn get_nth_element_by_tagname_and_property(&self, tag: String, prop_name: &str, prop_value: &str, nth: usize) -> BrowserResult<Option<RemoteObject>> {
        let mut elements = self.query_selector_all(tag)?.filter(|remote| {
            let prop_val = remote.get_property(prop_name).ok().and_then(|r| r.into_value());
            match prop_val {
                Some(val) => {
                    val.as_str().unwrap_or_default().eq_ignore_ascii_case(prop_value)
                },
                None => false,
            }
        });
        Ok(elements.nth(nth - 1))
    }
    pub fn get_data_by_name_value(&self, name: String, value: Option<String>, nth: usize) -> BrowserResult<Object> {
        match self.get_nth_element_by_name_value(name, value, nth)? {
            Some(remote) => remote.as_element_value(),
            None => Ok(Object::Empty),
        }
    }
    pub fn get_data_by_tagname(&self, tag: String, nth: usize) -> BrowserResult<Object> {
        let mut elements = self.query_selector_all(tag)?;
        match elements.nth(nth - 1) {
            Some(remote) => remote.as_element_value(),
            None => Ok(Object::Empty),
        }
    }
    pub fn get_data_by_tagname_and_property(&self, tag: String, prop_name: &str, prop_value: &str, nth: usize) -> BrowserResult<Object> {
        match self.get_nth_element_by_tagname_and_property(tag, prop_name, prop_value, nth)? {
            Some(remote) => remote.as_element_value(),
            None => Ok(Object::Empty),
        }
    }
    pub fn get_data_by_table_point(&self, nth: usize, row: usize, col: usize) -> BrowserResult<Object> {
        let mut tables = self.query_selector_all("table".into())?;
        match tables.nth(nth - 1) {
            Some(table) => {
                let row = format!("{}", row - 1);
                let rows = table.get_property_by_index("rows", &row)?;
                let col = format!("{}", col - 1);
                match rows.get_property_by_index("cells", &col) {
                    Ok(cell) => {
                        match cell.get_property("textContent")?.into_value() {
                            Some(v) => Ok(v.into()),
                            None => Ok(Object::Empty),
                        }
                    },
                    Err(_) => Ok(Object::Empty),
                }
            },
            None => Ok(Object::Empty),
        }
    }
    pub fn set_data_by_name_value(&self, new_value: Vec<String>, name: String, value: Option<String>, nth: usize, direct: bool) -> BrowserResult<Object> {
        match self.get_nth_element_by_name_value(name, value, nth)? {
            Some(remote) => {
                if direct {
                    let new = new_value.first().map(|s| s.to_string()).unwrap_or_default();
                    remote.set_property("value", RemoteFuncArg::Value(json!(&new)))?;
                    let v = remote.get_property("value")?;
                    let eq = v.into_value().unwrap_or_default() == json!(new);
                    Ok(eq.into())
                } else {
                    remote.emulate_key_input(new_value)
                        .map(|b| b.into())
                }
            },
            None => Ok(false.into()),
        }
    }
    pub fn click_by_name_value(&self, name: String, value: Option<String>, nth: usize) -> BrowserResult<Object> {
        match self.get_nth_element_by_name_value(name, value, nth)? {
            Some(remote) => remote.set_data_click(),
            None => Ok(false.into()),
        }
    }
    pub fn click_by_nth_tag(&self, tag: String, nth: usize) -> BrowserResult<Object> {
        let mut elements = self.query_selector_all(tag)?;
        match elements.nth(nth - 1) {
            Some(remote) => remote.set_data_click(),
            None => Ok(false.into()),
        }
    }
    pub fn click_by_tag_and_property(&self, tag: String, prop_name: &str, prop_value: &str, nth: usize) -> BrowserResult<Object> {
        match self.get_nth_element_by_tagname_and_property(tag, prop_name, prop_value, nth)? {
            Some(remote) => remote.set_data_click(),
            None => Ok(false.into()),
        }
    }
    pub fn click_img(&self, src: Option<String>, nth: usize) -> BrowserResult<Object> {
        let selector = match src {
            Some(src) => format!("img[src=\"{src}\"]"),
            None => "img".into(),
        };
        let mut images = self.query_selector_all(selector)?;
        match images.nth(nth - 1) {
            Some(remote) => remote.set_data_click(),
            None => Ok(false.into()),
        }
    }
    pub fn get_source(&self, tag: String, nth: usize) -> BrowserResult<Object> {
        let mut elements = self.query_selector_all(tag)?;
        match elements.nth(nth - 1) {
            Some(remote) => {
                match remote.get_property("outerHTML")?.into_value() {
                    Some(value) => Ok(value.into()),
                    None => Ok(Object::Empty),
                }
            },
            None => Ok(Object::Empty),
        }
    }
    pub fn click_link(&self, text: String, nth: usize, exact_match: bool) -> BrowserResult<Object> {
        let links = self.query_selector_all("a".into())?;
        let link = links.filter(|remote| remote.match_text_content(&text, exact_match))
            .nth(nth - 1);
        match link {
            Some(remote) => {
                remote.invoke_method("click", vec![], false)?;
                Ok(true.into())
            },
            None => Ok(false.into()),
        }
    }
}

#[derive(Clone)]
struct DevtoolsProtocol {
    // uri: String,
    // ws: Arc<Mutex<Option<WebSocket>>>,
    ws: Arc<Mutex<WebSocket>>,
}
impl fmt::Debug for DevtoolsProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DevtoolsProtocol").finish()
    }
}
impl DevtoolsProtocol {
    fn new(uri: String) -> BrowserResult<Self> {
        let mut ws = WebSocket::new(&uri)?;
        // PageとRuntimeを有効にする
        ws.send("Page.enable", json!({}))?;
        ws.send("Runtime.enable", json!({}))?;

        let dp = Self { ws: Arc::new(Mutex::new(ws)) };
        Ok(dp)
    }
    fn send(&self, method: &str, params: Value) -> BrowserResult<Option<Value>> {
        let mut ws = self.ws.lock().unwrap();
        match ws.send(method, params)? {
            CDPReceived::Result(result) => Ok(Some(result)),
            CDPReceived::Error(err) => {
                let code = err.code;
                let message = err.message.unwrap_or_default();
                Err(UError::new(UErrorKind::DevtoolsProtocolError, UErrorMessage::DTPError(code, message)))
            },
            CDPReceived::Dialog => Ok(None),
        }
    }
    fn send_t<T: DeserializeOwned + Default>(&self, method: &str, params: Value) -> BrowserResult<T> {
        match self.send(method, params)? {
            Some(value) => {
                let t: T = serde_json::from_value(value)?;
                Ok(t)
            },
            None => Ok(T::default()),
        }
    }
    fn runtime_evaluate(&self, expression: &str) -> BrowserResult<RemoteObject> {
        let result = self.send_t::<RuntimeResult>("Runtime.evaluate", json!({
            "expression": expression
        }))?;
        if let Some(exception) = result.exception_details {
            Err(exception.into())
        } else {
            Ok(RemoteObject::new(self.clone(), result.result))
        }
    }
    fn invoke_function(&self, id: &str, declaration: &str, args: Vec<RemoteFuncArg>, user_gesture: bool, await_promise: bool) -> BrowserResult<RemoteObject> {
        let args = args.into_iter()
            .map(|v| {
                match v {
                    RemoteFuncArg::Value(v) => json!({"value": v}),
                    RemoteFuncArg::RemoteObject(ro) => {
                        if let Some(id) = ro.remote.object_id {
                            json!({"objectId": id})
                        } else {
                            json!({"value": ro.remote.value})
                        }
                    },
                }
            })
            .collect();
        let arguments = Value::Array(args);
        let result = self.send_t::<RuntimeResult>("Runtime.callFunctionOn", json!({
            "functionDeclaration": declaration,
            "objectId": id,
            "arguments": arguments,
            "userGesture": user_gesture,
            "awaitPromise": await_promise,
        }))?;
        if let Some(exception) = result.exception_details {
            Err(exception.into())
        } else {
            let remote = RemoteObject::new(self.clone(), result.result);
            Ok(remote)
        }
    }
}

struct WebSocket {
    pub socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    pub id: u32,
    // event_handler: HashMap<String, fn(&Value) -> BrowserResult<()>>,
    dlg_message: Option<String>,
    dlg_type: Option<String>,
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        let _ = self.socket.close(None);
    }
}

impl WebSocket {
    fn new(uri: &str) -> BrowserResult<Self> {
        let (socket, response) = tungstenite::connect(uri)?;
        let status = response.status();
        if status.as_u16() >= 400 {
            return Err(UError::new(UErrorKind::WebSocketError, UErrorMessage::WebSocketConnectionError(status.to_string())));
        }
        let ws = Self {
            socket,
            id: 0,
            dlg_message: None,
            dlg_type: None,
        };
        Ok(ws)
    }
    fn next_id(&mut self) -> u32 {
        self.id += 1;
        self.id
    }
    fn genereate_ws_data(&mut self, method: &str, params: Value) -> Value {
        json!({
            "id": self.next_id(),
            "method": method,
            "params": params,
        })
    }
    fn send(&mut self, method: &str, params: Value) -> BrowserResult<CDPReceived> {
        let data = self.genereate_ws_data(method, params);
        let _id = data["id"].as_u64().unwrap_or_default() as u32;
        let msg = data.to_string();
        let message = tungstenite::Message::Text(msg);
        self.socket.send(message)?;
        let received = loop {
            let message = self.socket.read()?;
            if message.is_text() {
                let text = message.into_text()?;
                let msg = serde_json::from_str::<Message>(&text)?;
                match CDPMessage::from(msg) {
                    CDPMessage::Result(id, result) => {
                        if id == _id {
                            break CDPReceived::Result(result);
                        }
                    },
                    CDPMessage::Error(id, error) => {
                        if id == _id {
                            break CDPReceived::Error(error);
                        }
                    },
                    CDPMessage::Event(event) => {
                        match event.method() {
                            "Page.javascriptDialogOpening" => {
                                let message = &event.params["message"];
                                self.dlg_message = message.as_str().map(|msg| msg.into());
                                let r#type = &event.params["type"];
                                self.dlg_type = r#type.as_str().map(|t| t.into());
                                break CDPReceived::Dialog;
                            },
                            "Page.javascriptDialogClosed" => {
                                self.dlg_message = None;
                                self.dlg_type = None;
                            },
                            _ => {}
                        }
                    },
                    CDPMessage::Unknown(id) => {
                        let log = format!("received unknown message {id:?} on CDP");
                        util::logging::out_log(&log, util::logging::LogType::Info)
                    },
                }
            }
        };
        Ok(received)
    }
}

#[derive(Deserialize)]
struct Message {
    id: Option<u32>,
    result: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
    error: Option<CDPError>
}
#[derive(Deserialize)]
struct CDPError {
    code: i32,
    message: Option<String>
}
struct CDPEvent {
    method: String,
    params: Value,
}
impl CDPEvent {
    fn method(&self) -> &str {
        &self.method
    }
}
enum CDPMessage {
    Result(u32, Value),
    Event(CDPEvent),
    Error(u32, CDPError),
    Unknown(Option<u32>),
}
impl From<Message> for CDPMessage {
    fn from(msg: Message) -> Self {
        if let Some(id) = msg.id {
            if let Some(result) = msg.result {
                Self::Result(id, result)
            } else if let Some(error) = msg.error {
                Self::Error(id, error)
            } else {
                Self::Unknown(Some(id))
            }
        } else if let Some(method) = msg.method {
            Self::Event(CDPEvent {
                method,
                params: msg.params.unwrap_or_default(),
            })
        } else {
            Self::Unknown(None)
        }
    }
}
enum CDPReceived {
    Result(Value),
    Error(CDPError),
    Dialog,
}

enum ProcessFound{
    /// 対象プロセスも指定ポートもない
    None,
    /// 対象プロセスが指定ポートを開いている
    Found,
    /// 対象プロセスが指定ポートを開いていない
    NoPort,
    /// 指定ポートを開いているプロセスとマッチしない
    UnMatch,
}
struct BrowserProcess;
impl BrowserProcess {
    fn is_process_available(port: u16, name: &str) -> BrowserResult<ProcessFound> {
        // ポートを確認
        let ncon = Self::new_wmi_connection(Some("Root\\StandardCimv2"))?;
        let mut filters = HashMap::new();
        filters.insert("LocalPort".to_string(), FilterValue::Number(port.into()));
        filters.insert("State".to_string(), FilterValue::Number(2));
        let tcpcons: Vec<NetTCPConnection> = ncon.filtered_query(&filters)?;
        // プロセスを確認
        let pcon = Self::new_wmi_connection(None)?;
        let mut filters = HashMap::new();
        filters.insert("Name".into(), FilterValue::String(name.into()));
        let processes: Vec<Win32Process> = pcon.filtered_query(&filters)?;

        if let Some(tcpcon) = tcpcons.first() {
            if !processes.is_empty() {
                let found = processes.iter()
                    .any(|p| p.process_id == tcpcon.owning_process);
                if found {
                    Ok(ProcessFound::Found)
                } else {
                    Ok(ProcessFound::UnMatch)
                }
            } else {
                Ok(ProcessFound::UnMatch)
            }
        } else if !processes.is_empty() {
            Ok(ProcessFound::NoPort)
        } else {
            Ok(ProcessFound::None)
        }
    }
    fn new_wmi_connection(namespace: Option<&str>) -> WMIResult<WMIConnection> {
        unsafe {
            let com_lib = COMLibrary::assume_initialized();
            match namespace {
                Some(namespace_path) => WMIConnection::with_namespace_path(namespace_path, com_lib),
                None => WMIConnection::new(com_lib),
            }
        }
    }
    fn get_pid_from_port(port: u16) -> BrowserResult<u32>  {
        let connection = Self::new_wmi_connection(Some("Root\\StandardCimv2"))?;
        let mut filters = HashMap::new();
        filters.insert("LocalPort".to_string(), FilterValue::Number(port.into()));
        filters.insert("state".to_string(), FilterValue::Number(2));
        let result: Vec<NetTCPConnection> = connection.filtered_query(&filters)?;
        let pid = if !result.is_empty() {
            result[0].owning_process
        } else {
            0
        };
        Ok(pid)
    }
    fn get_hwnd_from_pid(pid: u32) -> HWND {
        let mut data = LparamData::new(pid);
        let lparam = &mut data as *mut LparamData as isize;
        unsafe {
            let _ = EnumWindows(Some(Self::enum_window_proc), LPARAM(lparam));
        }
        data.1
    }

    unsafe extern "system"
    fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let data = &mut *(lparam.0 as *mut LparamData);
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if data.0 == pid && GetWindow(hwnd, GW_OWNER) == HWND::default() && IsWindowVisible(hwnd).as_bool() {
                data.1 = hwnd;
                false.into()
            } else {
                true.into()
            }
        }
    }
}


#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_NetTCPConnection")]
#[serde(rename_all = "PascalCase")]
struct NetTCPConnection {
    owning_process: u32
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
struct Win32Process {
    process_id: u32,
}

struct LparamData(u32, HWND);
impl LparamData {
    pub fn new(pid: u32) -> Self {
        Self(pid, HWND::default())
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BrowserListItem {
    description : String,
    #[serde(rename="devtoolsFrontendUrl")]
    devtools_frontend_url : String,
    id : String,
    title : String,
    r#type : String,
    url : String,
    #[serde(rename="webSocketDebuggerUrl")]
    web_socket_debugger_url : String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct RuntimeResult {
    pub result: RemoteObject0,
    #[serde(rename="exceptionDetails")]
    pub exception_details: Option<ExceptionDetails>
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ExceptionDetails {
    #[serde(rename="exceptionId")]
    exception_id: i32,
    text: String,
    #[serde(rename="lineNumber")]
    line_number: i32,
    #[serde(rename="columnNumber")]
    column_number: i32,
    #[serde(rename="scriptId")]
    script_id: Option<String>,
    url: Option<String>,
    #[serde(rename="stackTrace")]
    stack_trace: Option<Value>,
    exception: Option<RemoteObject0>,
    #[serde(rename="executionContextId")]
    execution_context_id: Option<i32>,
    #[serde(rename="exceptionMetaData")]
    exception_meta_data: Option<Value>,
}
impl fmt::Display for ExceptionDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(remote) = &self.exception {
            if let Some(description) = &remote.description {
                return write!(f, "{description}");
            }
        }
        write!(f, "Runtime Exception: {}", self.text)
    }
}
impl From<ExceptionDetails> for UError {
    fn from(val: ExceptionDetails) -> Self {
        UError::new(
            UErrorKind::BrowserControlError,
            UErrorMessage::BrowserRuntimeException(val.to_string())
        )
    }
}

// #[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct RemoteObject0 {
    pub r#type: String,
    subtype: Option<String>,
    #[serde(rename="className")]
    class_name: Option<String>,
    pub value: Option<Value>,
    // #[serde(rename="unserializableValue")]
    // unserializable_value: Option<String>,
    description: Option<String>,
    // #[serde(rename="webDriverValue")]
    // web_driver_value: Option<Value>,
    #[serde(rename="objectId")]
    pub object_id: Option<String>,
    // preview: Option<Value>,
    // #[serde(rename="customPreview")]
    // custom_preview: Option<Value>
}

#[derive(Clone)]
pub struct RemoteObject {
    dp: DevtoolsProtocol,
    remote: RemoteObject0,
}
impl PartialEq for RemoteObject {
    fn eq(&self, other: &Self) -> bool {
        self.remote == other.remote
    }
}

impl fmt::Debug for RemoteObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteObject").field("remote", &self.remote).finish()
    }
}
impl fmt::Display for RemoteObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.remote.object_id {
            write!(f, "RemoteObject({id})")
        } else {
            match &self.remote.value {
                Some(value) => {
                    write!(f, "{value}")
                },
                None => write!(f, "NULL"),
            }
        }
    }
}

impl RemoteObject {
    fn new(dp: DevtoolsProtocol, remote: RemoteObject0) -> Self {
        Self { dp, remote }
    }

    fn get_property(&self, name: &str) -> BrowserResult<Self> {
        let func = format!("function() {{return this.{name};}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    fn set_property(&self, name: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
        let func = format!("function(value) {{return this.{name} = value;}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![value], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    fn get_property_by_index(&self, name: &str, index: &str) -> BrowserResult<Self> {
        let func = format!("function() {{return this.{name}[{index}];}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    fn set_property_by_index(&self, name: &str, index: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
        let func = format!("function(value) {{return this.{name}[{index}] = value;}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![value], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    fn get_by_index(&self, index: &str) -> BrowserResult<Self> {
        let func = format!("function() {{return this[{index}];}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
            ))
        }
    }
    fn set_by_index(&self, index: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
        let func = format!("function(value) {{return this[{index}] = value;}}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &func, vec![value], false, false)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
            ))
        }
    }
    fn into_value(self) -> Option<Value> {
        self.remote.value
    }
    fn into_string(self) -> String {
        let value = self.into_value().unwrap_or_default();
        value.as_str().unwrap_or_default().to_string()
    }
    pub fn get(&self, name: Option<&str>, index: Option<&str>) -> BrowserResult<Object> {
        let result = match (name, index) {
            (None, None) => todo!(),
            (None, Some(index)) => self.get_by_index(index),
            (Some(name), None) => self.get_property(name),
            (Some(name), Some(index)) => self.get_property_by_index(name, index),

        };
        result.map(|remote| remote.into_object())
    }
    pub fn set(&self, name: Option<&str>, index: Option<&str>, value: RemoteFuncArg) -> BrowserResult<Object> {
        let result = match (name, index) {
            (None, None) => todo!(),
            (None, Some(index)) => self.set_by_index(index, value),
            (Some(name), None) => self.set_property(name, value),
            (Some(name), Some(index)) => self.set_property_by_index(name, index, value),
        };
        result.map(|remote| remote.into_object())
    }
    pub fn invoke_method(&self, name: &str, args: Vec<RemoteFuncArg>, await_promise: bool) -> BrowserResult<Object> {
        let declaration = format!("function(...args) {{ return this.{name}(...args); }}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, args, true, await_promise)
                .map(|remote| remote.into_object())
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    pub fn invoke_as_function(&self, args: Vec<RemoteFuncArg>, await_promise: bool) -> BrowserResult<Object> {
        let declaration = "function(...args) { return this(...args); }".to_string();
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, args, true, await_promise)
                .map(|remote| remote.into_object())
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotFunction(self.remote.r#type.clone())
            ))
        }
    }
    fn into_object(self) -> Object {
        if self.remote.object_id.is_some() {
            Object::RemoteObject(self)
        } else {
            match self.remote.value {
                Some(value) => value.into(),
                None => Object::Empty,
            }
        }
    }
    pub fn is_object(&self) -> bool {
        self.remote.object_id.is_some()
    }
    pub fn is_promise(&self) -> bool {
        if let Some(sub) = &self.remote.subtype {
            sub == "promise"
        } else {
            false
        }
    }
    pub fn length(&self) -> BrowserResult<f64> {
        let len = self.get_property("length")?;
        if let Some(value) = &len.remote.value {
            if let Some(n) = value.as_f64() {
                Ok(n)
            } else {
                Err(UError::new(
                    UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectDoesNotHaveValidLength
                ))
            }
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
            UErrorMessage::RemoteObjectDoesNotHaveValidLength
        ))
        }
    }
    fn into_js_iterator(self) -> BrowserResult<RemoteObject> {
        let declaration = "function() { return [...this].values(); }";
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, declaration, vec![], false, false)
                .map_err(|_| UError::new(
                    UErrorKind::BrowserControlError,
                    UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
                ))
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
            ))
        }
    }
    fn js_iterator_next(&self) -> BrowserResult<RemoteObject> {
        let declaration = "function() { return this.next(); }";
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, declaration, vec![], false, false)
                .map_err(|_| UError::new(
                    UErrorKind::BrowserControlError,
                    UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
                ))
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())
            ))
        }
    }
    fn into_iter(self) -> BrowserResult<impl Iterator<Item = RemoteObject>> {
        let iter = self.into_js_iterator()?;
        let mut vec = vec![];
        loop {
            let next = iter.js_iterator_next()?;
            let done = next.get_property("done")?;
            if let Some(Value::Bool(b)) = done.remote.value {
                if b {
                    break;
                } else {
                    let value = next.get_property("value")?;
                    vec.push(value);
                }
            } else {
                break;
            }
        }
        Ok(vec.into_iter())
    }
    pub fn to_object_vec(self) -> BrowserResult<Vec<Object>> {
        let vec = self.into_iter()?.map(|remote| remote.into()).collect();
        Ok(vec)
    }
    pub fn get_type(&self) -> String {
        let mut t = self.remote.r#type.clone();
        if let Some(sub) = &self.remote.subtype {
            t.push_str(": ");
            t.push_str(sub);
        }
        if let Some(class) = &self.remote.class_name {
            t.push_str(" [");
            t.push_str(class);
            t.push(']')
        }
        t
    }
    /// Promiseであれば待つ、PromiseでなければNone
    pub fn await_promise(&self) -> BrowserResult<Option<Self>> {
        if self.is_promise() {
            if let Some(id) = &self.remote.object_id {
                let result = self.dp.send_t::<RuntimeResult>("Runtime.awaitPromise", json!({
                    "promiseObjectId": id
                }))?;
                if let Some(exception) = result.exception_details {
                    Err(exception.into())
                } else {
                    let remote = RemoteObject::new(self.dp.clone(), result.result);
                    Ok(Some(remote))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    /// IE関数互換関数群で使うエレメントの値を返す関数
    fn as_element_value(&self) -> BrowserResult<Object> {
        let value = self.get_property("tagName")?.into_value().unwrap_or_default();
        let tag_name = value.as_str().unwrap_or_default();
        match tag_name.to_ascii_uppercase().as_str() {
            "SELECT" => {
                // SELECT要素は選択されたOptionのテキストを返す
                let texts = self.get_property("selectedOptions")?.into_iter()?
                    .filter_map(|opt| opt.get_property("textContent").ok())
                    .filter_map(|text| text.into_value())
                    .filter_map(|value| value.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                Ok(texts.join(" ").to_string().into())
            },
            "INPUT" => {
                let value = self.get_property("type")?.into_value().unwrap_or_default();
                let type_name = value.as_str().unwrap_or_default();
                match type_name.to_ascii_uppercase().as_str() {
                    // 特定のINPUT要素はvalue以外を返す
                    "RADIO" | "CHECKBOX" => {
                        let checked = self.get_property("checked")?.into_value().unwrap_or_default().as_bool().unwrap_or(false);
                        Ok(checked.into())
                    },
                    _ => {
                        self.get_property("value").map(|remote| remote.into_object())
                    }
                }
            },
            _ => {
                // 上記以外の要素はtextContentを返す
                self.get_property("textContent").map(|remote| remote.into_object())
            }
        }
    }
    fn set_data_click(&self) -> BrowserResult<Object> {
        if let Some(id) = &self.remote.object_id {
            // イベントハンドラを作成
            let func = "(function() {this.uwscr_brsetdata_click_flg = true;})";
            let handler = self.dp.runtime_evaluate(func)?;
            let handler_id = handler.remote.object_id.clone().unwrap();
            // 自身にclickイベントハンドラを追加する
            self.invoke_method("addEventListener", vec![
                RemoteFuncArg::Value(json!("click")),
                RemoteFuncArg::RemoteObject(handler.clone()),
            ], false)?;
            // clickメソッドを実行
            self.invoke_method("click", vec![], false)?;
            // クリック成否を得る
            let clicked = self.get_property("uwscr_brsetdata_click_flg")?.into_value().unwrap_or_default().as_bool().unwrap_or(false);
            // 後始末
            // イベントハンドラの登録を解除
            self.invoke_method("removeEventListener", vec![
                RemoteFuncArg::Value(json!("click")),
                RemoteFuncArg::RemoteObject(handler),
            ], false)?;
            // 一時的なプロパティを消す
            let declaration = "function() {delete this.uwscr_brsetdata_click_flg;}";
            self.dp.invoke_function(id, declaration, vec![], false, false)?;
            // イベントハンドラをリリース
            self.dp.send("Runtime.releaseObject", json!({
                "objectId": handler_id
            }))?;
            Ok(clicked.into())
        } else {
            Ok(false.into())
        }
    }
    fn match_text_content(&self, text: &str, exact_match: bool) -> bool {
        match self.get_property("textContent") {
            Ok(remote) => match remote.into_value() {
                Some(value) => match value.as_str() {
                    Some(t) => if exact_match {
                        t == text
                    } else {
                        t.contains(text)
                    },
                    None => false,
                },
                None => false,
            },
            Err(_) => false,
        }
    }
    pub fn emulate_key_input(&self, input_value: Vec<String>) -> BrowserResult<bool> {
        if self.is_input_file()? {
            self.dp.send("DOM.setFileInputFiles", json!({
                "files": input_value,
                "objectId": self.remote.object_id,
            }))?;
            let files = self.get_property("files")?;
            files.into_iter()?
                .map(|file| {
                    let name = file.get_property("name")?.into_string();
                    let matched = input_value.iter().any(|path| path.ends_with(&name));
                    Ok(matched)
                })
                .reduce(|a, b| Ok(a? && b?))
                .unwrap_or(Ok(false))
        } else {
            self.invoke_method("focus", vec![], false)?;
            self.set_property("value", RemoteFuncArg::Value(json!(null)))?;
            let text = input_value.first().map(|s| s.to_string()).unwrap_or_default();
            self.dp.send("Input.insertText", json!({"text": text}))?;
            let value = self.get_property("value")?.into_string();
            let result = value == text;
            Ok(result)
        }
    }
    fn is_input_file(&self) -> BrowserResult<bool> {
        let t= self.get_property("type")?.into_string();
        Ok(t == "file")
    }

}

impl From<RemoteObject> for Value {
    fn from(val: RemoteObject) -> Self {
        serde_json::to_value(val.remote.to_owned()).unwrap_or_default()
    }
}
impl From<RemoteObject> for RemoteFuncArg {
    fn from(val: RemoteObject) -> Self {
        RemoteFuncArg::RemoteObject(val)
    }
}
impl From<RemoteObject> for Object {
    fn from(val: RemoteObject) -> Self {
        val.into_object()
    }
}
impl From<Value> for RemoteFuncArg {
    fn from(val: Value) -> Self {
        RemoteFuncArg::Value(val)
    }
}
impl TryFrom<Object> for RemoteFuncArg {
    type Error = UError;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        let value = value.try_into()?;
        Ok(RemoteFuncArg::Value(value))
    }
}

pub enum RemoteFuncArg {
    Value(Value),
    RemoteObject(RemoteObject),
}

impl RemoteFuncArg {
    pub fn from_object(o: Object) -> BrowserResult<Self> {
        if let Object::RemoteObject(remote) = o {
            Ok(RemoteFuncArg::RemoteObject(remote))
        } else {
            let value = o.try_into()?;
            Ok(RemoteFuncArg::Value(value))
        }
    }
}

trait BrowserArg {
    fn as_string(&self, index: usize) -> BrowserResult<String>;
    fn as_bool(&self, index: usize) -> BrowserResult<bool>;
    fn as_f64(&self, index: usize) -> BrowserResult<f64>;
}
impl BrowserArg for Vec<Object> {
    fn as_string(&self, index: usize) -> BrowserResult<String> {
        match self.get(index) {
            Some(obj) => Ok(obj.to_string()),
            None => Err(UError::new(UErrorKind::BrowserControlError, UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_bool(&self, index: usize) -> BrowserResult<bool> {
        match self.get(index) {
            Some(obj) => Ok(obj.is_truthy()),
            None => Err(UError::new(UErrorKind::BrowserControlError, UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_f64(&self, index: usize) -> BrowserResult<f64> {
        match self.get(index) {
            Some(obj) => match obj.as_f64(true) {
                Some(n) => Ok(n),
                None => Err(UError::new(UErrorKind::BrowserControlError, UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string()))),
            },
            None => Err(UError::new(UErrorKind::BrowserControlError, UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]

struct TargetInfo {
    #[serde(rename="targetId")]
    target_id: String,
    r#type: String,
    title: String,
    url: String,
    attached: bool,
    #[serde(rename="openerId")]
    opener_id: Option<String>,
    #[serde(rename="canAccessOpener")]
    can_access_opener: Option<bool>,
    #[serde(rename="openerFrameId")]
    opener_frame_id: Option<String>,
    #[serde(rename="browserContextId")]
    browser_context_id: Option<Value>,
    subtype: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
struct TargetInfos {
    #[serde(rename="targetInfos")]
    target_infos: Vec<TargetInfo>
}