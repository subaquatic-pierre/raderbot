use std::time::Duration;

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time;
use uuid::Uuid;

use crate::{
    account::{
        account::Account,
        trade::{OrderSide, Position, TradeTx},
    },
    algo::builder::AlgoBuilder,
    market::{
        interval::Interval,
        kline::{self, Kline},
        market::Market,
        types::{ArcMutex, ArcSender},
    },
    strategy::{
        algorithm::Algorithm,
        types::{AlgoError, AlgoEvalResult, FirstLastEnum, SignalMessage},
    },
    utils::time::{floor_mili_ts, generate_ts, timestamp_to_string, MIN_AS_MILI, SEC_AS_MILI},
};

pub type StrategyId = Uuid;

/// Manages the execution and lifecycle of trading strategies.
///
/// This struct is responsible for initializing strategies with their respective settings and
/// algorithm parameters, starting and stopping strategy execution,
/// and providing summaries of strategy performance.

pub struct Strategy {
    pub id: StrategyId,
    pub symbol: String,
    pub name: String,
    settings: StrategySettings,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
    pub algorithm: ArcMutex<Box<dyn Algorithm>>,
    start_time: Option<String>,
    end_time: Option<String>,
    kline_manager: ArcMutex<StrategyKlineManager>,
    running: bool,
}

impl Strategy {
    /// Instantiates a new trading strategy with specified parameters.
    ///
    /// # Arguments
    ///
    /// * `strategy_name` - Name of the strategy.
    /// * `symbol` - The trading symbol the strategy operates on.
    /// * `interval` - The time interval between market data points the strategy uses.
    /// * `strategy_tx` - A channel for sending signal messages generated by the strategy.
    /// * `market` - Shared access to market data.
    /// * `settings` - Configuration settings for the strategy.
    /// * `algorithm_params` - Parameters for the algorithm used by the strategy.
    ///
    /// # Returns
    ///
    /// A result containing the new `Strategy` instance or an `AlgoError` if an error occurs.

    pub fn new(
        strategy_name: &str,
        symbol: &str,
        strategy_tx: ArcSender<SignalMessage>,
        market: ArcMutex<Market>,
        settings: StrategySettings,
        algorithm_params: Value,
    ) -> Result<Self, AlgoError> {
        let algorithm = AlgoBuilder::build_algorithm(strategy_name, algorithm_params)?;

        Ok(Self {
            id: Uuid::new_v4(),
            name: strategy_name.to_string(),
            market,
            symbol: symbol.to_string(),
            strategy_tx,
            algorithm: ArcMutex::new(algorithm),
            settings,
            start_time: None,
            end_time: None,
            kline_manager: ArcMutex::new(StrategyKlineManager::new()),
            running: false,
        })
    }

    /// Starts the execution of the strategy in an asynchronous task.
    ///
    /// # Returns
    ///
    /// A handle to the spawned asynchronous task running the strategy.

    pub async fn start(&mut self) -> JoinHandle<()> {
        self.running = true;
        self.start_time = Some(timestamp_to_string(generate_ts()));
        // let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();
        let algorithm = self.algorithm.clone();
        let interval = self.settings.interval.clone();

        let market = self.market.clone();
        let kline_manager = self.kline_manager.clone();

        tokio::spawn(async move {
            // let market = market.clone();
            // wait until last 5 seconds of minute, to ensure getting latest kline
            // data from market, ie. each request for fresh kline will
            // the no older than last minute + 55 seconds, very close
            // to any kline interval closing time
            let next_minute_minus_5_sec =
                (floor_mili_ts(generate_ts(), MIN_AS_MILI) + MIN_AS_MILI) - SEC_AS_MILI * 5;
            loop {
                let now = generate_ts();
                if now > next_minute_minus_5_sec {
                    break;
                } else {
                    time::sleep(Duration::from_secs(1)).await;
                }
            }

            loop {
                // wait for duration of strategy interval first,
                // to ensure at least one kline of data is populated in the market
                time::sleep(interval.to_duration()).await;

                // get the latest kline from the market
                let kline = market.lock().await.last_kline(&symbol, interval).await;

                // perform some house keeping with klines before evaluating the data
                // check kline is fresh otherwise continue to next interval

                if let Some(kline) = &kline {
                    if kline_manager.lock().await.must_continue(kline) {
                        continue;
                    }
                }

                // get trades within the span of the kline open_time and close_time
                let algo_needs_trades = algorithm.lock().await.needs_trades();

                // only get trades if needed by the algorithm
                let trades = if algo_needs_trades {
                    match &kline {
                        Some(kline) => {
                            let trades = match market
                                .lock()
                                .await
                                .trade_data_range(
                                    &symbol,
                                    // get 5 seconds in passed to ensure all trades
                                    Some(kline.open_time - SEC_AS_MILI * 5),
                                    Some(kline.close_time),
                                    None,
                                )
                                .await
                            {
                                Some(trade_data) => trade_data.trades(),
                                None => vec![],
                            };
                            trades
                        }
                        None => vec![],
                    }
                } else {
                    vec![]
                };

                if let Some(kline) = kline {
                    // ---
                    // Main evaluation done here
                    // ---
                    info!("Trades to be passed into evaluate method, {:?}", trades);
                    let order_side = algorithm.lock().await.evaluate(kline.clone(), &trades);

                    let order_side = match order_side {
                        AlgoEvalResult::Buy => OrderSide::Buy,
                        AlgoEvalResult::Sell => OrderSide::Sell,
                        AlgoEvalResult::Ignore => {
                            continue;
                        }
                    };

                    let signal = SignalMessage {
                        strategy_id: id,
                        order_side,
                        symbol: symbol.clone(),
                        price: kline.close,
                        is_back_test: false,
                        timestamp: kline.close_time,
                    };

                    if strategy_tx.is_closed() {
                        break;
                    }

                    // send signal back to bot
                    if let Err(e) = strategy_tx.send(signal) {
                        log::warn!("Unable to send signal back to RaderBot, {e}")
                    }
                } else {
                    continue;
                };
            }
        })
    }

