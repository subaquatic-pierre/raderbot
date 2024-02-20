use actix_web::web::Data;
use dotenv_codegen::dotenv;
use log::{info, warn};

use std::{collections::HashMap, sync::Arc};

use crate::{
    account::account::Account,
    exchange::{api::ExchangeApi, bingx::BingXApi},
    market::{
        market::Market,
        messages::MarketMessage,
        types::{ArcMutex, ArcReceiver, ArcSender},
    },
    storage::manager::StorageManager,
    strategy::{
        backer::{BackTest, BackTestResult},
        signal::SignalManager,
        strategy::{Strategy, StrategySettings},
        types::{AlgorithmError, SignalMessage},
    },
    utils::channel::build_arc_channel,
};

use tokio::task::{AbortHandle, JoinHandle};

use crate::Message;

pub struct RaderBot {
    pub market: ArcMutex<Market>,
    pub account: ArcMutex<Account>,
    pub exchange_api: Arc<Box<dyn ExchangeApi>>,
    signal_manager: ArcMutex<SignalManager>,
    strategy_handles: HashMap<u32, JoinHandle<()>>,
    strategies: HashMap<u32, Strategy>,
    strategy_rx: ArcReceiver<SignalMessage>,
    strategy_tx: ArcSender<SignalMessage>,
}

impl RaderBot {
    pub async fn new() -> Self {
        // create new Arc of exchange API
        let api_key = dotenv!("BINANCE_API_KEY");
        let secret_key = dotenv!("BINANCE_SECRET_KEY");

        // create new channel for stream handler and market to communicate
        let (market_tx, market_rx) = build_arc_channel::<MarketMessage>();

        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(BingXApi::new(
            api_key,
            secret_key,
            market_tx.clone(),
        )));

        // create new storage manager
        let storage_manager = StorageManager::default();

        // create new market to hold market data
        let market = Market::new(market_rx.clone(), exchange_api.clone(), storage_manager).await;

        let market = ArcMutex::new(market);

        let account = Account::new(market.clone(), exchange_api.clone(), false).await;

        let account = ArcMutex::new(account);

        let (strategy_tx, strategy_rx) = build_arc_channel::<SignalMessage>();

        let signal_manager = ArcMutex::new(SignalManager::new(account.clone()));

        let mut _self = Self {
            market,
            signal_manager,
            account,
            exchange_api: exchange_api.clone(),
            strategy_handles: HashMap::new(),
            strategies: HashMap::new(),
            strategy_rx,
            strategy_tx,
        };

        _self.init().await;

        _self
    }

    pub async fn add_strategy(
        &mut self,
        strategy_name: &str,
        symbol: &str,
        interval: &str,
    ) -> Result<u32, AlgorithmError> {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();

        let strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            market,
            StrategySettings::default(),
        )?;

        let handle = strategy.start().await;
        let strategy_id = strategy.id;

        self.signal_manager
            .lock()
            .await
            .add_strategy_settings(strategy_id, strategy.settings());

        self.strategy_handles.insert(strategy.id, handle);
        self.strategies.insert(strategy.id, strategy);

        Ok(strategy_id)
    }

    pub async fn stop_strategy(&mut self, strategy_id: u32) -> String {
        if let Some(handle) = self.strategy_handles.get(&strategy_id) {
            handle.abort();

            self.strategy_handles.remove(&strategy_id);
            self.strategies.remove(&strategy_id);
            self.signal_manager
                .lock()
                .await
                .remove_strategy_settings(strategy_id)
        }

        strategy_id.to_string()
    }

    pub async fn get_strategies(&mut self) -> Vec<u32> {
        let mut strategies = vec![];
        for (strategy_id, _strategy) in self.strategies.iter() {
            strategies.push(*strategy_id)
        }

        strategies
    }

    pub async fn run_back_test(
        &mut self,
        strategy_name: &str,
        symbol: &str,
        interval: &str,
        from_ts: u64,
        to_ts: u64,
    ) -> Result<BackTestResult, AlgorithmError> {
        let strategy_tx = self.strategy_tx.clone();
        let strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            self.market.clone(),
            StrategySettings::default(),
        )?;

        let mut back_test = BackTest::new(strategy).await;

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

        Ok(back_test.result())
    }

    // ---
    // Private Methods
    // ---

    async fn init(&mut self) {
        let signal_manager = self.signal_manager.clone();
        let strategy_rx = self.strategy_rx.clone();

        tokio::spawn(async move {
            while let Some(signal) = strategy_rx.lock().await.recv().await {
                signal_manager.lock().await.handle_signal(signal);
            }
        });
    }
}

pub struct AppState {
    pub bot: ArcMutex<RaderBot>,
}

impl AppState {
    pub async fn get_account(&self) -> ArcMutex<Account> {
        self.bot.lock().await.account.clone()
    }

    pub async fn get_market(&self) -> ArcMutex<Market> {
        self.bot.lock().await.market.clone()
    }

    pub async fn get_exchange_api(&self) -> Arc<Box<dyn ExchangeApi>> {
        self.bot.lock().await.exchange_api.clone()
    }
}

pub async fn new_app_state() -> Data<AppState> {
    let bot = ArcMutex::new(RaderBot::new().await);

    Data::new(AppState { bot })
}
