use std::fmt::{self};

use serde::{Deserialize, Serialize};

use crate::account::trade::OrderSide;

use super::strategy::StrategyId;

/// Encapsulates a message signaling a trading decision based on a strategy's evaluation.
///
/// It contains the strategy's identification, the intended order side (buy/sell), the target trading symbol,
/// the price at which the signal was generated, a flag indicating if this signal is part of a backtest, and
/// the timestamp marking when the signal was created.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalMessage {
    pub strategy_id: StrategyId,
    pub order_side: OrderSide,
    pub symbol: String,
    pub price: f64,
    pub is_back_test: bool,
    pub timestamp: u64,
}

/// Outlines the potential outcomes of a trading algorithm's evaluation of market data.
///
/// This can indicate a recommendation to enter a long position, enter a short position, or to make no trade (ignore).

pub enum AlgorithmEvalResult {
    Long,
    Short,
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
pub enum AlgorithmError {
    UnkownName(String),
    UnknownInterval(String),
    InvalidParams(String),
}

/// Implements display formatting for `AlgorithmError`, providing clearer error descriptions.
///
/// This method formats different types of algorithm errors into human-readable strings, enhancing error reporting and debugging.

impl fmt::Display for AlgorithmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlgorithmError::UnkownName(msg) => write!(f, "Unknown Name error: {}", msg),
            AlgorithmError::UnknownInterval(msg) => write!(f, "Unknown Interval error: {}", msg),
            AlgorithmError::InvalidParams(msg) => write!(f, "Invalid Params error: {}", msg),
        }
    }
}
