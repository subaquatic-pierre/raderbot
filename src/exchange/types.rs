use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::market::types::ArcMutex;
/// Custom error type for API-related errors.
///
/// This enum represents various types of errors that can occur during API operations.
#[derive(Debug)]
pub enum ApiError {
    /// Represents a network-related error with a descriptive message.
    Network(String),
    /// Represents a parsing-related error with a descriptive message.
    Parsing(String),
    /// Represents a Reqwest error with a descriptive message.
    Reqwest(String),
}

/// Implementation of the `Display` trait for `ApiError`.
///
/// This implementation formats an `ApiError` for display purposes.
impl fmt::Display for ApiError {
    /// Formats the error for display.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Match on the enum variant to format the error message accordingly
        match self {
            ApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ApiError::Parsing(msg) => write!(f, "Parsing error: {}", msg),
            ApiError::Reqwest(msg) => write!(f, "Reqwest error: {}", msg),
        }
    }
}

/// Implementation of the `Error` trait for `ApiError`.
///
/// This implementation allows `ApiError` to be used as an error type in the `Result` type.
impl Error for ApiError {}

/// Custom result type used for API operations.
///
/// This type aliases a `Result` with the `ApiError` enum as the error type.
pub type ApiResult<T> = Result<T, ApiError>;

/// Conversion implementation for `std::io::Error` into `ApiError`.
///
/// This implementation allows conversion from `std::io::Error` to `ApiError::Network`.
impl From<std::io::Error> for ApiError {
    /// Converts a `std::io::Error` into an `ApiError`.
    ///
    /// # Arguments
    ///
    /// * `e` - The `std::io::Error` to convert.
    fn from(e: std::io::Error) -> Self {
        ApiError::Network(e.to_string())
    }
}

/// Conversion implementation for `String` into `ApiError`.
///
/// This implementation allows conversion from `String` to `ApiError::Parsing`.
impl From<String> for ApiError {
    /// Converts a `String` into an `ApiError`.
    ///
    /// # Arguments
    ///
    /// * `e` - The `String` to convert.
    fn from(e: String) -> Self {
        ApiError::Parsing(e)
    }
}

/// Conversion implementation for `serde_json::Error` into `ApiError`.
///
/// This implementation allows conversion from `serde_json::Error` to `ApiError::Parsing`.
impl From<serde_json::Error> for ApiError {
    /// Converts a `serde_json::Error` into an `ApiError`.
    ///
    /// # Arguments
    ///
    /// * `e` - The `serde_json::Error` to convert.
    fn from(e: serde_json::Error) -> Self {
        ApiError::Parsing(e.to_string())
    }
}

/// Conversion implementation for `reqwest::Error` into `ApiError`.
///
/// This implementation allows conversion from `reqwest::Error` to `ApiError::Reqwest`.
impl From<reqwest::Error> for ApiError {
    /// Converts a `reqwest::Error` into an `ApiError`.
    ///
    /// # Arguments
    ///
    /// * `e` - The `reqwest::Error` to convert.
    fn from(e: reqwest::Error) -> Self {
        ApiError::Reqwest(e.to_string())
    }
}

/// Conversion implementation for `std::num::ParseFloatError` into `ApiError`.
///
/// This implementation allows conversion from `std::num::ParseFloatError` to `ApiError::Reqwest`.
impl From<std::num::ParseFloatError> for ApiError {
    /// Converts a `std::num::ParseFloatError` into an `ApiError`.
    ///
    /// # Arguments
    ///
    /// * `e` - The `std::num::ParseFloatError` to convert.
    fn from(e: std::num::ParseFloatError) -> Self {
        ApiError::Reqwest(e.to_string())
    }
}

/// Type alias for a thread-safe reference to a WebSocket split sink.
///
/// This type alias simplifies the usage of `ArcMutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>`.
pub type ArcEsStreamSync = ArcMutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>;

/// Enum representing different types of streams.
///
/// This enum specifies the types of data streams that can be handled.
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StreamType {
    /// Represents a Kline stream type.
    Kline,
    /// Represents a Ticker stream type.
    Ticker,
    MarketTrade,
}

/// Implementation of the `Display` trait for `StreamType`.
///
/// This implementation allows `StreamType` to be formatted for display purposes.
impl Display for StreamType {
    /// Formats the `StreamType` for display.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Match on the enum variant to format the display string
        match self {
            StreamType::MarketTrade => write!(f, "trade"),
            StreamType::Kline => write!(f, "kline"),
            StreamType::Ticker => write!(f, "ticker"),
        }
    }
}
