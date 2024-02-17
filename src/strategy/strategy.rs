use std::time::Duration;

use actix_web::rt::signal;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time;

use crate::market::types::ArcReceiver;
use crate::utils::channel::build_arc_channel;
use crate::utils::number::{gen_random_milliseconds, generate_random_id};
use crate::{
    account::trade::OrderSide,
    app::INTERVAL,
    exchange::api::ExchangeApi,
    market::{
        market::Market,
        types::{ArcMutex, ArcSender},
    },
};

use super::signal::SignalMessage;

#[derive(Clone)]
pub struct Strategy {
    pub id: String,
    symbol: String,
    market: ArcMutex<Market>,
    strategy_tx: ArcSender<SignalMessage>,
}

impl Strategy {
    pub fn new(
        symbol: &str,
        strategy_tx: ArcSender<SignalMessage>,
        market: ArcMutex<Market>,
    ) -> Self {
        Self {
            id: generate_random_id().to_string(),
            market,
            symbol: symbol.to_string(),
            strategy_tx,
        }
    }

    pub async fn start(&self) -> JoinHandle<()> {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();

        tokio::spawn(async move {
            loop {
                let market = market.clone();
                let price = market.lock().await.last_price(&symbol).await;

                let signal = SignalMessage {
                    strategy_id: id.clone(),
                    order_side: OrderSide::Buy,
                    symbol: symbol.clone(),
                    price,
                };

                if strategy_tx.is_closed() {
                    break;
                }

                if let Err(e) = strategy_tx.send(signal) {
                    log::warn!("Unable to send signal back to RaderBot, {e}")
                }

                time::sleep(INTERVAL).await;
            }
        })
    }
}
