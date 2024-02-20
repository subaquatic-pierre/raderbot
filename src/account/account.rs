use actix_web::rt::signal;
use serde_json::Value;
use std::collections::hash_map::Values;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::time::sleep;
use uuid::Uuid;

use crate::strategy::strategy::StrategyId;
use crate::utils::time::generate_ts;
use crate::{
    account::trade::{OrderSide, Position},
    exchange::api::ExchangeApi,
    market::{market::Market, types::ArcMutex},
};

use super::trade::{PositionId, TradeTx};

pub struct Account {
    market: ArcMutex<Market>,
    positions: HashMap<PositionId, Position>,
    trade_txs: HashMap<Uuid, TradeTx>,
    exchange_api: Arc<Box<dyn ExchangeApi>>,
}

impl Account {
    pub async fn new(market: ArcMutex<Market>, exchange_api: Arc<Box<dyn ExchangeApi>>) -> Self {
        let _self = Self {
            market,
            exchange_api,
            positions: HashMap::new(),
            trade_txs: HashMap::new(),
        };

        // _self.init(init_listeners).await;
        _self
    }

    pub async fn open_position(
        &mut self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        stop_loss: Option<f64>,
        open_price: f64,
    ) -> Option<&Position> {
        if let Ok(mut position) = self
            .exchange_api
            .open_position(symbol, margin_usd, leverage, order_side, open_price)
            .await
        {
            position.set_stop_loss(stop_loss);
            let position_id = position.id;
            // insert new position into account positions
            self.positions.insert(position.id, position);

            return self.positions.get(&position_id);
        };

        None
    }

    pub async fn close_position(
        &mut self,
        position_id: PositionId,
        close_price: f64,
    ) -> Option<&TradeTx> {
        if let Some(position) = self.positions.get(&position_id).cloned() {
            if let Ok(trade_tx) = self
                .exchange_api
                .close_position(position.clone(), close_price)
                .await
            {
                self.positions.remove(&position.id);

                let trade_tx_id = trade_tx.id;

                self.trade_txs.insert(trade_tx_id, trade_tx);

                return self.trade_txs.get(&trade_tx_id);
            };
        };

        None
    }

    pub async fn open_positions(&self) -> Values<'_, PositionId, Position> {
        self.positions.values()
    }

    pub async fn trade_txs(&self) -> Values<'_, Uuid, TradeTx> {
        self.trade_txs.values()
    }

    pub async fn strategy_open_positions(&self, strategy_id: StrategyId) -> Vec<Position> {
        let mut positions = vec![];
        for pos in self.positions.values() {
            if let Some(pos_strategy_id) = pos.strategy_id {
                if pos_strategy_id == strategy_id {
                    positions.push(pos.clone())
                }
            }
        }
        positions
    }

    // ---
    // Private Methods
    // ---

    // async fn init(&self, init_listeners: bool) {
    //     // monitor positions stop loss
    //     if init_listeners {
    //         self.init_stop_loss_monitor().await
    //     }
    // }

    // async fn init_stop_loss_monitor(&self) {
    //     let market = self.market.clone();
    //     let positions = self.positions.clone();

    //     tokio::spawn(async move {
    //         loop {
    //             for (_id, position) in positions.lock().await.iter() {
    //                 // get last price from position

    //                 if let Some(last_price) = market
    //                     .lock()
    //                     .await
    //                     .last_price(&position.symbol.to_string())
    //                     .await
    //                 {
    //                     if let Some(stop_loss) = position.stop_loss {
    //                         if last_price < stop_loss {
    //                             // TODO: close position if stop loss hit
    //                             println!("Stop loss hit")
    //                         }
    //                     }
    //                 }

    //                 // get last price from position

    //                 // check if
    //             }

    //             // only check stop loss hit every 1min
    //             sleep(Duration::from_secs(60)).await;
    //         }
    //     });
    // }
}
