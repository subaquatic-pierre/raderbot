use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinHandle;
use tokio::time;

use crate::account::trade::TradeTx;
use crate::utils::number::generate_random_id;
use crate::{
    account::trade::OrderSide,
    market::{
        market::Market,
        types::{ArcMutex, ArcSender},
    },
    strategy::algorithm::Algorithm,
};

use super::algorithm::AlgorithmBuilder;
use super::types::{AlgorithmError, AlgorithmEvalResult, SignalMessage};

pub type StrategyId = u32;

pub struct Strategy {
    pub id: StrategyId,
    pub symbol: String,
    interval: String,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
    pub algorithm: ArcMutex<Box<dyn Algorithm>>,
    settings: StrategySettings,
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
            id: generate_random_id(),
            market,
            interval: interval.to_string(),
            symbol: symbol.to_string(),
            strategy_tx,
            algorithm: ArcMutex::new(algorithm),
            settings,
        })
    }

    pub async fn start(&self) -> JoinHandle<()> {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();
        let algorithm = self.algorithm.clone();
        let interval_str = self.interval.clone();
        let interval_duration = algorithm.lock().await.interval();

        tokio::spawn(async move {
            loop {
                let market = market.clone();

                if let Some(kline) = market.lock().await.kline_data(&symbol, &interval_str).await {
                    let order_side = algorithm.lock().await.evaluate(kline.clone());

                    let order_side = match order_side {
                        AlgorithmEvalResult::Buy => OrderSide::Buy,
                        AlgorithmEvalResult::Sell => OrderSide::Sell,
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

    pub fn settings(&self) -> StrategySettings {
        self.settings.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StrategySettings {
    pub max_open_orders: u32,
    pub margin_usd: f64,
    pub leverage: u32,
}

impl Default for StrategySettings {
    fn default() -> Self {
        Self {
            max_open_orders: 1,
            margin_usd: 100.0,
            leverage: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StrategyResult {
    pub profit: f64,
    pub trade_txs: Vec<TradeTx>,
    // pub signals: Vec<SignalMessage>,
    pub buy_count: usize,
    pub sell_count: usize,
    // pub buy_signal_count: usize,
    // pub sell_signal_count: usize,
    pub period_start_price: f64,
    pub period_end_price: f64,
    pub symbol: String,
    pub max_drawdown: f64,
    pub max_profit: f64,
}
