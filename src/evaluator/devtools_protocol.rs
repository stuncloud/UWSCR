use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};

use std::{
    fmt,
    process::Command,
    // os::windows::process::CommandExt,
    str::FromStr,
    sync::{Arc, Mutex},
    net::TcpStream,
    thread::sleep,
    time::{Duration, Instant},
};

use winreg;
use reqwest;
use serde_json::{Value, json};
use websocket::{
    client::ClientBuilder,
    Message, OwnedMessage,
    sender::Writer,
    receiver::Reader,
};

#[derive(Debug, Clone, Copy)]
pub enum BrowserType {
    Chrome,
    MSEdge,
}

impl fmt::Display for BrowserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowserType::Chrome => write!(f, "chrome.exe"),
            BrowserType::MSEdge => write!(f, "msedge.exe"),
        }
    }
}

pub struct Browser {
    pub port: u16,
    pub btype: BrowserType,
    pub id: String,
    dp: Arc<Mutex<DevtoolsProtocol>>,
}

impl Clone for Browser {
    fn clone(&self) -> Self {
        Self {
            port: self.port.clone(),
            btype: self.btype.clone(),
            id: self.id.clone(),
            dp: Arc::clone(&self.dp)
        }
    }
}
impl fmt::Debug for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Browser")
        .field("port", &self.port)
            .field("btype", &self.btype)
            .field("id", &self.id)
            .finish()
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let dp = self.dp.lock().unwrap();
        drop(dp);
    }
}

pub struct Element {
    dp: Arc<Mutex<DevtoolsProtocol>>,
    pub node_id: u32,
    pub value: Value
}

impl Clone for Element {
    fn clone(&self) -> Self {
        Self {
            dp: Arc::clone(&self.dp),
            node_id: self.node_id.clone(),
            value: self.value.clone(),
        }
    }
}
impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Element")
        .field("node_id", &self.node_id)
        .field("value", &self.value)
        .finish()
    }
}

pub struct DevtoolsProtocol {
    id: u32,
    session_id: String,
    pub receiver: Reader<TcpStream>,
    pub sender: Writer<TcpStream>,
}

impl Drop for DevtoolsProtocol {
    fn drop(&mut self) {
        let _ = self.sender.shutdown();
        let _ = self.receiver.shutdown();
    }
}

type DevtoolsProtocolResult<T> = Result<T, UError>;

#[derive(Debug)]
pub struct BrowserTab {
    pub title: String,
    pub id: String,
    pub url: String
}

impl Browser {
    pub fn new(port: u16, btype: BrowserType, id: String, dp: DevtoolsProtocol) -> Self {
        Self {
            port,
            btype,
            id,
            dp: Arc::new(Mutex::new(dp)),
        }
    }

    pub fn new_chrome(port: u16, filter: Option<String>, headless: bool) -> DevtoolsProtocolResult<Self> {
        // let mut chrome = Self::new(port, BrowserType::Chrome);
        let chrome = Self::connect(port, filter, BrowserType::Chrome, headless)?;
        Ok(chrome)
    }

    pub fn new_msedge(port: u16, filter: Option<String>, headless: bool) -> DevtoolsProtocolResult<Self> {
        // let mut edge = Self::new(port, BrowserType::MSEdge);
        let edge = Self::connect(port, filter, BrowserType::MSEdge, headless)?;
        Ok(edge)
    }

