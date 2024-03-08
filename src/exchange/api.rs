use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::{error::Error, fmt};

use crate::{
    account::trade::{OrderSide, Position, TradeTx},
    market::interval::Interval,
    market::{kline::Kline, ticker::Ticker, types::ArcMutex},
};

use super::{
    stream::{StreamManager, StreamMeta},
    types::{ApiResult, StreamType},
};

/// Represents an error encountered within the API operations.
///
/// This structure implements the standard `Error` trait, allowing it to be used in contexts where error handling is performed.

#[derive(Debug)]
pub struct ApiError;

impl Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

/// Defines the interface for interacting with an exchange's API.
///
/// This trait encapsulates the functionality necessary for querying account details, managing orders, and accessing market data. It is designed to be implemented for any exchange, providing a unified interface for interaction.

#[async_trait]
pub trait ExchangeApi: Send + Sync {
    // ---
    // Account methods
    // ---
    /// Retrieves account details from the exchange.
    ///
    /// # Returns
    ///
    /// A `Result` containing the account details as `Value` if successful, or an `ApiError` otherwise.

    async fn get_account(&self) -> ApiResult<Value>;

    /// Retrieves the account's balance.
    ///
    /// # Returns
    ///
    /// A `Result` containing the account balance as `f64` if successful, or an `ApiError` otherwise.

    async fn get_account_balance(&self) -> ApiResult<f64>;

    /// Opens a new position on the exchange with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `symbol` - A string slice representing the trading pair.
    /// * `margin_usd` - The amount of USD to use for the margin.
    /// * `leverage` - The leverage to apply to the position.
    /// * `order_side` - The side of the order (`OrderSide::Buy` or `OrderSide::Sell`).
    /// * `open_price` - The price at which to open the position.
    ///
    /// # Returns
    ///
    /// A `Result` containing the opened position as `Position` if successful, or an `ApiError` otherwise.

    async fn open_position(
        &self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
    ) -> ApiResult<Position>;

    /// Closes an existing position at the specified price.
    ///
    /// # Arguments
    ///
    /// * `position` - The position to close.
    /// * `close_price` - The price at which to close the position.
    ///
    /// # Returns
    ///
    /// A `Result` containing the trade transaction as `TradeTx` if successful, or an `ApiError` otherwise.

    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx>;

    /// Retrieves all orders for the account.
    ///
    /// # Returns
    ///
    /// A `Result` containing all orders as `Value` if successful, or an `ApiError` otherwise.

    async fn all_orders(&self) -> ApiResult<Value>;

    /// Lists all open orders for the account.
    ///
    /// # Returns
    ///
    /// A `Result` containing open orders as `Value` if successful, or an `ApiError` otherwise.

    async fn list_open_orders(&self) -> ApiResult<Value>;

    /// Retrieves the stream manager instance.
    ///
    /// # Returns
    ///
    /// An `ArcMutex` wrapping a `Box<dyn StreamManager>` that manages stream connections.

    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>>;

    /// Retrieves information about active streams.
    ///
    /// # Returns
    ///
    /// A `Vec` of `StreamMeta` containing metadata of active streams.

    async fn active_streams(&self) -> Vec<StreamMeta> {
        let stream_manager = self.get_stream_manager();
        let stream_manager = stream_manager.lock().await;
        stream_manager.active_streams().await
    }

    // ---
    // Exchange Methods
    // ---

    /// Retrieves a single k-line data point for the specified symbol and interval.
    ///
    /// # Arguments
    ///
    /// * `symbol` - A string slice representing the trading pair.
    /// * `interval` - A string slice representing the k-line interval.
    ///
    /// # Returns
    ///
    /// A `Result` containing the k-line as `Kline` if successful, or an `ApiError` otherwise.

    async fn get_kline(&self, symbol: &str, interval: Interval) -> ApiResult<Kline>;

    /// Retrieves the ticker information for a specific symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - A string slice representing the trading pair.
    ///
    /// # Returns
    ///
    /// A `Result` containing the ticker as `Ticker` if successful, or an `ApiError` otherwise.

    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker>;

    /// Retrieves information about the exchange.
    ///
    /// # Returns
    ///
    /// A `Result` containing exchange information as `ExchangeInfo` if successful, or an `ApiError` otherwise.

    async fn info(&self) -> ApiResult<ExchangeInfo>;

    /// Builds a URL for subscribing to a stream based on the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `symbol` - A string slice representing the trading pair.
    /// * `stream_type` - The type of stream to subscribe to.
    /// * `interval` - An optional string slice representing the interval for k-line streams.
    ///
    /// # Returns
    ///
    /// A `String` representing the constructed stream URL.

    fn build_stream_url(
        &self,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<Interval>,
    ) -> String;
}

/// A utility for constructing query strings from key-value pairs.
///
/// This struct is used to assemble a query string for HTTP requests by accepting a vector of key-value pairs (`&str`). The `ToString` trait implementation concatenates these pairs into a well-formed query string.

pub struct QueryStr<'a> {
    params: Vec<(&'a str, &'a str)>,
}

impl<'a> QueryStr<'a> {
    /// Creates a new instance of `QueryStr` with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - A vector of tuples, where each tuple consists of a reference to a string slice representing the key and a reference to a string slice representing the value.
    ///
    /// # Returns
    ///
    /// Returns a `QueryStr` instance containing the provided parameters for query string construction.

    pub fn new(params: Vec<(&'a str, &'a str)>) -> Self {
        Self { params }
    }
}

/// Converts the stored key-value pairs into a single query string.
///
/// This method concatenates all key-value pairs into a query string format, separating keys from values with `=` and pairs from each other with `&`. The final string does not end with `&`.
///
/// # Returns
///
/// Returns a `String` representing the assembled query string.

impl<'a> ToString for QueryStr<'a> {
    fn to_string(&self) -> String {
        let str_vec: Vec<String> = self
            .params
            .iter()
            .map(|(key, val)| format!("{key}={val}&"))
            .collect();

        let mut query_str = str_vec.join("");

        // remove last & from query_str
        query_str.pop();
        query_str
    }
}

/// Represents exchange-specific information.
///
/// This structure stores metadata about an exchange, such as its name. It is intended for serialization and deserialization of data related to exchange information.

#[derive(Serialize, Deserialize)]
pub struct ExchangeInfo {
    pub name: String,
}
