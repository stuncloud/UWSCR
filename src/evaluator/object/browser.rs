use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use crate::settings::USETTINGS;
use super::{Object, UObject};
use crate::evaluator::builtins::window_control::get_id_from_hwnd;
use crate::evaluator::Evaluator;

use std::str::FromStr;
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

use windows::{
    Win32::{
        Foundation::{LPARAM, HWND, BOOL},
        UI::WindowsAndMessaging::{
            GW_OWNER,
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible, GetWindow,
        }
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
}
impl fmt::Display for BrowserBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.r#type, self.port)
    }
}
impl BrowserBuilder {
    pub fn new(r#type: BrowserType, port: u16) -> Self {
        Self { port, r#type, headless: false, private: false, profile: None }
    }
    pub fn port(&mut self, port: u16) {
        self.port = port;
    }
    pub fn headless(&mut self, headless: bool) {
        self.headless = headless;
    }
    pub fn private(&mut self, private: bool) {
        self.private = private;
    }
    pub fn profile(&mut self, profile: Option<String>) {
        self.profile = profile;
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
            "start" => {
                let browser = self.start()?;
                Ok(Some(browser))
            }
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

        let path = self.get_browser_path()?;
        Command::new(&path)
            .args(args)
            .spawn()?;

        if Browser::wait_for_connection(self.port) {
            Browser::get_request(self.port, "/json/version")?;
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
        Ok(Browser::new(self.port, self.r#type))
    }
}

/// Browserオブジェクト
#[derive(Debug, Clone, PartialEq)]
pub struct Browser {
    pub port: u16,
    pub r#type: BrowserType,
}

impl fmt::Display for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.r#type, self.port)
    }
}

impl Browser {
    fn new(port: u16, r#type: BrowserType) -> Self {
        Self { port, r#type }
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
        let response = if put {
            let client = reqwest::blocking::Client::new();
            client.put(uri).send()?
        } else {
            reqwest::blocking::get(uri)?
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
    fn put_request<T: DeserializeOwned>(port: u16, path: &str) -> BrowserResult<T> {
        Self::request_t::<T>(port, path, true)
    }
    fn tabs(&self) -> BrowserResult<BrowserList> {
        let list = Self::get_request_t::<BrowserList>(self.port, "/json/list")?;
        let tabs = list.into_iter()
            .filter(|item| item.r#type == "page")
            .collect();
        Ok(tabs)
    }
    pub fn count(&self) -> BrowserResult<usize> {
        let count = self.tabs()?.len();
        Ok(count)
    }
    pub fn get_tabs(&self) -> BrowserResult<Vec<TabWindow>> {
        let items = self.tabs()?;
        items.into_iter()
            .map(|item| TabWindow::new(self.port, item))
            .collect()
    }
    pub fn get_tab(&self, index: usize) -> BrowserResult<TabWindow> {
        let tabs = self.tabs()?;
        let nth = tabs.into_iter().nth(index);
        if let Some(item) = nth {
            TabWindow::new(self.port, item)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::IndexOutOfBounds(index.into())
            ))
        }
    }
    pub fn close(&self) -> BrowserResult<()> {
        let tabs = self.tabs()?;
        for item in tabs.into_iter().rev() {
            let path = format!("/json/close/{}", item.id);
            Self::get_request(self.port, &path)?;
        }
        Ok(())
    }
    pub fn new_tab(&self, uri: &str) -> BrowserResult<TabWindow> {
        let path = format!("/json/new?{}", uri);
        let item = Self::put_request::<BrowserListItem>(self.port, &path)?;
        if item.r#type == "page" {
            let tab = TabWindow::new(self.port, item)?;
            Ok(tab)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidTabPage(uri.into())
            ))
        }
    }
    pub fn get_window_id(&self) -> BrowserResult<Object> {
        let pid = BrowserProcess::get_pid_from_port(self.port)?;
        let hwnd = BrowserProcess::get_hwnd_from_pid(pid);
        let id = get_id_from_hwnd(hwnd);
        Ok(id.into())
    }
    pub fn get_property(&self, name: &str) -> BrowserResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "count" => {
                let count = self.count()?;
                Ok(count.into())
            },
            "tabs" => {
                let tabs = self.tabs()?
                    .into_iter()
                    .map(|item| {
                        TabWindow::new(self.port, item)
                            .map(|tab| Object::TabWindow(tab))
                    })
                    .collect::<BrowserResult<Vec<Object>>>()?;
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
    fn new(port: u16, item: BrowserListItem) -> BrowserResult<Self> {
        let dp = DevtoolsProtocol::new(&item.web_socket_debugger_url)?;
        let id = item.id;
        Ok(Self { port, id, dp })
    }
    pub fn document(&self) -> BrowserResult<RemoteObject> {
        self.dp.runtime_evaluate("document")
    }
    pub fn close(&self) -> BrowserResult<()> {
        let path = format!("/json/close/{}", self.id);
        Browser::get_request(self.port, &path)?;
        Ok(())
    }
    fn is_navigate_completed(&self) -> bool {
        // エラーは握りつぶしてfalseを返す
        if let Ok(document) = self.document() {
            if let Ok(state) = document.get_property("readyState") {
                match state.get_value() {
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
            _ => Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
}

#[derive(Clone)]
struct DevtoolsProtocol {
    ws: Arc<Mutex<WebSocket>>,
}
impl fmt::Debug for DevtoolsProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DevtoolsProtocol").finish()
    }
}
impl DevtoolsProtocol {
    fn new(uri: &str) -> BrowserResult<Self> {
        let (socket, response) = tungstenite::connect(uri)?;
        let status = response.status();
        if status.as_u16() >= 400 {
            return Err(UError::new(UErrorKind::WebSocketError, UErrorMessage::WebSocketConnectionError(status.to_string())));
        }
        let mut ws = WebSocket {
            socket,
            id: 0,
            // event_handler: HashMap::new(),
        };
        ws.init()?;
        Ok(Self { ws: Arc::new(Mutex::new(ws)) })
    }
    fn send(&self, method: &str, params: Value) -> BrowserResult<Value> {
        let mut ws = self.ws.lock().unwrap();
        let value = ws.send(method, params)?;
        if let Some(error) = value.get("error") {
            let code = error["code"].as_i64().unwrap_or_default() as i32;
            let message = error["message"].as_str().unwrap_or_default().to_string();
            Err(UError::new(UErrorKind::DevtoolsProtocolError, UErrorMessage::DTPError(code, message)))
        } else {
            Ok(value["result"].to_owned())
        }
    }
    fn send_t<T: DeserializeOwned>(&self, method: &str, params: Value) -> BrowserResult<T> {
        let value = self.send(method, params)?;
        let t: T = serde_json::from_value(value)?;
        Ok(t)
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
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        let _ = self.socket.close(None);
    }
}

impl WebSocket {
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
    fn send(&mut self, method: &str, params: Value) -> BrowserResult<Value> {
        let data = self.genereate_ws_data(method, params);
        let msg = data.to_string();
        let message = tungstenite::Message::Text(msg);
        self.socket.write_message(message)?;
        loop {
            let received = self.socket.read_message()?;
            if received.is_text() {
                let msg = received.into_text()?;
                let value = Value::from_str(&msg)?;
                if value["id"] == data["id"] {
                    break Ok(value)
                }
            }
        }
    }
    fn init(&mut self) -> BrowserResult<()> {
        self.send("Page.enable", json!({}))?;
        self.send("Runtime.enable", json!({}))?;
        Ok(())
    }
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
            if processes.len() > 0 {
                let found = processes.iter()
                    .find(|p| p.process_id == tcpcon.owning_process)
                    .is_some();
                if found {
                    Ok(ProcessFound::Found)
                } else {
                    Ok(ProcessFound::UnMatch)
                }
            } else {
                Ok(ProcessFound::UnMatch)
            }
        } else {
            if processes.len() > 0 {
                Ok(ProcessFound::NoPort)
            } else {
                Ok(ProcessFound::None)
            }
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
        let pid = if result.len() > 0 {
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
            EnumWindows(Some(Self::enum_window_proc), LPARAM(lparam));
        }
        data.1
    }

    unsafe extern "system"
    fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut data = &mut *(lparam.0 as *mut LparamData);
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

type BrowserList = Vec<BrowserListItem>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RuntimeResult {
    result: RemoteObject0,
    #[serde(rename="exceptionDetails")]
    exception_details: Option<ExceptionDetails>
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ExceptionDetails {
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
        write!(f, "Error text: {}", self.text)
    }
}
impl Into<UError> for ExceptionDetails {
    fn into(self) -> UError {
        UError::new(
            UErrorKind::BrowserControlError,
            UErrorMessage::BrowserRuntimeException(self.to_string())
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct RemoteObject0 {
    r#type: String,
    subtype: Option<String>,
    #[serde(rename="className")]
    class_name: Option<String>,
    value: Option<Value>,
    #[serde(rename="unserializableValue")]
    unserializable_value: Option<String>,
    description: Option<String>,
    #[serde(rename="webDriverValue")]
    web_driver_value: Option<Value>,
    #[serde(rename="objectId")]
    object_id: Option<String>,
    preview: Option<Value>,
    #[serde(rename="customPreview")]
    custom_preview: Option<Value>
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
            write!(f, "RemoteObject: {id}")
        } else {
            match &self.remote.value {
                Some(value) => {
                    let obj = Object::from(value);
                    write!(f, "{obj}")
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

    pub fn get_property(&self, name: &str) -> BrowserResult<Self> {
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
    pub fn set_property(&self, name: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
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
    pub fn get_property_by_index(&self, name: &str, index: &str) -> BrowserResult<Self> {
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
    pub fn set_property_by_index(&self, name: &str, index: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
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
    pub fn get_by_index(&self, index: &str) -> BrowserResult<Self> {
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
    pub fn set_by_index(&self, index: &str, value: RemoteFuncArg) -> BrowserResult<Self> {
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
    pub fn get_value(&self) -> Option<Value> {
        self.remote.value.clone()
    }
    pub fn invoke_method(&self, name: &str, args: Vec<RemoteFuncArg>, await_promise: bool) -> BrowserResult<RemoteObject> {
        let declaration = format!("function(...args) {{ return this.{name}(...args); }}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, args, true, await_promise)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotObject(self.remote.r#type.clone(), name.into())
            ))
        }
    }
    pub fn invoke_as_function(&self, args: Vec<RemoteFuncArg>, await_promise: bool) -> BrowserResult<RemoteObject> {
        let declaration = format!("function(...args) {{ return this(...args); }}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, args, true, await_promise)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::RemoteObjectIsNotFunction(self.remote.r#type.clone())
            ))
        }
    }
    pub fn to_value(&self) -> Option<Value> {
        serde_json::to_value(self.remote.clone()).ok()
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
    fn to_iter(&self) -> BrowserResult<RemoteObject> {
        let declaration = format!("function() {{ return this.values(); }}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, vec![], false, false)
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
    fn next(&self) -> BrowserResult<RemoteObject> {
        let declaration = format!("function() {{ return this.next(); }}");
        if let Some(id) = &self.remote.object_id {
            self.dp.invoke_function(id, &declaration, vec![], false, false)
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
    pub fn to_object_vec(&self) -> BrowserResult<Vec<Object>> {
        let iter = self.to_iter()?;
        let mut vec = vec![];
        loop {
            let next = iter.next()?;
            let done = next.get_property("done")?;
            if let Some(Value::Bool(b)) = done.remote.value {
                if b {
                    break;
                } else {
                    let value = next.get_property("value")?;
                    vec.push(Object::RemoteObject(value));
                }
            } else {
                break;
            }
        }
        Ok(vec)
    }
    pub fn get_type(&self) -> String {
        let mut t = self.remote.r#type.clone();
        if let Some(sub) = &self.remote.subtype {
            t.push('.');
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
    pub fn as_bool(&self) -> bool {
        if let Some(value) = &self.remote.value {
            let obj = Object::from(value);
            obj.is_truthy()
        } else {
            true
        }
    }
    pub fn as_num(&self) -> Option<f64> {
        if let Some(value) = &self.remote.value {
            value.as_f64()
        } else {
            None
        }
    }
    pub fn as_uobject(&self) -> Option<UObject> {
        if let Some(value) = &self.remote.value {
            match value {
                Value::Array(_) |
                Value::Object(_) => Some(UObject::new(value.clone())),
                _ => None
            }
        } else {
            None
        }
    }
}

impl Into<Value> for RemoteObject {
    fn into(self) -> Value {
        serde_json::to_value(self.remote.to_owned()).unwrap_or_default()
    }
}
impl Into<RemoteFuncArg> for RemoteObject {
    fn into(self) -> RemoteFuncArg {
        RemoteFuncArg::RemoteObject(self)
    }
}
impl Into<Object> for RemoteObject {
    fn into(self) -> Object {
        Object::RemoteObject(self)
    }
}
impl Into<RemoteFuncArg> for Value {
    fn into(self) -> RemoteFuncArg {
        RemoteFuncArg::Value(self)
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
            let value = Evaluator::object_to_serde_value(o)?;
            Ok(RemoteFuncArg::Value(value))
        }
    }
}

#[derive(Debug, Clone)]
pub enum BrowserObject {
    Browser(Browser),
    TabWindow(TabWindow),
    RemoteObject(RemoteObject),
    Builder(Arc<Mutex<BrowserBuilder>>),
}
#[derive(Debug, Clone)]
pub struct BrowserFunction {
    pub object: BrowserObject,
    pub member: String,
}
impl BrowserFunction {
    pub fn from_browser(browser: Browser, member: String) -> Self {
        Self {
            object: BrowserObject::Browser(browser),
            member
        }
    }
    pub fn from_tabwindow(tabwindow: TabWindow, member: String) -> Self {
        Self {
            object: BrowserObject::TabWindow(tabwindow),
            member
        }
    }
    pub fn from_remote_object(remote_object: RemoteObject, member: String) -> Self {
        Self {
            object: BrowserObject::RemoteObject(remote_object),
            member
        }
    }
    pub fn from_builder(builder: Arc<Mutex<BrowserBuilder>>, member: String) -> Self {
        Self {
            object: BrowserObject::Builder(builder),
            member
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
