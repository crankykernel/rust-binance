// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::common;
use crate::common::client::{Authentication, ListenKeyResponse};
use crate::error::Error;
use crate::futures::client::ApiError;
use crate::parsers::*;

pub const API_ROOT: &str = "https://api.binance.com";

#[derive(Clone)]
pub struct Client {
    client: crate::common::client::Client,
}

impl Client {
    pub fn new(authentication: Option<Authentication>) -> Self {
        Self {
            client: common::client::Client::new(API_ROOT, authentication),
        }
    }

    pub async fn get_account(&self) -> anyhow::Result<AccountResponse> {
        let endpoint = "/api/v3/account";
        let form: Vec<(&str, &str)> = vec![];
        let form = build_form(&form[..]);
        let response = self.authenticated_get(endpoint, &form).await?;
        Ok(response)
    }

    pub async fn post_order(&self, order: &OrderRequest) -> Result<OrderResponse, Error> {
        let endpoint = "/api/v3/order";
        let form = serde_urlencoded::to_string(&order)?;
        let form = self.client.sign_form(Some(&form))?;
        let url = self.client.url2(endpoint, None)?;
        let request = self
            .client
            .client
            .post(url)
            .headers(self.client.headers()?)
            .body(form);
        let response = request.send().await?;
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

    pub async fn get_ticker_price(&self) -> Result<Vec<TickerPriceEntry>, Error> {
        let endpoint = "/api/v3/ticker/price";
        self.get(endpoint, None).await
    }

    pub async fn get_exchange_info(&self) -> Result<ExchangeInfoResponse, Error> {
        let endpoint = "/api/v3/exchangeInfo";
        self.get(endpoint, None).await
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

    pub async fn post_listenkey(&self) -> Result<ListenKeyResponse, Error> {
        let endpoint = "/api/v3/userDataStream";
        self.client.post_listenkey(endpoint).await
    }

    pub async fn put_listenkey(&self) -> Result<ListenKeyResponse, Error> {
        let endpoint = "/api/v1/listenKey";
        self.client.put_listenkey(endpoint).await
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

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct ExchangeInfoResponse {
    pub rateLimits: Vec<serde_json::Value>,
    pub exchangeFilters: Vec<serde_json::Value>,
    pub symbols: Vec<SymbolInfo>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct SymbolInfo {
    pub symbol: String,
    #[serde(rename = "baseAssetPrecision")]
    pub base_asset_precision: u64,
    #[serde(rename = "quoteAsset")]
    pub quote_asset: String,
    #[serde(rename = "quoteAssetPrecision")]
    pub quote_asset_precision: Option<u64>,
    pub filters: Vec<SymbolFilter>,
    pub status: String,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl SymbolInfo {
    pub fn get_filter(&self, filter_name: &str) -> Option<&SymbolFilter> {
        self.filters.iter().find(|s| s.filterType == filter_name)
    }

    pub fn get_lot_size_filter(&self) -> Option<&SymbolFilter> {
        self.get_filter("LOT_SIZE")
    }

    pub fn is_trading(&self) -> bool {
        self.status == "TRADING"
    }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct SymbolFilter {
    pub filterType: String,

    // PRICE_FILTER
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub minPrice: Option<f64>,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub maxPrice: Option<f64>,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub tickSize: Option<f64>,

    // LOT_SIZE
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub minQty: Option<f64>,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub maxQty: Option<f64>,
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub stepSize: Option<f64>,

    // MIN_NOTIONAL
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub minNotional: Option<f64>,

    // For futures the field name is `notional` in the `MIN_NOTIONAL` filter.
    #[serde(default, deserialize_with = "parse_opt_f64_string")]
    pub notional: Option<f64>,

    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct TickerPriceEntry {
    pub symbol: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
}

#[derive(Serialize, Debug)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    #[serde(rename = "quoteOrderQty")]
    pub quote_order_qty: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
pub enum OrderSide {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Clone)]
pub enum OrderType {
    #[serde(rename = "MARKET")]
    Market,
    #[serde(rename = "LIMIT")]
    Limit,

    // Futures
    STOP,
    STOP_MARKET,
    TAKE_PROFIT,
    TAKE_PROFIT_MARKET,
    TRAILING_STOP_MARKET,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct AccountResponse {
    pub canTrade: bool,
    pub balances: Vec<BalanceEntry>,
}

#[derive(Deserialize, Debug)]
pub struct BalanceEntry {
    pub asset: String,
    #[serde(deserialize_with = "parse_f64_string")]
    pub free: f64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub locked: f64,
}

impl BalanceEntry {
    pub fn total(&self) -> f64 {
        self.free + self.locked
    }
}

#[derive(Deserialize, Debug)]
pub struct OrderResponse {
    pub symbol: String,
    #[serde(rename = "orderId")]
    pub order_id: u64,
    #[serde(rename = "orderListId")]
    pub order_list_id: i64,
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde(rename = "transactTime")]
    pub transact_time: u64,
    #[serde(deserialize_with = "parse_f64_string")]
    pub price: f64,
    #[serde(rename = "origQty", deserialize_with = "parse_f64_string")]
    pub orig_qty: f64,
    #[serde(rename = "executedQty", deserialize_with = "parse_f64_string")]
    pub executed_qty: f64,
    #[serde(rename = "cummulativeQuoteQty", deserialize_with = "parse_f64_string")]
    pub cummulative_quote_qty: f64,
    pub status: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub fills: Vec<serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_order_response_success() {
        let _response_text = "{\
            \"symbol\":\"BNBUSDT\",\
            \"orderId\":2946045072,\
            \"orderListId\":-1,\
            \"clientOrderId\":\"6nIs4NPTXkOcO0LbJ6A8Ol\",\
            \"transactTime\":1630366113477,\
            \"price\":\"0.00000000\",\
            \"origQty\":\"0.03200000\",\
            \"executedQty\":\"0.03200000\",\
            \"cummulativeQuoteQty\":\"14.86080000\",\
            \"status\":\"FILLED\",\
            \"timeInForce\":\"GTC\",\
            \"type\":\"MARKET\",\
            \"side\":\"BUY\",\
            \"fills\":[\
                {\"price\":\"464.40000000\",\
                \"qty\":\"0.03200000\",\
                \"commission\":\"0.00002400\",\
                \"commissionAsset\":\"BNB\",\
                \"tradeId\":398890750}]}";
        let _response: OrderResponse = serde_json::from_str(_response_text).unwrap();
    }
}
