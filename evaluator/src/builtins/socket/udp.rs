#![allow(clippy::result_large_err)]

use std::net::UdpSocket;
use std::sync::{Arc, RwLock};

use super::{SocketResult, SOCKET_CLOSED_ERROR};

#[derive(Debug, Clone)]
pub struct UdpClient {
    socket: Arc<RwLock<Option<UdpSocket>>>,
}
impl std::fmt::Display for UdpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let read = self.socket.read().unwrap();
        match &*read {
            Some(socket) => match socket.local_addr() {
                Ok(addr) => {
                    write!(f, "UdpClient[{}:{}]", addr.ip(), addr.port())
                },
                Err(e) => {
                    write!(f, "UdpClient[{e}]")
                },
            },
            None => write!(f, "UdpClient (Closed)"),
        }
    }
}
impl PartialEq for UdpClient {
    fn eq(&self, other: &Self) -> bool {
        let _dum = self.socket.write();
        other.socket.try_write().is_err()
    }
}

impl UdpClient {
    pub fn new(address: &str, port: u16) -> SocketResult<Self> {
        let socket = UdpSocket::bind((address, port))?;
        Ok(Self { socket: Arc::new(RwLock::new(Some(socket))) })
    }
    fn use_socket<T>(&self, f: impl FnOnce(&UdpSocket) -> SocketResult<T>) -> SocketResult<T> {
        let guard = self.socket.read().unwrap();
        let socket = guard.as_ref()
            .ok_or(SOCKET_CLOSED_ERROR)?;
        f(socket)
    }
    /// データを送信する
    /// - address: 送信先IPアドレス
    /// - port: 送信先ポート番号
    /// - buf: 送信するデータ
    pub fn send(&self, address: &str, port: u16, buf: &[u8]) -> SocketResult<usize> {
        self.use_socket(|s| {
            let r = s.send_to(buf, (address, port))?;
            Ok(r)
        })
    }
    /// データを受信する
    /// - size: 受信するデータのサイズ
    pub fn receive(&self, size: usize) -> SocketResult<(Vec<u8>, String, u16)> {
        self.use_socket(|s| {
            let mut buf = vec![0; size];
            let (bytes_received, addr) = s.recv_from(&mut buf)?;
            let buf = buf[..bytes_received].to_vec();
            let address = addr.ip().to_string();
            let port = addr.port();
            Ok((buf, address, port))
        })
    }
    pub fn close(&self) {
        let mut write = self.socket.write().unwrap();
        *write = None;
    }
}