    fn connect(port: u16, filter: Option<String>, btype: BrowserType, headless: bool) -> DevtoolsProtocolResult<Self> {
        if Self::test_connection(port).is_err() {
            let path = Self::get_path(btype)?;
            Self::start(port, &path, btype, headless)?;
        }
        let target = Self::get_target(port, filter)?;
        let dp = match target["webSocketDebuggerUrl"].as_str() {
            Some(ws_uri) => {
                DevtoolsProtocol::new(ws_uri/* , btype */)?
            },
            None => return Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::DTPControlablePageNotFound
            ))
        };
        let id = match target["id"].as_str() {
            Some(id) => {
                id.to_string()
            },
            None => return Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::DTPControlablePageNotFound
            ))
        };
        let browser = Browser {
            port,
            btype,
            id,
            dp: Arc::new(Mutex::new(dp)),
        };
        Ok(browser)
    }

    fn get_path(btype: BrowserType) -> std::io::Result<String> {
        /*
            1. 設定ファイル
            2. レジストリ
            の順にパスを確認し、いずれも得られなかった場合はエラーを返す
        */
        // HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\
        let key = format!(r#"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{}"#, btype);
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        let subkey = hklm.open_subkey(key)?;
        subkey.get_value("")
    }

    fn get_request(port: u16, path: &str) -> DevtoolsProtocolResult<String> {
        let uri = format!("http://localhost:{}{}", port, path);
        let response = reqwest::blocking::get(uri)?;
        if response.status().is_success() {
            Ok(response.text()?)
        } else {
            Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::WebResponseWasNotOk(
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap_or("").to_string()
                )
            ))
        }
    }

    fn test_connection(port: u16) -> DevtoolsProtocolResult<()>{
        Browser::get_request(port, "/json/version")?;
        Ok(())
    }

    fn start(port: u16, path: &str, btype: BrowserType, headless: bool) -> std::io::Result<()> {
        let mut args = match btype {
            BrowserType::Chrome |
            BrowserType::MSEdge => {
                let arg = format!("--remote-debugging-port={}", port);
                vec![arg]
            },
        };
        if headless {
            args.push("--headless".into());
            args.push("--disable-gpu".into());
        }
        Command::new(&path)
                    .args(&args)
                    .spawn()?;
        Ok(())
    }

    fn get_target(port: u16, filter: Option<String>) -> DevtoolsProtocolResult<Value> {
        let res = Browser::get_request(port, "/json/list")?;
        let json = serde_json::from_str::<Value>(&res)?;
        if let Value::Array(arr) = json {
            let mut vec = arr.into_iter().filter(
                |v| v["type"].as_str() == Some("page")
            ).collect::<Vec<Value>>();
            if filter.is_some() {
                let pat = filter.unwrap();
                vec = vec.into_iter().filter(
                    |v| {
                        let title = v["title"].as_str().unwrap_or("");
                        let url = v["url"].as_str().unwrap_or("");
                        title.contains(&pat) || url.contains(&pat)
                    }
                ).collect::<Vec<Value>>();
            }
            let target = if vec.len() > 0 {
                vec[0].to_owned()
            } else {
                Value::Null
            };
            Ok(target)
        } else {
            Ok(Value::Null)
        }
    }

    fn dp_send(&self, method: &str, params: Value) -> DevtoolsProtocolResult<Value> {
        let mut dp = self.dp.lock().unwrap();
        let res = dp.send(method, params)?;
        if let Some(e) = res.get("error") {
            let code = e.get("code").unwrap().as_i64().unwrap() as i32;
            let message = e.get("message").unwrap().as_str().unwrap().to_string();
            return Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::DTPError(code, message)
            ));
        }
        Ok(res["result"].to_owned())
    }

    fn _dp_wait_event(&self, event: &str) -> DevtoolsProtocolResult<()> {
        let mut dp = self.dp.lock().unwrap();
        dp._wait_for_event(event)?;
        Ok(())
    }

    pub fn get_tabs(&self, filter: Option<String>) -> DevtoolsProtocolResult<Vec<BrowserTab>> {
        let res = Browser::get_request(self.port, "/json/list")?;
        let json = serde_json::from_str::<Value>(&res)?;
        if let Value::Array(arr) = json {
            let mut pages = arr.into_iter().filter(
                |v| match v["type"].as_str() {
                    Some("page") |
                    Some("frame") => true,
                    _ => false
                }
            ).collect::<Vec<Value>>();
            if filter.is_some() {
                let pat = filter.unwrap();
                pages = pages.into_iter().filter(
                    |v| {
                        let title = v["title"].as_str().unwrap_or("");
                        let url = v["url"].as_str().unwrap_or("");
                        title.contains(&pat) || url.contains(&pat)
                    }
                ).collect::<Vec<Value>>();
            }
            let tabs = pages.into_iter()
                .map(|p| BrowserTab {
                    title: p["title"].as_str().unwrap().to_string(),
                    id: p["id"].as_str().unwrap().to_string(),
                    url: p["url"].as_str().unwrap().to_string(),
                })
                .collect();
            Ok(tabs)
        } else {
            Ok(vec![])
        }
    }

    pub fn new_tab(&self, uri: &str) -> DevtoolsProtocolResult<Browser> {
        let path = format!("/json/new?{}", uri);
        let res = Self::get_request(self.port, &path)?;
        let v = Value::from_str(&res)?;
        let dp = match v["webSocketDebuggerUrl"].as_str() {
            Some(uri) => DevtoolsProtocol::new(uri)?,
            None => return Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::DTPControlablePageNotFound
            ))
        };
        let id = match v["id"].as_str() {
            Some(id) => id.to_string(),
            None => return Err(UError::new(
                UErrorKind::BrowserControlError,
                UErrorMessage::DTPControlablePageNotFound
            ))
        };
        let browser = Self::new(self.port, self.btype, id, dp);
        Ok(browser)
    }

    pub fn activate(&self) -> DevtoolsProtocolResult<()> {
        let path = format!("/json/activate/{}", &self.id);
        Self::get_request(self.port, &path)?;
        Ok(())
    }

    pub fn document(&self) -> DevtoolsProtocolResult<Element> {
        let value = self.dp_send("DOM.getDocument", json!({"depth": 1}))?;
// println!("[debug] json: {}", serde_json::to_string_pretty(&value)?);
        let element = Element::new(value["root"].to_owned(), Arc::clone(&self.dp))?;
        Ok(element)
    }

    pub fn navigate(&self, uri: &str) -> DevtoolsProtocolResult<Value> {
        // self.dp_send("Page.enable", json!({}))?;
        let res = self.dp_send("Page.navigate", json!({"url": uri}))?;
        while ! self.wait_for_page_load()? {
            sleep(Duration::from_millis(100));
        }
        sleep(Duration::from_millis(100));
        // self.dp_wait_event("Page.loadEventFired")?;
        // self.dp_send("Page.disable", json!({}))?;
        Ok(res)
    }

    fn wait_for_page_load(&self) -> DevtoolsProtocolResult<bool> {
        let completed = match self.execute_script("document.readyState", None, None)? {
            Some(v) => v.as_str().unwrap() == "complete",
            None => false
        };
        Ok(completed)
    }

    pub fn execute_script(&self, script: &str, value: Option<Value>, name: Option<&str>) -> DevtoolsProtocolResult<Option<Value>> {
        let expression = format!(
            r#"(({}) => {})({})"#,
            name.unwrap_or("arg"),
            script,
            value.unwrap_or(Value::Null).to_string(),
        );

        let res = self.dp_send("Runtime.evaluate", json!({
            "expression": expression,
            "returnByValue": true
        }))?;
        let ret_value = match res.get("result") {
            Some(res) => match res.get("value") {
                Some(v) => Some(v.to_owned()),
                None => match res.get("description") {
                    Some(v) => {
                        let err_msg = v.as_str().unwrap().to_string();
                        return Err(UError::new(
                            UErrorKind::DevtoolsProtocolError,
                            UErrorMessage::DTPError(0, err_msg)
                        ));
                    },
                    None => None
                }
            },
            None => None
        };
        Ok(ret_value)
    }

    pub fn reload(&self, ignore_cache: bool) -> DevtoolsProtocolResult<()> {
        self.dp_send("Page.reload", json!({
            "ignoreCache": ignore_cache
        }))?;
        while ! self.wait_for_page_load()? {
            sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn close(&self) -> DevtoolsProtocolResult<()> {
        Self::get_request(self.port, &format!("/json/close/{}", &self.id))?;
        drop(self);
        Ok(())
    }
}

impl Element {
    fn new(value: Value, dp: Arc<Mutex<DevtoolsProtocol>>) -> DevtoolsProtocolResult<Element> {
        let node_id = match &value["nodeId"] {
            Value::Number(n) => n.as_u64().unwrap() as u32,
            _ => if let Value::Number(n) = &value {
                n.as_u64().unwrap() as u32
            } else {
                0
            }
        };
        if node_id == 0 {
            return Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::DTPInvalidElement(value)
            ));
        }
        let elem = Self {
            value,
            dp,
            node_id,
        };
        Ok(elem)
    }

    fn dp_send(&self, method: &str, params: Value) -> DevtoolsProtocolResult<Value> {
        let mut dp = self.dp.lock().unwrap();
        let res = dp.send(method, params)?;
        if let Some(e) = res.get("error") {
            let code = e.get("code").unwrap().as_i64().unwrap() as i32;
            let message = e.get("message").unwrap().as_str().unwrap().to_string();
            return Err(UError::new(
                UErrorKind::DevtoolsProtocolError,
                UErrorMessage::DTPError(code, message)
            ));
        }
        Ok(res["result"].to_owned())
    }

    pub fn url(&self) -> DevtoolsProtocolResult<Option<String>> {
        let uri = match self.value["documentURL"].as_str() {
            Some(s) => Some(s.to_string()),
            None => None
        };
        Ok(uri)
    }

    pub fn query_selector(&self, selector: &str) -> DevtoolsProtocolResult<Element> {
        let v = self.dp_send("DOM.querySelector", json!({
            "nodeId": self.node_id,
            "selector": selector
        }))?;
        let elem = Element::new(v, Arc::clone(&self.dp))?;
        Ok(elem)
    }

    pub fn query_selector_all(&self, selector: &str) -> DevtoolsProtocolResult<Vec<Element>> {
        let v = self.dp_send("DOM.querySelectorAll", json!({
            "nodeId": self.node_id,
            "selector": selector
        }))?;
        let mut elems = vec![];
        if let Value::Array(a) = v.get("nodeIds").unwrap_or(&Value::Null) {
            for v in a {
                let elem = Element::new(v.to_owned(), Arc::clone(&self.dp))?;
                elems.push(elem);
            }
        }
        Ok(elems)
    }

    pub fn wait_for_element(&self, selector: &str, limit: f64) -> DevtoolsProtocolResult<Element> {
        let now = Instant::now();
        loop {
            match self.query_selector(selector) {
                Ok(e) => return Ok(e),
                Err(_) => {sleep(Duration::from_millis(100))}
            }
            if now.elapsed().as_secs_f64() >= limit {
                break;
            }
        }
        Err(UError::new(
            UErrorKind::DevtoolsProtocolError,
            UErrorMessage::DTPElementNotFound(selector.into())
        ))
    }

    pub fn _get_element_from_point(&self, x: i32, y: i32) -> DevtoolsProtocolResult<Element> {
        let v = self.dp_send("DOM.getNodeForLocation", json!({
            "x": x,
            "y": y
        }))?;
        let elem = Element::new(v, Arc::clone(&self.dp))?;
        Ok(elem)
    }

    // $0 が自身になる
    pub fn execute_script(&self, script: &str, value: Option<Value>, name: Option<&str>) -> DevtoolsProtocolResult<Value> {
        let expression = format!(
            r#"(({}) => {})({});"#,
            name.unwrap_or("arg"),
            script,
            value.unwrap_or(Value::Null).to_string(),
        );

        self.dp_send("DOM.setInspectedNode", json!({
            "nodeId": self.node_id
        }))?;
        let res = self.dp_send("Runtime.evaluate", json!({
            "includeCommandLineAPI": true,
            "expression": expression,
            "returnByValue": true
        }))?;
        let ret_value = match res.get("result") {
            Some(res) => match res.get("value") {
                Some(v) => v.to_owned(),
                None => match res.get("description") {
                    Some(v) => {
                        let err_msg = v.as_str().unwrap().to_string();
                        return Err(UError::new(
                            UErrorKind::DevtoolsProtocolError,
                            UErrorMessage::Any(err_msg)
                        ));
                    },
                    None => Value::Null
                }
            },
            None => Value::Null
        };
        Ok(ret_value)
    }

    pub fn get_property(&self, name: &str) -> DevtoolsProtocolResult<Value> {
        let script = format!("$0.{}", name);
        self.execute_script(&script, None, None)
    }

    pub fn set_property(&self, name: &str, value: Value) -> DevtoolsProtocolResult<()> {
        let script = format!("$0.{} = setter", name);
        self.execute_script(&script, Some(value), Some("setter"))?;
        Ok(())
    }

    pub fn focus(&self) -> DevtoolsProtocolResult<()> {
        self.dp_send("DOM.focus", json!({
            "nodeId": self.node_id
        }))?;
        Ok(())
    }

    pub fn input(&self, text: &str) -> DevtoolsProtocolResult<()> {
        self.focus()?;
        for char in text.chars() {
            self.dp_send("Input.dispatchKeyEvent", json!({
                "type": "char",
                "text": char,
                "unmodifiedText": char
            }))?;
        }
        Ok(())
    }

    pub fn clear(&self) -> DevtoolsProtocolResult<()> {
        self.set_property("value", json!(""))?;
        Ok(())
    }

    pub fn set_node_value(&self, text: &str) -> DevtoolsProtocolResult<()> {
        self.dp_send("DOM.setNodeValue", json!({
            "nodeId": self.node_id,
            "value": text
        }))?;
        Ok(())
    }

    pub fn set_file_input(&self, path: Vec<String>) -> DevtoolsProtocolResult<()> {
        self.dp_send("DOM.setFileInputFiles", json!({
            "files": path,
            "nodeId": self.node_id
        }))?;
        Ok(())
    }

    pub fn click(&self) -> DevtoolsProtocolResult<()> {
        self.execute_script("$0.click()", None, None)?;
        Ok(())
    }

    pub fn select(&self) -> DevtoolsProtocolResult<()> {
        self.execute_script("$0.selected = true", None, None)?;
        Ok(())
    }
}

