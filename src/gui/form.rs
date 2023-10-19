use crate::winapi::show_message;
use crate::write_locale;
use crate::error::{
    Locale, CURRENT_LOCALE,
    UWSCRErrorTitle,
    evaluator::{UError, UErrorKind, UErrorMessage},
};
use crate::evaluator::{
    Evaluator,
    object::{
        Object, UObject, Function,
        browser::{RuntimeResult, RemoteObject0, ExceptionDetails},
    },
    builtins::{
        window_control::U32Ext,
        dialog::FormOptions,
        BuiltinFuncError,
    },
};
use crate::ast::Expression;

use std::sync::{mpsc, OnceLock, Arc, Mutex};
use std::fmt;

use windows::{
    core::{self, w, PCWSTR, PWSTR, HSTRING},
    Win32::{
        Foundation::{
            HWND, LPARAM, WPARAM, LRESULT, RECT, SIZE,
            E_POINTER, E_FAIL,
        },
        UI::{
            WindowsAndMessaging as wm,
            Input::KeyboardAndMouse as km,
        },
        Graphics::{Gdi, Dwm},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::GetCurrentProcessId,
            WinRT::EventRegistrationToken
        }
    }
};
use webview2_com::{
    self as wv2,
    Microsoft::Web::WebView2::Win32 as wv2w32,
};
use serde_json::{Value, json};
use serde::{Deserialize, de::DeserializeOwned};

pub enum WebViewError {
    /// WebView2 Runtimeがない
    EnvironmentNotFound,
    Win32(core::Error),
    WebView2Com(wv2::Error),
    JsonError(String),
    UError(UErrorMessage),
    RecvError(String),
    SendError(String),
    HtmlFileError(std::io::Error),
    JavaScriptError(ExceptionDetails),
    NotRemoteObject
}
impl From<core::Error> for WebViewError {
    fn from(err: core::Error) -> Self {
        Self::Win32(err)
    }
}
impl From<wv2::Error> for WebViewError {
    fn from(err: wv2::Error) -> Self {
        Self::WebView2Com(err)
    }
}
impl From<serde_json::Error> for WebViewError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}
impl From<mpsc::RecvError> for WebViewError {
    fn from(err: mpsc::RecvError) -> Self {
        Self::RecvError(err.to_string())
    }
}
impl<T> From<mpsc::SendError<T>> for WebViewError {
    fn from(err: mpsc::SendError<T>) -> Self {
        Self::SendError(err.to_string())
    }
}
impl From<std::io::Error> for WebViewError {
    fn from(err: std::io::Error) -> Self {
        Self::HtmlFileError(err)
    }
}
impl fmt::Display for WebViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebViewError::EnvironmentNotFound => write_locale!(f,
                "Microsoft Edge WebView2 Runtimeがインストールされていません",
                "Microsoft Edge WebView2 Runtime is not installed"
            ),
            WebViewError::Win32(err) => write!(f, "Win32 Error: {err}"),
            WebViewError::JsonError(err) => write!(f, "Json Error: {err}"),
            WebViewError::UError(msg) => write!(f, "{msg}"),
            WebViewError::WebView2Com(err) => write!(f, "webview2-com Error: {err}"),
            WebViewError::RecvError(err) => write!(f, "Receiver Error: {err}"),
            WebViewError::SendError(err) => write!(f, "Sender Error: {err}"),
            WebViewError::HtmlFileError(err) => write!(f, "{err}"),
            WebViewError::JavaScriptError(err) => write!(f, "{err}"),
            WebViewError::NotRemoteObject => write_locale!(f,
                "RemoteObjectではありません",
                "Object is not a RemoteObject"
            ),
        }
    }
}
impl From<WebViewError> for UError {
    fn from(err: WebViewError) -> Self {
        match err {
            WebViewError::UError(message) => Self::new(UErrorKind::FormError, message),
            err => UError::new(UErrorKind::FormError, UErrorMessage::FormError(err.to_string())),
        }
    }
}
impl From<WebViewError> for BuiltinFuncError {
    fn from(err: WebViewError) -> Self {
        Self::UError(err.into())
    }
}

type WebViewResult<T> = std::result::Result<T, WebViewError>;
static REGISTER_CLASS: OnceLock<core::Result<()>> = OnceLock::new();
const FORM_CLASS_NAME: PCWSTR = w!("UWSCR.Form");

