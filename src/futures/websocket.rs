// Copyright (c) 2021-2022 Cranky Kernel
//
// SPDX-License-Identifier: MIT

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};

use crate::common::stream::AggTrade;
use crate::parsers::*;

pub const BASE_URL: &str = "wss://fstream.binance.com";

pub struct WebSocket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocket {
    pub fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws }
    }

    pub async fn next(&mut self) -> Option<Result<Event, tokio_tungstenite::tungstenite::Error>> {
        loop {
            let next = self.ws.next().await;
            match next {
                Some(Ok(message)) => match message {
                    Message::Ping(_) | Message::Text(_) => {
                        return Some(Ok(Event::decode_message(message)));
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

pub async fn connect<T: AsRef<str>>(url: T) -> Result<WebSocket, tungstenite::Error> {
    let (ws, _response) = connect_async(url.as_ref()).await?;
    Ok(WebSocket::new(ws))
}

pub async fn connect_stream<T: AsRef<str>>(name: T) -> Result<WebSocket, tungstenite::Error> {
    let url = format!("{}/ws/{}", BASE_URL, name.as_ref());
    connect(&url).await
}

pub async fn connect_combined<T: AsRef<str>>(
    streams: &[T],
) -> Result<WebSocket, tungstenite::Error> {
    let streams: Vec<&str> = streams.iter().map(|e| e.as_ref()).collect();
    let url = format!("{}/stream?streams={}", BASE_URL, streams.join("/"));
    connect(&url).await
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Event {
    /// Undecoded WebSocket message.
    Message(Message),
    /// Kline/OHLC event.
    Kline(KlineEvent),
    /// Aggregate trade event.
    AggTrade(AggTrade),
    /// Order update event (user stream).
    OrderTradeUpdate(OrderTradeUpdateEvent),
    /// Account update event (user stream).
    AccountUpdate(AccountUpdate),
    /// Public liquidation event.
    LiquidationEvent(LiquidationEvent),
    Ticker(Ticker),

    /// A serde deserialize error. We use a string for the serde error so we can implement clone.
    /// The second string is the input that failed to parse.
    ParseError(String, String),
    Unknown(String),

    /// WebSocket ping.
    Ping(Vec<u8>),
}

impl Event {
    pub fn decode_message(message: Message) -> Event {
        match message {
            Message::Text(message) => serde_json::from_str::<Value>(&message)
                .and_then(Self::decode_value)
                .unwrap_or_else(|err| Some(Self::ParseError(err.to_string(), message.to_string())))
                .unwrap_or_else(|| Self::Unknown(message.to_string())),
            Message::Ping(data) => Event::Ping(data),
            _ => unreachable!(),
        }
    }

    pub fn decode_value(mut value: Value) -> Result<Option<Event>, serde_json::Error> {
        if value["stream"].is_string() && value["data"]["e"].is_string() {
            return Self::decode_data(value["data"].take());
        } else if value["e"].is_string() {
            return Self::decode_data(value);
        }
        Ok(None)
    }

    pub fn decode_data(mut value: Value) -> Result<Option<Event>, serde_json::Error> {
        if let Some(e) = value["e"].as_str() {
            match e {
                "kline" => {
                    return Ok(Some(Event::Kline(serde_json::from_value(value)?)));
                }
                "aggTrade" => {
                    return Ok(Some(Event::AggTrade(serde_json::from_value(value)?)));
                }
                "ORDER_TRADE_UPDATE" => {
                    return Ok(Some(Event::OrderTradeUpdate(serde_json::from_value(
                        value,
                    )?)));
                }
                "ACCOUNT_UPDATE" => {
                    return Ok(Some(Event::AccountUpdate(serde_json::from_value(value)?)));
                }
                "forceOrder" => {
                    return Ok(Some(Event::LiquidationEvent(serde_json::from_value(
                        value["o"].take(),
                    )?)));
                }
                "24hrTicker" => {
                    return Ok(Some(Event::Ticker(serde_json::from_value(value)?)));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    pub fn is_liquidation_event(&self) -> bool {
        match self {
            Self::LiquidationEvent(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct KlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: f64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: Kline,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Kline {
    #[serde(rename = "t")]
    pub open_time: f64,
    #[serde(rename = "T")]
    pub close_time: f64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o", deserialize_with = "parse_f64_string")]
    pub open: f64,
    #[serde(rename = "c", deserialize_with = "parse_f64_string")]
    pub close: f64,
    #[serde(rename = "h", deserialize_with = "parse_f64_string")]
    pub high: f64,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub low: f64,

    /// Base asset volume.
    #[serde(rename = "v", deserialize_with = "parse_f64_string")]
    pub volume: f64,

    // Number of trades.
    #[serde(rename = "n")]
    pub trade_count: u64,

    // Quote asset volume.
    #[serde(rename = "q", deserialize_with = "parse_f64_string")]
    pub quote_volume: f64,

    // Taker buy base asset volume.
    #[serde(rename = "V", deserialize_with = "parse_f64_string")]
    pub taker_base_volume: f64,

    // Taker buy quote asset volume.
    #[serde(rename = "Q", deserialize_with = "parse_f64_string")]
    pub taker_buy_quote_volume: f64,

    #[serde(rename = "x")]
    pub closed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: f64,
    #[serde(rename = "T")]
    pub transaction_time: f64,
    #[serde(rename = "o")]
    pub update: OrderTradeUpdate,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub order_side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q", deserialize_with = "parse_f64_string")]
    pub orig_qty: f64,
    #[serde(rename = "p", deserialize_with = "parse_f64_string")]
    pub orig_price: f64,
    #[serde(rename = "ap", deserialize_with = "parse_f64_string")]
    pub avg_price: f64,
    #[serde(rename = "sp", deserialize_with = "parse_f64_string")]
    pub stop_price: f64,
    #[serde(rename = "x")]
    pub execution_type: String,
    #[serde(rename = "X")]
    pub order_status: String,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub last_fill_amount: f64,
    #[serde(rename = "z", deserialize_with = "parse_f64_string")]
    pub cum_fill_amount: f64,
    #[serde(rename = "L", deserialize_with = "parse_f64_string")]
    pub last_fill_price: f64,
    #[serde(default, rename = "N")]
    pub commission_asset: Option<String>,
    #[serde(default, rename = "n", deserialize_with = "parse_opt_f64_string")]
    pub commission: Option<f64>,
    #[serde(rename = "T")]
    pub order_trade_time: u64,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "b", deserialize_with = "parse_f64_string")]
    pub bids_notional: f64,
    #[serde(rename = "a", deserialize_with = "parse_f64_string")]
    pub asks_notional: f64,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "R")]
    pub is_reduce_only: bool,
    #[serde(rename = "wt")]
    pub stop_price_working_type: String,
    #[serde(rename = "ot")]
    pub orig_order_type: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "cp")]
    pub is_close_all: bool,
    #[serde(default, rename = "AP", deserialize_with = "parse_opt_f64_string")]
    pub activation_price: Option<f64>,
    #[serde(default, rename = "cr", deserialize_with = "parse_opt_f64_string")]
    pub callback_rate: Option<f64>,
    #[serde(rename = "rp", deserialize_with = "parse_f64_string")]
    pub realized_profit: f64,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct AccountUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub tx_time: u64,
    #[serde(rename = "a")]
    pub data: AccountUpdateData,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct AccountUpdateData {
    #[serde(rename = "m")]
    pub reason: String,
    #[serde(rename = "B")]
    pub balances: Vec<AccountUpdateBalances>,
    #[serde(rename = "P")]
    pub positions: Vec<AccountUpdatePosition>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct AccountUpdateBalances {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb", deserialize_with = "parse_f64_string")]
    pub wallet_balance: f64,
    #[serde(rename = "cw", deserialize_with = "parse_f64_string")]
    pub cross_wallet_balance: f64,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct AccountUpdatePosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa", deserialize_with = "parse_f64_string")]
    pub position_amount: f64,
    #[serde(rename = "ep", deserialize_with = "parse_f64_string")]
    pub entry_price: f64,
    #[serde(rename = "cr", deserialize_with = "parse_f64_string")]
    pub accumulated_realized: f64,
    #[serde(rename = "up", deserialize_with = "parse_f64_string")]
    pub unrealized_profit: f64,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw", deserialize_with = "parse_f64_string")]
    pub isolated_wallet: f64,
    #[serde(rename = "ps")]
    pub position_side: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct LiquidationEvent {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q", deserialize_with = "parse_f64_string")]
    pub original_quantity: f64,
    #[serde(rename = "p", deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "ap", deserialize_with = "parse_f64_string")]
    pub average_price: f64,
    #[serde(rename = "X")]
    pub order_status: String,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub last_fill_quantity: f64,
    #[serde(rename = "z", deserialize_with = "parse_f64_string")]
    pub accumulated_quantity: f64,
    #[serde(rename = "T")]
    pub trade_time: u64,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Ticker {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p", deserialize_with = "parse_f64_string")]
    pub price_change: f64,
    #[serde(rename = "P", deserialize_with = "parse_f64_string")]
    pub price_change_percent: f64,
    #[serde(rename = "w", deserialize_with = "parse_f64_string")]
    pub weight_avg_price: f64,
    #[serde(rename = "c", deserialize_with = "parse_f64_string")]
    pub last_price: f64,
    #[serde(rename = "Q", deserialize_with = "parse_f64_string")]
    pub last_quantity: f64,
    #[serde(rename = "o", deserialize_with = "parse_f64_string")]
    pub open_price: f64,
    #[serde(rename = "h", deserialize_with = "parse_f64_string")]
    pub high_price: f64,
    #[serde(rename = "l", deserialize_with = "parse_f64_string")]
    pub low_price: f64,
    #[serde(rename = "v", deserialize_with = "parse_f64_string")]
    pub base_asset_volume: f64,
    #[serde(rename = "q", deserialize_with = "parse_f64_string")]
    pub quote_asset_volume: f64,
    #[serde(rename = "O")]
    pub stats_open_time: u64,
    #[serde(rename = "C")]
    pub stats_close_time: u64,
    #[serde(rename = "F")]
    pub first_trade_id: u64,
    #[serde(rename = "L")]
    pub last_trade_id: u64,
    #[serde(rename = "n")]
    pub trade_count: u64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_order_trade_update() {
        let _text = r#"{
            "e":"ORDER_TRADE_UPDATE",
            "T":1612418801174,
            "E":1612418801179,
            "o":{
                "s":"BTCUSDT",
                "c":"electron_Zh7oBoYbpj7MiQLtom2u",
                "S":"SELL",
                "o":"LIMIT",
                "f":"GTC",
                "q":"0.100",
                "p":"40000","ap":"0","sp":"0","x":"NEW","X":"NEW","i":13584185467,"l":"0","z":"0","L":"0","T":1612418801174,"t":0,"b":"0","a":"4000","m":false,"R":false,"wt":"CONTRACT_PRICE","ot":"LIMIT","ps":"SHORT","cp":false,"rp":"0","pP":false,"si":0,"ss":0}}"#;
        let _order_trade_update: OrderTradeUpdateEvent = serde_json::from_str(&_text).unwrap();
    }

    #[test]
    fn test_deserialize_liquidation_event() {
        let text = "\
            {\"stream\":\"dogebusd@forceOrder\",\
             \"data\":\
                {\"e\":\"forceOrder\",\
                 \"E\":1643948075159,\
                 \"o\":{\
                    \"s\":\"DOGEBUSD\",\
                    \"S\":\"BUY\",\
                    \"o\":\"LIMIT\",\
                    \"f\":\"IOC\",\
                    \"q\":\"568\",\
                    \"p\":\"0.142229\",\
                    \"ap\":\"0.138770\",\
                    \"X\":\"FILLED\",\
                    \"l\":\"568\",\
                    \"z\":\"568\",\
                    \"T\":1643948075152}}}";

        let mut value: Value = serde_json::from_str(text).unwrap();
        let _: LiquidationEvent = serde_json::from_value(value["data"]["o"].take()).unwrap();

        let event = Event::decode_message(Message::Text(text.to_string()));
        assert!(event.is_liquidation_event());
        if let Event::LiquidationEvent(event) = event {
            assert_eq!(event.symbol, "DOGEBUSD");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_deserialize_ticker() {
        let text = "{\
            \"stream\":\"btcusdt@ticker\",\
            \"data\":{\
                \"e\":\"24hrTicker\",\
                \"E\":1643956077872,\
                \"s\":\"BTCUSDT\",\
                \"p\":\"1070.58\",\
                \"P\":\"2.905\",\
                \"w\":\"36828.93\",\
                \"c\":\"37920.00\",\
                \"Q\":\"0.095\",\
                \"o\":\"36849.42\",\
                \"h\":\"38000.00\",\
                \"l\":\"36204.30\",\
                \"v\":\"338765.425\",\
                \"q\":\"12476366874.26\",\
                \"O\":1643869620000,\
                \"C\":1643956077866,\
                \"F\":1888682750,\
                \"L\":1891841228,\
                \"n\":3158308}}";
    }
}
