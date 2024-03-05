use async_trait::async_trait;
use futures::{TryFutureExt, TryStreamExt};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;
use std::{any, error::Error};
use uuid::Uuid;

use super::manager::StorageManager;
use crate::{
    account::trade::OrderSide,
    market::{kline::Kline, trade::MarketTrade},
    strategy::strategy::{StrategyId, StrategyInfo, StrategySummary},
    utils::{
        kline::build_kline_key,
        time::{elapsed_time, start_timer, timestamp_to_datetime},
        trade::build_market_trade_key,
    },
};

pub struct InfluxStorage {
    client: Client,
    uri: String, // Base URI for InfluxDB
    token: String,
    org: String,
    bucket: String,
}

impl InfluxStorage {
    pub async fn new(uri: &str, token: &str) -> Result<Self, Box<dyn Error>> {
        let client = Client::new();
        Ok(InfluxStorage {
            client,
            uri: uri.to_string(),
            token: token.to_string(),
            bucket: "trade_data".to_string(),
            org: "raderbot".to_string(),
        })
    }

    pub async fn make_request(&self, uri: &str, body: &str) -> Result<(), String> {
        self.client
            .post(uri)
            .body(body.to_string())
            .header("Authorization", format!("Token {}", &self.token))
            .header("Accept-Encoding", "gzip")
            .header("Content-type", "application/vnd.flux")
            .header("Accept", "application/csv")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn build_kline_body(&self, klines: &[Kline]) -> String {
        let str_data = klines
            .iter()
            .map(|kline| {
                format!(
                    "symbol={} open={},high={},low={},close={},volume={}",
                    kline.symbol, kline.open, kline.high, kline.low, kline.close, kline.volume,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        str_data
    }
}

#[async_trait]
impl StorageManager for InfluxStorage {
    async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<Kline> {
        let write_uri = format!(
            "{}/api/v2/write?bucket={}&org={}",
            self.uri, self.bucket, self.org
        );
        unimplemented!()
    }

    async fn save_klines(
        &self,
        klines: &[Kline],
        kline_key: &str,
        is_bootstrap: bool,
    ) -> io::Result<()> {
        let query =
            "q=SELECT used_percent FROM example-db.example-rp.example-measurement WHERE host=host1";
        unimplemented!()
    }

    // TODO: Docs
    async fn get_trades(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<MarketTrade> {
        unimplemented!()
    }

    // TODO: docs
    async fn save_trades(&self, trades: &[MarketTrade], trade_key: &str) -> std::io::Result<()> {
        unimplemented!()
    }

    async fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>> {
        unimplemented!()
    }
    async fn save_strategy_summary(&self, _summary: StrategySummary) -> Result<(), Box<dyn Error>> {
        // TODO: Implement save strategy summary on DBStorageManager
        unimplemented!()
    }
    async fn get_strategy_summary(
        &self,
        _strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>> {
        // TODO: Implement get strategy summary on DBStorageManager
        unimplemented!()
    }
}
