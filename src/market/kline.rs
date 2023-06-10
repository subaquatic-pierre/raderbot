use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    exchange::types::ApiResult,
    market::market::MarketDataSymbol,
    utils::{
        number::parse_f64_from_lookup,
        time::{calculate_kline_open_time, generate_ts},
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlineMeta {
    pub symbol: String,
    pub interval: String,
    pub len: u64,
    pub last_update: u64,
}

impl KlineMeta {
    pub fn new(symbol: &str, interval: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            len: 0,
            last_update: generate_ts(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlineData {
    pub meta: KlineMeta,
    pub klines: Vec<Kline>,
}

impl KlineData {
    pub fn new(symbol: &str, interval: &str) -> Self {
        Self {
            meta: KlineMeta::new(symbol, interval),
            klines: vec![],
        }
    }

    pub fn add_kline(&mut self, kline: Kline) -> bool {
        // get last kline
        if let Some(last) = self.klines.last() {
            // if last kline exists
            // replace with latest if kline exists with same open time
            if kline.open_time == last.open_time {
                let last_dx = self.klines.len() - 1;
                let _ = std::mem::replace(&mut self.klines[last_dx], kline);

                false
            } else {
                // add kline to end if open_time is not the same
                self.klines.push(kline);
                self.meta.len += 1;
                true
            }
        } else {
            // no klines in data, add new kline
            self.klines.push(kline);
            self.meta.len += 1;

            true
        }
    }

    pub fn clear_klines(&mut self) {
        self.klines = vec![];
        self.meta.len = 0;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Kline {
    pub symbol: String,
    pub interval: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub open_time: u64,
    pub close_time: u64,
}

impl Default for Kline {
    fn default() -> Self {
        Self {
            interval: "UNkown".to_string(),
            symbol: "Unknown".to_string(),
            open_time: 42,
            open: 42.2,
            high: 42.2,
            low: 42.2,
            close: 42.2,
            volume: 42.2,
            close_time: 42,
        }
    }
}

impl Kline {
    pub fn from_binance_lookup(lookup: HashMap<String, Value>) -> ApiResult<Self> {
        let _kline = lookup.get("k").ok_or_else(|| {
            // Create an error message or construct an error type
            "Missing 'k' key from data kline lookup".to_string()
        })?;
        let _kline: HashMap<String, Value> = serde_json::from_value(_kline.to_owned())?;

        let interval = _kline
            .get("i")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'i' key from data kline lookup".to_string()
            })?
            .as_str()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_str' from 'i' key in data kline lookup".to_string()
            })?;

        let symbol = lookup
            .get("s")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 's' key from data kline lookup".to_string()
            })?
            .as_str()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_str' from 's' key in data kline lookup".to_string()
            })?;

        let open_time = _kline
            .get("t")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 't' key from data kline lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 't' key in data kline lookup".to_string()
            })?;

        let close_time = _kline
            .get("T")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'T' key from data kline lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'T' key in data kline lookup".to_string()
            })?;

        let open = parse_f64_from_lookup("o", &_kline)?;
        let close = parse_f64_from_lookup("c", &_kline)?;

        let high = parse_f64_from_lookup("h", &_kline)?;
        let low = parse_f64_from_lookup("l", &_kline)?;

        let volume = parse_f64_from_lookup("v", &_kline)?;

        Ok(Self {
            interval: interval.to_string(),
            symbol: symbol.to_string(),
            open_time,
            open,
            high,
            low,
            close,
            volume,
            close_time,
        })
    }

    pub fn from_bingx_lookup(
        lookup: HashMap<String, Value>,
        symbol: &str,
        interval: &str,
    ) -> ApiResult<Self> {
        // {
        //     "open": "float64",
        //     "close": "float64",
        //     "high": "float64",
        //     "low": "float64",
        //     "volume": "float64",
        //     "time": "int64"
        //   }

        let data = lookup.get("data").ok_or_else(|| {
            // Create an error message or construct an error type
            "Missing 'data' key from data kline lookup".to_string()
        })?;
        let data: HashMap<String, Value> = serde_json::from_value(data.to_owned())?;

        let close_time = data
            .get("time")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'time' key from data kline lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to parse as u64".to_string()
            })?;

        let open_time = calculate_kline_open_time(close_time, interval);

        let open = parse_f64_from_lookup("open", &data)?;
        let close = parse_f64_from_lookup("close", &data)?;

        let high = parse_f64_from_lookup("high", &data)?;
        let low = parse_f64_from_lookup("low", &data)?;

        let volume = parse_f64_from_lookup("volume", &data)?;

        Ok(Self {
            interval: interval.to_string(),
            symbol: symbol.to_string(),
            open_time,
            open,
            high,
            low,
            close,
            volume,
            close_time,
        })
    }

    pub fn from_bingx_lookup_ws(lookup: HashMap<String, Value>) -> ApiResult<Self> {
        // {
        //     "code": 0,
        //     "data": {
        //       "T": 1649832779999,  //k line time
        //       "c": "54564.31",
        //       "h": "54711.73",
        //       "l": "54418.27",
        //       "o": "54577.41",
        //       "v": "1607.0727000000002"
        //     },
        //     "s": "BTC-USDT" //trading pair
        //     "dataType": "BTC-USDT@kline_1m"
        //   }
        let data: HashMap<String, Value> =
            serde_json::from_value(lookup.get("data").unwrap().to_owned()).unwrap();

        let data_type = data.get("dataType").unwrap().as_str().unwrap();
        // BTC-USDT@kline_1m
        let split = data_type.split('_');
        let interval = split.last().unwrap().to_string();
        let symbol = lookup.get("s").unwrap().as_str().unwrap();

        let close_time = data.get("T").unwrap().as_u64().unwrap();

        let open_time = calculate_kline_open_time(close_time, &interval);

        let open = parse_f64_from_lookup("o", &data)?;
        let close = parse_f64_from_lookup("c", &data)?;

        let high = parse_f64_from_lookup("h", &data)?;
        let low = parse_f64_from_lookup("l", &data)?;

        let volume = parse_f64_from_lookup("v", &data)?;

        Ok(Self {
            interval,
            symbol: symbol.to_string(),
            open_time,
            open,
            high,
            low,
            close,
            volume,
            close_time,
        })
    }
}

impl MarketDataSymbol for Kline {
    fn symbol(&self) -> String {
        self.symbol.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BinanceKline {
    pub open_time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub close_time: u64,
    pub quote_volume: f64,
    pub count: u64,
    pub taker_buy_volume: f64,
    pub taker_buy_quote_volume: f64,
    pub ignore: u8,
}
