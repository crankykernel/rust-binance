// SPDX-License-Identifier: MIT

use tokio_tungstenite::{connect_async, tungstenite};

pub struct WebSocket {}

pub async fn connect(url: &str) -> Result<KrakenWebSocket, tungstenite::Error> {
    let (ws, _response) = connect_async(url).await?;
    Ok(KrakenWebSocket::new(ws))
}
