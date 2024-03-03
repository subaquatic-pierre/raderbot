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

/// Provides metadata for a ticker, including the symbol and the last update timestamp.
///
/// # Attributes
/// - `symbol`: A string representing the trading symbol of the ticker.
/// - `last_update`: A Unix timestamp (u64) indicating the last time the ticker was updated.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerMeta {
    pub symbol: String,
    pub last_update: u64,
}

impl TickerMeta {
    /// Constructs a new `TickerMeta` instance with the given symbol and current timestamp.
    ///
    /// # Parameters
    /// - `symbol`: A string slice that holds the symbol for which metadata is being created.

    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            last_update: generate_ts(),
        }
    }
}

/// Represents detailed data for a ticker, including its metadata and current state.
///
/// # Attributes
/// - `meta`: Metadata about the ticker including the symbol and last update time.
/// - `ticker`: The current state of the ticker including price, volume, and other trading information.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerData {
    pub meta: TickerMeta,
    pub ticker: Ticker,
}

impl TickerData {
    /// Creates a new `TickerData` instance for a given symbol and its ticker information.
    ///
    /// # Parameters
    /// - `symbol`: A string slice that holds the symbol for the ticker.
    /// - `ticker`: A `Ticker` instance containing the current state of the ticker.

    pub fn new(symbol: &str, ticker: Ticker) -> Self {
        Self {
            meta: TickerMeta::new(symbol),
            ticker,
        }
    }

    /// Updates the ticker with new data and updates the last update timestamp.
    ///
    /// # Parameters
    /// - `ticker`: A `Ticker` instance containing the new state of the ticker.
    /// - `update_time`: The Unix timestamp (u64) at which the ticker is being updated.

    pub fn update_ticker(&mut self, ticker: Ticker, update_time: u64) {
        self.ticker = ticker;

        // increment len of tickers on meta
        self.meta.last_update = update_time;

        // return true ticker added
    }
}

/// Represents the current state of a market ticker, including price information and volume.
///
/// # Attributes
/// - `time`: A Unix timestamp (u64) indicating when the ticker data was published.
/// - `symbol`: The trading symbol associated with the ticker.
/// - `price_change`: The absolute change in price since the last update.
/// - `percent_change`: The percentage change in price since the last update.
/// - `high`: The highest price at which the ticker traded during the period.
/// - `low`: The lowest price at which the ticker traded during the period.
/// - `traded_vol`: The total volume traded in the period.
/// - `quote_vol`: The total quote volume traded in the period.
/// - `last_price`: The last traded price for the ticker.
/// - `open_price`: The price at which the ticker opened during the period.
/// - `open_time`: The start time of the period for which the ticker is reported.
/// - `close_time`: The end time of the period for which the ticker is reported.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ticker {
    pub time: u64,
    pub symbol: String,
    pub high: f64,
    pub low: f64,
    pub traded_vol: f64,
    pub last_price: f64,
    pub open_price: f64,
}

impl Ticker {
    /// Constructs a `Ticker` instance by extracting relevant information from a Binance API response.
    ///
    /// # Parameters
    /// - `lookup`: A hashmap containing the raw ticker data from the Binance API response.

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
            open_price,
            high,
            low,
            traded_vol,
        })
    }

    /// Constructs a `Ticker` instance by extracting relevant information from a BingX API response.
    ///
    /// # Parameters
    /// - `data`: A hashmap containing the raw ticker data from the BingX API response.

    pub fn from_bingx_lookup(data: HashMap<String, Value>) -> ApiResult<Self> {
        //  {
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
        //  }

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

        Ok(Self {
            time,
            symbol: symbol.to_string(),
            last_price,
            open_price,
            high,
            low,
            traded_vol,
        })
    }
}

/// Provides a default instance of a `Ticker` with placeholder values.
///
/// This implementation is primarily for testing or when a default value is necessary before real data is available.

impl Default for Ticker {
    fn default() -> Self {
        let price = generate_random_id() as f64 * 0.8;
        Self {
            time: 42,
            symbol: "BTCUSDT".to_string(),
            last_price: price,
            open_price: 42.2,
            high: 42.2,
            low: 42.2,
            traded_vol: 42.2,
        }
    }
}

/// Implements the `MarketDataSymbol` trait for `Ticker`, allowing retrieval of the ticker's symbol as a string.
impl MarketDataSymbol for Ticker {
    fn symbol(&self) -> String {
        self.symbol.to_string()
    }
}
