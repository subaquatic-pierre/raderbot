use serde_json::Value;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::time::sleep;

use crate::{
    account::trade::{OrderSide, Position},
    exchange::api::ExchangeApi,
    market::{market::Market, types::ArcMutex},
};

pub struct Account {
    initial_balance: f64,
    market: ArcMutex<Market>,
    positions: ArcMutex<HashMap<String, Position>>,
    exchange_api: Arc<Box<dyn ExchangeApi>>,
}

impl Account {
    pub async fn new(
        market: ArcMutex<Market>,
        exchange_api: Arc<Box<dyn ExchangeApi>>,
        init_stop_loss_monitor: bool,
    ) -> Self {
        // let initial_balance = exchange_api
        //     .get_account_balance()
        //     .await
        //     .unwrap_or_else(|e| 0.0);
        let _self = Self {
            market,
            positions: ArcMutex::new(HashMap::new()),
            exchange_api,
            initial_balance: 0.0,
        };

        _self.init(init_stop_loss_monitor).await;
        _self
    }

    pub async fn open_position(
        &mut self,
        symbol: &str,
        margin: f64,
        leverage: u32,
        order_side: OrderSide,
        stop_loss: Option<f64>,
    ) -> Option<Value> {
        // TODO: start stream to update last_price on position
        // close position if stop loss hit
        let market = self.market.clone();
        let symbol = Arc::new(symbol.to_string());

        let positions = self.positions.clone();

        let last_price = market.lock().await.last_price(&symbol).await;

        let mut method_res = None;

        // only open position if market has last price for symbol
        if let Some(last_price) = last_price {
            let new_position =
                Position::new(&symbol, last_price, order_side, stop_loss, margin, leverage);

            let pos_clone = new_position.clone();

            // TODO: make api call to open position
            // if successful position open spawn thread to update last price
            if let Ok(res) = self
                .exchange_api
                .open_position(&pos_clone.symbol, pos_clone.order_side, pos_clone.quantity)
                .await
            {
                let position_id = "order_id";
                // insert new position into account positions
                positions
                    .lock()
                    .await
                    .insert(position_id.to_string(), new_position);

                // create arc of position id to use in last_price updater thread
                let position_id = Arc::new(position_id.to_string());

                method_res = Some(res);
            };
        };

        method_res
    }

    pub fn close_position(&mut self, _position_id: u64) {}

    pub async fn positions(&self) -> Vec<Position> {
        self.positions
            .lock()
            .await
            .iter()
            .map(|(_id, pos)| pos.clone())
            .collect()
    }

    pub async fn init(&self, init_stop_loss_monitor: bool) {
        // monitor positions stop loss
        if init_stop_loss_monitor {
            self.init_stop_loss_monitor().await
        }
    }

    async fn init_stop_loss_monitor(&self) {
        let market = self.market.clone();
        let positions = self.positions.clone();

        tokio::spawn(async move {
            loop {
                for (_id, position) in positions.lock().await.iter() {
                    // get last price from position

                    if let Some(last_price) = market
                        .lock()
                        .await
                        .last_price(&position.symbol.to_string())
                        .await
                    {
                        if let Some(stop_loss) = position.stop_loss {
                            if last_price < stop_loss {
                                // TODO: close position if stop loss hit
                                println!("Stop loss hit")
                            }
                        }
                    }

                    // get last price from position

                    // check if
                }

                // only check stop loss hit every 1min
                sleep(Duration::from_secs(60)).await;
            }
        });
    }
}
