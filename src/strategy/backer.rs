use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::info;
use serde_json::Value;

use crate::account::account::Account;
use crate::account::trade::{OrderSide, Position, PositionId, TradeTx};
use crate::exchange::api::ExchangeApi;
use crate::exchange::mock::MockExchangeApi;
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
use super::strategy::{Strategy, StrategyResult};
use super::types::{AlgorithmEvalResult, SignalMessage};

pub struct BackTest {
    pub strategy: Strategy,
    pub signals: Vec<SignalMessage>,
    pub signal_manager: SignalManager,
    account: ArcMutex<Account>,
    period_start_price: f64,
    period_end_price: f64,
    initial_balance: f64,
}

impl BackTest {
    pub async fn new(strategy: Strategy, initial_balance: Option<f64>) -> Self {
        let (_, market_rx) = build_arc_channel::<MarketMessage>();
        let exchange_api: Arc<Box<dyn ExchangeApi>> =
            Arc::new(Box::new(MockExchangeApi::default()));

        let storage_manager = StorageManager::default();

        let market = ArcMutex::new(
            Market::new(market_rx, exchange_api.clone(), storage_manager, false).await,
        );

        // create new storage manager
        let account = ArcMutex::new(Account::new(exchange_api.clone(), false).await);

        let mut signal_manager = SignalManager::new(account.clone(), market.clone());
        signal_manager.add_strategy_settings(strategy.id, strategy.settings());

        Self {
            strategy,
            signals: vec![],
            signal_manager,
            account,
            initial_balance: initial_balance.unwrap_or_else(|| 0.0),
            period_end_price: 0.0,
            period_start_price: 0.0,
        }
    }

    pub async fn run(&mut self, kline_data: KlineData) {
        if let Some(first) = kline_data.klines.first() {
            self.period_start_price = first.open
        }
        if let Some(last) = kline_data.klines.last() {
            self.period_end_price = last.close
        }

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
                strategy_id: self.strategy.id,
                order_side,
                symbol: self.strategy.symbol.to_string(),
                price: kline.close.clone(),
                is_back_test: true,
                timestamp: kline.close_time,
            };

            self.add_signal(signal)
        }
    }

    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }

    pub fn calc_max_profit(&self, trade_txs: &Vec<TradeTx>) -> f64 {
        let mut max_balance = 0.0;
        let mut current_balance = 0.0;

        for trade_tx in trade_txs {
            let profit = trade_tx.calc_profit();
            current_balance += profit;

            if current_balance > max_balance {
                max_balance = current_balance;
            }
        }

        max_balance
    }

    pub fn calc_max_drawdown(&self, trade_txs: &Vec<TradeTx>) -> f64 {
        let mut min_balance = f64::MAX;
        let mut current_balance = 0.0;

        for trade_tx in trade_txs {
            let profit = trade_tx.calc_profit();
            current_balance += profit;

            if current_balance < min_balance {
                min_balance = current_balance;
            }
        }

        min_balance
    }

    pub async fn result(&mut self) -> StrategyResult {
        for signal in &self.signals {
            self.signal_manager.handle_signal(signal.clone()).await
        }

        let active_positions: Vec<(PositionId, f64)> = self
            .account
            .lock()
            .await
            .open_positions()
            .into_iter()
            .map(|item| (item.id, item.open_price))
            .collect();

        info!("Before Close Remaining, {active_positions:?}");

        // close any remaining positions
        for (id, open_price) in active_positions {
            self.account
                .lock()
                .await
                .close_position(id, open_price)
                .await;
        }

        let active_positions: Vec<(PositionId, f64)> = self
            .account
            .lock()
            .await
            .open_positions()
            .into_iter()
            .map(|item| (item.id, item.open_price))
            .collect();

        info!("After Close Remaining, {active_positions:?}");

        // get all trade txs
        let trade_txs: Vec<TradeTx> = self.account.lock().await.trade_txs();

        let max_profit = self.calc_max_profit(&trade_txs);
        let max_drawdown = self.calc_max_drawdown(&trade_txs);

        let profit: f64 = trade_txs.iter().map(|trade| trade.calc_profit()).sum();
        let buy_count = trade_txs
            .iter()
            .filter(|trade| trade.position.order_side == OrderSide::Buy)
            .count();
        let sell_count = trade_txs.len() - buy_count;

        let signals = self.signals.clone();
        let buy_signal_count = signals
            .iter()
            .filter(|sig| sig.order_side == OrderSide::Buy)
            .count();

        let sell_signal_count = signals.len() - buy_signal_count;

        let active_positions: Vec<(PositionId, f64)> = self
            .account
            .lock()
            .await
            .open_positions()
            .into_iter()
            .map(|item| (item.id, item.open_price))
            .collect();

        StrategyResult {
            profit,
            trade_txs,
            buy_count,
            sell_count,
            // signals,
            // // buy_signal_count,
            // sell_signal_count,
            symbol: self.strategy.symbol.to_string(),
            period_end_price: self.period_end_price,
            period_start_price: self.period_start_price,
            max_drawdown,
            max_profit,
        }
    }
}
