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

use std::collections::HashMap;
use std::fmt::Formatter;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::common::client::{Authentication, ListenKeyResponse};
use crate::parsers::*;
use crate::spot::client::{ExchangeInfoResponse, OrderSide, OrderType};
use crate::types::{BookTickerResponse, CancelOrder, TimeInForce};
use crate::Error;

pub const API_ROOT: &str = "https://fapi.binance.com";

#[derive(Clone)]
pub struct Client {
    client: crate::common::client::Client,
}

impl Client {
    pub fn new(authentication: Option<Authentication>) -> Self {
        Self {
            client: crate::common::client::Client::new(API_ROOT, authentication),
        }
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query_string: Option<&str>,
    ) -> Result<T, Error> {
        let url = self.client.url2(endpoint, query_string)?;
        let response = self.client.client.get(url).send().await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    pub async fn authenticated_get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        form: &str,
    ) -> Result<T, Error> {
        let form = self.client.sign_form(Some(form))?;
        let request = self
            .client
            .get3(endpoint, Some(&form))?
            .headers(self.client.headers()?);
        let response = request.send().await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    pub fn decode_response<T>(&self, status: StatusCode, body: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        match status {
            StatusCode::OK => serde_json::from_str(body).map_err(|error| Error::Decode {
                error,
                text: body.to_string(),
            }),
            _ => {
                // Error, attempt to decode the response, if that fails fake it out.
                if let Ok(error) = serde_json::from_str::<ApiError>(body) {
                    Err(Error::ApiError(error))
                } else {
                    Err(Error::ApiError(ApiError {
                        code: status.as_u16() as i64,
                        msg: body.to_string(),
                        other: HashMap::new(),
                    }))
                }
            }
        }
    }

    pub async fn get_open_orders<S: AsRef<str> + std::fmt::Display + std::fmt::Debug>(
        &self,
        symbol: Option<S>,
    ) -> Result<Vec<OpenOrder>, Error> {
        let endpoint = "/fapi/v1/openOrders";
        let mut form = vec![];
        if let Some(symbol) = symbol {
            form.push(("symbol", symbol));
        }
        let form = build_form(&form[..]);
        let response = self.authenticated_get(endpoint, &form).await?;
        Ok(response)
    }

    pub async fn cancel_order(&self, request: &CancelOrder) -> Result<CancelOrderResponse, Error> {
        let endpoint = "/fapi/v1/order";
        let _form = serde_urlencoded::to_string(request)?;
        let _form = self.client.sign_form(Some(&_form))?;
        let url = self.client.url2(endpoint, Some(&_form))?;
        let request = self
            .client
            .client
            .delete(url)
            .headers(self.client.headers()?);
        let response = request.send().await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u16>,
    ) -> Result<Vec<Kline>, Error> {
        use std::fmt::Write;
        let endpoint = "/fapi/v1/klines";
        let mut qs = format!("symbol={}&interval={}", symbol, interval);
        if let Some(limit) = limit {
            write!(qs, "&limit={}", limit).unwrap();
        }
        let response = self.client.get3(endpoint, Some(&qs))?.send().await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    pub async fn get_exchange_info(&self) -> Result<ExchangeInfoResponse, Error> {
        let endpoint = "/fapi/v1/exchangeInfo";
        self.get(endpoint, None).await
    }

    pub async fn post_listenkey(&self) -> Result<ListenKeyResponse, Error> {
        let endpoint = "/fapi/v1/listenKey";
        self.client.post_listenkey(endpoint).await
    }

    pub async fn put_listenkey(&self) -> Result<ListenKeyResponse, Error> {
        let endpoint = "/fapi/v1/listenKey";
        self.client.put_listenkey(endpoint).await
    }

    pub async fn post_new_order(&self, request: &NewOrder) -> Result<OrderResponse, Error> {
        let endpoint = "/fapi/v1/order";
        let form = serde_urlencoded::to_string(&request)?;
        let form = self.client.sign_form(Some(&form))?;
        let response = self
            .client
            .client
            .post(&format!("{}{}", API_ROOT, endpoint))
            .headers(self.client.headers()?)
            .body(form)
            .send()
            .await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    // pub async fn post_batch_orders(
    //     &self,
    //     orders: &[&NewOrder],
    // ) -> Result<Vec<OrderResponse>, Error> {
    //     let endpoint = "/fapi/v1/batchOrders";
    //
    //     let orders = serde_json::to_string(orders).unwrap();
    //     dbg!(&orders);
    //
    //     #[derive(Serialize)]
    //     struct BatchOrder {
    //         batchOrders: String,
    //     }
    //
    //     let batchOrder = BatchOrder {
    //         batchOrders: orders,
    //     };
    //
    //     let form = serde_urlencoded::to_string(&batchOrder)?;
    //     dbg!(&form);
    //     let form = self.client.sign_form(Some(&form))?;
    //     let response = self
    //         .client
    //         .client
    //         .post(&format!("{}{}", API_ROOT, endpoint))
    //         .headers(self.client.headers()?)
    //         .body(form)
    //         .send()
    //         .await?;
    //     let code = response.status();
    //     let body = response.text().await?;
    //     self.decode_response(code, &body)
    // }

    pub async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<PositionEntry>, Error> {
        let endpoint = "/fapi/v2/positionRisk";
        let form = if let Some(symbol) = symbol {
            format!("symbol={}", symbol)
        } else {
            "".to_string()
        };
        self.authenticated_get(endpoint, &form).await
    }

    pub async fn book_ticker(&self, symbol: &str) -> Result<BookTickerResponse, Error> {
        let endpoint = "/fapi/v1/ticker/bookTicker";
        let url = self
            .client
            .url2(endpoint, Some(&format!("symbol={}", symbol)))?;
        let response = self.client.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        self.decode_response(status, &body)
    }
}

fn build_form<S: AsRef<str> + std::fmt::Display + std::fmt::Debug>(vals: &[(&str, S)]) -> String {
    let mut buf = String::new();
    for (key, val) in vals {
        if !buf.is_empty() {
            buf.push('&');
        }
        buf.push_str(key);
        buf.push('=');
        buf.push_str(val.as_ref());
    }
    buf
}

/// Open Order Response
///
/// TODO: Renames...
#[derive(Debug, Deserialize)]
pub struct OpenOrder {
    #[serde(rename = "avgPrice", deserialize_with = "parse_f64_string")]
    pub avg_price: f64,
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde(rename = "cumQuote", deserialize_with = "parse_f64_string")]
    pub cum_quote: f64,
    #[serde(rename = "executedQty", deserialize_with = "parse_f64_string")]
    pub executed_qty: f64,
    #[serde(rename = "orderId")]
    pub order_id: u64,
    #[serde(rename = "origQty", deserialize_with = "parse_f64_string")]
    pub orig_qty: f64,
    #[serde(rename = "origType")]
    pub orig_type: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    pub side: String,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    pub status: String,
    #[serde(rename = "stopPrice", deserialize_with = "parse_f64_string")]
    pub stop_price: f64,
    #[serde(rename = "closePosition")]
    pub close_position: bool,
    pub symbol: String,
    pub time: u64,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub xtype: String,
    #[serde(
        rename = "activatePrice",
        default,
        deserialize_with = "parse_f64_string_opt"
    )]
    pub active_price: Option<f64>,
    #[serde(
        rename = "priceRate",
        default,
        deserialize_with = "parse_f64_string_opt"
    )]
    pub price_rate: Option<f64>,
    #[serde(rename = "updateTime")]
    pub update_time: u64,
    #[serde(rename = "workingType")]
    pub working_type: String,
    #[serde(rename = "priceProtect")]
    pub price_protect: bool,
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

