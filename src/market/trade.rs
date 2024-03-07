use std::collections::{BTreeMap, HashMap};

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    account::trade::OrderSide,
    exchange::types::ApiResult,
    utils::{
        number::parse_f64_from_lookup,
        time::{floor_mili_ts, generate_ts, timestamp_to_string, SEC_AS_MILI},
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TradeDataMeta {
    pub symbol: String,
    pub len: usize,
    pub last_update: u64,
}

impl TradeDataMeta {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            len: 0,
            last_update: generate_ts(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TradeData {
    pub meta: TradeDataMeta,
    trades: BTreeMap<(u64, OrderSide), Trade>,
}

impl TradeData {
    pub fn new(symbol: &str) -> Self {
        Self {
            meta: TradeDataMeta::new(symbol),
            trades: BTreeMap::new(),
        }
    }

    pub fn add_trade(&mut self, trade: &mut Trade) {
        // update meta
        self.meta.last_update = generate_ts();
        self.meta.len += 1;

        // ensure trade timestamp is floored to second
        trade.timestamp = floor_mili_ts(trade.timestamp, SEC_AS_MILI);
        let key = (trade.timestamp, trade.order_side);

        // aggregate trade qty and price if exists with same
        // ts and buy side
        if let Some(existing_trade) = self.trades.get_mut(&key) {
            existing_trade.qty += trade.qty;
            existing_trade.price = (existing_trade.price + trade.price) / 2.0;
        } else {
            self.trades.insert(key, trade.clone());
        }
    }

    pub fn trades(&self) -> Vec<Trade> {
        self.trades.values().cloned().collect()
    }

    pub fn drain_trades(&mut self, before_ts: u64) -> Vec<Trade> {
        // info!(
        //     "Removing all trades before {} ...",
        //     timestamp_to_string(before_ts)
        // );
        let mut trades = vec![];
        for trade in self.trades.values() {
            if trade.timestamp < before_ts {
                trades.push(trade.clone())
            }
        }
        self.trades.retain(|_k, v| v.timestamp >= before_ts);
        self.meta.len = self.trades.len();

        trades
    }
}

pub type MarketTradeId = Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Trade {
    pub symbol: String,
    pub timestamp: u64,
    pub qty: f64,
    pub price: f64,
    pub order_side: OrderSide,
}

impl Trade {
    pub fn floor_price(&self, to: f64) -> f64 {
        (self.price / to).floor() * 10.0
    }
    pub fn from_binance_lookup(lookup: HashMap<String, Value>) -> ApiResult<Self> {
        // {
        //     "e": "aggTrade",  // Event type
        //     "E": 123456789,   // Event time
        //     "s": "BTCUSDT",    // Symbol
        //     "a": 5933014,     // Aggregate trade ID
        //     "p": "0.001",     // Price
        //     "q": "100",       // Quantity
        //     "f": 100,         // First trade ID
        //     "l": 105,         // Last trade ID
        //     "T": 123456785,   // Trade time
        //     "m": true,        // Is the buyer the market maker?
        //   }

        let trade_time = lookup
            .get("T")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'T' key from data trade lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'T' key in data trade lookup".to_string()
            })?;
        let id = lookup
            .get("a")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'a' key from data trade lookup".to_string()
            })?
            .as_u64()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_u64' from 'a' key in data trade lookup".to_string()
            })?;

        let is_maker_buyer = lookup
            .get("m")
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Missing 'm' key from data kline lookup".to_string()
            })?
            .as_bool()
            .ok_or_else(|| {
                // Create an error message or construct an error type
                "Unable to 'as_bool' from 'm' key in data trade lookup".to_string()
            })?;

        let order_side = if is_maker_buyer {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };

        let qty = parse_f64_from_lookup("q", &lookup)?;
        let price = parse_f64_from_lookup("p", &lookup)?;

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

        Ok(Self {
            symbol: symbol.to_string(),
            timestamp: trade_time,
            qty,
            price,
            order_side,
        })
    }
}

impl Default for Trade {
    fn default() -> Self {
        Self {
            symbol: "default".to_string(),
            timestamp: generate_ts(),
            qty: 42.2,
            price: 42.2,
            order_side: OrderSide::Buy,
        }
    }
}
