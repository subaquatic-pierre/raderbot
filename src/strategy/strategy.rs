use tokio::task::JoinHandle;
use tokio::time;

use crate::utils::number::generate_random_id;
use crate::{
    account::trade::OrderSide,
    app::INTERVAL,
    exchange::api::ExchangeApi,
    market::{
        market::Market,
        types::{ArcMutex, ArcSender},
    },
    strategy::algorithm::Algorithm,
};

use super::algorithm::AlgorithmEvalResult;
use super::signal::SignalMessage;

pub struct Strategy {
    pub id: String,
    symbol: String,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
    algorithm: ArcMutex<Box<dyn Algorithm>>,
}

impl Strategy {
    pub fn new(
        symbol: &str,
        strategy_tx: ArcSender<SignalMessage>,
        market: ArcMutex<Market>,
        algorithm: Box<dyn Algorithm>,
    ) -> Self {
        Self {
            id: generate_random_id().to_string(),
            market,
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

        tokio::spawn(async move {
            loop {
                let market = market.clone();
                let price = market.lock().await.ticker_data(&symbol).await;

                if let Some(ticker_data) = price {
                    let order_side = algorithm.lock().await.evaluate(ticker_data.clone());

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
                        price: ticker_data.ticker.last_price,
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
        })
    }
}
