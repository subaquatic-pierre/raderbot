use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time;
use uuid::Uuid;

use crate::{
    account::{
        self,
        account::Account,
        trade::{OrderSide, Position, TradeTx},
    },
    market::{
        kline::Kline,
        market::Market,
        types::{ArcMutex, ArcSender},
    },
    strategy::algorithm::{Algorithm, AlgorithmBuilder},
    utils::time::{generate_ts, timestamp_to_string},
};

use super::types::{AlgorithmError, AlgorithmEvalResult, FirstLastEnum, SignalMessage};

pub type StrategyId = Uuid;

pub struct Strategy {
    pub id: StrategyId,
    pub symbol: String,
    pub name: String,
    interval: String,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
    pub algorithm: ArcMutex<Box<dyn Algorithm>>,
    settings: StrategySettings,
    start_time: Option<String>,
    end_time: Option<String>,
    kline_manager: ArcMutex<StrategyKlineManager>,
    running: bool,
}

impl Strategy {
    pub fn new(
        strategy_name: &str,
        symbol: &str,
        interval: &str,
        strategy_tx: ArcSender<SignalMessage>,
        market: ArcMutex<Market>,
        settings: StrategySettings,
        algorithm_params: Value,
    ) -> Result<Self, AlgorithmError> {
        let algorithm =
            AlgorithmBuilder::build_algorithm(strategy_name, interval, algorithm_params)?;

        Ok(Self {
            id: Uuid::new_v4(),
            name: strategy_name.to_string(),
            market,
            interval: interval.to_string(),
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

    pub async fn start(&mut self) -> JoinHandle<()> {
        self.running = true;
        self.start_time = Some(timestamp_to_string(generate_ts()));
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();
        let algorithm = self.algorithm.clone();
        let interval_str = self.interval.clone();
        let interval_duration = algorithm.lock().await.interval();
        let kline_manager = self.kline_manager.clone();

        tokio::spawn(async move {
            loop {
                let market = market.clone();

                if let Some(kline) = market.lock().await.kline_data(&symbol, &interval_str).await {
                    // check kline is fresh otherwise continue to next interval
                    let last_kline = kline_manager.lock().await.get_kline(FirstLastEnum::Last);

                    if let Some(last_kline) = &last_kline {
                        if last_kline.open_time == kline.open_time {
                            continue;
                        }
                    } else {
                        kline_manager
                            .lock()
                            .await
                            .set_kline(kline.clone(), FirstLastEnum::Last);
                    }

                    // if first kline is empty, set it
                    let first_kline = kline_manager.lock().await.get_kline(FirstLastEnum::First);

                    if first_kline.is_none() {
                        kline_manager
                            .lock()
                            .await
                            .set_kline(kline.clone(), FirstLastEnum::First);
                    }

                    let order_side = algorithm.lock().await.evaluate(kline.clone());

                    let order_side = match order_side {
                        AlgorithmEvalResult::Long => OrderSide::Long,
                        AlgorithmEvalResult::Short => OrderSide::Short,
                        AlgorithmEvalResult::Ignore => {
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

                    if let Err(e) = strategy_tx.send(signal) {
                        log::warn!("Unable to send signal back to RaderBot, {e}")
                    }
                };

                time::sleep(interval_duration).await;
            }
        })
    }

    pub async fn stop(
        &mut self,
        account: ArcMutex<Account>,
        close_positions: bool,
    ) -> StrategySummary {
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
                if let Some(close_price) =
                    self.market.lock().await.last_price(&position.symbol).await
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

    pub async fn summary(&self, account: ArcMutex<Account>) -> StrategySummary {
        let (positions, trades) = account.lock().await.strategy_positions_trades(self.id);
        self.calc_summary(&trades, &positions).await
    }

    pub fn settings(&self) -> StrategySettings {
        self.settings.clone()
    }

    pub fn change_settings(&mut self, settings: StrategySettings) {
        self.settings = settings;
    }

    pub async fn get_algorithm_params(&self) -> Value {
        self.algorithm.lock().await.get_params().clone()
    }

    pub async fn set_algorithm_params(&self, params: Value) -> Result<(), AlgorithmError> {
        self.algorithm.lock().await.set_params(params)
    }

    pub async fn info(&self) -> StrategyInfo {
        StrategyInfo {
            id: self.id,
            name: self.name.clone(),
            settings: self.settings.clone(),
            params: self.algorithm.lock().await.get_params().clone(),
            symbol: self.symbol.clone(),
            interval: self.interval.clone(),
            running: self.running,
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
        }
    }

    // ---
    // Private Methods
    // ---

    async fn calc_summary(
        &self,
        trades: &Vec<TradeTx>,
        positions: &Vec<Position>,
    ) -> StrategySummary {
        let max_profit = Strategy::calc_max_profit(&trades);
        let max_drawdown = Strategy::calc_max_drawdown(&trades);
        let long_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Long);
        let short_trade_count = Strategy::calc_trade_count(&trades, OrderSide::Short);
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

    pub fn calc_max_drawdown(trades: &Vec<TradeTx>) -> f64 {
        let mut min_balance = f64::MAX;
        let mut current_balance = 0.0;

        for trade_tx in trades {
            let profit = trade_tx.calc_profit();
            current_balance += profit;

            if current_balance < min_balance {
                min_balance = current_balance;
            }
        }

        min_balance
    }

    pub fn calc_trade_count(trades: &Vec<TradeTx>, order_side: OrderSide) -> usize {
        trades
            .iter()
            .filter(|trade| trade.position.order_side == order_side)
            .count()
    }

    pub fn calc_profit(trades: &Vec<TradeTx>) -> f64 {
        trades.iter().map(|trade| trade.calc_profit()).sum()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategyInfo {
    pub id: StrategyId,
    pub name: String,
    pub symbol: String,
    pub interval: String,
    pub settings: StrategySettings,
    pub params: Value,
    pub running: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

impl Default for StrategyInfo {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "".to_string(),
            symbol: "".to_string(),
            interval: "".to_string(),
            settings: StrategySettings::default(),
            params: json!({}),
            start_time: None,
            end_time: None,
            running: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySettings {
    pub max_open_orders: u32,
    pub margin_usd: f64,
    pub leverage: u32,
    pub stop_loss: Option<f64>,
}

impl Default for StrategySettings {
    fn default() -> Self {
        Self {
            max_open_orders: 1,
            margin_usd: 100.0,
            leverage: 1,
            stop_loss: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

pub struct StrategyKlineManager {
    pub last_kline: Option<Kline>,
    pub first_kline: Option<Kline>,
}

impl StrategyKlineManager {
    pub fn new() -> Self {
        Self {
            last_kline: None,
            first_kline: None,
        }
    }

    pub fn get_kline(&self, first_last: FirstLastEnum) -> Option<Kline> {
        match first_last {
            FirstLastEnum::First => self.first_kline.clone(),
            FirstLastEnum::Last => self.last_kline.clone(),
        }
    }

    pub fn set_kline(&mut self, kline: Kline, first_last: FirstLastEnum) {
        match first_last {
            FirstLastEnum::First => self.first_kline = Some(kline),
            FirstLastEnum::Last => self.last_kline = Some(kline),
        };
    }
}
