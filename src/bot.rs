use dotenv_codegen::dotenv;

use serde_json::Value;

use std::{collections::HashMap, sync::Arc};

use crate::{
    account::account::Account,
    exchange::{api::ExchangeApi, bingx::BingXApi, mock::MockExchangeApi},
    market::{
        market::Market,
        messages::MarketMessage,
        types::{ArcMutex, ArcReceiver, ArcSender},
    },
    storage::{fs::FsStorageManager, manager::StorageManager},
    strategy::{
        backer::BackTest,
        signal::SignalManager,
        strategy::{Strategy, StrategyId, StrategyInfo, StrategySettings, StrategySummary},
        types::{AlgorithmError, SignalMessage},
    },
    utils::channel::build_arc_channel,
};

use tokio::task::JoinHandle;

pub struct RaderBot {
    pub market: ArcMutex<Market>,
    pub account: ArcMutex<Account>,
    strategy_manager: ArcMutex<StrategyManager>,
    pub exchange_api: Arc<Box<dyn ExchangeApi>>,
    storage_manager: Arc<Box<dyn StorageManager>>,
    strategy_tx: ArcSender<SignalMessage>,
    strategy_rx: ArcReceiver<SignalMessage>,
}

impl RaderBot {
    pub async fn new() -> Self {
        // create new Arc of exchange API
        let api_key = dotenv!("BINGX_API_KEY");
        let secret_key = dotenv!("BINGX_SECRET_KEY");
        let dry_run = dotenv!("DRY_RUN");

        // create new channel for stream handler and market to communicate
        let (market_tx, market_rx) = build_arc_channel::<MarketMessage>();

        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(BingXApi::new(
            api_key,
            secret_key,
            market_tx.clone(),
        )));

        // create new storage manager
        let storage_manager: Arc<Box<dyn StorageManager>> =
            Arc::new(Box::new(FsStorageManager::default()));

        // create new market to hold market data
        let market = Market::new(
            market_rx.clone(),
            exchange_api.clone(),
            storage_manager.clone(),
            true,
        )
        .await;

        let market = ArcMutex::new(market);

        // Account can use different API from market exchange API
        // that is to allow for retrieving market data from separate source
        // and to open and close positions on different API source
        let (account_exchange_api, dry_run) = if dry_run == "True" {
            let api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
            (api, true)
        } else {
            // possible to create different exchange API if needed
            // instead of using the same API as Market
            (exchange_api.clone(), false)
        };

        let account = Account::new(account_exchange_api, true, dry_run).await;

        let account = ArcMutex::new(account);

        let (strategy_tx, strategy_rx) = build_arc_channel::<SignalMessage>();

        let strategy_manager = StrategyManager::new();

        let mut _self = Self {
            market,
            account,
            exchange_api: exchange_api.clone(),
            strategy_manager: ArcMutex::new(strategy_manager),
            strategy_tx,
            strategy_rx,
            storage_manager,
        };

        _self.init().await;

        _self
    }

    pub async fn start_strategy(
        &mut self,
        strategy_name: &str,
        symbol: &str,
        interval: &str,
        settings: StrategySettings,
        algorithm_params: Value,
    ) -> Result<StrategyInfo, AlgorithmError> {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let mut strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            market.clone(),
            settings,
            algorithm_params,
        )?;

        let handle = strategy.start().await;

        let strategy_info = strategy.info().await;

        self.strategy_manager
            .clone()
            .lock()
            .await
            .insert(strategy, handle);

        Ok(strategy_info)
    }

    pub async fn stop_strategy(
        &mut self,
        strategy_id: StrategyId,
        close_positions: bool,
    ) -> Option<StrategySummary> {
        let mut summary: Option<StrategySummary> = None;
        let account = self.account.clone();
        let strategy_manager = self.strategy_manager.clone();

        // Remove strategy handles
        if let Some((handle, strategy)) = strategy_manager.lock().await.get(&strategy_id) {
            handle.abort();

            let _summary = strategy.stop(account.clone(), close_positions).await;

            // Save summary
            self.storage_manager
                .save_strategy_summary(_summary.clone())
                .ok();

            summary = Some(_summary);
        };

        // Remove all handles and settings from signal_manager
        strategy_manager.lock().await.remove(&strategy_id);

        summary
    }

    pub async fn get_active_strategy_ids(&mut self) -> Vec<StrategyId> {
        let strategy_manager = self.strategy_manager.clone();
        let strategy_manger = strategy_manager.lock().await;
        strategy_manger.list_ids()
    }

    pub fn list_historical_strategies(&mut self) -> Option<Vec<StrategyInfo>> {
        self.storage_manager.list_saved_strategies().ok()
    }

    pub fn get_historical_strategy_summary(
        &mut self,
        strategy_id: StrategyId,
    ) -> Option<StrategySummary> {
        self.storage_manager.get_strategy_summary(strategy_id).ok()
    }

    pub async fn run_back_test(
        &mut self,
        strategy_name: &str,
        symbol: &str,
        interval: &str,
        from_ts: u64,
        to_ts: u64,
        settings: StrategySettings,
        algorithm_params: Value,
    ) -> Result<StrategySummary, AlgorithmError> {
        let strategy_tx = self.strategy_tx.clone();
        let strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            self.market.clone(),
            settings,
            algorithm_params,
        )?;

        // TODO: Get initial_balance from params
        let initial_balance = Some(10_000.0);
        let mut back_test = BackTest::new(strategy, self.market.clone(), initial_balance).await;

        if let Some(kline_data) = self
            .market
            .clone()
            .lock()
            .await
            .kline_data_range(&symbol, &interval, Some(from_ts), Some(to_ts), None)
            .await
        {
            back_test.run(kline_data).await;
        };

        Ok(back_test.result().await)
    }

    pub async fn get_strategy_info(&mut self, strategy_id: StrategyId) -> Option<StrategyInfo> {
        let manager = self.strategy_manager.clone();
        let mut manager = manager.lock().await;
        if let Some((_handle, strategy)) = manager.get(&strategy_id) {
            return Some(strategy.info().await.clone());
        }
        None
    }

    pub async fn get_strategy_summary(
        &mut self,
        strategy_id: StrategyId,
    ) -> Option<StrategySummary> {
        let manager = self.strategy_manager.clone();
        let account = self.account.clone();
        let mut manager = manager.lock().await;
        if let Some((_handle, strategy)) = manager.get(&strategy_id) {
            return Some(strategy.summary(account).await.clone());
        }
        None
    }

    pub async fn change_strategy_settings(
        &mut self,
        strategy_id: StrategyId,
        settings: StrategySettings,
    ) -> Option<StrategyInfo> {
        let manager = self.strategy_manager.clone();
        let mut manager = manager.lock().await;
        if let Some((_handle, strategy)) = manager.get(&strategy_id) {
            strategy.change_settings(settings);
            return Some(strategy.info().await);
        }
        None
    }

    pub async fn set_strategy_params(
        &mut self,
        strategy_id: StrategyId,
        params: Value,
    ) -> Result<(), AlgorithmError> {
        let manager = self.strategy_manager.clone();
        let mut manager = manager.lock().await;
        if let Some((_handle, strategy)) = manager.get(&strategy_id) {
            strategy.set_algorithm_params(params).await?
        }
        Ok(())
    }
    pub async fn get_strategy_params(&mut self, strategy_id: StrategyId) -> Option<Value> {
        let manager = self.strategy_manager.clone();
        let mut manager = manager.lock().await;
        if let Some((_handle, strategy)) = manager.get(&strategy_id) {
            return Some(strategy.get_algorithm_params().await);
        }
        None
    }

    // ---
    // Private Methods
    // ---

    async fn init(&mut self) {
        let strategy_manager = self.strategy_manager.clone();
        let strategy_rx = self.strategy_rx.clone();
        let account = self.account.clone();
        let market = self.market.clone();

        tokio::spawn(async move {
            while let Some(signal) = strategy_rx.lock().await.recv().await {
                let strategy_manager = strategy_manager.lock().await;
                let signal_manager = strategy_manager.get_signal_manager();
                signal_manager
                    .handle_signal(signal, market.clone(), account.clone())
                    .await;
            }
        });
    }
}

