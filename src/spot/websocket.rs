// SPDX-License-Identifier: MIT

use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};

pub const BASE_URL: &str = "https://stream.binance.com:9443";

pub struct WebSocket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocket {
    pub fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws }
    }

    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.ws.next().await
    }
}

pub async fn connect(url: &str) -> Result<WebSocket, tungstenite::Error> {
    let (ws, _response) = connect_async(url).await?;
    Ok(WebSocket::new(ws))
}

pub async fn connect_stream(name: &str) -> Result<WebSocket, tungstenite::Error> {
    let url = format!("{}/ws/{}", BASE_URL, name);
    connect(&url).await
}

pub async fn connect_combined<T: AsRef<str>>(
    streams: &[T],
) -> Result<WebSocket, tungstenite::Error> {
    let streams: Vec<&str> = streams.iter().map(|e| e.as_ref()).collect();
    let url = format!("{}/stream?streams={}", BASE_URL, streams.join("/"));
    connect(&url).await
}