#[derive(Clone)]
pub struct WebViewForm {
    hwnd: HWND,
    webview: WebView,
    no_hide: bool,
    no_submit: bool,
    size_fixed: Arc<Mutex<bool>>,
}
impl fmt::Display for WebViewForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebViewForm({})", self.hwnd.0)
    }
}
impl fmt::Debug for WebViewForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebViewForm")
            .field("hwnd", &self.hwnd)
            .field("no_hide", &self.no_hide)
            .field("no_submit", &self.no_submit)
            .finish()
    }
}
impl PartialEq for WebViewForm {
    fn eq(&self, other: &Self) -> bool {
        self.hwnd == other.hwnd && self.no_hide == other.no_hide && self.no_submit == other.no_submit
    }
}
impl WebViewForm {
    pub fn new(title: &str, size: FormSize, opt: u32) -> WebViewResult<Self> {
        let no_hide = opt.includes(FormOptions::FOM_NOHIDE);
        let no_submit = opt.includes(FormOptions::FOM_NOSUBMIT);
        let is_visible = ! opt.includes(FormOptions::FOM_FORMHIDE);
        let hwnd = unsafe {
            Self::create(title, size, opt)?
        };
        let webview = WebView::new(hwnd)?;
        let form = Self { hwnd, webview, no_hide, no_submit, size_fixed: Arc::new(Mutex::new(false)) };
        if is_visible {
            form.fix_window_rect();
        }
        Ok(form)
    }
    pub fn run(&self, path: &str) -> WebViewResult<()> {
        let buf = std::path::PathBuf::from(path).canonicalize()?;
        let uri = match url::Url::from_file_path(&buf) {
            Ok(uri) => HSTRING::from(uri.as_str()),
            Err(_) => HSTRING::from(path),
        };

        self.webview.navigate(&uri)?;
        self.webview.set_submit_event(self.no_submit)?;

        Ok(())
    }
    unsafe fn create(title: &str, size: FormSize, opt: u32) -> WebViewResult<HWND> {
        let hinstance = GetModuleHandleW(None)?;
        REGISTER_CLASS.get_or_init(|| {
            let class = wm::WNDCLASSEXW {
                cbSize: std::mem::size_of::<wm::WNDCLASSEXW>() as u32,
                style: wm::CS_HREDRAW|wm::CS_VREDRAW,
                lpfnWndProc: Some(Self::wndproc),
                // cbClsExtra: todo!(),
                // cbWndExtra: todo!(),
                hInstance: hinstance.into(),
                hIcon: wm::LoadIconW(hinstance, PCWSTR(1 as _))?,
                // hCursor: wm::LoadCursorW(hinstance, wm::IDC_ARROW)?,
                // hbrBackground: todo!(),
                // lpszMenuName: todo!(),
                lpszClassName: FORM_CLASS_NAME,
                // hIconSm: wm::LoadIconW(hinstance, PCWSTR(1 as _))?,
                ..Default::default()
            };
            wm::RegisterClassExW(&class);
            Ok(())
        }).clone()?;

        let title = HSTRING::from(title);
        let dwstyle = Self::new_style(opt);
        let dwexstyle = Self::new_ex_style(opt);

        let hwnd = wm::CreateWindowExW(
            dwexstyle,
            FORM_CLASS_NAME,
            &title,
            dwstyle,
            size.x, size.y, size.w, size.h,
            None,
            None,
            hinstance,
            None
        );
        Ok(hwnd)
    }
    fn new_style(opt: u32) -> wm::WINDOW_STYLE {
        let mut style = wm::WS_OVERLAPPED|wm::WS_CAPTION;
        if ! opt.includes(FormOptions::FOM_NOICON) {
            style |= wm::WS_SYSMENU;
        }
        if opt.includes(FormOptions::FOM_MINIMIZE) {
            style |= wm::WS_MINIMIZEBOX;
        }
        if opt.includes(FormOptions::FOM_MAXIMIZE) {
            style |= wm::WS_MAXIMIZEBOX;
        }
        if opt.includes(FormOptions::FOM_MAXIMIZE) {
            style |= wm::WS_MAXIMIZEBOX;
        }
        if ! opt.includes(FormOptions::FOM_NORESIZE) {
            style |= wm::WS_SIZEBOX;
        }
        if ! opt.includes(FormOptions::FOM_FORMHIDE) {
            style |= wm::WS_VISIBLE;
        }
        style
    }
    fn new_ex_style(opt: u32) -> wm::WINDOW_EX_STYLE {
        let mut style = wm::WINDOW_EX_STYLE::default();
        if opt.includes(FormOptions::FOM_TOPMOST) {
            style |= wm::WS_EX_TOPMOST;
        }
        if opt.includes(FormOptions::FOM_NOTASKBAR) {
            style |= wm::WS_EX_TOOLWINDOW;
        }
        style
    }
    fn fix_window_rect(&self) {
        let hwnd = self.hwnd;
        let mut fixed = self.size_fixed.lock().unwrap();
        if ! *fixed {
            unsafe {
                // AEROが有効であれば座標やサイズを補正する
                if Dwm::DwmIsCompositionEnabled().unwrap_or_default().as_bool() {
                    // 見た目のRECT
                    let mut drect = RECT::default();
                    let pvattribute = &mut drect as *mut RECT as *mut std::ffi::c_void;
                    let cbattribute = std::mem::size_of::<RECT>() as u32;
                    if Dwm::DwmGetWindowAttribute(hwnd, Dwm::DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_ok() {
                        let mut wrect = RECT::default();
                        let _ = wm::GetWindowRect(hwnd, &mut wrect);
                        let x = wrect.left - (drect.left - wrect.left);
                        let y = wrect.top - (drect.top - wrect.top);
                        let dw = drect.right - drect.left;
                        let ww = wrect.right - wrect.left;
                        let nwidth = ww - (dw - ww);
                        let dh = drect.bottom - drect.top;
                        let wh = wrect.bottom - wrect.top;
                        let nheight = wh - (dh - wh);
                        let _ = wm::MoveWindow(hwnd, x, y, nwidth, nheight, true);
                    }
                }
            }
            *fixed = true;
        }
    }
    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let webview = match WebView::get_window_webview(hwnd) {
            Some(b) => b,
            None => {
                return wm::DefWindowProcW(hwnd, msg, wparam, lparam);
            },
        };
        match msg {
            wm::WM_SIZE => {
                if let Ok(size) = Self::get_size(hwnd) {
                    let _ = webview.controller.0.SetBounds(size.into_rect());
                }
                LRESULT(0)
            },
            wm::WM_CLOSE => {
                let _ = wm::DestroyWindow(hwnd);
                LRESULT(0)
            },
            wm::WM_DESTROY => {
                // dispose webview
                WebView::remove_window_webview(hwnd);
                wm::PostQuitMessage(0);
                LRESULT(0)
            },
            msg => wm::DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
    pub fn message_loop(&self) -> WebViewResult<Value> {
        unsafe {
            let mut msg = wm::MSG::default();
            let hwnd = HWND::default();
            loop {
                while let Ok(f) = self.webview.rx.try_recv() {
                    match (f)(self.webview.clone()) {
                        WebViewValue::None => {},
                        WebViewValue::Submit(value) => {
                            if self.no_hide {
                                self.webview.core.remove_WebMessageReceived(self.webview.webmsg_token)?;
                                WebView::remove_window_webview(hwnd);
                            } else {
                                let _ = wm::DestroyWindow(self.hwnd);
                            }
                            return Ok(value);
                        }
                    }
                }

                match wm::GetMessageW(&mut msg, hwnd, 0, 0).0 {
                    -1 => break Err(core::Error::from_win32().into()),
                    0 => {
                        // show/hide対策
                        // WebView2の表示状態が変更される？となぜかWM_QUITが来るがその場合lParamに値が入るっぽい
                        // なので0じゃなければループを抜けないようにする
                        if msg.lParam.0 == 0 {
                            break Ok(json!({
                                "submit": null,
                                "data": []
                            }));
                        }
                    },
                    _ => {}
                }
                wm::TranslateMessage(&msg);
                wm::DispatchMessageW(&msg);
            }
        }
    }
    fn show(&self) {
        unsafe {
            wm::ShowWindow(self.hwnd, wm::SW_SHOW);
            self.fix_window_rect();
            Gdi::UpdateWindow(self.hwnd);
            km::SetFocus(self.hwnd);
        }
    }
    fn hide(&self) {
        unsafe {
            wm::ShowWindow(self.hwnd, wm::SW_HIDE);
        }
    }
    fn close(&self) {
        unsafe {
            let _ = wm::DestroyWindow(self.hwnd);
        }
    }
    /// Formオブジェクトのメソッドを実行する
    pub fn invoke_method(&self, name: &str, args: Vec<Object>, evaluator: &Evaluator) -> WebViewResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "wait" => {
                let value = self.message_loop()?;
                Ok(Object::UObject(UObject::new(value)))
            },
            "setvisible" => {
                let visible = args.as_bool(0)?;
                if visible {
                    self.show();
                } else {
                    self.hide();
                }
                Ok(Object::Empty)
            },
            "close" => {
                self.close();
                Ok(Object::Empty)
            },
            "seteventhandler" => {
                let remote = args.as_remote(0)?;
                let event = args.as_string(1)?;
                let func = args.as_func(2)?;
                self.webview.set_event_handler(remote, &event, evaluator, func)?;
                Ok(Object::Empty)
            },
            _ => Err(WebViewError::UError(UErrorMessage::InvalidMember(name.to_string())))
        }
    }
    /// Formオブジェクトのプロパティを得る
    pub fn get_property(&self, name: &str) -> WebViewResult<Object> {
        match name.to_ascii_lowercase().as_str() {
            "document" => {
                let remote = self.webview.invoke_runtime_evaluate("document")?;
                Ok(Object::WebViewRemoteObject(remote))
            },
            "hwnd" => {
                Ok(Object::Num(self.hwnd.0 as f64))
            }
            _ => Err(WebViewError::UError(UErrorMessage::InvalidMember(name.to_string())))
        }
    }
    fn get_size(hwnd: HWND) -> WebViewResult<SIZE> {
        let rect = Self::get_client_rect(hwnd)?;
        let size = SIZE {
            cx: rect.right - rect.left,
            cy: rect.bottom - rect.top,
        };
        Ok(size)
    }
    fn get_client_rect(hwnd: HWND) -> WebViewResult<RECT> {
        unsafe {
            let mut rect = RECT::default();
            wm::GetClientRect(hwnd, &mut rect)?;
            Ok(rect)
        }
    }
}
pub struct FormSize {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl FormSize {
    pub fn new(x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) -> Self {
        Self {
            x: x.unwrap_or(wm::CW_USEDEFAULT),
            y: y.unwrap_or(wm::CW_USEDEFAULT),
            w: w.unwrap_or(wm::CW_USEDEFAULT),
            h: h.unwrap_or(wm::CW_USEDEFAULT),
        }
    }
}


#[derive(Deserialize)]
struct DomEvent {
    index: usize,
    params: Vec<Value>,
}
#[derive(Deserialize)]
struct SubmitEvent {
    submit: String,
    data: Vec<Value>,
}
impl Into<Value> for SubmitEvent {
    fn into(self) -> Value {
        json!({
            "submit": self.submit,
            "data": self.data
        })
    }
}
enum EventMessage {
    Dom(DomEvent),
    Submit(SubmitEvent),
}

struct WebViewController(wv2w32::ICoreWebView2Controller);
impl Drop for WebViewController {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.Close();
        }
    }
}
type WebViewSender = mpsc::Sender<Box<dyn FnOnce(WebView) -> WebViewValue + Send>>;
type WebViewReceiver = mpsc::Receiver<Box<dyn FnOnce(WebView) -> WebViewValue + Send>>;

