use std::collections::HashMap;

use log::info;

use crate::{account::account::Account, market::types::ArcMutex};

use super::{strategy::StrategySettings, types::SignalMessage};

pub struct SignalManager {
    account: ArcMutex<Account>,
    active_strategy_settings: HashMap<u32, StrategySettings>,
}

impl SignalManager {
    pub fn new(account: ArcMutex<Account>) -> Self {
        Self {
            account,
            active_strategy_settings: HashMap::new(),
        }
    }

    pub fn handle_signal(&mut self, signal: SignalMessage) {
        info!("{signal:?}");
    }

    pub fn add_strategy_settings(&mut self, strategy_id: u32, settings: StrategySettings) {
        self.active_strategy_settings.insert(strategy_id, settings);
    }

    pub fn remove_strategy_settings(&mut self, strategy_id: u32) {
        self.active_strategy_settings.remove(&strategy_id);
    }
}
