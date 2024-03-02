use mongodb::bson::{doc, from_bson, to_bson};
use mongodb::{bson, Client, Collection};

use super::manager::StorageManager;
use crate::{
    market::{kline::Kline, trade::MarketTrade},
    strategy::strategy::{StrategyId, StrategyInfo, StrategySummary},
};
use std::error::Error;

pub struct DbStorageManager {}

impl StorageManager for DbStorageManager {
    fn get_klines(
        &self,
        _symbol: &str,
        _interval: &str,
        _from_ts: Option<u64>,
        _to_ts: Option<u64>,
    ) -> Vec<Kline> {
        unimplemented!()
    }

    fn save_klines(&self, _klines: &[Kline], _kline_key: &str) -> std::io::Result<()> {
        unimplemented!()
    }

    fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>> {
        unimplemented!()
    }
    fn save_strategy_summary(&self, _summary: StrategySummary) -> Result<(), Box<dyn Error>> {
        // TODO: Implement save strategy summary on DBStorageManager
        unimplemented!()
    }
    fn get_strategy_summary(
        &self,
        _strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>> {
        // TODO: Implement get strategy summary on DBStorageManager
        unimplemented!()
    }

    // TODO: Docs
    fn get_trades(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<MarketTrade> {
        // TODO: Implement get strategy summary on DBStorageManager
        unimplemented!()
    }

    // TODO: docs
    fn save_trades(&self, klines: &[MarketTrade], kline_key: &str) -> std::io::Result<()> {
        unimplemented!()
    }
}
