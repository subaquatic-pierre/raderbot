use super::manager::StorageManager;
use crate::{
    market::kline::Kline,
    strategy::strategy::{StrategyId, StrategySummary},
};
use std::error::Error;

pub struct DbStorageManager {}

impl StorageManager for DbStorageManager {
    fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<Kline> {
        unimplemented!()
    }

    fn save_klines(&self, klines: &[Kline], kline_key: &str) -> std::io::Result<()> {
        unimplemented!()
    }

    fn list_all_saved_strategy_summaries(&self) -> Result<Vec<StrategySummary>, Box<dyn Error>> {
        unimplemented!()
    }
    fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>> {
        // TODO: Implement save strategy summary on FSStorageManager
        unimplemented!()
    }
    fn get_strategy_summary(
        &self,
        strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>> {
        // TODO: Implement get strategy summary on FSStorageManager
        unimplemented!()
    }
}
