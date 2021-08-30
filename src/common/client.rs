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

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use hmac::{Mac, NewMac};
use serde::Deserialize;

use crate::futures::client::ApiError;

#[derive(Clone)]
pub struct Authentication {
    pub api_secret: String,
    pub api_key: String,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Raw serde_json error.
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    /// A serde_json::Error with the text that failed.
    #[error("json parse error: {error}")]
    Decode {
        error: serde_json::Error,
        text: String,
    },

    /// Wrap errorrs that were generated with anyhow.
    #[error("anyhow: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// Error from the reqwuest http client library.
    #[error("reqwest: {0}")]
    Request(#[from] reqwest::Error),

    /// Errors from the URL encoding library.
    #[error("urlencode: {0}")]
    UrlEncode(#[from] serde_urlencoded::ser::Error),

    /// An error sent from the remote API in response to a request.
    #[error("api: {0}")]
    ApiError(#[from] ApiError),

    #[error("url: {0}")]
    UrlError(String),
}

#[derive(Clone)]
pub struct Client {
    auth: Option<Authentication>,
    pub client: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new<S: Into<String>>(base_url: S, authentication: Option<Authentication>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
            auth: authentication,
        }
    }

    pub fn compute_signature(&self, request_body: &str) -> Result<String> {
        let mut macr = hmac::Hmac::<sha2::Sha256>::new_varkey(
            self.auth.as_ref().unwrap().api_secret.as_bytes(),
        )
        .map_err(|err| anyhow!("hmac: {:?}", err))?;
        macr.update(request_body.as_bytes());
        let signature = macr.finalize();
        Ok(format!("{:x}", signature.into_bytes()))
    }

    pub fn headers(&self) -> Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(auth) = &self.auth {
            headers.insert("X-MBX-APIKEY", auth.api_key.parse()?);
        }
        headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
        Ok(headers)
    }

    /// Create a full URL from the base_url and the endpoint.
    pub fn url(&self, endpoint: &str) -> Result<String, Error> {
        Ok(self.url2(endpoint, None)?.to_string())
    }

    /// Create a URL from an end-point with an optional query string.
    pub fn url2(&self, endpoint: &str, query_string: Option<&str>) -> Result<reqwest::Url, Error> {
        let mut url = reqwest::Url::parse(&format!("{}{}", self.base_url, endpoint))
            .map_err(|e| Error::UrlError(e.to_string()))?;
        url.set_query(query_string);
        Ok(url)
    }

    pub fn sign_form(&self, form: Option<&str>) -> Result<String> {
        let form = form.unwrap_or("");
        let timestamp = self.get_timestamp();
        let form = format!("{}&recvWindow=1000&timestamp={}", form, timestamp);
        let signature = self.compute_signature(&form)?;
        let form = format!("{}&signature={}", &form, &signature);
        Ok(form)
    }

    fn get_timestamp(&self) -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    pub fn get<E: AsRef<str>>(
        &self,
        endpoint: E,
        query_string: Option<E>,
    ) -> reqwest::RequestBuilder {
        let url = if let Some(query_string) = query_string {
            format!(
                "{}?{}",
                self.url(endpoint.as_ref()).unwrap(),
                query_string.as_ref()
            )
        } else {
            self.url(endpoint.as_ref()).unwrap()
        };
        self.client.get(&url)
    }

    pub fn get3(
        &self,
        endpoint: &str,
        query_string: Option<&str>,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let mut url = reqwest::Url::parse(&format!("{}{}", self.base_url, endpoint))?;
        url.set_query(query_string);
        Ok(self.client.get(url))
    }

    pub fn delete(&self, endpoint: &str, query_string: Option<&str>) -> reqwest::RequestBuilder {
        self.client
            .delete(self.url2(endpoint, query_string).unwrap())
    }

    pub async fn post_listenkey(&self, endpoint: &str) -> Result<ListenKeyResponse, Error> {
        let response = self
            .client
            .post(&self.url(endpoint)?)
            .headers(self.headers()?)
            .send()
            .await?;
        let response_body = response.text().await?;
        Ok(serde_json::from_str(&response_body)?)
    }

    pub async fn put_listenkey(&self, endpoint: &str) -> Result<ListenKeyResponse, Error> {
        let timestamp = self.get_timestamp();
        let form = format!("timestamp={}", timestamp);
        let form = format!("{}&signature={}", &form, self.compute_signature(&form)?);
        let response = self
            .client
            .post(self.url(endpoint)?)
            .body(form)
            .headers(self.headers()?)
            .send()
            .await?;
        let response_body = response.text().await?;
        Ok(serde_json::from_str(&response_body)?)
    }
}

#[derive(Deserialize, Debug)]
pub struct ListenKeyResponse {
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}