pub fn parse_f64_string_opt<'de, D>(d: D) -> Result<Option<f64>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use serde::de::Error;
    let s: String = serde::de::Deserialize::deserialize(d)?;
    let val = s.parse::<f64>().map_err(D::Error::custom)?;
    Ok(Some(val))
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub code: i64,
    pub msg: String,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl std::error::Error for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "code: {}, msg: {}", self.code, self.msg)
    }
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: u64,
    pub symbol: String,
    pub status: String,
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "avgPrice", deserialize_with = "parse_f64_string")]
    pub avg_price: f64,

    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Debug, Clone)]
pub enum PositionSide {
    #[serde(rename = "BOTH")]
    Both,
    #[serde(rename = "LONG")]
    Long,
    #[serde(rename = "SHORT")]
    Short,
}

#[derive(Serialize, Debug, Clone)]
pub enum ReduceOnly {
    #[serde(rename = "YES")]
    Yes,
    #[serde(rename = "NO")]
    No,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PositionEntry {
    #[serde(rename = "entryPrice", deserialize_with = "parse_f64_string")]
    pub entry_price: f64,
    #[serde(rename = "marginType")]
    pub margin_type: String,
    #[serde(rename = "isAutoAddMargin", deserialize_with = "parse_bool_string")]
    pub auto_add_margin: bool,
    #[serde(rename = "isolatedMargin", deserialize_with = "parse_f64_string")]
    pub isolated_margin: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub leverage: f64,
    #[serde(rename = "liquidationPrice", deserialize_with = "parse_f64_string")]
    pub liquidation_price: f64,
    #[serde(rename = "markPrice", deserialize_with = "parse_f64_string")]
    pub mark_price: f64,
    #[serde(rename = "maxNotionalValue", deserialize_with = "parse_f64_string")]
    pub max_notional_value: f64,
    #[serde(rename = "positionAmt", deserialize_with = "parse_f64_string")]
    pub position_amount: f64,
    pub symbol: String,
    #[serde(rename = "unRealizedProfit", deserialize_with = "parse_f64_string")]
    pub unrealized_profit: f64,
    #[serde(rename = "positionSide")]
    pub position_side: String,
}

impl PositionEntry {
    // Returns a position ID. For Binance Futures this is imply the symbol
    // and the position side.
    //
    // TODO: Does NOT belong in generic Binance crate.
    pub fn id(&self) -> (String, String) {
        (self.symbol.to_string(), self.position_side.to_string())
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct NewOrder {
    // Required fields.
    pub symbol: String,
    pub side: OrderSide,
    #[serde(rename = "type")]
    pub order_type: OrderType,

    // Not always required, and optional fields.
    #[serde(rename = "positionSide")]
    pub position_side: Option<PositionSide>,
    #[serde(serialize_with = "serialize_opt_f64")]
    pub quantity: Option<f64>,
    #[serde(serialize_with = "serialize_opt_f64")]
    pub price: Option<f64>,
    #[serde(rename = "timeInForce")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: Option<bool>,
    #[serde(rename = "stopPrice")]
    pub stop_price: Option<f64>,
    #[serde(rename = "newClientOrderId")]
    pub client_order_id: Option<String>,

    #[serde(rename = "closePosition")]
    pub close_position: Option<bool>,
}

impl NewOrder {
    pub fn new(symbol: &str, side: OrderSide, order_type: OrderType) -> Self {
        Self {
            symbol: symbol.to_uppercase(),
            side,
            order_type,
            position_side: None,
            quantity: None,
            price: None,
            time_in_force: None,
            reduce_only: None,
            stop_price: None,
            client_order_id: None,
            close_position: None,
        }
    }

    pub fn new_market_buy(symbol: &str, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Buy, OrderType::Market);
        order.quantity = Some(quantity);
        order
    }

    pub fn new_market_sell(symbol: &str, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Sell, OrderType::Market);
        order.quantity = Some(quantity);
        order
    }

