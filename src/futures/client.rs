// SPDX-License-Identifier: MIT
//
// Copyright (C) 2021-2022 Cranky Kernel

use std::collections::HashMap;
use std::fmt::Formatter;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Mac, NewMac};
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
    auth: Option<Authentication>,
    client: crate::common::client::Client,
}

impl Client {
    pub fn new(authentication: Option<Authentication>) -> Self {
        Self {
            auth: authentication.clone(),
            client: crate::common::client::Client::new(API_ROOT, authentication),
        }
    }

    pub fn compute_signature(&self, request_body: &str) -> anyhow::Result<String> {
        let mut macr = hmac::Hmac::<sha2::Sha256>::new_varkey(
            self.auth.as_ref().unwrap().api_secret.as_bytes(),
        )
        .map_err(|err| anyhow::anyhow!("hmac: {:?}", err))?;
        macr.update(request_body.as_bytes());
        let signature = macr.finalize();
        Ok(format!("{:x}", signature.into_bytes()))
    }

    pub fn sign_form(&self, form: Option<&str>) -> anyhow::Result<String> {
        let form = form.unwrap_or("");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let form = format!("{}&recvWindow=1000&timestamp={}", form, timestamp);
        let signature = self.compute_signature(&form)?;
        let form = format!("{}&signature={}", &form, &signature);
        Ok(form)
    }

    pub fn headers(&self) -> anyhow::Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(auth) = &self.auth {
            headers.insert("X-MBX-APIKEY", auth.api_key.parse()?);
        }
        headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
        Ok(headers)
    }

    /// Public (unauthenticated) get.
    pub async fn get<F, T>(&self, endpoint: &str, query_string: F) -> Result<T, Error>
    where
        F: Serialize,
        T: DeserializeOwned,
    {
        let url = format!("{}{}", API_ROOT, endpoint);
        let response = self
            .client
            .client
            .get(url)
            .query(&query_string)
            .send()
            .await?;
        let code = response.status();
        let body = response.text().await?;
        self.decode_response(code, &body)
    }

    /// Private/user (authenticated) get.
    pub async fn authenticated_get<T: DeserializeOwned, F: Serialize>(
        &self,
        endpoint: &str,
        form: F,
    ) -> Result<T, Error> {
        let form = serde_urlencoded::to_string(form)?;
        let form = self.sign_form(Some(&form))?;
        let url = format!("{}{}?{}", API_ROOT, endpoint, &form);
        let request = self.client.client.get(url).headers(self.headers()?);
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

    pub async fn get_open_orders<
        S: AsRef<str> + Serialize + std::fmt::Display + std::fmt::Debug,
    >(
        &self,
        symbol: Option<S>,
    ) -> Result<Vec<OpenOrder>, Error> {
        let endpoint = "/fapi/v1/openOrders";
        let form = vec![("symbol", symbol)];
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

    pub async fn cancel_all_open_orders(&self, symbol: &str) -> Result<serde_json::Value, Error> {
        let endpoint = "/fapi/v1/allOpenOrders";
        let form = serde_urlencoded::to_string(&[("symbol", symbol)])?;
        let form = self.client.sign_form(Some(&form))?;
        let url = self.client.url2(endpoint, Some(&form))?;
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

    pub async fn get_klines<S: AsRef<str>, I: AsRef<str>>(
        &self,
        symbol: S,
        interval: I,
        limit: Option<u16>,
    ) -> Result<Vec<Kline>, Error> {
        let endpoint = "/fapi/v1/klines";
        let mut form = vec![
            ("symbol", symbol.as_ref().to_string()),
            ("interval", interval.as_ref().to_string()),
        ];
        if let Some(limit) = limit {
            form.push(("limit", limit.to_string()));
        }
        self.get(endpoint, form).await
    }

    pub async fn get_exchange_info(&self) -> Result<ExchangeInfoResponse, Error> {
        let endpoint = "/fapi/v1/exchangeInfo";
        self.get(endpoint, ()).await
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

    pub async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<PositionEntry>, Error> {
        let endpoint = "/fapi/v2/positionRisk";
        let form = vec![("symbol", symbol)];
        self.authenticated_get(endpoint, form).await
    }

    pub async fn get_account_info(&self) -> Result<Account, Error> {
        let endpoint = "/fapi/v2/account";
        self.authenticated_get(endpoint, ()).await
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

    pub async fn get_position_mode(&self) -> Result<PositionModeResponse, Error> {
        self.authenticated_get("/fapi/v1/positionSide/dual", ())
            .await
    }

    pub async fn is_hedge_mode(&self) -> Result<bool, Error> {
        let response = self.get_position_mode().await?;
        Ok(response.dual_side_position)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PositionModeResponse {
    #[serde(rename = "dualSidePosition")]
    pub dual_side_position: bool,
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

#[derive(Deserialize, Debug, Clone, PartialEq)]
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

    /// Base volume.
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

#[derive(Serialize, Debug, Clone, Default)]
pub struct NewOrder {
    /// Symbol
    pub symbol: Option<String>,

    /// Order side
    pub side: Option<OrderSide>,

    /// Order type
    #[serde(rename = "type")]
    pub order_type: Option<OrderType>,

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
    pub fn new<S: AsRef<str>>(symbol: S, side: OrderSide, order_type: OrderType) -> Self {
        Self {
            symbol: Some(symbol.as_ref().to_uppercase()),
            side: Some(side),
            order_type: Some(order_type),
            ..Default::default()
        }
    }

    pub fn new_market_buy<S: AsRef<str>>(symbol: S, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Buy, OrderType::Market);
        order.quantity = Some(quantity);
        order
    }

    pub fn new_market_sell<S: AsRef<str>>(symbol: S, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Sell, OrderType::Market);
        order.quantity = Some(quantity);
        order
    }

    pub fn new_limit_buy<S: AsRef<str>>(symbol: S, price: f64, quantity: f64) -> Self {
        let mut order = Self::new(symbol, OrderSide::Buy, OrderType::Limit);
        order.price = Some(price);
        order.quantity = Some(quantity);
        order.time_in_force = Some(TimeInForce::GTC);
        order
    }

    pub fn new_limit_sell<S: AsRef<str>>(symbol: S, price: f64, quantity: f64) -> Self {
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

    pub fn close_position(mut self) -> Self {
        self.close_position = Some(true);
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

#[derive(Deserialize, Debug, Clone)]
pub struct Account {
    #[serde(rename = "totalMarginBalance", deserialize_with = "parse_f64_string")]
    pub total_margin_balance: f64,
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
