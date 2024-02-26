use std::{collections::HashMap, marker};

use log::info;

use crate::{
    account::{account::Account, trade::Position},
    market::{market::Market, types::ArcMutex},
};

use super::{
    strategy::{StrategyId, StrategySettings},
    types::SignalMessage,
};

/// Manages the handling of trading signals for active trading strategies.
///
/// This manager is responsible for executing trading signals by opening or closing positions
/// based on the strategy's settings and the nature of the incoming signal. It interacts with
/// both the account to manage positions and the market to fetch current prices.

pub struct SignalManager {
    active_strategy_settings: HashMap<StrategyId, StrategySettings>,
}

impl SignalManager {
    /// Initializes a new `SignalManager` with references to the account and market.
    ///
    /// # Arguments
    ///
    /// * `account` - A shared, thread-safe reference to the trading account.
    /// * `market` - A shared, thread-safe reference to the market data.
    ///
    /// # Returns
    ///
    /// Returns an instance of `SignalManager`.

    pub fn new() -> Self {
        Self {
            active_strategy_settings: HashMap::new(),
        }
    }

    /// Processes a trading signal, potentially opening or closing positions based on the strategy's settings.
    ///
    /// # Arguments
    ///
    /// * `signal` - The trading signal to process.
    ///
    /// This method considers the current active positions, the strategy settings, and the nature of the signal
    /// to decide on the appropriate trading action.

    pub async fn handle_signal(
        &self,
        signal: SignalMessage,
        market: ArcMutex<Market>,
        account: ArcMutex<Account>,
    ) {
        let account = account.clone();
        let market = market.clone();
        let active_positions: Vec<Position> = account
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
            market.lock().await.last_price(&signal.symbol).await
        };

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

        // get last open position
        if let Some(last) = active_positions.last() {
            // if last.signal is different to new signal then close all positions
            if signal.order_side != last.order_side {
                if let Some(close_price) = trigger_price {
                    for position in &active_positions {
                        account
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
                    account
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
                account
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

    /// Adds settings for a trading strategy to the manager.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The unique identifier of the strategy.
    /// * `settings` - The trading settings for the strategy.
    ///
    /// This allows the `SignalManager` to enforce strategy-specific trading parameters.

    pub fn add_strategy_settings(&mut self, strategy_id: &StrategyId, settings: StrategySettings) {
        self.active_strategy_settings
            .insert(strategy_id.clone(), settings);
    }

    /// Removes the trading settings associated with a strategy from the manager.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The unique identifier of the strategy whose settings are to be removed.
    ///
    /// This is used when a strategy is no longer active or has been removed.

    pub fn remove_strategy_settings(&mut self, strategy_id: &StrategyId) {
        self.active_strategy_settings.remove(&strategy_id);
    }
}
