use std::fmt::{self};

use crate::{account::trade::OrderSide, market::kline::Kline};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;

use super::strategy::StrategyId;

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
