use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::account::account::Account;
use crate::account::trade::OrderSide;
use crate::exchange::api::ExchangeApi;
use crate::exchange::stream::{StreamManager, StreamMeta};
use crate::exchange::types::{ApiResult, StreamType};
use crate::market::kline::{Kline, KlineData};
use crate::market::market::Market;
use crate::market::messages::MarketMessage;
use crate::market::ticker::Ticker;
use crate::market::types::ArcMutex;
use crate::storage::manager::StorageManager;
use crate::utils::channel::build_arc_channel;

use super::signal::SignalManager;
use super::strategy::Strategy;
use super::types::{AlgorithmEvalResult, SignalMessage};

pub struct BackTest {
    pub strategy: Strategy,
    pub signals: Vec<SignalMessage>,
    pub signal_manager: SignalManager,
    pub balance: f64,
    pub positions: i32,
    pub buy_count: u32,
    pub sell_count: u32,
}

impl BackTest {
    pub async fn new(strategy: Strategy) -> Self {
        let (market_tx, market_rx) = build_arc_channel::<MarketMessage>();
        let exchange_api: Arc<Box<dyn ExchangeApi>> =
            Arc::new(Box::new(BackTestExchangeApi::default()));

        let storage_manager = StorageManager::default();

        let market =
            ArcMutex::new(Market::new(market_rx, exchange_api.clone(), storage_manager).await);

        // create new storage manager
        let account =
            ArcMutex::new(Account::new(market.clone(), exchange_api.clone(), false).await);

        let signal_manager = SignalManager::new(account);

        Self {
            strategy,
            signals: vec![],
            signal_manager,
            balance: 10_000.0,
            positions: 0,
            buy_count: 0,
            sell_count: 0,
        }
    }

    pub async fn run(&mut self, kline_data: KlineData) {
        for kline in kline_data.klines {
            let eval_result = self.strategy.algorithm.lock().await.evaluate(kline.clone());

            let order_side = match eval_result {
                AlgorithmEvalResult::Buy => OrderSide::Buy,
                AlgorithmEvalResult::Sell => OrderSide::Sell,
                AlgorithmEvalResult::Ignore => {
                    continue;
                }
            };

            let signal = SignalMessage {
                strategy_id: "backtest".to_string(),
                order_side,
                symbol: self.strategy.symbol.to_string(),
                price: kline.close.clone(),
            };

            self.add_signal(signal)
        }
    }

    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }

    pub fn result(&mut self) -> BackTestResult {
        for signal in &self.signals {
            self.signal_manager.handle_signal(signal.clone())
        }

        let mut last_price: f64 = 0.0;
        for signal in &self.signals {
            match signal.order_side {
                OrderSide::Buy => {
                    self.positions += 1;
                    self.balance -= signal.price;
                    self.buy_count += 1
                }
                OrderSide::Sell => {
                    self.positions -= 1;
                    self.balance += signal.price;
                    self.sell_count += 1
                }
            }
            last_price = signal.price;
        }

        if self.positions.is_negative() {
            self.balance -= last_price * self.positions.abs() as f64;
        } else {
            self.balance += last_price * self.positions.abs() as f64;
        }

        BackTestResult {
            balance: self.balance,
            positions: self.positions,
            buy_count: self.buy_count,
            sell_count: self.sell_count,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackTestResult {
    pub balance: f64,
    pub positions: i32,
    pub buy_count: u32,
    pub sell_count: u32,
}

pub struct BackTestExchangeApi {}

#[async_trait]
impl ExchangeApi for BackTestExchangeApi {
    // ---
    // Account methods
    // ---
    async fn get_account(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }
    async fn open_position(
        &self,
        symbol: &str,
        side: OrderSide,
        quantity: f64,
    ) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn close_position(&self, position_id: &str) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn all_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn list_open_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }

    // ---
    // Stream Methods
    // ---
    async fn open_stream(
        &self,
        stream_type: StreamType,
        symbol: &str,
        interval: Option<&str>,
    ) -> ApiResult<String> {
        unimplemented!()
    }
    async fn close_stream(&self, stream_id: &str) -> Option<StreamMeta> {
        unimplemented!()
    }

    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>> {
        unimplemented!()
    }

    async fn active_streams(&self) -> Vec<StreamMeta> {
        unimplemented!()
    }

    // --
    // Exchange Methods
    // ---
    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline> {
        unimplemented!()
    }
    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker> {
        unimplemented!()
    }
    async fn exchange_info(&self) -> ApiResult<Value> {
        unimplemented!()
    }

    // ---
    // HTTP Methods
    // ---
    async fn get(
        &self,
        endpoint: &str,
        query_str: Option<&str>,
    ) -> Result<Response, reqwest::Error> {
        unimplemented!()
    }
    async fn post(&self, endpoint: &str, query_str: &str) -> Result<Response, reqwest::Error> {
        unimplemented!()
    }

    // ---
    // API Util methods
    // ---
    async fn handle_response(&self, response: Response) -> ApiResult<Value> {
        unimplemented!()
    }

    fn build_headers(&self, json: bool) -> HeaderMap {
        unimplemented!()
    }
    fn build_stream_url(
        &self,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) -> String {
        unimplemented!()
    }
    fn sign_query_str(&self, query_str: &str) -> String {
        unimplemented!()
    }
}

impl Default for BackTestExchangeApi {
    fn default() -> Self {
        Self {}
    }
}
