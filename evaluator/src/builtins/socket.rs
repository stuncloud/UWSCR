#![allow(clippy::result_large_err)]

mod udp;
mod tcp;
mod websocket;

use crate::error::{UError, UErrorKind, UErrorMessage, UErrorLine};
use crate::builtins::*;
use crate::Evaluator;
use num_derive::{FromPrimitive, ToPrimitive};
use strum_macros::{EnumString, VariantNames};
pub use udp::UdpClient;
pub use tcp::{TcpClient, TcpListener, EndOfData};
pub use websocket::{WebSocket, Message};

type SocketResult<T> = Result<T, UError>;
pub(crate) const SOCKET_CLOSED_ERROR: UError = UError {
    kind: UErrorKind::SocketError,
    message: UErrorMessage::SocketHasBeenClosed,
    is_com_error: false,
    line: UErrorLine::None,
};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("sclose", sclose, get_desc!(sclose));
    sets.add("udpclient", udp_client, get_desc!(udp_client));
    sets.add("udpsend", udp_send, get_desc!(udp_send));
    sets.add("udprecv", udp_recv, get_desc!(udp_recv));
    sets.add("tcpsend", tcp_send, get_desc!(tcp_send));
    sets.add("tcplistener", tcp_listener, get_desc!(tcp_listener));
    sets.add("websocket", websocket, get_desc!(websocket));
    sets.add("wssend", ws_send, get_desc!(ws_send));
    sets.add("wsrecv", ws_recv, get_desc!(ws_recv));
    sets
}

/// ネットワーク系オブジェクト
#[derive(Debug, Clone, PartialEq)]
pub enum USocket {
    /// UDPクライアント
    Udp(udp::UdpClient),
    /// WebSocket
    WebSocket(WebSocket),
}
impl std::fmt::Display for USocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            USocket::Udp(udp_client) => udp_client.fmt(f),
            USocket::WebSocket(websocket) => websocket.fmt(f),
        }
    }
}

#[builtin_func_desc(
    desc="ソケットを閉じる",
    args=[
        {n="ソケット",t="ソケットオブジェクト",d="ソケットを示すオブジェクト"},
    ],
)]
pub fn sclose(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let socket = _args.get_as_socket(0)?;
    match socket {
        USocket::Udp(udp_client) => udp_client.close(),
        USocket::WebSocket(websocket) => websocket.close(),
    }
    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="任意のアドレスとポートで待ち受けるUDPクライアントオブジェクトを返す",
    rtype={desc="UDP送受信を行うためのオブジェクト",types="UDPクライアント"}
    args=[
        {n="IPアドレス",t="文字列",d="自身の待ち受けIPアドレス"},
        {n="ポート",t="数値",d="自身の待ち受けポート"},
    ],
)]
pub fn udp_client(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let addr = args.get_as_string(0, None)?;
    let port = args.get_as_int(1, None)?;
    let client = udp::UdpClient::new(&addr, port)?;
    Ok(Object::Socket(USocket::Udp(client)))
}

#[builtin_func_desc(
    desc="UDPによるデータ送信を行う",
    rtype={desc="送信成功時TRUE",types="真偽値"}
    args=[
        {n="udp",t="UDPクライアント",d="データを送信するUDPクライアント"},
        {n="IPアドレス",t="文字列",d="送信先IPアドレス"},
        {n="ポート",t="数値",d="送信先ポート"},
        {n="送信データ",t="値",d="送信するデータ"},
    ],
)]
pub fn udp_send(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let client = args.get_as_udp(0)?;
    let address = args.get_as_string(1, None)?;
    let port = args.get_as_int(2, None)?;
    let data = args.get_as_bytearray(3)?;
    let result = client.send(&address, port, &data)?;
    Ok(result.into())
}

#[builtin_func_desc(
    desc="UDPによるデータ受信を行う",
    rtype={desc="受信データを示すバイト配列、送信元IPアドレスを示す文字列、送信元ポートを示す数値の配列",types="[バイト配列, 文字列, 数値]"}
    args=[
        {n="udp",t="UDPクライアント",d="データを送信するUDPクライアント"},
        {n="バッファサイズ",t="数値",d="受信バッファのサイズ"},
    ],
)]
pub fn udp_recv(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let client = args.get_as_udp(0)?;
    let size = args.get_as_int(1, None)?;
    let (data, addr, port) = client.receive(size)?;
    Ok(Object::Array(vec![
        Object::ByteArray(data),
        Object::String(addr),
        Object::Num(port as _)
    ]))
}

