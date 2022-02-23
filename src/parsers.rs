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

//! Helper parser/deserializes for decoding some Binance messages.

use serde::de::Error;
use serde::{Deserialize, Serializer};

pub fn parse_f64_string<'de, D>(d: D) -> Result<f64, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: String = serde::de::Deserialize::deserialize(d)?;
    s.parse::<f64>().map_err(D::Error::custom)
}

pub fn parse_opt_f64_string<'de, D>(d: D) -> Result<Option<f64>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(d)?;
    let val = s.parse::<f64>().map_err(D::Error::custom)?;
    Ok(Some(val))
}

pub fn parse_bool_string<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(d)?;
    s.parse::<bool>().map_err(D::Error::custom)
}

pub(crate) fn serialize_opt_f64<S>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        None => s.serialize_none(),
        Some(v) => s.serialize_str(&format!("{:09}", v)),
    }
}
