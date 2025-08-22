#![allow(clippy::result_large_err)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use super::SocketResult;

// #[derive(Debug, Clone)]
pub struct TcpClient;
impl TcpClient {
    /// サーバーにデータを送信し、レスポンスを得る
    pub fn send(address: &str, port: u16, buf: &[u8]) -> SocketResult<Vec<u8>> {
        let mut stream = TcpStream::connect((address, port))?;
        // 送信
        stream.write_all(buf)?;
        // 受信
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Debug)]
pub struct TcpListener;
impl TcpListener {
    pub fn listen<F>(address: &str, port: u16, handler: &mut F, eod: EndOfData, timeout_sec: f64) -> SocketResult<()>
    where F: FnMut(Vec<u8>) -> SocketResult<Option<Vec<u8>>>,
    {
        let listener = std::net::TcpListener::bind((address, port))?;

        for stream in listener.incoming() {
            let mut stream = stream?;
            let dur = Some(Duration::from_secs_f64(timeout_sec));
            stream.set_read_timeout(dur)?;

            let mut reader = BufReader::new(&stream);

            let buf = match eod {
                EndOfData::Crlf => {
                    let mut line = String::new();
                    reader.read_line(&mut line)?;
                    line.trim_end().as_bytes().to_vec()
                },
                EndOfData::Byte(eod) => {
                    let mut buf = Vec::new();
                    reader.read_until(eod, &mut buf)?;
                    if buf.ends_with(&[eod]) {
                        buf.pop();
                    }
                    buf
                },
            };

            if let Some(response) = handler(buf)? {
                stream.write_all(&response)?;
            } else {
                stream.write_all(&[])?;
                break;
            }
        }
        Ok(())
    }
}

pub enum EndOfData {
    Crlf,
    Byte(u8),
}
