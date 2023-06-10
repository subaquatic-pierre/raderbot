use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::market::types::ArcMutex;

// Custom error types
#[derive(Debug)]
pub enum ApiError {
    Network(String),
    Parsing(String),
    Reqwest(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ApiError::Parsing(msg) => write!(f, "Parsing error: {}", msg),
            ApiError::Reqwest(msg) => write!(f, "Reqwest error: {}", msg),
        }
    }
}

impl Error for ApiError {}

// Custom result type
pub type ApiResult<T> = Result<T, ApiError>;

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Network(e.to_string())
    }
}

impl From<String> for ApiError {
    fn from(e: String) -> Self {
        ApiError::Parsing(e)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Parsing(e.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Reqwest(e.to_string())
    }
}
impl From<std::num::ParseFloatError> for ApiError {
    fn from(e: std::num::ParseFloatError) -> Self {
        ApiError::Reqwest(e.to_string())
    }
}

pub type ArcEsStreamSync = ArcMutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>;

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StreamType {
    Kline,
    Ticker,
}

impl Display for StreamType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StreamType::Kline => write!(f, "kline"),
            StreamType::Ticker => write!(f, "ticker"),
        }
    }
}
