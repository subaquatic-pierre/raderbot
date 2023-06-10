use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use serde_json::Value;

use crate::{
    exchange::types::ApiResult,
    market::market::MarketDataSymbol,
    utils::{
        number::{generate_random_id, parse_f64_from_lookup},
        time::generate_ts,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerMeta {
    pub symbol: String,
    pub last_update: u64,
}

impl TickerMeta {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            last_update: generate_ts(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerData {
    pub meta: TickerMeta,
    pub ticker: Ticker,
}

impl TickerData {
    pub fn new(symbol: &str, ticker: Ticker) -> Self {
        Self {
            meta: TickerMeta::new(symbol),
            ticker,
        }
    }

    pub fn update_ticker(&mut self, ticker: Ticker, update_time: u64) {
        self.ticker = ticker;

        // increment len of tickers on meta
        self.meta.last_update = update_time;

        // return true ticker added
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ticker {
    pub time: u64,

    pub symbol: String,
    pub price_change: f64,
    pub percent_change: f64,
    pub high: f64,
    pub low: f64,
    pub traded_vol: f64,
    pub quote_vol: f64,
    pub last_price: f64,
    pub open_price: f64,
    pub open_time: u64,
    pub close_time: u64,
}

impl Ticker {
    pub fn from_binance_lookup(lookup: HashMap<String, Value>) -> ApiResult<Self> {
        let symbol = lookup
            .get("s")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 's' key from data ticker lookup".to_string()
            })?
            .as_str()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_str' from 's' key in data ticker lookup".to_string()
            })?;

        let time = lookup
            .get("E")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'E' key from data ticker lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'E' key in data ticker lookup".to_string()
            })?;

        let last_price = parse_f64_from_lookup("c", &lookup)?;
        let price_change = parse_f64_from_lookup("p", &lookup)?;
        let percent_change = parse_f64_from_lookup("P", &lookup)?;

        let high = parse_f64_from_lookup("h", &lookup)?;
        let low = parse_f64_from_lookup("l", &lookup)?;
        let open_price = parse_f64_from_lookup("o", &lookup)?;

        let traded_vol = parse_f64_from_lookup("v", &lookup)?;
        let quote_vol = parse_f64_from_lookup("q", &lookup)?;

        let open_time = lookup
            .get("O")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'O' key from data ticker lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'O' key in data ticker lookup".to_string()
            })?;
        let close_time = lookup
            .get("C")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'C' key from data ticker lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'C' key in data ticker lookup".to_string()
            })?;

        // {
        //     "l": String("26613.00000000"),
        //     "v": String("34270.86586000"),
        //     "x": String("27291.64000000"),
        //     "P": String("-2.182"),
        //     "s": String("BTCUSDT"),
        //     "h": String("27359.93000000"),
        //     "q": String("924029314.52493560"),
        //     "c": String("26696.12000000"),
        //     "E": Number(1684932971410),
        //     "A": String("13.52668000"),
        //     "Q": String("0.10673000"),
        //     "w": String("26962.53191558"),
        //     "o": String("27291.63000000"),
        //     "b": String("26696.12000000"),
        //     "F": Number(3124447613),
        //     "B": String("0.09550000"),
        //     "a": String("26696.13000000"),
        //     "e": String("24hrTicker"),
        //     "O": Number(1684846571410),
        //     "p": String("-595.51000000"),
        //     "C": Number(1684932971410),
        //     "L": Number(3125208169),
        //     "n": Number(760557)
        // }

        Ok(Self {
            time,
            symbol: symbol.to_string(),
            last_price,
            price_change,
            percent_change,
            open_price,
            high,
            low,
            traded_vol,
            quote_vol,
            open_time,
            close_time,
        })
    }

    pub fn from_bingx_lookup(lookup: HashMap<String, Value>) -> ApiResult<Self> {
        let data = lookup.get("data").ok_or_else(|| {
            // Create an error message or construct an error type
            "Missing 'data' key from data ticker lookup".to_string()
        })?;
        let data: HashMap<String, Value> = serde_json::from_value(data.to_owned()).unwrap();

        let symbol = data
            .get("symbol")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'symbol' key from data ticker lookup".to_string()
            })?
            .as_str()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_str' from 's' key in data ticker lookup".to_string()
            })?;

        let time = generate_ts();

        let last_price = parse_f64_from_lookup("lastPrice", &data)?;
        let price_change = parse_f64_from_lookup("priceChange", &data)?;
        let percent_change = parse_f64_from_lookup("priceChangePercent", &data)?;

        let high = parse_f64_from_lookup("highPrice", &data)?;
        let low = parse_f64_from_lookup("lowPrice", &data)?;
        let open_price = parse_f64_from_lookup("openPrice", &data)?;

        let traded_vol = parse_f64_from_lookup("volume", &data)?;
        let quote_vol = parse_f64_from_lookup("quoteVolume", &data)?;

        let open_time = data
            .get("openTime")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'openTime' key from data ticker lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'openTime' key in data ticker lookup".to_string()
            })?;
        let close_time = data
            .get("closeTime")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'closeTime' key from data ticker lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'closeTime' key in data ticker lookup".to_string()
            })?;

        // {
        //     "code": 0,
        //     "msg": "",
        //     "data": {
        //       "symbol": "BTC-USDT",
        //       "priceChange": "52.5",
        //       "priceChangePercent": "0.31",
        //       "lastPrice": "16880.5",
        //       "lastQty": "2.2238",
        //       "highPrice": "16897.5",
        //       "lowPrice": "16726.0",
        //       "volume": "245870.1692",
        //       "quoteVolume": "4151395117.73",
        //       "openPrice": "16832.0",
        //       "openTime": 1672026667803,
        //       "closeTime": 1672026648425
        //     }
        //   }

        Ok(Self {
            time,
            symbol: symbol.to_string(),
            last_price,
            price_change,
            percent_change,
            open_price,
            high,
            low,
            traded_vol,
            quote_vol,
            open_time,
            close_time,
        })
    }
}

impl Default for Ticker {
    fn default() -> Self {
        let price = generate_random_id() as f64 * 0.8;
        Self {
            time: 42,
            symbol: "BTCUSDT".to_string(),
            price_change: 42.2,
            percent_change: 42.2,
            last_price: price,
            open_price: 42.2,
            high: 42.2,
            low: 42.2,
            traded_vol: 42.2,
            quote_vol: 42.2,
            open_time: 42,
            close_time: 42,
        }
    }
}

impl MarketDataSymbol for Ticker {
    fn symbol(&self) -> String {
        self.symbol.to_string()
    }
}
