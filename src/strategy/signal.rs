use std::{collections::HashMap, sync::Arc};

use log::info;

use crate::{
    account::{
        account::Account,
        trade::{OrderSide, Position},
    },
    market::{market::Market, types::ArcMutex},
};

use super::{
    strategy::{StrategyId, StrategySettings},
    types::SignalMessage,
};

pub struct SignalManager {
    account: ArcMutex<Account>,
    market: ArcMutex<Market>,
    active_strategy_settings: HashMap<StrategyId, StrategySettings>,
}

impl SignalManager {
    pub fn new(account: ArcMutex<Account>, market: ArcMutex<Market>) -> Self {
        Self {
            account,
            market,
            active_strategy_settings: HashMap::new(),
        }
    }

    pub async fn handle_signal(&mut self, signal: SignalMessage) {
        let active_positions = self
            .account
            .lock()
            .await
            .strategy_open_positions(signal.strategy_id)
            .await;

        if let Some(strategy_settings) = self.active_strategy_settings.get(&signal.strategy_id) {
            // get last open position
            if let Some(last) = active_positions.last() {
                // if last is buy and signal is sell then close all position
                if signal.order_side != last.order_side {
                    if let Some(last_price) =
                        self.market.lock().await.last_price(&signal.symbol).await
                    {
                        let close_price = if signal.is_back_test {
                            signal.price
                        } else {
                            last_price
                        };
                        for position in &active_positions {
                            self.account
                                .lock()
                                .await
                                .close_position(position.id, close_price)
                                .await;
                        }
                    }
                }

                if signal.order_side == last.order_side
                    && active_positions.len() < strategy_settings.max_open_orders as usize
                {
                    if signal.is_back_test {
                        self.account
                            .lock()
                            .await
                            .open_position(
                                &signal.symbol,
                                strategy_settings.margin_usd,
                                strategy_settings.leverage,
                                signal.order_side.clone(),
                                None,
                                signal.price,
                            )
                            .await;
                    } else {
                        if let Some(last_price) =
                            self.market.lock().await.last_price(&signal.symbol).await
                        {
                            self.account
                                .lock()
                                .await
                                .open_position(
                                    &signal.symbol,
                                    strategy_settings.margin_usd,
                                    strategy_settings.leverage,
                                    signal.order_side.clone(),
                                    None,
                                    last_price,
                                )
                                .await;
                        }
                    }
                }
            } else {
                if let Some(last_price) = self.market.lock().await.last_price(&signal.symbol).await
                {
                    self.account
                        .lock()
                        .await
                        .open_position(
                            &signal.symbol,
                            strategy_settings.margin_usd,
                            strategy_settings.leverage,
                            signal.order_side.clone(),
                            None,
                            last_price,
                        )
                        .await;
                }
            }
        }
        info!("{signal:?}");
    }

    pub fn add_strategy_settings(&mut self, strategy_id: u32, settings: StrategySettings) {
        self.active_strategy_settings.insert(strategy_id, settings);
    }

    pub fn remove_strategy_settings(&mut self, strategy_id: u32) {
        self.active_strategy_settings.remove(&strategy_id);
    }
}
