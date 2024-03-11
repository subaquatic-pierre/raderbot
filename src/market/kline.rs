// #![feature(btree_extract_if)]
use std::collections::{BTreeMap, HashMap};

use log::info;
use mongodb::{
    bson::{self, doc, to_document},
    IndexModel,
};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::Value;

use uuid::Uuid;

use crate::{
    account::trade::OrderSide,
    exchange::types::ApiResult,
    market::{interval::Interval, market::MarketDataSymbol},
    utils::{
        number::parse_f64_from_lookup,
        time::{calculate_kline_open_time, generate_ts, timestamp_to_string},
    },
};

use super::trade::Trade;

/// Represents metadata for a series of klines, including the symbol, interval, length, and last update timestamp.
///
/// This struct is used to track the metadata associated with a collection of kline data for a specific
/// trading pair and interval.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlineMeta {
    pub symbol: String,
    pub interval: Interval,
    pub len: usize,
    pub last_update: u64,
}

impl KlineMeta {
    pub fn new(symbol: &str, interval: Interval) -> Self {
        Self {
            symbol: symbol.to_string(),
            interval: interval,
            len: 0,
            last_update: generate_ts(),
        }
    }
}

/// Contains kline data and associated metadata for a specific trading pair and interval.
///
/// This struct is designed to hold a collection of klines along with their metadata. It provides methods
/// to add new klines to the collection, clear existing klines, and manipulate the kline data.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KlineData {
    pub meta: KlineMeta,
    klines: BTreeMap<u64, Kline>,
}

impl KlineData {
    /// Creates a new `KlineData` instance for a specific symbol and interval.
    ///
    /// This method initializes a `KlineData` object with empty kline data and associated metadata.

    pub fn new(symbol: &str, interval: Interval) -> Self {
        Self {
            meta: KlineMeta::new(symbol, interval),
            klines: BTreeMap::new(),
        }
    }

    /// Adds a kline to the data set, ensuring chronological order and uniqueness based on open time.
    ///
    /// This method adds a new kline to the collection, replacing any existing kline with the same open time.
    pub fn add_kline(&mut self, kline: Kline) {
        self.meta.last_update = generate_ts();

        self.klines.insert(kline.open_time, kline.clone());
        self.meta.len = self.klines.len();
    }

    pub fn klines(&self) -> Vec<Kline> {
        self.klines.values().cloned().collect()
    }

    pub fn drain_klines(&mut self, before_ts: u64) -> Vec<Kline> {
        // info!(
        //     "Removing all klines before {} ...",
        //     timestamp_to_string(before_ts)
        // );
        let mut klines = vec![];
        for kline in self.klines.values() {
            if kline.open_time < before_ts {
                klines.push(kline.clone())
            }
        }
        self.klines.retain(|k, _v| k >= &before_ts);
        self.meta.len = self.klines.len();

        klines
    }
}

/// Represents a single kline or candlestick data point, including open, high, low, close, and volume information.
///
/// This struct is the fundamental data structure for representing a single kline or candlestick in market data.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kline {
    pub symbol: String,
    pub interval: Interval,
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
            interval: Interval::Min1,
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
    /// Constructs a kline from a lookup hashmap containing kline data from Binance.
    ///
    /// This method is responsible for parsing the kline data provided by Binance's API and constructing a `Kline` instance.

    pub fn make_trades(&self) -> Vec<Trade> {
        let trade_sell = Trade {
            symbol: self.symbol.to_string(),
            timestamp: self.close_time,
            qty: self.volume / 2.0,
            price: self.close,
            order_side: OrderSide::Sell,
        };
        let trade_buy = Trade {
            symbol: self.symbol.to_string(),
            timestamp: self.close_time,
            qty: self.volume / 2.0,
            price: self.close,
            order_side: OrderSide::Buy,
        };
        vec![trade_buy, trade_sell]
    }

    pub fn floor_price(&self, to: f64) -> f64 {
        (self.close / to).floor() * 10.0
    }

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
            interval: interval.try_into()?,
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

    /// Constructs a kline from a lookup hashmap containing kline data from BingX.
    ///
    /// This method is similar to `from_binance_lookup` but tailored for parsing kline data specific to BingX's API.

    pub fn from_bingx_lookup(
        data: HashMap<String, Value>,
        symbol: &str,
        interval: Interval,
    ) -> ApiResult<Self> {
        // {
        //     "open": "float64",
        //     "close": "float64",
        //     "high": "float64",
        //     "low": "float64",
        //     "volume": "float64",
        //     "time": "int64"
        //   }

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
            interval: interval,
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

    /// Constructs a kline from a websocket lookup hashmap containing kline data from BingX.
    ///
    /// This method is designed to parse kline data received from BingX's websocket feed and construct a `Kline` instance.

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

        let open_time = calculate_kline_open_time(close_time, interval.clone().try_into()?);

        let open = parse_f64_from_lookup("o", &data)?;
        let close = parse_f64_from_lookup("c", &data)?;

        let high = parse_f64_from_lookup("h", &data)?;
        let low = parse_f64_from_lookup("l", &data)?;

        let volume = parse_f64_from_lookup("v", &data)?;

        Ok(Self {
            interval: interval.try_into()?,
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

/// Provides an interface for a market data symbol, allowing retrieval of the symbol string.
///
/// This trait defines a common interface for any data structure that represents market data and includes a trading pair symbol.

impl MarketDataSymbol for Kline {
    fn symbol(&self) -> String {
        self.symbol.to_string()
    }
}

/// Represents a kline data structure specifically formatted to match Binance's API response.
///
/// This struct is tailored to match the kline data format returned by Binance's API, including additional fields like quote volume and trade count.

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