#[builtin_func_desc(
    desc="サーバーにデータを送信し、そのレスポンスデータを返す",
    rtype={desc="レスポンスデータを示すバイト配列",types="バイト配列"}
    args=[
        {n="IPアドレス",t="文字列",d="接続先IPアドレス"},
        {n="ポート",t="数値",d="接続先ポート"},
        {n="送信データ",t="値",d="送信するデータ"},
    ],
)]
pub fn tcp_send(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let addr = _args.get_as_string(0, None)?;
    let port = _args.get_as_int(1, None)?;
    let data = _args.get_as_bytearray(2)?;
    let res = TcpClient::send(&addr, port, &data)?;
    Ok(Object::ByteArray(res))
}

#[builtin_func_desc(
    desc="TCPサーバー",
    args=[
        {n="IPアドレス",t="文字列",d="待ち受けIPアドレス"},
        {n="ポート",t="数値",d="待ち受けポート"},
        {n="ハンドラ",t="関数",d="受信データを受けてレスポンスを返す関数"},
        {o, n="終端文字",t="ASCII文字",d="受信データの終端を示すASCII文字"},
        {o, n="タイムアウト秒",t="数値",d="受信タイムアウト秒"},
    ],
)]
pub fn tcp_listener(evaluator: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let address = _args.get_as_string(0, None)?;
    let port = _args.get_as_int(1, None)?;
    let func = _args.get_as_user_function(2)?;
    let eod = match _args.get_as_ascii(3)? {
        Some(b) => EndOfData::Byte(b),
        None => EndOfData::Crlf,
    };
    let to_sec = _args.get_as_f64(4, Some(10.0))?;
    let mut handler = |bytes| {
        let bytes = Object::ByteArray(bytes);
        let arguments = vec![(Some(Expression::EmptyArgument), bytes)];
        func.invoke(evaluator, arguments, None)
            .and_then(|obj| {
                let response = if let Some(bytes) = obj.as_bytearray() {
                    Some(bytes)
                } else if matches!(obj, Object::Bool(false)|Object::Null|Object::Empty) {
                    None
                } else {
                    Some(obj.to_string().as_bytes().to_vec())
                };
                Ok(response)
            })
    };
    TcpListener::listen(&address, port, &mut handler, eod, to_sec)?;
    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="WebSocketセッションを張る",
    rtype={desc="WebSocketオブジェクト",types="WebSocket"}
    args=[
        {n="wsuri",t="文字列",d="ws:// から始まるURI"},
    ],
)]
pub fn websocket(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let uri = _args.get_as_string(0, None)?;
    let ws = WebSocket::new(&uri)?;
    Ok(Object::Socket(USocket::WebSocket(ws)))
}


#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum WebSocketConst {
    #[strum[props(desc="WebSocketでCLOSEを送信する")]]
    WS_CLOSE = 0,
    #[strum[props(desc="WebSocketでPINGを送信する")]]
    WS_PING = 1,
    #[strum[props(desc="WebSocketでPONGを送信する")]]
    WS_PONG = 2,
}
impl From<WebSocketConst> for Object {
    fn from(value: WebSocketConst) -> Self {
        value.to_f64().unwrap().into()
    }
}

#[builtin_func_desc(
    desc="WebSocketでデータを送信する",
    args=[
        {n="WebSocket",t="WebSocket",d="WebSocketオブジェクト"},
        {n="送信データ",t="値",d="送信するデータ"},
    ],
)]
pub fn ws_send(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let ws = _args.get_as_websocket(0)?;
    let message = match _args.get_as_object(1, None)? {
        Object::Num(n) if [0, 1, 2].contains(&(n as i32)) => {
            match n as i32 {
                0 => Message::Close(None),
                1 => Message::Ping(Vec::new()),
                2 => Message::Pong(Vec::new()),
                _ => unreachable!(),
            }
        },
        Object::String(s) => Message::Text(s),
        Object::UObject(uo) => Message::Text(uo.to_json_string().map_err(UError::from)?),
        Object::ByteArray(bytes) => Message::Binary(bytes),
        o if matches!(o, Object::Array(_)) => match o.as_bytearray() {
            Some(bytes) => Message::Binary(bytes),
            None => Err(BuiltinFuncError::new(UErrorMessage::Any("can not convert array to byte array".into())))?,
        }
        o => Message::Text(o.to_string()),
    };
    ws.send(message)?;
    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="",
    rtype={desc="受信データ",types="文字列、バイト配列、定数"}
    args=[
        {n="WebSocket",t="WebSocket",d="WebSocketオブジェクト"},
    ],
)]
pub fn ws_recv(_: &mut Evaluator, _args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let ws = _args.get_as_websocket(0)?;
    let message = ws.receive()?;
    match message {
        Message::Text(s) => Ok(Object::String(s)),
        Message::Binary(bytes) => Ok(Object::ByteArray(bytes)),
        Message::Ping(_) => Ok(WebSocketConst::WS_PING.into()),
        Message::Pong(_) => Ok(WebSocketConst::WS_PONG.into()),
        Message::Close(_) => Ok(WebSocketConst::WS_CLOSE.into()),
        Message::Frame(_) => Ok(Object::Empty),
    }
}