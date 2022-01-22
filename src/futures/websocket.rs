// Copyright (c) 2021 Cranky Kernel
//
// Permission is hereby granted, free of charge, to any person
// obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT.  IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
// HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
// WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use futures_util::{Sink, Stream, TryStreamExt};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::Message;

use crate::common::stream::AggTrade;
use crate::parsers::*;

pub const BASE_URL: &str = "wss://fstream.binance.com";

pub trait BinanceFuturesStream:
    Stream<Item = Result<Event, tokio_tungstenite::tungstenite::Error>> + Sink<Message>
{
}

impl<T: Stream<Item = Result<Event, tokio_tungstenite::tungstenite::Error>> + Sink<Message>>
    BinanceFuturesStream for T
{
}

pub async fn connect<S: AsRef<str>>(
    endpoint: S,
) -> Result<(impl BinanceFuturesStream, Response), tokio_tungstenite::tungstenite::Error> {
    let url = format!("{}/{}", BASE_URL, endpoint.as_ref());
    let (ws, response) = tokio_tungstenite::connect_async(url.clone()).await?;
    let ws = ws.try_filter(|msg| std::future::ready(msg.is_text() || msg.is_ping()));
    let ws = ws.map_ok(Event::decode_message);
    Ok((ws, response))
}

pub async fn connect_stream<S: AsRef<str>>(
    stream: S,
) -> Result<(impl BinanceFuturesStream, Response), tokio_tungstenite::tungstenite::Error> {
    let endpoint = format!("ws/{}", stream.as_ref());
    connect(endpoint).await
}

pub async fn connect_combined<S: AsRef<str>>(
    streams: &[S],
) -> Result<(impl BinanceFuturesStream, Response), tokio_tungstenite::tungstenite::Error> {
    let streams: Vec<&str> = streams.iter().map(|s| s.as_ref()).collect();
    let streams = streams.join("/");
    let endpoint = format!("stream?streams={}", streams);
    connect(endpoint).await
}

#[derive(Debug)]
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
    ParseError(serde_json::Error, String),
    Unknown(String),

    /// WebSocket ping.
    Ping(Vec<u8>),
}

impl Event {
    pub fn decode_message(message: Message) -> Event {
        match message {
            Message::Text(message) => serde_json::from_str::<serde_json::Value>(&message)
                .and_then(Self::decode_value)
                .unwrap_or_else(|err| Some(Self::ParseError(err, message.to_string())))
                .unwrap_or_else(|| Self::Unknown(message.to_string())),
            Message::Ping(data) => Event::Ping(data),
            _ => unreachable!(),
        }
    }

    pub fn decode_value(mut value: serde_json::Value) -> Result<Option<Event>, serde_json::Error> {
        if value["stream"].is_string() && value["data"]["e"].is_string() {
            return Self::decode_data(value["data"].take());
        } else if value["e"].is_string() {
            return Self::decode_data(value);
        }
        Ok(None)
    }

    pub fn decode_data(value: serde_json::Value) -> Result<Option<Event>, serde_json::Error> {
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
                _ => {}
            }
        }
        Ok(None)
    }
}

pub fn agg_trade_stream<S: AsRef<str>>(symbol: S) -> String {
    format!("{}@aggTrade", symbol.as_ref().to_lowercase())
}

pub fn kline_stream<S: AsRef<str>, I: AsRef<str>>(symbol: S, interval: I) -> String {
    format!(
        "{}@kline_{}",
        symbol.as_ref().to_lowercase(),
        interval.as_ref()
    )
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

    // Base asset volume.
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
pub struct AccountUpdateData {
    #[serde(rename = "m")]
    pub reason: String,
    #[serde(rename = "B")]
    pub balances: Vec<AccountUpdateBalances>,
    #[serde(rename = "P")]
    pub positions: Vec<AccountUpdatePosition>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AccountUpdateBalances {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb", deserialize_with = "parse_f64_string")]
    pub wallet_balance: f64,
    #[serde(rename = "cw", deserialize_with = "parse_f64_string")]
    pub cross_wallet_balance: f64,
}

#[derive(Deserialize, Clone, Debug)]
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
}
