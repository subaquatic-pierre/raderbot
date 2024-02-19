use std::fmt::{self};

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

#[derive(Debug)]
pub enum AlgorithmError {
    UnkownName(String),
    UnknownInterval(String),
}

impl fmt::Display for AlgorithmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlgorithmError::UnkownName(msg) => write!(f, "Unknown Name error: {}", msg),
            AlgorithmError::UnknownInterval(msg) => write!(f, "Unknown Interval error: {}", msg),
        }
    }
}