    pub fn new_limit_buy(symbol: &str, price: f64, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Buy, OrderType::Limit);
        order.price = Some(price);
        order.quantity = Some(quantity);
        order.time_in_force = Some(TimeInForce::GTC);
        order
    }

    pub fn new_limit_sell(symbol: &str, price: f64, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Sell, OrderType::Limit);
        order.price = Some(price);
        if quantity > 0.0 {
            order.quantity = Some(quantity);
        }
        order.time_in_force = Some(TimeInForce::GTC);
        order
    }

    pub fn client_order_id(mut self, order_id: String) -> Self {
        self.client_order_id = Some(order_id);
        self
    }

    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = Some(true);
        self
    }

    pub fn post_only(mut self) -> Self {
        self.time_in_force = Some(TimeInForce::GTX);
        self
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderResponse {
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde(rename = "cumQty", deserialize_with = "parse_f64_string")]
    pub cum_qty: f64,
    #[serde(rename = "cumQuote", deserialize_with = "parse_f64_string")]
    pub cum_quote: f64,
    #[serde(rename = "executedQty", deserialize_with = "parse_f64_string")]
    pub executed_qty: f64,
    #[serde(rename = "orderId")]
    pub order_id: u64,
    #[serde(rename = "avgPrice", deserialize_with = "parse_f64_string")]
    pub avg_price: f64,
    #[serde(rename = "origQty", deserialize_with = "parse_f64_string")]
    pub orig_qty: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    pub side: String,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    pub status: String,
    #[serde(rename = "stopPrice", deserialize_with = "parse_f64_string")]
    pub stop_price: f64,
    #[serde(rename = "closePosition")]
    pub close_position: bool,
    pub symbol: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    #[serde(rename = "origType")]
    pub orig_type: String,
    #[serde(
        default,
        rename = "activatePrice",
        deserialize_with = "parse_opt_f64_string"
    )]
    pub activate_price: Option<f64>,
    #[serde(
        default,
        rename = "priceRate",
        deserialize_with = "parse_opt_f64_string"
    )]
    pub price_rate: Option<f64>,
    #[serde(rename = "updateTime")]
    pub update_time: u64,
    #[serde(rename = "workingType")]
    pub working_type: String,
    #[serde(rename = "priceProtect")]
    pub price_protect: bool,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_cancel_order_response() {
        let response_text = "\
            {\"orderId\":1544589222,\
             \"symbol\":\"TOMOUSDT\",\
             \"status\":\"CANCELED\",\
             \"clientOrderId\":\"electron_vIOn6GOGIi2nNsIkngt3\",\
             \"price\":\"2.4000\",\
             \"avgPrice\":\"0.0000\",\
             \"origQty\":\"10\",\
             \"executedQty\":\"0\",\
             \"cumQty\":\"0\",\
             \"cumQuote\":\"0\",\
             \"timeInForce\":\"GTC\",\
             \"type\":\"LIMIT\",\
             \"reduceOnly\":false,\
             \"closePosition\":false,\
             \"side\":\"BUY\",\
             \"positionSide\":\"LONG\",\
             \"stopPrice\":\"0\",\
             \"workingType\":\"CONTRACT_PRICE\",\
             \"priceProtect\":false,\
             \"origType\":\"LIMIT\",\
             \"updateTime\":1629929626599\
             }";
        let _response: OrderResponse = serde_json::from_str(response_text).unwrap();
    }
}
