#![allow(clippy::result_large_err)]

use tungstenite::stream::MaybeTlsStream;
pub use tungstenite::Message;
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use super::SocketResult;
use crate::{UError, UErrorKind, UErrorMessage};

#[derive(Debug)]
struct WebSocketInner {
    socket: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    closed: bool
}
impl WebSocketInner {
    fn close(&mut self) {
        let _ = self.socket.close(None);
        self.closed = true;
    }
    fn is_closed(&self) -> bool {
        self.closed
    }
}
impl Drop for WebSocketInner {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Debug, Clone)]
pub struct WebSocket {
    inner: Arc<RwLock<WebSocketInner>>,
}
impl std::fmt::Display for WebSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let socket = self.inner.read().unwrap();
        if socket.is_closed() {
            write!(f, "WebSocket (Closed)")
        } else {
            let stream = socket.socket.get_ref();
            if let MaybeTlsStream::Plain(s) = stream && let Ok(addr) = s.peer_addr() {
                write!(f, "WebSocket -> {}:{}", addr.ip(), addr.port())
            } else {
                write!(f, "WebSocket")
            }
        }
    }
}
impl PartialEq for WebSocket {
    fn eq(&self, other: &Self) -> bool {
        let _dummy = self.inner.write();
        other.inner.try_write().is_err()
    }
}
impl WebSocket {
    pub fn new(uri: &str) -> SocketResult<Self> {
        let (socket, response) = tungstenite::connect(uri)?;
        let status = response.status();
        if status.as_u16() >= 400 {
            Err(UError::new(UErrorKind::SocketError, UErrorMessage::Any(status.to_string())))
        } else {
            let inner = WebSocketInner { socket, closed: false };
            Ok(Self { inner: Arc::new(RwLock::new(inner))})
        }
    }
    pub fn close(&self) {
        let mut socket = self.inner.write().unwrap();
        socket.close();
    }
    pub fn send(&self, message: Message) -> SocketResult<()> {
        let mut write = self.inner.write().unwrap();
        write.socket.send(message)?;
        Ok(())
    }
    pub fn receive(&self) -> SocketResult<Message> {
        let mut write = self.inner.write().unwrap();
        let msg = write.socket.read()?;
        Ok(msg)
    }
}