#[derive(Clone)]
pub struct WebView {
    controller: Arc<WebViewController>,
    core: Arc<wv2w32::ICoreWebView2>,
    tx: WebViewSender,
    rx: Arc<WebViewReceiver>,
    thread_id: u32,
    dom_event_handler: Arc<Mutex<Option<DomEventHandler>>>,
    parent: HWND,
    webmsg_token: EventRegistrationToken,
}
impl Drop for WebView {
    fn drop(&mut self) {
        // FOM_NOHIDE時の"Failed to unregister class Chrome_WidgetWin_0"対策
        // drop時に強参照が既定値以下ならDestroyWindowする
        // 現時点では2以下とする
        let strong = Arc::strong_count(&self.controller);
        if strong <= 2 {
            unsafe {
                let _ = wm::DestroyWindow(self.parent);
            }
        }
    }
}

impl WebView {
    fn new(parent: HWND) -> WebViewResult<Self> {
        unsafe {
            let environment = {
                let (tx, rx) = mpsc::channel();
                wv2::CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
                    Box::new(|handler| {
                        wv2w32::CreateCoreWebView2Environment(&handler)
                            .map_err(wv2::Error::WindowsError)
                    }),
                    Box::new(move |error, environment| {
                        error?;
                        match environment {
                            Some(environment) => {
                                tx.send(environment).map_err(|_| core::Error::from(E_FAIL))
                            },
                            None => Err(core::Error::from(E_POINTER)),
                        }
                    })
                ).map_err(|_| WebViewError::EnvironmentNotFound)?;
                rx.recv()?
            };
            let controller = {
                let (tx, rx) = mpsc::channel();
                wv2::CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
                    Box::new(move |handler| {
                        environment.CreateCoreWebView2Controller(parent, &handler)
                            .map_err(wv2::Error::WindowsError)
                    }),
                    Box::new(move |error, controller| {
                        error?;
                        match controller {
                            Some(controller) => {
                                tx.send(controller).map_err(|_| core::Error::from(E_FAIL))
                            },
                            None => Err(core::Error::from(E_POINTER)),
                        }
                    })
                )?;
                rx.recv()?
            };

