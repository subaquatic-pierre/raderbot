use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    account::trade::OrderSide,
    exchange::types::ApiResult,
    utils::{number::parse_f64_from_lookup, time::generate_ts},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketTradeDataMeta {
    pub symbol: String,
    pub len: u64,
    pub last_update: u64,
}

impl MarketTradeDataMeta {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            len: 0,
            last_update: generate_ts(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketTradeData {
    pub meta: MarketTradeDataMeta,
    pub trades: Vec<MarketTrade>,
}

impl MarketTradeData {
    pub fn new(symbol: &str) -> Self {
        Self {
            meta: MarketTradeDataMeta::new(symbol),
            trades: vec![],
        }
    }

    pub fn add_trade(&mut self, trade: MarketTrade) -> bool {
        // get last trade
        if let Some(last) = self.trades.last() {
            // if last trade exists
            // replace with latest if trade exists with same open time
            if trade.id == last.id {
                let last_dx = self.trades.len() - 1;
                let _ = std::mem::replace(&mut self.trades[last_dx], trade);

                false
            } else {
                // add trade to end if open_time is not the same
                self.trades.push(trade);
                self.meta.len += 1;
                true
            }
        } else {
            // no trades in data, add new trade
            self.trades.push(trade);
            self.meta.len += 1;

            true
        }
    }

    pub fn clear_trades(&mut self) {
        self.trades = vec![];
        self.meta.len = 0;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketTrade {
    pub id: u64,
    pub symbol: String,
    pub timestamp: u64,
    pub qty: f64,
    pub price: f64,
    pub order_side: OrderSide,
}

impl MarketTrade {
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
        let symbol = symbol.replace("USDT", "-USDT");

        Ok(Self {
            id,
            symbol: symbol.to_string(),
            timestamp: trade_time,
            qty,
            price,
            order_side,
        })
    }
}

impl Default for MarketTrade {
    fn default() -> Self {
        Self {
            id: 1,
            symbol: "default".to_string(),
            timestamp: generate_ts(),
            qty: 42.2,
            price: 42.2,
            order_side: OrderSide::Buy,
        }
    }
}
