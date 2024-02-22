
use std::error::Error;
use std::io::{self};

use crate::{
    market::kline::Kline,
    strategy::strategy::{StrategyId, StrategySummary},
};

pub trait StorageManager: Send + Sync {
    fn save_klines(&self, klines: &[Kline], kline_key: &str) -> io::Result<()>;

    fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<Kline>;

    fn list_all_saved_strategy_summaries(&self) -> Result<Vec<StrategySummary>, Box<dyn Error>>;
    fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>>;
    fn get_strategy_summary(
        &self,
        strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>>;
}
