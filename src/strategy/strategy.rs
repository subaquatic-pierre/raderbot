use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time;

use crate::market::kline::Kline;
use crate::utils::number::generate_random_id;
use crate::{
    account::trade::OrderSide,
    bot::INTERVAL,
    exchange::api::ExchangeApi,
    market::{
        market::Market,
        types::{ArcMutex, ArcSender},
    },
    strategy::algorithm::Algorithm,
};

use super::types::{AlgorithmEvalResult, SignalMessage};

pub struct Strategy {
    pub id: String,
    strategy_name: String,
    symbol: String,
    interval: String,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
    algorithm: ArcMutex<Box<dyn Algorithm>>,
}

impl Strategy {
    pub fn new(
        strategy_name: &str,
        symbol: &str,
        interval: &str,
        strategy_tx: ArcSender<SignalMessage>,
        market: ArcMutex<Market>,
        algorithm: Box<dyn Algorithm>,
    ) -> Self {
        Self {
            id: generate_random_id().to_string(),
            market,
            interval: interval.to_string(),
            strategy_name: strategy_name.to_string(),
            symbol: symbol.to_string(),
            strategy_tx,
            algorithm: ArcMutex::new(algorithm),
        }
    }

    pub async fn start(&self) -> JoinHandle<()> {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();
        let algorithm = self.algorithm.clone();
        let interval = self.interval.clone();

        tokio::spawn(async move {
            loop {
                let market = market.clone();
                let kline_data = market.lock().await.kline_data(&symbol, &interval).await;

                if kline_data.is_none() {
                    time::sleep(INTERVAL).await;
                    continue;
                } else {
                    let kline = kline_data.unwrap();
                    let order_side = algorithm.lock().await.evaluate(kline.clone());

                    let order_side = match order_side {
                        AlgorithmEvalResult::Buy => OrderSide::Buy,
                        AlgorithmEvalResult::Sell => OrderSide::Sell,
                        AlgorithmEvalResult::Ignore => {
                            continue;
                        }
                    };

                    let signal = SignalMessage {
                        strategy_id: id.clone(),
                        order_side,
                        symbol: symbol.clone(),
                        price: kline.close,
                    };

                    if strategy_tx.is_closed() {
                        break;
                    }

                    if let Err(e) = strategy_tx.send(signal) {
                        log::warn!("Unable to send signal back to RaderBot, {e}")
                    }

                    time::sleep(INTERVAL).await;
                }
            }
        })
    }

    pub async fn run_back_test(&self, from_ts: u64, to_ts: u64) -> BackTestResult {
        let market = self.market.clone();
        let symbol = self.symbol.clone();
        let mut algorithm = self.algorithm.lock().await;
        let interval = self.interval.clone();

        let mut result = BackTestResult::new(&self.strategy_name);

        let market = market.clone();
        let kline_data = market
            .lock()
            .await
            .kline_data_range(&symbol, &interval, Some(from_ts), Some(to_ts), None)
            .await;

        if let Some(kline_data) = kline_data {
            for kline in kline_data.klines {
                let eval_result = algorithm.evaluate(kline.clone());

                let order_side = match eval_result {
                    AlgorithmEvalResult::Buy => OrderSide::Buy,
                    AlgorithmEvalResult::Sell => OrderSide::Sell,
                    AlgorithmEvalResult::Ignore => {
                        continue;
                    }
                };

                let signal = SignalMessage {
                    strategy_id: self.id.clone(),
                    order_side,
                    symbol: symbol.clone(),
                    price: kline.close.clone(),
                };

                result.add_signal(signal)
            }
        }

        result.calculate_strategy_profit_loss();

        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackTestResult {
    pub strategy_name: String,
    pub signals: Vec<SignalMessage>,
    pub balance: f64,
    pub positions: i32,
    pub buy_count: u32,
    pub sell_count: u32,
}

impl BackTestResult {
    pub fn new(strategy_name: &str) -> Self {
        Self {
            strategy_name: strategy_name.to_string(),
            signals: vec![],
            balance: 10_000.0,
            positions: 0,
            buy_count: 0,
            sell_count: 0,
        }
    }
    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }

    pub fn calculate_strategy_profit_loss(&mut self) {
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
    }
}
