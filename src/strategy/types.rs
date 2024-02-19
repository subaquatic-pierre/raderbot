use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{
    account::trade::OrderSide,
    market::{kline::KlineData, ticker::TickerData},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct SignalMessage {
    pub strategy_id: String,
    pub order_side: OrderSide,
    pub symbol: String,
    pub price: f64,
}

pub enum AlgorithmEvalResult {
    Buy,
    Sell,
    Ignore,
}
