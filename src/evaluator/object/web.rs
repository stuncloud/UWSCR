use crate::error::evaluator::{UError, UErrorKind, UErrorMessage};
use super::{Object, UObject, HashTbl};

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    blocking::{Client, Response, RequestBuilder},
    Method,
};

impl From<InvalidHeaderValue> for UError {
    fn from(e: InvalidHeaderValue) -> Self {
        UError::new(UErrorKind::WebRequestError, UErrorMessage::Any(e.to_string()))
    }
}
impl From<InvalidHeaderName> for UError {
    fn from(e: InvalidHeaderName) -> Self {
        UError::new(UErrorKind::WebRequestError, UErrorMessage::Any(e.to_string()))
    }
}

type WebResult<T> = Result<T, UError>;

#[derive(Debug, Clone, PartialEq)]
pub struct WebRequest {
    // builder: ClientBuilder,
    user_agent: Option<String>,
    headers: HeaderMap,
    timeout: Option<Duration>,
    body: Option<String>,
    basic: Option<(String, Option<String>)>,
    bearer: Option<String>,
}
impl std::fmt::Display for WebRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebRequest")
    }
}
impl WebRequest {
    pub fn new() -> Self {
        Self {
            user_agent: None,
            headers: HeaderMap::new(),
            timeout: None,
            body: None,
            basic: None,
            bearer: None,
        }
    }
    fn add_header(&mut self, key: &str, value: &str) -> WebResult<()> {
        let key = HeaderName::from_str(key)?;
        let val = HeaderValue::from_str(value)?;
        self.headers.insert(key, val);
        Ok(())
    }
    fn set_user_agent(&mut self, ua: String) {
        self.user_agent = Some(ua);
    }
    fn set_timeout(&mut self, secs: f64) {
        let duration = Duration::from_secs_f64(secs);
        self.timeout = Some(duration);
    }
    fn set_body(&mut self, body: String) {
        self.body = Some(body);
    }
    fn set_basic_auth(&mut self, name: String, password: Option<String>) {
        self.basic = Some((name, password));
    }
    fn set_bearer_auth(&mut self, token: String) {
        self.bearer = Some(token);
    }
    fn client(&self) -> WebResult<Client> {
        let builder = Client::builder();
        let builder = if let Some(ua) = &self.user_agent {
            builder.user_agent(ua)
        } else {
            builder
        };
        let client = builder.build()?;
        Ok(client)
    }
    fn request(&self, method: Method, url: &str) -> WebResult<WebResponse> {
        let client = self.client()?;
        let builder = client.request(method, url)
            .set_header(&self.headers)
            .set_timeout(self.timeout)
            .set_body(&self.body)
            .set_basic_auth(&self.basic)
            .set_bearer_auth(&self.bearer);

        let res = builder.send()?;
        Ok(res.into())
    }
    pub fn get(&self, url: &str) -> WebResult<WebResponse> {
        self.request(Method::GET, url)
    }
    pub fn invoke_method(&mut self, name: &str, args: Vec<Object>) -> WebResult<Option<Object>> {
        let obj = match name.to_ascii_lowercase().as_str() {
            "header" => {
                let key = args.as_string(0)?;
                let value = args.as_string(1)?;
                self.add_header(&key, &value)?;
                None
            },
            "useragent" => {
                let ua = args.as_string(0)?;
                self.set_user_agent(ua);
                None
            },
            "timeout" => {
                let secs = args.as_f64(0)?;
                self.set_timeout(secs);
                None
            },
            "body" => {
                let body = args.as_string(0)?;
                self.set_body(body);
                None
            },
            "basic" => {
                let name = args.as_string(0)?;
                let password = args.as_string(1).ok();
                self.set_basic_auth(name, password);
                None
            },
            "bearer" => {
                let token = args.as_string(0)?;
                self.set_bearer_auth(token);
                None
            },
            "get" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::GET, &url)?;
                Some(Object::WebResponse(res))
            },
            "put" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::PUT, &url)?;
                Some(Object::WebResponse(res))
            },
            "post" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::POST, &url)?;
                Some(Object::WebResponse(res))
            },
            "delete" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::DELETE, &url)?;
                Some(Object::WebResponse(res))
            },
            "patch" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::PATCH, &url)?;
                Some(Object::WebResponse(res))
            },
            "head" => {
                let url = args.as_string(0)?;
                let res = self.request(Method::HEAD, &url)?;
                Some(Object::WebResponse(res))
            },
            _ => Err(UError::new(
                UErrorKind::WebRequestError,
                UErrorMessage::InvalidMember(name.to_string())
            ))?
        };
        Ok(obj)
    }
    pub fn get_property(&self, name: &str) -> WebResult<Object> {
        Err(UError::new(
            UErrorKind::WebRequestError,
            UErrorMessage::InvalidMember(name.to_string())
        ))?
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WebResponse {
    // response: Response
    status: StatusCode,
    body: Option<String>,
    header: HashTbl,
}
impl std::fmt::Display for WebResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.status)
    }
}

