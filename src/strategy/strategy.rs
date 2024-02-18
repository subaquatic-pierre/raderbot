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
                let kline_data = market
                    .lock()
                    .await
                    .kline_data(&symbol, &interval, None, None, None)
                    .await;

                if kline_data.is_none() {
                    time::sleep(INTERVAL).await;
                    continue;
                } else {
                    let kline_data = kline_data.unwrap();
                    if let Some(kline) = kline_data.klines.last() {
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

        let mut result = BackTestResult {
            strategy_name: self.strategy_name.to_string(),
            signals: vec![],
        };

        let market = market.clone();
        let kline_data = market
            .lock()
            .await
            .kline_data(&symbol, &interval, Some(from_ts), Some(to_ts), None)
            .await;

        if let Some(kline_data) = kline_data {
            for kline in kline_data.klines {
                let eval_result = algorithm.evaluate(kline.clone());

                if algorithm.data_points().len() > 20 {
                    break;
                }

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

        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackTestResult {
    pub strategy_name: String,
    pub signals: Vec<SignalMessage>,
}

impl BackTestResult {
    pub fn add_signal(&mut self, signal: SignalMessage) {
        self.signals.push(signal)
    }
}
