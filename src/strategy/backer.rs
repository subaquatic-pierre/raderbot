use std::sync::Arc;

use crate::{
    account::{
        account::Account,
        trade::{OrderSide, PositionId, TradeTx},
    },
    exchange::{api::ExchangeApi, mock::MockExchangeApi},
    market::{kline::KlineData, market::Market, messages::MarketMessage, types::ArcMutex},
    storage::{fs::FsStorageManager, manager::StorageManager},
    strategy::{
        signal::SignalManager,
        strategy::{Strategy, StrategySummary},
        types::{AlgorithmEvalResult, SignalMessage},
    },
    utils::channel::build_arc_channel,
};

/// Represents a backtest environment for a trading strategy.
///
/// This struct encapsulates the logic to simulate the execution of a trading strategy over
/// historical data. It includes methods to run the backtest, add trading signals generated during
/// the backtest, and compute a summary of the backtest results.

pub struct BackTest {
    pub strategy: Strategy,
    pub signals: Vec<SignalMessage>,
    pub signal_manager: SignalManager,
    account: ArcMutex<Account>,
    period_start_price: f64,
    period_end_price: f64,
}

impl BackTest {
    /// Creates a new `BackTest` instance for a given trading strategy.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The trading strategy to backtest.
    /// * `_initial_balance` - An optional initial balance for the backtest account (not currently used).
    ///
    /// # Returns
    ///
    /// Returns a new instance of `BackTest`.

    pub async fn new(strategy: Strategy, _initial_balance: Option<f64>) -> Self {
        let (_, market_rx) = build_arc_channel::<MarketMessage>();
        let exchange_api: Arc<Box<dyn ExchangeApi>> =
            Arc::new(Box::new(MockExchangeApi::default()));

        let storage_manager: Arc<Box<dyn StorageManager>> =
            Arc::new(Box::new(FsStorageManager::default()));

        let market = ArcMutex::new(
            Market::new(
                market_rx,
                exchange_api.clone(),
                storage_manager.clone(),
                false,
            )
            .await,
        );

        // create new storage manager
        let account = ArcMutex::new(Account::new(exchange_api.clone(), false, true).await);

        let mut signal_manager = SignalManager::new(account.clone(), market.clone());
        signal_manager.add_strategy_settings(strategy.id, strategy.settings());

        Self {
            strategy,
            signals: vec![],
            signal_manager,
            account,
            period_end_price: 0.0,
            period_start_price: 0.0,
        }
    }

    /// Executes the backtest over a set of historical k-line data.
    ///
    /// # Arguments
    ///
    /// * `kline_data` - Historical k-line data over which the backtest will be run.

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
                AlgorithmEvalResult::Long => OrderSide::Long,
                AlgorithmEvalResult::Short => OrderSide::Short,
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

    /// Adds a trading signal to the backtest.
    ///
    /// # Arguments
    ///
    /// * `signal` - The trading signal to add.

    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }

    /// Computes and returns a summary of the backtest results.
    ///
    /// # Returns
    ///
    /// Returns a `StrategySummary` detailing the results of the backtest, including profit, drawdown,
    /// trade counts, and other relevant metrics.

    pub async fn result(&mut self) -> StrategySummary {
        for signal in &self.signals {
            self.signal_manager.handle_signal(signal.clone()).await
        }

        let info = self.strategy.info().await;

        let active_positions: Vec<(PositionId, f64)> = self
            .account
            .lock()
            .await
            .positions()
            .into_iter()
            .map(|item| (item.id, item.open_price))
            .collect();

        // close any remaining positions
        for (id, open_price) in active_positions {
            self.account
                .lock()
                .await
                .close_position(id, open_price)
                .await;
        }

        // get all trade txs
        let trades: Vec<TradeTx> = self.account.lock().await.trades();

        let max_profit = Strategy::calc_max_profit(&trades);
        let max_drawdown = Strategy::calc_max_drawdown(&trades);
        let long_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Long);
        let short_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Short);
        let profit: f64 = Strategy::calc_profit(&trades);

        StrategySummary {
            info,
            profit,
            trades,
            positions: vec![],
            long_trade_count,
            short_trade_count,
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
