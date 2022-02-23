// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use anyhow::Result;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tracing::error;

use crate::parsers::*;

pub const BASE_URL: &str = "wss://stream.binance.com:9443";

pub struct WebSocket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocket {
    pub fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws }
    }

    pub async fn next(&mut self) -> Option<Result<Event, Error>> {
        loop {
            let next = self.ws.next().await;
            match next {
                Some(Ok(message)) => match message {
                    Message::Ping(_) | Message::Text(_) => {
                        return Some(Ok(Decoder {}.decode_event(message)));
                    }
                    _ => {
                        // Ignore, move onto the next incoming message.
                    }
                },
                Some(Err(err)) => {
                    return Some(Err(err));
                }
                None => {
                    return None;
                }
            }
        }
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

pub struct Decoder {}

impl Decoder {
    fn decode_event(&self, message: Message) -> Event {
        if let Message::Text(s) = &message {
            if let Ok(value) = serde_json::from_str::<Value>(s) {
                if value["e"].as_str().is_some() {
                    match self.decode_value(value) {
                        Ok(Some(event)) => {
                            return event;
                        }
                        Err(err) => {
                            error!("Failed to decode event: {} -- {}", err, message);
                        }
                        _ => {}
                    }
                }
            }
        }
        Event::Message(message)
    }

    fn decode_value(&self, value: Value) -> Result<Option<Event>> {
        match value["e"].as_str() {
            Some("executionReport") => {
                Ok(Some(Event::ExecutionReport(serde_json::from_value(value)?)))
            }
            Some("outboundAccountPosition") => {
                Ok(Some(Event::AccountUpdate(serde_json::from_value(value)?)))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    ExecutionReport(ExecutionReport),
    AccountUpdate(AccountUpdate),

    /// Undecoded WebSocket message.
    Message(Message),
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExecutionReport {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q", deserialize_with = "parse_f64_string")]
    pub quantity: f64,
    #[serde(rename = "p", deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "P", deserialize_with = "parse_f64_string")]
    pub stop_price: f64,
    #[serde(rename = "F", deserialize_with = "parse_f64_string")]
    pub iceberg_quantity: f64,
    #[serde(rename = "C")]
    pub orig_client_order_id: String,
    #[serde(rename = "x")]
    pub current_execution_type: String,
    #[serde(rename = "X")]
    pub current_order_status: String,
    #[serde(rename = "r")]
    pub reject_reason: String,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub last_executed_quantity: f64,
    #[serde(rename = "z", deserialize_with = "parse_f64_string")]
    pub cumulative_filled_quantity: f64,
    #[serde(rename = "L", deserialize_with = "parse_f64_string")]
    pub last_executed_price: f64,
    #[serde(rename = "n", deserialize_with = "parse_f64_string")]
    pub commission_amount: f64,
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "t")]
    pub trade_id: i64,
    #[serde(rename = "I")]
    pub ignore0: i64,
    #[serde(rename = "w")]
    pub on_book: bool,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "M")]
    pub ignore1: bool,
    #[serde(rename = "O")]
    pub order_creation_time: u64,
    #[serde(rename = "Z", deserialize_with = "parse_f64_string")]
    pub cumulative_quote_quantity: f64,
    #[serde(rename = "Y", deserialize_with = "parse_f64_string")]
    pub last_quote_quantity: f64,
    #[serde(rename = "Q", deserialize_with = "parse_f64_string")]
    pub quote_order_quantity: f64,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AccountUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "u")]
    pub last_update_time: u64,
    #[serde(rename = "B")]
    pub balances: Vec<AccountUpdateBalance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AccountUpdateBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "f", deserialize_with = "parse_f64_string")]
    pub free: f64,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub locked: f64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_execution_report() {
        let text = "{\
            \"e\":\"executionReport\",\
            \"E\":1617686659383,\
            \"s\":\"TFUELUSDT\",\
            \"c\":\"iWoC4jk0fET4rFkrvFFY1y\",\
            \"S\":\"BUY\",\
            \"o\":\"MARKET\",\
            \"f\":\"GTC\",\
            \"q\":\"30.00000000\",\
            \"p\":\"0.00000000\",\
            \"P\":\"0.00000000\",\
            \"F\":\"0.00000000\",\
            \"g\":-1,\
            \"C\":\"\",\
            \"x\":\"TRADE\",\
            \"X\":\"FILLED\",\
            \"r\":\"NONE\",\
            \"i\":197554424,\
            \"l\":\"30.00000000\",\
            \"z\":\"30.00000000\",\
            \"L\":\"0.36653900\",\
            \"n\":\"0.00002190\",\
            \"N\":\"BNB\",\
            \"T\":1617686659382,\
            \"t\":13155802,\
            \"I\":408139578,\
            \"w\":false,\
            \"m\":false,\
            \"M\":true,\
            \"O\":1617686659382,\
            \"Z\":\"10.99617000\",\
            \"Y\":\"10.99617000\",\
            \"Q\":\"0.00000000\"}";
        let _report: ExecutionReport = serde_json::from_str(text).unwrap();
    }

    #[test]
    fn test_decode_outbound_account_position() {
        let _text = "{\
            \"e\":\"outboundAccountPosition\",\
            \"E\":1617686659383,\
            \"u\":1617686659382,\
            \"B\":[\
                {\
                    \"a\":\"BNB\",\
                    \"f\":\"0.49669914\",\
                    \"l\":\"0.00000000\"\
                },\
                {\
                    \"a\":\"USDT\",\
                    \"f\":\"124.93107143\",\
                    \"l\":\"1343.75000000\"\
                },\
                {\
                    \"a\":\"TFUEL\",\
                    \"f\":\"1530.00000000\",\
                    \"l\":\"0.00000000\"\
                }\
            ]\
        }";
    }
}
