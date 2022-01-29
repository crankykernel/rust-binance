// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use hmac::{Mac, NewMac};
use serde::Deserialize;

use crate::error::Error;

#[derive(Clone)]
pub struct Authentication {
    pub api_secret: String,
    pub api_key: String,
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
