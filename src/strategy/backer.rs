use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::info;
use serde_json::Value;

use crate::account::account::Account;
use crate::account::trade::{OrderSide, Position, PositionId, TradeTx};
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
use crate::utils::number::generate_random_id;
use crate::utils::time::generate_ts;

use super::signal::SignalManager;
use super::strategy::{Strategy, StrategyResult};
use super::types::{AlgorithmEvalResult, SignalMessage};

pub struct BackTest {
    pub strategy: Strategy,
    pub signals: Vec<SignalMessage>,
    pub signal_manager: SignalManager,
    account: ArcMutex<Account>,
    initial_balance: f64,
}

impl BackTest {
    pub async fn new(strategy: Strategy, initial_balance: Option<f64>) -> Self {
        let (_, market_rx) = build_arc_channel::<MarketMessage>();
        let exchange_api: Arc<Box<dyn ExchangeApi>> =
            Arc::new(Box::new(BackTestExchangeApi::default()));

        let storage_manager = StorageManager::default();

        let market =
            ArcMutex::new(Market::new(market_rx, exchange_api.clone(), storage_manager).await);

        // create new storage manager
        let account = ArcMutex::new(Account::new(market.clone(), exchange_api.clone()).await);

        let signal_manager = SignalManager::new(account.clone(), market.clone());

        Self {
            strategy,
            signals: vec![],
            signal_manager,
            account,
            initial_balance: initial_balance.unwrap_or_else(|| 0.0),
        }
    }

    pub async fn run(&mut self, kline_data: KlineData) {
        let strategy_id = generate_random_id();

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
                strategy_id,
                order_side,
                symbol: self.strategy.symbol.to_string(),
                price: kline.close.clone(),
                is_back_test: true,
            };

            self.add_signal(signal)
        }
    }

    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }

    pub async fn result(&mut self) -> StrategyResult {
        for signal in &self.signals {
            self.signal_manager.handle_signal(signal.clone()).await
        }

        // close any positions still open
        for active_position in self.account.lock().await.open_positions().await {
            self.account
                .lock()
                .await
                .close_position(active_position.id, active_position.open_price)
                .await;
        }

        let active_position_count = self.account.lock().await.open_positions().await.len();

        info!("All Positions are closed, active_position count: {active_position_count}");

        let all_trades: Vec<TradeTx> = self
            .account
            .lock()
            .await
            .trade_txs()
            .await
            .into_iter()
            .map(|tx| tx.clone())
            .collect();

        StrategyResult {
            balance: 0.0,
            positions: 0,
            buy_count: 0,
            sell_count: 0,
        }
    }
}

pub struct BackTestExchangeApi {
    // positions: HashMap<PositionId, Position>,
}

#[async_trait]
impl ExchangeApi for BackTestExchangeApi {
    async fn open_position(
        &self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
    ) -> ApiResult<Position> {
        let position = Position::new(symbol, open_price, order_side, margin_usd, leverage, None);
        Ok(position)
    }
    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx> {
        let trade_tx = TradeTx::new(close_price, generate_ts(), position);
        Ok(trade_tx)
    }

    // ---
    // All Other methods not used on this mock BackTestExchangeApi
    // Will fail if called
    // ---
    async fn get_account(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }
    async fn all_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn list_open_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>> {
        unimplemented!()
    }
    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline> {
        unimplemented!()
    }
    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker> {
        unimplemented!()
    }

    async fn exchange_info(&self) -> ApiResult<Value> {
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
}

impl Default for BackTestExchangeApi {
    fn default() -> Self {
        Self {}
    }
}