/// Manages multiple trading strategies by storing their handles, settings, and providing methods for insertion, removal, and retrieval.
pub struct StrategyManager {
    /// A mapping of strategy IDs to their corresponding join handles for managing strategy execution.
    strategy_handles: HashMap<StrategyId, JoinHandle<()>>,
    /// A mapping of strategy IDs to their corresponding strategies.
    strategies: HashMap<StrategyId, Strategy>,
    /// Manages signals for strategies.
    signal_manager: SignalManager,
}

impl StrategyManager {
    /// Constructs a new `StrategyManager`.
    ///
    /// # Returns
    ///
    /// A new instance of `StrategyManager` with an empty set of strategies and signal manager.
    pub fn new() -> Self {
        let signal_manager = SignalManager::new();

        Self {
            signal_manager,
            strategy_handles: HashMap::new(),
            strategies: HashMap::new(),
        }
    }

    /// Inserts a strategy along with its join handle into the manager.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The strategy to insert.
    /// * `handle` - The join handle associated with the strategy execution.
    pub fn insert(&mut self, strategy: Strategy, handle: JoinHandle<()>) {
        let strategy_id = strategy.id.clone();
        let settings = strategy.settings();

        self.signal_manager
            .add_strategy_settings(&strategy_id, settings);

        self.strategy_handles.insert(strategy.id, handle);
        self.strategies.insert(strategy.id, strategy);
    }

    /// Removes a strategy and its associated join handle from the manager.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The ID of the strategy to remove.
    pub fn remove(&mut self, strategy_id: &StrategyId) {
        self.strategy_handles.remove(&strategy_id);
        self.strategies.remove(&strategy_id);

        self.signal_manager.remove_strategy_settings(strategy_id);
    }

    /// Retrieves the join handle and mutable reference to a strategy with the specified ID, if present.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The ID of the strategy to retrieve.
    ///
    /// # Returns
    ///
    /// A tuple containing the join handle and mutable reference to the strategy, if found.
    pub fn get(&mut self, strategy_id: &StrategyId) -> Option<(&JoinHandle<()>, &mut Strategy)> {
        if let (Some(handle), Some(strategy)) = (
            self.strategy_handles.get(strategy_id),
            self.strategies.get_mut(strategy_id),
        ) {
            return Some((handle, strategy));
        }
        None
    }

    /// Retrieves a list of strategy IDs currently managed by the manager.
    ///
    /// # Returns
    ///
    /// A vector containing the IDs of the managed strategies.
    pub fn list_ids(&self) -> Vec<StrategyId> {
        let mut strategies = vec![];
        for (strategy_id, _strategy) in self.strategies.iter() {
            strategies.push(*strategy_id)
        }

        strategies
    }

    /// Retrieves a reference to the signal manager associated with this strategy manager.
    ///
    /// # Returns
    ///
    /// A reference to the signal manager.
    pub fn get_signal_manager(&self) -> &SignalManager {
        &self.signal_manager
    }
}