impl DevtoolsProtocol {
    fn new(uri: &str/* , btype: BrowserType */) -> DevtoolsProtocolResult<Self>{
        let client = ClientBuilder::new(uri)?
                .connect_insecure()?;
        let (receiver, sender) = client.split()?;

        let dp = Self {
            receiver,
            sender,
            id: 0,
            // page_id: String::new(),
            session_id: String::new(),
            // btype,
        };
        Ok(dp)
    }

    fn next_id(&mut self) -> u32 {
        let id = self.id;
        self.id += 1;
        id
    }

    fn new_data(&mut self, method: &str, params: Value) -> Value {
        let mut value = json!({
            "id": self.next_id(),
            "method": method,
            "params": params
        });
        if self.session_id.len() > 0 {
            if let Value::Object(ref mut m) = value {
                m.insert("sessionId".into(), json!(self.session_id));
            }
        }
        value
    }

    fn send(&mut self, method: &str, params: Value) -> Result<Value, UError> {
        let data = self.new_data(method, params);
        let msg = data.to_string();
        // println!("[DevtoolsProtocol::send] sent message: {}", &msg);
        self.sender.send_message(&Message::text(msg))?;
        self.get_result(&data["id"])
    }

    fn search_receiver(&mut self, key: &str, value: &Value) -> Option<String> {
        let mut took = self.receiver.incoming_messages().take_while(|r| {
            match r {
                Ok(m) => if let OwnedMessage::Text(ref s) = m {
                    match Value::from_str(s) {
                        Ok(ref obj) => &obj[key] == value,
                        _ => false
                    }
                } else {
                    false
                },
                _ => false
            }
        }).map(|m| if let Ok(OwnedMessage::Text(t)) = m {
            t.to_string()
        } else {
            String::new()
        });
        let result = took.next();
        // println!("[DevtoolsProtocol::search_receiver] received: {}", &result.as_ref().unwrap_or(&"not yet received".to_string()));
        result
    }

    fn get_result(&mut self, id: &Value) -> DevtoolsProtocolResult<Value> {
        loop {
            let result = self.search_receiver("id", id);
            if result.is_some() {
                return Ok(Value::from_str(&result.unwrap())?);
            }
            sleep(Duration::from_millis(100));
        }
    }

    fn _wait_for_event(&mut self, event: &str) -> DevtoolsProtocolResult<()> {
        let value = json!({
            "method": event
        });
        while ! self.search_receiver("method", &value).is_some() {
            sleep(Duration::from_millis(100))
        }
        Ok(())
    }

}

impl From<websocket::client::ParseError> for UError {
    fn from(e: websocket::client::ParseError) -> Self {
        Self::new(
            UErrorKind::WebSocketError,
            UErrorMessage::Any(e.to_string())
        )
    }
}
impl From<websocket::result::WebSocketError> for UError {
    fn from(e: websocket::result::WebSocketError) -> Self {
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
impl From<std::io::Error> for UError {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            UErrorKind::FileIOError,
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