    /// Stops the execution of the strategy and optionally closes all open positions associated with it.
    ///
    /// # Arguments
    ///
    /// * `account` - Shared access to the trading account for managing positions.
    /// * `close_positions` - Whether to close all open positions associated with this strategy.
    ///
    /// # Returns
    ///
    /// A summary of the strategy's performance including trades, positions, and profit.

    pub async fn stop(
        &mut self,
        account: ArcMutex<Account>,
        close_positions: bool,
    ) -> StrategySummary {
        let account = account.clone();
        // Get all positions associated with the strategy
        let positions: Vec<Position> = account
            .lock()
            .await
            .strategy_positions(self.id)
            .iter()
            .map(|&p| p.clone())
            .collect();

        // Close all positions on account attached to this strategy
        if close_positions {
            for position in positions {
                if let Some(close_price) = self
                    .market
                    .clone()
                    .lock()
                    .await
                    .last_price(&position.symbol)
                    .await
                {
                    account
                        .lock()
                        .await
                        .close_position(position.id, close_price)
                        .await;
                }
            }
        }

        let (positions, trades) = account.lock().await.strategy_positions_trades(self.id);

        self.end_time = Some(timestamp_to_string(generate_ts()));
        self.running = false;

        self.calc_summary(&trades, &positions).await
    }

    /// Generates a summary of the strategy's performance.
    ///
    /// # Arguments
    ///
    /// * `account` - Shared access to the trading account for accessing positions and trades.
    ///
    /// # Returns
    ///
    /// A summary of the strategy's performance including trades, positions, and profit.

    pub async fn summary(&self, account: ArcMutex<Account>) -> StrategySummary {
        let (positions, trades) = account.lock().await.strategy_positions_trades(self.id);
        self.calc_summary(&trades, &positions).await
    }

    /// Retrieves the settings for the strategy.
    ///
    /// # Returns
    ///
    /// The settings currently configured for the strategy.

    pub fn settings(&self) -> StrategySettings {
        self.settings.clone()
    }

    /// Updates the settings for the strategy.
    ///
    /// # Arguments
    ///
    /// * `settings` - The new settings to apply to the strategy.

    pub fn change_settings(&mut self, settings: StrategySettings) {
        self.settings = settings;
    }

    /// Gets the algorithm parameters used by the strategy.
    ///
    /// # Returns
    ///
    /// The current parameters of the algorithm as a JSON `Value`.

    pub async fn get_algorithm_params(&self) -> impl Serialize {
        self.algorithm.lock().await.get_params().clone()
    }

    /// Sets the parameters for the algorithm used by the strategy.
    ///
    /// # Arguments
    ///
    /// * `params` - The new parameters for the algorithm as a JSON `Value`.
    ///
    /// # Returns
    ///
    /// A result indicating success or containing an `AlgoError`.

    pub async fn set_algorithm_params(&self, params: Value) -> Result<(), AlgoError> {
        self.algorithm.lock().await.set_params(params)
    }

    /// Provides information about the strategy including its identifier, name, and configuration.
    ///
    /// # Returns
    ///
    /// An instance of `StrategyInfo` containing details about the strategy.

    pub async fn info(&self) -> StrategyInfo {
        StrategyInfo {
            id: self.id,
            name: self.name.clone(),
            settings: self.settings.clone(),
            params: self.algorithm.lock().await.get_params().clone(),
            symbol: self.symbol.clone(),
            interval: self.settings.interval.clone(),
            running: self.running,
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
        }
    }

