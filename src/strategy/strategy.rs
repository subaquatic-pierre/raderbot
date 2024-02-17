use std::thread;
use std::{marker, sync::Arc, thread::JoinHandle};

use actix_web::rt::signal;
use serde::{Deserialize, Serialize};
use tokio::time;

use crate::market::types::ArcReceiver;
use crate::utils::channel::build_arc_channel;
use crate::utils::number::generate_random_id;
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
    id: String,
    symbol: String,
    pub strategy_chan: (ArcSender<SignalMessage>, ArcReceiver<SignalMessage>),
    market: ArcMutex<Market>,
}

impl Strategy {
    pub fn new(symbol: &str, market: ArcMutex<Market>) -> Self {
        let strategy_chan = build_arc_channel::<SignalMessage>();
        Self {
            id: generate_random_id().to_string(),
            market,
            symbol: symbol.to_string(),
            strategy_chan,
        }
    }

    pub async fn start(&self) -> (String, ArcReceiver<SignalMessage>) {
        let market = self.market.clone();
        let strategy_tx = self.strategy_chan.0.clone();

        let id = self.id.clone();
        let symbol = self.symbol.clone();

        tokio::spawn(async move {
            loop {
                let market = market.lock().await;
                let price = market.last_price(&symbol).await;

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
        });

        (self.id.clone(), self.strategy_chan.1.clone())
    }
}
