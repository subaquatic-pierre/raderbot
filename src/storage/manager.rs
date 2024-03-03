use async_trait::async_trait;
use std::error::Error;
use std::io::{self};

use crate::market::trade::MarketTrade;
use crate::strategy::strategy::StrategyInfo;
use crate::{
    market::kline::Kline,
    strategy::strategy::{StrategyId, StrategySummary},
};

/// Defines operations for managing storage of trading data and strategy summaries.
///
/// Includes methods for saving and retrieving kline data, listing saved strategies,
/// and managing strategy summaries.

#[async_trait]
pub trait StorageManager: Send + Sync {
    /// Saves kline data to storage.
    ///
    /// Takes an array of `Kline` objects and a key for identification. Returns an `io::Result<()>` indicating success or failure.
    async fn save_klines(&self, klines: &[Kline], kline_key: &str) -> io::Result<()>;

    /// Retrieves kline data from storage.
    ///
    /// Fetches klines based on symbol, interval, and optional timestamp bounds and limit. Returns a vector of `Kline`.
    async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<Kline>;

    // TODO: Docs
    async fn get_trades(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<MarketTrade>;

    async fn save_trades(&self, trades: &[MarketTrade], trade_key: &str) -> io::Result<()>;

    /// Lists saved strategy information.
    ///
    /// Returns a list of `StrategyInfo` detailing saved strategies or an error if retrieval fails.
    async fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>>;

    /// Saves a strategy summary.
    ///
    /// Persists a given `StrategySummary` to storage, returning success or error.
    async fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>>;

    /// Retrieves a strategy summary by its ID.
    ///
    /// Fetches the summary for a given strategy identified by `StrategyId`. Returns the summary or an error if not found.
    async fn get_strategy_summary(
        &self,
        strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>>;
}