    // ---
    // Private Methods
    // ---

    /// Calculates the summary of the strategy's performance including profit, drawdown, trade counts, and more.
    ///
    /// This private method aggregates the results of the strategy's trades and positions to compute key performance
    /// indicators such as total profit, maximum drawdown, and trade counts. It's used internally to generate
    /// a comprehensive summary of the strategy's outcome after its execution.
    ///
    /// # Arguments
    ///
    /// * `trades` - A reference to a vector of `TradeTx` instances representing executed trades.
    /// * `positions` - A reference to a vector of `Position` instances representing open positions.
    ///
    /// # Returns
    ///
    /// Returns a `StrategySummary` containing detailed performance metrics of the strategy.

    async fn calc_summary(
        &self,
        trades: &Vec<TradeTx>,
        positions: &Vec<Position>,
    ) -> StrategySummary {
        let max_profit = Strategy::calc_max_profit(&trades);
        let max_drawdown = Strategy::calc_max_drawdown(&trades);
        let long_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Buy);
        let short_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Sell);
        let profit: f64 = Strategy::calc_profit(&trades);

        let start_price = match self
            .kline_manager
            .lock()
            .await
            .get_kline(FirstLastEnum::First)
        {
            Some(kline) => kline.open,
            None => 0.0,
        };
        let end_price = match self
            .kline_manager
            .lock()
            .await
            .get_kline(FirstLastEnum::Last)
        {
            Some(kline) => kline.close,
            None => 0.0,
        };

        StrategySummary {
            info: self.info().await,
            profit: profit,
            trades: trades.clone(),
            positions: positions.clone(),
            long_trade_count,
            short_trade_count,
            // signals,
            // // buy_signal_count,
            // sell_signal_count,
            symbol: self.symbol.to_string(),
            period_end_price: end_price,
            period_start_price: start_price,
            max_drawdown,
            max_profit,
        }
    }

    // ---
    // Static Methods
    // ---

    /// Computes the maximum profit achieved by the strategy.
    ///
    /// This static method calculates the highest cumulative profit across all trades executed by the strategy.
    /// It iterates through the trades, summing up the profits, and tracking the highest value reached.
    ///
    /// # Arguments
    ///
    /// * `trades` - A reference to a vector of `TradeTx` instances representing executed trades.
    ///
    /// # Returns
    ///
    /// Returns a `f64` representing the maximum cumulative profit achieved.

    pub fn calc_max_profit(trades: &Vec<TradeTx>) -> f64 {
        let mut max_balance = 0.0;
        let mut current_balance = 0.0;

        for trade_tx in trades {
            let profit = trade_tx.calc_profit();
            current_balance += profit;

            if current_balance > max_balance {
                max_balance = current_balance;
            }
        }

        max_balance
    }

    /// Computes the maximum drawdown experienced by the strategy.
    ///
    /// This static method calculates the largest drop from peak to trough in the cumulative profit across
    /// all trades executed by the strategy, representing the largest loss from a peak to a trough.
    ///
    /// # Arguments
    ///
    /// * `trades` - A reference to a vector of `TradeTx` instances representing executed trades.
    ///
    /// # Returns
    ///
    /// Returns a `f64` representing the maximum drawdown experienced.

    pub fn calc_max_drawdown(trades: &Vec<TradeTx>) -> f64 {
        let mut min_balance = if trades.is_empty() { 0.0 } else { f64::MAX };
        let mut current_balance = 0.0;

        let mut trades = trades.clone();
        trades.sort_by(|a, b| a.close_time.cmp(&b.close_time));

        for trade_tx in trades {
            let profit = trade_tx.calc_profit();
            current_balance += profit;

            if current_balance <= min_balance {
                min_balance = current_balance;
            }
        }

        min_balance
    }

    /// Calculates the number of trades executed by the strategy for a specific order side.
    ///
    /// This static method counts the number of trades executed by the strategy that match the specified order side
    /// (e.g., long or short).
    ///
    /// # Arguments
    ///
    /// * `trades` - A reference to a vector of `TradeTx` instances representing executed trades.
    /// * `order_side` - The `OrderSide` (e.g., Buy or Sell) to filter the trades by.
    ///
    /// # Returns
    ///
    /// Returns a `usize` representing the count of trades for the specified order side.

    pub fn calc_trade_count(trades: &Vec<TradeTx>, order_side: OrderSide) -> usize {
        trades
            .iter()
            .filter(|trade| trade.position.order_side == order_side)
            .count()
    }

    /// Calculates the total profit or loss achieved by the strategy.
    ///
    /// This static method sums up the profit or loss from all trades executed by the strategy to determine
    /// the overall financial outcome.
    ///
    /// # Arguments
    ///
    /// * `trades` - A reference to a vector of `TradeTx` instances representing executed trades.
    ///
    /// # Returns
    ///
    /// Returns a `f64` representing the total profit or loss.

    pub fn calc_profit(trades: &Vec<TradeTx>) -> f64 {
        trades.iter().map(|trade| trade.calc_profit()).sum()
    }
}

