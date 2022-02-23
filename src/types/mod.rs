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

pub mod interval;
use std::collections::HashMap;

pub use interval::*;
use serde::{Deserialize, Serialize};

use crate::parsers::*;

/// Cancel order datatype. Currently valid for Spot and Futures.
#[derive(Serialize, Debug, Clone)]
pub struct CancelOrder {
    pub symbol: String,
    #[serde(rename = "orderId", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<u64>,
    #[serde(rename = "origClientOrderId", skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

impl CancelOrder {
    pub fn by_order_id(symbol: &str, order_id: u64) -> Self {
        Self {
            symbol: symbol.into(),
            order_id: Some(order_id),
            client_order_id: None,
        }
    }

    pub fn by_client_order_id(symbol: &str, client_order_id: &str) -> Self {
        Self {
            symbol: symbol.into(),
            order_id: None,
            client_order_id: Some(client_order_id.into()),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum TimeInForce {
    /// Good till cancel.
    GTC,
    /// Immediate or cancel.
    IOC,
    /// Fill or kill.
    FOK,
    /// Good till cross (post only).
    GTX,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BookTickerResponse {
    pub symbol: String,
    #[serde(rename = "bidPrice", deserialize_with = "parse_f64_string")]
    pub bid_price: f64,
    #[serde(rename = "bidQty", deserialize_with = "parse_f64_string")]
    pub bid_qty: f64,
    #[serde(rename = "askPrice", deserialize_with = "parse_f64_string")]
    pub ask_price: f64,
    #[serde(rename = "askQty", deserialize_with = "parse_f64_string")]
    pub ask_qty: f64,
    pub time: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct OrderResponse {
    pub clientOrderId: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub cumQty: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub cumQuote: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub executedQty: f64,
    pub orderId: u64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub avgPrice: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub origQty: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
    pub reduceOnly: bool,
    pub side: String,
    pub positionSide: String,
    pub status: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub stopPrice: f64,
    pub closePosition: bool,
    pub symbol: String,
    pub timeInForce: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub origType: String,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub activatePrice: Option<f64>,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub priceRate: Option<f64>,
    pub updateTime: u64,
    pub workingType: String,
    pub priceProtect: bool,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Kline {
    pub open_time: u64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub open: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub high: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub low: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub close: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub volume: f64,
    pub close_time: u64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub quote_asset_volume: f64,
    pub trade_count: u64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub taker_buy_base_volume: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub taker_buy_quote_volume: f64,

    #[serde(deserialize_with = "parse_f64_string")]
    pub ignore: f64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_parse_new_order_response() {
        let text = r#"{
            "orderId":13617515110,
            "symbol":"BTCUSDT",
            "status":"NEW",
            "clientOrderId":"p9Ezrfm1GgNBN1WFDz5nce",
            "price":"20000",
            "avgPrice":"0.00000",
            "origQty":"0.010",
            "executedQty":"0",
            "cumQty":"0",
            "cumQuote":"0",
            "timeInForce":"GTC",
            "type":"LIMIT",
            "reduceOnly":false,
            "closePosition":false,
            "side":"BUY",
            "positionSide":"LONG",
            "stopPrice":"0",
            "workingType":"CONTRACT_PRICE",
            "priceProtect":false,
            "origType":"LIMIT",
            "updateTime":1612459859211}"#;
        let _response: OrderResponse = serde_json::from_str(text).unwrap();
    }
}