            let size = WebViewForm::get_size(parent)?;
            controller.SetBounds(size.into_rect())?;
            controller.SetIsVisible(true)?;

            let core = controller.CoreWebView2()?;

            // 初設定
            let settings = core.Settings()?;
            let aredevtoolsenabled = cfg!(debug_assertions);
            settings.SetAreDevToolsEnabled(aredevtoolsenabled)?; // DevTools

            let (tx, rx) = mpsc::channel();
            let thread_id = GetCurrentProcessId();

            let mut webview = WebView {
                controller: Arc::new(WebViewController(controller)),
                core: Arc::new(core),
                tx,
                rx: Arc::new(rx),
                thread_id,
                dom_event_handler: Arc::new(Mutex::new(None)),
                parent,
                webmsg_token: EventRegistrationToken::default(),
            };

            // let mut _token = EventRegistrationToken::default();
            let webview2: WebView = webview.clone();
            let handler = wv2::WebMessageReceivedEventHandler::create(Box::new(
                move |_, args| {
                    if let Some(args) = args {
                        let mut buf = PWSTR(std::ptr::null_mut());
                        if args.WebMessageAsJson(&mut buf).is_ok() {
                            let json = buf.to_string().unwrap_or_default();
                            if let Ok(submit) = serde_json::from_str::<SubmitEvent>(&json) {
                                webview2.invoke_event_handler(EventMessage::Submit(submit))?;
                            } else if let Ok(dom) = serde_json::from_str::<DomEvent>(&json) {
                                webview2.invoke_event_handler(EventMessage::Dom(dom))?;
                            }
                        }
                    }
                    Ok(())
                }
            ));
            webview.core.add_WebMessageReceived(&handler, &mut webview.webmsg_token)?;

