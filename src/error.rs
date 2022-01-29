// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::futures::client::ApiError;

#[derive(Error, Debug)]
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
