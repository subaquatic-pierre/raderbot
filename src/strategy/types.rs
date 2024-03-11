use std::fmt::{self};

use crate::{account::trade::OrderSide, market::kline::Kline};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;

use super::strategy::StrategyId;

/// Encapsulates a message signaling a trading decision based on a strategy's evaluation.
///
/// It contains the strategy's identification, the intended order side (buy/sell), the target trading symbol,
/// the price at which the signal was generated, a flag indicating if this signal is part of a backtest, and
/// the timestamp marking when the signal was created.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignalMessageType {
    Standard,
    ForcedClose(String),
    StopLoss,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalMessage {
    pub strategy_id: StrategyId,
    pub order_side: OrderSide,
    pub symbol: String,
    pub price: f64,
    pub is_back_test: bool,
    pub close_time: String,
    #[serde(rename = "type")]
    pub ty: SignalMessageType,
    // pub kline: Kline,
}

/// Outlines the potential outcomes of a trading algorithm's evaluation of market data.
///
/// This can indicate a recommendation to enter a long position, enter a short position, or to make no trade (ignore).

pub enum AlgoEvalResult {
    Buy,
    Sell,
    Ignore,
}

/// Specifies selection between the first or last element in a sequence.
///
/// Useful in contexts where it's necessary to distinguish between the initial and concluding elements of a dataset.

pub enum FirstLastEnum {
    First,
    Last,
}

/// Enumerates errors that can arise within trading algorithms.
///
/// Covers scenarios such as unrecognized strategy names, unsupported intervals for analysis, and improperly configured parameters.

#[derive(Debug)]
pub enum AlgoError {
    UnkownName(String),
    UnknownInterval(String),
    InvalidParams(String),
    SerdeJsonError(SerdeJsonError),
}

impl From<SerdeJsonError> for AlgoError {
    fn from(err: SerdeJsonError) -> Self {
        AlgoError::SerdeJsonError(err)
    }
}

/// Implements display formatting for `AlgoError`, providing clearer error descriptions.
///
/// This method formats different types of algorithm errors into human-readable strings, enhancing error reporting and debugging.

impl fmt::Display for AlgoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlgoError::UnkownName(msg) => write!(f, "Unknown Name error: {}", msg),
            AlgoError::UnknownInterval(msg) => write!(f, "Unknown Interval error: {}", msg),
            AlgoError::InvalidParams(msg) => write!(f, "Invalid Params error: {}", msg),
            AlgoError::SerdeJsonError(msg) => write!(f, "Invalid Params error: {}", msg),
        }
    }
}
