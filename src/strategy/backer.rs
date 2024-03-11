use std::sync::Arc;

use actix_web::rt::signal;
use log::info;

use crate::{
    account::{
        account::Account,
        trade::{OrderSide, PositionId, TradeTx},
    },
    exchange::{api::ExchangeApi, mock::MockExchangeApi},
    market::{kline::KlineData, market::Market, messages::MarketMessage, types::ArcMutex},
    storage::{fs::FsStorage, manager::StorageManager, mongo::MongoDbStorage},
    strategy::{
        signal::SignalManager,
        strategy::{Strategy, StrategySummary},
        types::{AlgoEvalResult, SignalMessage},
    },
    utils::{
        channel::build_arc_channel,
        time::{timestamp_to_string, SEC_AS_MILI},
    },
};

use super::{strategy::StrategySignals, types::SignalMessageType};

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
    market: ArcMutex<Market>,
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

    pub async fn new(
        strategy: Strategy,
        market: ArcMutex<Market>,
        _initial_balance: Option<f64>,
    ) -> Self {
        let (_, market_rx) = build_arc_channel::<MarketMessage>();
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi::default());

        let storage_manager: Arc<dyn StorageManager> = market.lock().await.storage_manager.clone();

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

        let mut signal_manager = SignalManager::new();
        signal_manager.add_strategy_settings(&strategy.id, strategy.settings());

        Self {
            strategy,
            signals: vec![],
            signal_manager,
            market,
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
        if let Some(first) = kline_data.klines().first() {
            self.period_start_price = first.open
        }
        if let Some(last) = kline_data.klines().last() {
            self.period_end_price = last.close
        }

        for kline in kline_data.klines() {
            let algo_needs_trades = self.strategy.algorithm.lock().await.needs_trades();

            // only get trades if needed by the algorithm
            let trades = if algo_needs_trades {
                let trades = match self
                    .market
                    .lock()
                    .await
                    .trade_data_range(
                        &self.strategy.symbol,
                        // get 5 seconds in passed to ensure all trades
                        Some(kline.open_time),
                        Some(kline.close_time),
                        None,
                    )
                    .await
                {
                    Some(trade_data) => trade_data.trades(),
                    None => vec![],
                };
                trades
            } else {
                vec![]
            };

            let eval_result = self
                .strategy
                .algorithm
                .lock()
                .await
                .evaluate(kline.clone(), &trades);

            let order_side = match eval_result {
                AlgoEvalResult::Buy => OrderSide::Buy,
                AlgoEvalResult::Sell => OrderSide::Sell,
                AlgoEvalResult::Ignore => {
                    continue;
                }
            };

            let signal = SignalMessage {
                strategy_id: self.strategy.id,
                order_side,
                symbol: self.strategy.symbol.to_string(),
                price: kline.close.clone(),
                is_back_test: true,
                close_time: timestamp_to_string(kline.close_time),
                ty: SignalMessageType::Standard,
                // kline: kline.clone(),
            };

            self.strategy.add_signal(&signal).await
        }
    }

    /// Computes and returns a summary of the backtest results.
    ///
    /// # Returns
    ///
    /// Returns a `StrategySummary` detailing the results of the backtest, including profit, drawdown,
    /// trade counts, and other relevant metrics.

    pub async fn result(&mut self) -> StrategySummary {
        for signal in &self.strategy.get_signals().await {
            self.signal_manager
                .handle_signal(signal.clone(), self.market.clone(), self.account.clone())
                .await
        }

        let info = self.strategy.info().await;

        // lock account here to use for rest of method
        // do not lock again
        let mut account = self.account.lock().await;

        let active_positions: Vec<(PositionId, f64)> = account
            .positions()
            .into_iter()
            .map(|item| (item.id, item.open_price))
            .collect();

        // close any remaining positions
        for (id, open_price) in active_positions {
            let trade = account.close_position(id, open_price).await;

            if let Some(trade) = trade.cloned() {
                let signal = SignalMessage {
                    strategy_id: self.strategy.id,
                    order_side: trade.position.order_side,
                    symbol: trade.position.symbol,
                    price: trade.close_price,
                    is_back_test: true,
                    close_time: trade.close_time,
                    ty: SignalMessageType::ForcedClose("Closed Remaining Positions".to_string()),
                };

                account.add_position_meta(id, &signal)
            }
        }

        // get all trade txs
        let mut trades: Vec<TradeTx> = account.trades();

        for trade in trades.iter_mut() {
            if let Some(signals) = account.get_position_meta(trade.position.id) {
                for signal in signals {
                    trade.add_signal(&signal);
                }
            }
        }

        let max_profit = Strategy::calc_max_profit(&trades);
        let max_drawdown = Strategy::calc_max_drawdown(&trades);
        let long_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Buy);
        let short_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Sell);
        let profit: f64 = Strategy::calc_profit(&trades);

        StrategySummary {
            info,
            profit,
            trades,
            positions: vec![],
            long_trade_count,
            short_trade_count,
            symbol: self.strategy.symbol.to_string(),
            period_end_price: self.period_end_price,
            period_start_price: self.period_start_price,
            max_drawdown,
            max_profit,
            // signals: self.strategy.get_signals().await,
        }
    }
}