impl From<Response> for WebResponse {
    fn from(response: Response) -> Self {
        let status = response.status();
        let mut header = HashTbl::new(false, false);
        for (k, v) in response.headers() {
            let value = v.to_str().ok();
            header.insert(k.to_string(), value.into());
        }

        let body = response.text().ok();
        Self { status, body, header }
    }
}

impl WebResponse {
    fn status(&self) -> Object {
        let n = self.status.as_u16();
        n.into()
    }
    fn status_text(&self) -> Object {
        let text = self.status.canonical_reason();
        text.into()
    }
    fn succeed(&self) -> Object {
        let b = self.status.is_success();
        b.into()
    }
    fn header(&self) -> Object {
        let h = self.header.clone();
        Object::HashTbl(Arc::new(Mutex::new(h)))
    }
    fn body(&self) -> Object {
        let body = self.body.as_deref();
        body.into()
    }
    fn json(&self) -> Object {
        if let Some(body) = &self.body {
            if let Ok(value) = serde_json::from_str(body) {
                Object::UObject(UObject::new(value))
            } else {
                Object::Empty
            }
        } else {
            Object::Empty
        }
    }
    pub fn get_property(&self, name: &str) -> WebResult<Object> {
        let obj = match name.to_ascii_lowercase().as_str() {
            "status" => self.status(),
            "statustext" => self.status_text(),
            "succeed" => self.succeed(),
            "header" => self.header(),
            "body" => self.body(),
            "json" => self.json(),
            _ => Err(UError::new(
                UErrorKind::WebRequestError,
                UErrorMessage::InvalidMember(name.to_string())
            ))?
        };
        Ok(obj)
    }
    pub fn invoke_method(&self, name: &str, _args: Vec<Object>) -> WebResult<Object> {
        Err(UError::new(
            UErrorKind::WebRequestError,
            UErrorMessage::InvalidMember(name.to_string())
        ))
    }
}

trait WebArg {
    fn as_string(&self, index: usize) -> WebResult<String>;
    fn as_bool(&self, index: usize) -> WebResult<bool>;
    fn as_f64(&self, index: usize) -> WebResult<f64>;
}
impl WebArg for Vec<Object> {
    fn as_string(&self, index: usize) -> WebResult<String> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        Ok(obj.to_string())
    }

    fn as_bool(&self, index: usize) -> WebResult<bool> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        Ok(obj.is_truthy())
    }

    fn as_f64(&self, index: usize) -> WebResult<f64> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        obj.as_f64(false)
            .ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string())))
    }
}
#[derive(Debug, Clone)]
pub enum WebFunction {
    WebRequest(Arc<Mutex<WebRequest>>, String),
    WebResponse(WebResponse, String)
}

trait RequestBuilderExt {
    fn set_header(self, headers: &HeaderMap<HeaderValue>) -> Self;
    fn set_timeout(self, timeout: Option<Duration>) -> Self;
    fn set_body(self, body: &Option<String>) -> Self;
    fn set_basic_auth(self, basic: &Option<(String, Option<String>)>) -> Self;
    fn set_bearer_auth(self, token: &Option<String>) -> Self;
}

impl RequestBuilderExt for RequestBuilder {
    fn set_header(self, headers: &HeaderMap<HeaderValue>) -> Self {
        if headers.len() > 0 {
            self.headers(headers.clone())
        } else {
            self
        }
    }
    fn set_timeout(self, timeout: Option<Duration>) -> Self {
        if let Some(timeout) = timeout {
            self.timeout(timeout)
        } else {
            self
        }
    }
    fn set_body(self, body: &Option<String>) -> Self {
        if let Some(body) = body {
            self.body(body.to_string())
        } else {
            self
        }
    }
    fn set_basic_auth(self, basic: &Option<(String, Option<String>)>) -> Self {
        if let Some((username, password)) = basic {
            self.basic_auth(username, password.as_deref())
        } else {
            self
        }
    }
    fn set_bearer_auth(self, token: &Option<String>) -> Self {
        if let Some(token) = token {
            self.bearer_auth(token)
        } else {
            self
        }
    }
}