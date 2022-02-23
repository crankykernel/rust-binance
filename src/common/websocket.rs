// Copyright (c) 2022 Cranky Kernel
//
// SPDX-License-Identifier: MIT

pub fn stream_name_trade(symbol: &str) -> String {
    format!("{}@trade", symbol.to_lowercase())
}

pub fn stream_name_aggtrade<S: AsRef<str>>(symbol: S) -> String {
    format!("{}@aggTrade", symbol.as_ref().to_lowercase())
}

pub fn stream_name_kline<S: AsRef<str>, I: AsRef<str>>(symbol: S, interval: I) -> String {
    format!(
        "{}@kline_{}",
        symbol.as_ref().to_lowercase(),
        interval.as_ref()
    )
}

pub fn stream_name_liquidation<S: AsRef<str>>(symbol: S) -> String {
    format!("{}@forceOrder", symbol.as_ref().to_lowercase())
}

pub fn stream_name_ticker<S: AsRef<str>>(symbol: S) -> String {
    format!("{}@ticker", symbol.as_ref().to_lowercase())
}
