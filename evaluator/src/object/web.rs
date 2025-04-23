use crate::error::{UError, UErrorKind, UErrorMessage};
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

#[derive(Debug, Clone, PartialEq, Default)]
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
        Self::default()
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
        match &self.body {
            Some(body) => write!(f, "{}", body),
            None => write!(f, ""),
        }
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
            UObject::from_json_str(body)
                .map(Object::UObject)
                .unwrap_or_default()
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
    fn _as_bool(&self, index: usize) -> WebResult<bool>;
    fn as_f64(&self, index: usize) -> WebResult<f64>;
}
impl WebArg for Vec<Object> {
    fn as_string(&self, index: usize) -> WebResult<String> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        Ok(obj.to_string())
    }

    fn _as_bool(&self, index: usize) -> WebResult<bool> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        Ok(obj.is_truthy())
    }

    fn as_f64(&self, index: usize) -> WebResult<f64> {
        let obj = self.get(index).ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::BuiltinArgRequiredAt(index+1)))?;
        obj.as_f64(false)
            .ok_or(UError::new(UErrorKind::WebRequestError, UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string())))
    }
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
        if !headers.is_empty() {
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

/* ParseHTML */
use scraper::{Html, ElementRef, element_ref::Select, Selector, error::SelectorErrorKind};

impl From<SelectorErrorKind<'_>> for UError {
    fn from(e: SelectorErrorKind) -> Self {
        UError::new(UErrorKind::HtmlNodeError, UErrorMessage::Any(e.to_string()))
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct ElementNode {
//     html: String,
//     inner_html: String,
//     text: Vec<String>,
//     element: Element,
// }
// impl From<ElementRef<'_>> for ElementNode {
//     fn from(element_ref: ElementRef) -> Self {
//         let element = element_ref.value().to_owned();

//         // let html = elem.html();
//         // let inner_html = elem.inner_html();
//         // let text = elem.text().map(|t| t.to_string()).collect();
//         // let element = elem.value().to_owned();
//         // Self { html, inner_html, text, element }
//     }
// }
// impl From<ElementNode> for Object {
//     fn from(val: ElementNode) -> Self {
//         Object::HtmlNode(HtmlNode::Element(val))
//     }
// }
// impl ElementNode {
//     fn find(&self, selectors: &str) -> WebResult<Vec<Self>> {
//         let fragment = Html::parse_fragment(&self.inner_html);
//         let selector = Selector::parse(selectors)?;
//         let nodes = fragment.select(&selector)
//             .map(|elem| Self::from(elem))
//             .collect();
//         Ok(nodes)
//     }
// }

/// ノードへのアクセサ
#[derive(Debug, Clone, PartialEq)]
pub enum Accessor {
    Selector(Selector),
    Index(usize),
}
impl Accessor {
    fn as_selector(&self) -> Option<&Selector> {
        match self {
            Accessor::Selector(selector) => Some(selector),
            Accessor::Index(_) => None,
        }
    }
    fn as_index(&self) -> Option<usize> {
        match self {
            Accessor::Selector(_) => None,
            Accessor::Index(index) => Some(*index),
        }
    }
}
enum Accessed<'a, 'b> {
    Select(Box<Select<'a, 'b>>),
    ElementRef(ElementRef<'a>),
}
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlNode {
    html: Html,
    accessors: Vec<Accessor>,
}
impl std::fmt::Display for HtmlNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self.access() {
            Some(a) => match a {
                Accessed::Select(select) => {
                    let names = select.map(|e| e.value().name()).collect::<Vec<_>>();
                    write!(f, "Collection: {names:?}")
                },
                Accessed::ElementRef(element_ref) => {
                    let name = element_ref.value().name();
                    if self.accessors.is_empty() {
                        write!(f, "Root: {name}")
                    } else {
                        write!(f, "Element: {name}")
                    }
                },
            },
            None => write!(f, ""),
        }
    }
}

impl HtmlNode {
    pub fn new(html: &str) -> Self {
        let html = Html::parse_fragment(html);
        Self {
            html,
            accessors: Vec::new()
        }
    }
    fn access(&self) -> Option<Accessed> {
        let root = Accessed::ElementRef(self.html.root_element());
        let accessed = self.accessors.iter()
            .try_fold(root, |accessed, accessor| {
                match accessed {
                    Accessed::Select(mut select) => {
                        select.nth(accessor.as_index()?)
                            .map(Accessed::ElementRef)
                    },
                    Accessed::ElementRef(element_ref) => {
                        let select = element_ref.select(accessor.as_selector()?);
                        Some(Accessed::Select(Box::new(select)))
                    },
                }
            });
        accessed
    }
    fn push_accessor(&mut self, accessor: Accessor) {
        self.accessors.push(accessor);
    }
    pub fn set_index(&mut self, index: usize) {
        self.push_accessor(Accessor::Index(index));
    }
    pub fn into_vec(self) -> Option<Vec<Object>> {
        match self.access()? {
            Accessed::Select(select) => {
                let vec = select.enumerate().map(|(index, _)| {
                    let mut node = self.clone();
                    node.set_index(index);
                    Object::HtmlNode(node)
                })
                .collect();
                Some(vec)
            },
            Accessed::ElementRef(_) => None,
        }
    }

    pub fn get_property(&self, name: &str) -> WebResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "outerhtml" => self.outer_html(),
            "innerhtml" => self.inner_html(),
            "text" => self.texts(),
            "isroot" => Ok(self.accessors.is_empty().into()),
            "iselement" => {
                let obj = match self.accessors.last() {
                    Some(a) => match a {
                        Accessor::Selector(_) => false,
                        Accessor::Index(_) => true,
                    },
                    // 空の場合はルートなのでtrue
                    None => true,
                }.into();
                Ok(obj)
            },
            "iscollection" => {
                let obj = match self.accessors.last() {
                    Some(a) => match a {
                        Accessor::Selector(_) => true,
                        Accessor::Index(_) => false,
                    },
                    None => false,
                }.into();
                Ok(obj)
            },
            // "isempty" => Ok(self.is_empty().into()),
            _ => Err(UError::new(
                UErrorKind::HtmlNodeError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    pub fn invoke_method(self, name: &str, args: Vec<Object>) -> WebResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "find" => {
                let selectors = args.as_string(0)?;
                self.find(&selectors)
            },
            "first" | "findfirst" => {
                let selectors = args.as_string(0)?;
                self.first(&selectors)
            },
            "attr" | "attribute" => {
                let name = args.as_string(0)?;
                self.attr(&name)
            },
            _ => Err(UError::new(
                UErrorKind::HtmlNodeError,
                UErrorMessage::InvalidMember(name.to_string())
            ))
        }
    }
    // fn is_empty(&self) -> bool {
    //     self.0.is_none()
    // }
    fn outer_html(&self) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(select) => {
                    let arr = select.map(|e| e.html().into()).collect();
                    Object::Array(arr)
                },
                Accessed::ElementRef(element_ref) => element_ref.html().into(),
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
    fn inner_html(&self) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(select) => {
                    let arr = select.map(|e| e.inner_html().into()).collect();
                    Object::Array(arr)
                },
                Accessed::ElementRef(element_ref) => element_ref.inner_html().into(),
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
    fn texts(&self) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(select) => {
                    let arr = select
                        .map(|e| {
                            let arr = e.text().map(|t| t.into()).collect();
                            Object::Array(arr)
                        })
                        .collect();
                    Object::Array(arr)
                },
                Accessed::ElementRef(element_ref) => {
                    let arr = element_ref.text().map(|t| t.into()).collect();
                    Object::Array(arr)
                }
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
    fn find(mut self, selectors: &str) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(_) => Object::Empty,
                Accessed::ElementRef(_) => {
                    let selector = Selector::parse(selectors)?;
                    self.push_accessor(Accessor::Selector(selector));
                    Object::HtmlNode(self)
                }
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
    fn first(mut self, selectors: &str) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(_) => Object::Empty,
                Accessed::ElementRef(_) => {
                    let selector = Selector::parse(selectors)?;
                    self.push_accessor(Accessor::Selector(selector));
                    self.push_accessor(Accessor::Index(0));
                    Object::HtmlNode(self)
                }
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
    fn attr(&self, attr: &str) -> WebResult<Object> {
        let obj = match self.access() {
            Some(a) => match a {
                Accessed::Select(s) => {
                    let arr = s.map(|e| e.attr(attr).into()).collect();
                    Object::Array(arr)
                },
                Accessed::ElementRef(e) => {
                    e.attr(attr).into()
                }
            },
            None => Object::Empty,
        };
        Ok(obj)
    }
}