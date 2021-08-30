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

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Interval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "3m")]
    ThreeMinute,
    #[serde(rename = "5m")]
    FiveMinute,
    #[serde(rename = "15m")]
    FifteenMinute,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHour,

    // For other values...
    Other(String),
}

impl Interval {
    pub fn from_str_non_strict<S: AsRef<str>>(s: S) -> Self {
        match s.as_ref() {
            "1m" => Self::OneMinute,
            "3m" => Self::ThreeMinute,
            "5m" => Self::FiveMinute,
            "15m" => Self::FifteenMinute,
            "1h" => Self::OneHour,
            "4h" => Self::FourHour,
            _ => Self::Other(s.as_ref().to_string()),
        }
    }

    pub fn to_seconds(&self) -> u64 {
        match self {
            Self::OneMinute => 60,
            Self::ThreeMinute => 60 * 3,
            Self::FiveMinute => 60 * 5,
            Self::FifteenMinute => 60 * 15,
            Self::OneHour => 60 * 60,
            Self::FourHour => 60 * 60 * 4,

            // Should probably error?
            Self::Other(_) => 0,
        }
    }

    pub fn to_millis(&self) -> u64 {
        self.to_seconds() * 1000
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Interval::OneMinute => "1m",
            Interval::ThreeMinute => "3m",
            Interval::FiveMinute => "5m",
            Interval::FifteenMinute => "15m",
            Interval::OneHour => "1h",
            Interval::FourHour => "4h",
            Interval::Other(s) => s,
        };
        write!(f, "{}", v)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_interval() {
        assert_eq!(Interval::from_str_non_strict("1m"), Interval::OneMinute);
        assert_eq!(format!("{}", Interval::OneMinute), "1m");

        assert_eq!(format!("{}", Interval::Other("1.5m".to_string())), "1.5m");
    }
}