/// Contains information about a trading strategy including its configuration and state.
///
/// This struct provides detailed information about a strategy, such as its unique identifier,
/// name, trading symbol, interval for trading signals, strategy settings, and operational
/// parameters. It also tracks the strategy's running state and the time range of its operation.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategyInfo {
    pub id: StrategyId,
    pub name: String,
    pub symbol: String,
    pub interval: Interval,
    pub settings: StrategySettings,
    pub params: Value,
    pub running: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

/// Provides default values for `StrategyInfo`.
///
/// This implementation ensures that a new instance of `StrategyInfo` starts with sensible defaults,
/// facilitating the creation of strategy instances without requiring initial values for every field.

impl Default for StrategyInfo {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "".to_string(),
            symbol: "".to_string(),
            interval: Interval::Invalid,
            settings: StrategySettings::default(),
            params: json!({}),
            start_time: None,
            end_time: None,
            running: false,
        }
    }
}

/// Configuration settings for a trading strategy.
///
/// This struct defines essential settings that control the execution of a trading strategy,
/// including the maximum number of open orders, margin usage, leverage, and an optional stop loss.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySettings {
    pub interval: Interval,
    pub max_open_orders: u32,
    pub margin_usd: f64,
    pub leverage: u32,
    pub stop_loss: Option<f64>,
}

/// Provides default values for `StrategySettings`.
///
/// Ensures that a new instance of `StrategySettings` starts with default values, making it easier
/// to instantiate a strategy without specifying each setting explicitly.

impl Default for StrategySettings {
    fn default() -> Self {
        Self {
            interval: Interval::Min1,
            max_open_orders: 1,
            margin_usd: 100.0,
            leverage: 1,
            stop_loss: None,
        }
    }
}

/// Summarizes trading strategy's execution results and statistics.
///
/// Includes details about performance, such as profit, trades, positions, trade counts, and price
/// information at the start and end of execution. Also covers maximum profit and drawdown
/// experienced.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySummary {
    pub info: StrategyInfo,
    pub profit: f64,
    pub trades: Vec<TradeTx>,
    pub positions: Vec<Position>,
    // pub signals: Vec<SignalMessage>,
    pub long_trade_count: usize,
    pub short_trade_count: usize,
    // pub buy_signal_count: usize,
    // pub sell_signal_count: usize,
    pub period_start_price: f64,
    pub period_end_price: f64,
    pub symbol: String,
    pub max_drawdown: f64,
    pub max_profit: f64,
}

/// Sets default values for `StrategySummary`.
///
/// Initiates with zero values and empty lists for trades and positions, aiding summary creation
/// post-execution.

impl Default for StrategySummary {
    fn default() -> Self {
        Self {
            info: StrategyInfo::default(),
            profit: 0.0,
            trades: vec![],
            positions: vec![],
            long_trade_count: 0,
            short_trade_count: 0,
            period_start_price: 0.0,
            period_end_price: 0.0,
            symbol: "".to_string(),
            max_drawdown: 0.0,
            max_profit: 0.0,
        }
    }
}

/// Manages k-line data for a strategy's execution period.
///
/// Tracks the initial and final k-lines, providing strategies with price data at the beginning
/// and end of their execution window.

pub struct StrategyKlineManager {
    pub last_kline: Option<Kline>,
    pub first_kline: Option<Kline>,
}

impl StrategyKlineManager {
    /// Initializes with no k-line data, ready to track initial and final k-lines during strategy execution.

    pub fn new() -> Self {
        Self {
            last_kline: None,
            first_kline: None,
        }
    }

    /// Retrieves a k-line based on specified criterion (first or last).
    ///
    /// * `first_last` - Whether to fetch the initial or final k-line recorded.
    ///
    /// Returns an `Option<Kline>` which is `Some` if the k-line exists, or `None` if not.

    pub fn get_kline(&self, first_last: FirstLastEnum) -> Option<Kline> {
        match first_last {
            FirstLastEnum::First => self.first_kline.clone(),
            FirstLastEnum::Last => self.last_kline.clone(),
        }
    }

    pub fn must_continue(&mut self, kline: &Kline) -> bool {
        let mut must_continue = false;

        if let Some(last_kline) = &self.last_kline {
            if last_kline.open_time == kline.open_time {
                must_continue = true
            }
        } else {
            self.last_kline = Some(kline.clone());
        }

        if self.first_kline.is_none() {
            self.first_kline = Some(kline.clone());
        }

        must_continue
    }
}
