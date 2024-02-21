use std::collections::HashMap;

use log::info;

use crate::{
    account::{account::Account, trade::Position},
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
        info!("Signal: {signal:?}");
        let active_positions: Vec<Position> = self
            .account
            .lock()
            .await
            .strategy_positions(signal.strategy_id)
            .iter()
            .map(|&el| el.clone())
            .collect();

        // get trigger price used in all account actions
        // from market or signal if signal.is_back_test
        let trigger_price = if signal.is_back_test {
            Some(signal.price)
        } else {
            self.market.lock().await.last_price(&signal.symbol).await
        };

        info!("Trigger Price: {signal:?}");

        if self
            .active_strategy_settings
            .get(&signal.strategy_id)
            .is_none()
        {
            return;
        }

        // SAFETY: None check above, used to make method more clear
        let settings = self
            .active_strategy_settings
            .get(&signal.strategy_id)
            .unwrap();

        info!("Strategy Settings: {settings:?}");

        // get last open position
        if let Some(last) = active_positions.last() {
            // if last.signal is different to new signal then close all positions
            if signal.order_side != last.order_side {
                if let Some(close_price) = trigger_price {
                    for position in &active_positions {
                        self.account
                            .lock()
                            .await
                            .close_position(position.id, close_price)
                            .await;
                    }
                }

            // if is same signal as last position and settings allow more than one
            // open position
            } else if active_positions.len() < settings.max_open_orders as usize {
                if let Some(close_price) = trigger_price {
                    self.account
                        .lock()
                        .await
                        .open_position(
                            &signal.symbol,
                            settings.margin_usd,
                            settings.leverage,
                            signal.order_side.clone(),
                            close_price,
                            Some(signal.strategy_id),
                            None,
                        )
                        .await;
                }
            }

        // no open positions yet for given strategy
        } else {
            if let Some(last_price) = trigger_price {
                self.account
                    .lock()
                    .await
                    .open_position(
                        &signal.symbol,
                        settings.margin_usd,
                        settings.leverage,
                        signal.order_side.clone(),
                        last_price,
                        Some(signal.strategy_id),
                        None,
                    )
                    .await;
            }
        }
    }

    pub fn add_strategy_settings(&mut self, strategy_id: StrategyId, settings: StrategySettings) {
        self.active_strategy_settings.insert(strategy_id, settings);
    }

    pub fn remove_strategy_settings(&mut self, strategy_id: StrategyId) {
        self.active_strategy_settings.remove(&strategy_id);
    }
}