            Self::set_window_webview(parent, Some(Box::new(webview.clone())));
            Ok(webview)
        }
    }
    fn navigate(&self, uri: &HSTRING) -> WebViewResult<()> {
        unsafe {
            let (tx, rx) = mpsc::channel();
            let handler = wv2::NavigationCompletedEventHandler::create(Box::new(move |_,_| {
                tx.send(()).map_err(|_| core::Error::from(E_FAIL))
            }));
            let mut token = EventRegistrationToken::default();
            self.core.add_NavigationCompleted(&handler, &mut token)?;
            self.core.Navigate(uri)?;
            let result = wv2::wait_with_pump(rx);
            self.core.remove_NavigationCompleted(token)?;
            result.map_err(|e| e.into())
        }
    }

    fn eval(&self, js: &str) -> WebViewResult<String> {
        let core = self.core.clone();
        let javascript = HSTRING::from(js);
        let (tx, rx) = mpsc::channel();
        wv2::ExecuteScriptCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                core.ExecuteScript(&javascript, &handler)
                    .map_err(webview2_com::Error::WindowsError)
            }),
            Box::new(move |error, result| {
                error?;
                tx.send(result)
                    .map_err(|_| core::Error::from(E_FAIL))
            }),
        )?;
        rx.recv().map_err(|e| e.into())
    }
    fn set_submit_event(&self, no_submit: bool) -> WebViewResult<()> {
        let js = format!(r#"
        document.querySelectorAll('form').forEach(f => {{
            f.addEventListener('submit', event => {{
                let submit = event.submitter.name;
                let data = new FormData(event.srcElement);
                let msg = {{
                    "submit": submit,
                    "data": [...data.entries()].map(kv => {{return {{"name": kv[0], "value": kv[1]}};}})
                }};
                window.chrome.webview.postMessage(msg);
                {}
            }});
        }})"#, if no_submit {"event.preventDefault();"} else {""});
        self.eval(&js)?;
        Ok(())
    }
    fn set_event_handler(&self, remote: WebViewRemoteObject, event: &str, evaluator: &Evaluator, func: Function) -> WebViewResult<()> {
        let mut guard = self.dom_event_handler.lock().unwrap();
        if guard.is_none() {
            *guard = Some(DomEventHandler::new(evaluator.clone()));
        }
        if let Some(handlers) = guard.as_mut() {
            let index = handlers.len();
            let declaration = format!(r#"
            function(target) {{
                target.addEventListener('{event}', (event) => {{
                    let msg = {{
                        "index": {index},
                        "params": [
                            event.target.value,
                            event.target.name
                        ]
                    }};
                    window.chrome.webview.postMessage(msg);
                }})
            }}"#);
            let args = vec![Object::WebViewRemoteObject(remote.clone())];
            remote.invoke_runtime_function(&declaration, args, true, false)
                .and_then(|_| {
                    handlers.push(func);
                    Ok(())
                })?;
        }
        Ok(())
    }
    fn invoke_event_handler(&self, msg: EventMessage) -> core::Result<()> {
        match msg {
            EventMessage::Dom(dom) => {
                let mut guard = self.dom_event_handler.lock().unwrap();
                if let Some(handler) = guard.as_mut() {
                    if let Some((evaluator, f)) = handler.get(dom.index) {
                        let mut arguments = dom.params.into_iter()
                            .map(|v| {
                                match serde_json::from_value::<RemoteObject0>(v.clone()) {
                                    Ok(remote0) => {
                                        let remote = WebViewRemoteObject {
                                            webview: self.clone(),
                                            remote: remote0,
                                        };
                                        (Some(Expression::EmptyArgument), Object::WebViewRemoteObject(remote))
                                    },
                                    Err(_) => (Some(Expression::EmptyArgument), v.into()),
                                }
                            })
                            .collect::<Vec<_>>();
                        arguments.resize(f.params.len(), (Some(Expression::EmptyArgument), Object::Empty));
                        if let Err(err) = f.invoke(evaluator, arguments) {
                            use crate::logging::{out_log, LogType};
                            unsafe {
                                let _ = wm::DestroyWindow(self.parent);
                                evaluator.clear();
                                let msg = err.to_string();
                                out_log(&msg, LogType::Error);
                                let title = UWSCRErrorTitle::RuntimeError.to_string();
                                show_message(&msg, &title, true);
                                std::process::exit(0);
                            }
                        }
                    }
                }
            },
            EventMessage::Submit(submit) => {
                self.dispatch(move |_| {
                    let value = submit.into();
                    WebViewValue::Submit(value)
                }).map_err(|_| core::Error::from(E_FAIL))?;
            },
        }
        Ok(())
    }
    fn invoke_devtools_protocol_method<T: DeserializeOwned>(&self, method: &str, param: Value) -> WebViewResult<T> {
        let core = self.core.clone();
        let methodname = HSTRING::from(method);
        let parametersasjson = HSTRING::from(param.to_string());
        let (tx, rx) = mpsc::channel();
        wv2::CallDevToolsProtocolMethodCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                core.CallDevToolsProtocolMethod(&methodname, &parametersasjson, &handler)
                    .map_err(wv2::Error::WindowsError)
            }),
            Box::new(move |error, result| {
                error?;
                tx.send(result).map_err(|_| core::Error::from(E_FAIL))
            })
        )?;
        let result = rx.recv()?;
        let result = serde_json::from_str::<T>(&result)?;
        Ok(result)
    }
    fn invoke_runtime_evaluate(&self, expression: &str) -> WebViewResult<WebViewRemoteObject> {
        let result = self.invoke_devtools_protocol_method::<RuntimeResult>("Runtime.evaluate", json!({
            "expression": expression
        }))?;
        result.into_result(self.clone())
    }
    fn invoke_runtime_call_function_on(&self, object_id: &str, declaration: &str, args: Vec<Object>, user_gesture: bool, await_promise: bool) -> WebViewResult<WebViewRemoteObject> {
        let args = WebViewRemoteObject::convert_args(args)?;
        let result = self.invoke_devtools_protocol_method::<RuntimeResult>("Runtime.callFunctionOn", json!({
            "functionDeclaration": declaration,
            "objectId": object_id,
            "arguments": Value::Array(args),
            "userGesture": user_gesture,
            "awaitPromise": await_promise,
        }))?;
        result.into_result(self.clone())
    }

    fn set_window_webview(hwnd: HWND, webview: Option<Box<Self>>) {
        unsafe {
            let dwnewlong = match webview {
                Some(b) => Box::into_raw(b) as isize,
                None => 0isize,
            };
            set_window_long(hwnd, wm::GWLP_USERDATA, dwnewlong);
        }
    }
    fn get_window_webview(hwnd: HWND) -> Option<Box<Self>> {
        unsafe {
            match get_window_long(hwnd, wm::GWLP_USERDATA) {
                0 => None,
                n => {
                    let ptr = n as *mut Self;
                    let raw = Box::from_raw(ptr);
                    let webview = raw.clone();
                    std::mem::forget(raw);
                    Some(webview)
                }
            }
        }
    }
    fn remove_window_webview(hwnd: HWND) {
        unsafe {
            match get_window_long(hwnd, wm::GWLP_USERDATA) {
                0 => {},
                n => {
                    let ptr = n as *mut Self;
                    let raw = Box::from_raw(ptr);
                    drop(raw);
                    Self::set_window_webview(hwnd, None);
                }
            }
        }
    }
    fn dispatch<F>(&self, f: F) -> WebViewResult<()>
        where F: FnOnce(Self) -> WebViewValue + Send + 'static
    {
        self.tx.send(Box::new(f))?;
        unsafe {
            wm::PostThreadMessageW(self.thread_id, wm::WM_APP, WPARAM::default(), LPARAM::default())?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct WebViewRemoteObject {
    webview: WebView,
    remote: RemoteObject0,
}
impl std::fmt::Display for WebViewRemoteObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.remote.object_id {
            write!(f, "WebViewRemoteObject({id})")
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
impl std::fmt::Debug for WebViewRemoteObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebViewRemoteObject").field("remote", &self.remote).finish()
    }
}
impl PartialEq for WebViewRemoteObject {
    fn eq(&self, other: &Self) -> bool {
        self.remote == other.remote
    }
}
impl WebViewRemoteObject {
    pub fn new(webview: WebView, remote: RemoteObject0) -> Self {
        Self { webview, remote }
    }
    fn convert_arg(obj: Object) -> WebViewResult<Value> {
        match obj {
            Object::WebViewRemoteObject(remote) => {
                match remote.remote.object_id {
                    Some(id) => Ok(json!({"objectId": id})),
                    None => Ok(json!({"value": remote.remote.value})),
                }
            },
            o => {
                let value = Evaluator::object_to_serde_value(o)
                    .map_err(|err| WebViewError::UError(err.message))?;
                Ok(json!({"value": value}))
            }
        }
    }
    fn convert_args(args: Vec<Object>) -> WebViewResult<Vec<Value>> {
        args.into_iter()
            .map(|obj| Self::convert_arg(obj))
            .collect()
    }
    fn invoke_runtime_function(&self, declaration: &str, args: Vec<Object>, user_gesture: bool, await_promise: bool) -> WebViewResult<Object> {
        match &self.remote.object_id {
            Some(object_id) => {
                self.webview.invoke_runtime_call_function_on(object_id, &declaration, args, user_gesture, await_promise)
                    .map(|r| r.to_object())
            },
            None => Err(WebViewError::NotRemoteObject),
        }
    }
    pub fn get_property(&self, name: &str, index: Option<&str>) -> WebViewResult<Object> {
        let declaration = match index {
            Some(index) => format!("function() {{return this.{name}[{index}];}}"),
            None => format!("function() {{return this.{name};}}"),
        };
        self.invoke_runtime_function(&declaration, vec![], false, false)
    }
    pub fn set_property(&self, name: &str, value: Object, index: Option<&str>) -> WebViewResult<Object> {
        let declaration = match index {
            Some(index) => format!("function(value) {{return this.{name}[{index}] = value;}}"),
            None => format!("function(value) {{return this.{name} = value;}}"),
        };
        self.invoke_runtime_function(&declaration, vec![value], false, false)
    }
    pub fn get_self_by_index(&self, index: &str) -> WebViewResult<Object> {
        let declaration = format!("function() {{return this[{index}];}}");
        self.invoke_runtime_function(&declaration, vec![], false, false)
    }
    pub fn set_self_by_index(&self, index: &str, value: Object) -> WebViewResult<Object> {
        let declaration = format!("function(value) {{return this[{index}] = value;}}");
        self.invoke_runtime_function(&declaration, vec![value], false, false)
    }
    pub fn invoke_method(&self, name: &str, args: Vec<Object>, await_promise: bool) -> WebViewResult<Object> {
        let declaration = format!("function(...args) {{ return this.{name}(...args); }}");
        self.invoke_runtime_function(&declaration, args, true, await_promise)
    }
    pub fn invoke_self_as_function(&self, args: Vec<Object>, await_promise: bool) -> WebViewResult<Object> {
        let declaration = format!("function(...args) {{ return this(...args); }}");
        self.invoke_runtime_function(&declaration, args, true, await_promise)
    }
    fn to_object(self) -> Object {
        if self.remote.object_id.is_some() {
            Object::WebViewRemoteObject(self)
        } else {
            match self.remote.value {
                Some(value) => value.into(),
                None => Object::Empty,
            }
        }
    }

    fn as_js_iterator(&self) -> WebViewResult<Self> {
        let declaration = "function() { return [...this].values(); }";
        if let Some(id) = &self.remote.object_id {
            self.webview.invoke_runtime_call_function_on(id, declaration, vec![], false, false)
        } else {
            Err(WebViewError::UError(UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())))
        }
    }
    fn js_iterator_next(&self) -> WebViewResult<Self> {
        let declaration = "function() { return this.next(); }";
        if let Some(id) = &self.remote.object_id {
            self.webview.invoke_runtime_call_function_on(id, declaration, vec![], false, false)
        } else {
            Err(WebViewError::UError(UErrorMessage::RemoteObjectIsNotArray(self.remote.r#type.clone())))
        }
    }
    pub fn to_object_vec(&self) -> WebViewResult<Vec<Object>> {
        let iter = self.as_js_iterator()?;
        let mut vec = vec![];
        loop {
            let next = iter.js_iterator_next()?;
            let done = next.get_property("done", None)?;
            if done.is_truthy() {
                break;
            } else {
                let value = next.get_property("value", None)?;
                vec.push(value);
            }
        }
        Ok(vec)
    }
}

pub struct DomEventHandler {
    evaluator: Evaluator,
    handlers: Vec<Function>,
}
impl DomEventHandler {
    fn new(evaluator: Evaluator) -> Self {
        Self { evaluator, handlers: vec![] }
    }
    fn len(&self) -> usize {
        self.handlers.len()
    }
    fn push(&mut self, func: Function) {
        self.handlers.push(func);
    }
    fn get(&mut self, index: usize) -> Option<(&mut Evaluator, &Function)> {
        self.handlers.get(index)
            .map(|f| (&mut self.evaluator, f))
    }
}

#[allow(unused)]
enum WebViewValue {
    None,
    Submit(Value),
}

trait FormMethodArg {
    fn as_string(&self, index: usize) -> WebViewResult<String>;
    fn as_bool(&self, index: usize) -> WebViewResult<bool>;
    fn as_f64(&self, index: usize) -> WebViewResult<f64>;
    fn as_func(&self, index: usize) -> WebViewResult<Function>;
    fn as_remote(&self, index: usize) -> WebViewResult<WebViewRemoteObject>;
}

impl FormMethodArg for Vec<Object> {
    fn as_string(&self, index: usize) -> WebViewResult<String> {
        match self.get(index) {
            Some(obj) => Ok(obj.to_string()),
            None => Err(WebViewError::UError(UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_bool(&self, index: usize) -> WebViewResult<bool> {
        match self.get(index) {
            Some(obj) => Ok(obj.is_truthy()),
            None => Err(WebViewError::UError(UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_f64(&self, index: usize) -> WebViewResult<f64> {
        match self.get(index) {
            Some(obj) => match obj.as_f64(true) {
                Some(n) => Ok(n),
                None => Err(WebViewError::UError(UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string()))),
            },
            None => Err(WebViewError::UError(UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_func(&self, index: usize) -> WebViewResult<Function> {
        match self.get(index) {
            Some(obj) => match obj {
                Object::Function(f) |
                Object::AnonFunc(f)=> Ok(f.clone()),
                obj => Err(WebViewError::UError(UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string()))),
            },
            None => Err(WebViewError::UError(UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }

    fn as_remote(&self, index: usize) -> WebViewResult<WebViewRemoteObject> {
        match self.get(index) {
            Some(obj) => match obj {
                Object::WebViewRemoteObject(remote)=> Ok(remote.clone()),
                obj => Err(WebViewError::UError(UErrorMessage::ArgumentIsNotNumber(index+1, obj.to_string()))),
            },
            None => Err(WebViewError::UError(UErrorMessage::BuiltinArgRequiredAt(index+1))),
        }
    }


}

trait IntoRect {
    fn into_rect(self) -> RECT;
}

impl IntoRect for SIZE {
    fn into_rect(self) -> RECT {
        RECT {
            left: 0,
            top: 0,
            right: self.cx,
            bottom: self.cy,
        }
    }
}

impl RuntimeResult {
    fn into_result(self, webview: WebView) -> WebViewResult<WebViewRemoteObject> {
        if let Some(exception) = self.exception_details {
            Err(WebViewError::JavaScriptError(exception))
        } else {
            let remote = WebViewRemoteObject::new(webview, self.result);
            Ok(remote)
        }
    }
}

#[cfg(target_pointer_width="32")]
unsafe fn set_window_long(hwnd: HWND, nindex: Wam::WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> isize {
    wm::SetWindowLongW(hwnd, nindex, dwnewlong as i32) as isize
}
#[cfg(target_pointer_width="32")]
unsafe fn get_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX,) -> isize {
    wm::GetWindowLongW(hwnd, nindex) as isize
}
#[cfg(target_pointer_width="64")]
unsafe fn set_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> isize {
    wm::SetWindowLongPtrW(hwnd, nindex, dwnewlong)
}
#[cfg(target_pointer_width="64")]
unsafe fn get_window_long(hwnd: HWND, nindex: wm::WINDOW_LONG_PTR_INDEX,) -> isize {
    wm::GetWindowLongPtrW(hwnd, nindex)
}
