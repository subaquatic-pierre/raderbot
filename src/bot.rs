use actix::fut::future::result;
use actix_web::web::Data;
use dotenv_codegen::dotenv;
use log::{info, warn};

use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    account::account::Account,
    exchange::{api::ExchangeApi, bingx::BingXApi, stream::StreamManager},
    market::{
        market::Market,
        messages::MarketMessage,
        types::{ArcMutex, ArcReceiver, ArcSender},
    },
    storage::manager::StorageManager,
    strategy::{
        algorithm::MovingAverage,
        strategy::{BackTestResult, Strategy},
        types::SignalMessage,
    },
    utils::channel::build_arc_channel,
};

use tokio::{
    sync::watch::{channel, Receiver, Sender},
    task::{AbortHandle, JoinHandle},
};

use crate::Message;

pub struct RaderBot {
    pub market: ArcMutex<Market>,
    // TODO: remove stream manager
    // only handle streams through the market field
    // pub stream_manager: ArcMutex<StreamManager>,
    pub account: ArcMutex<Account>,
    pub exchange_api: Arc<Box<dyn ExchangeApi>>,
    strategy_handles: HashMap<String, JoinHandle<()>>,
    strategies: HashMap<String, Strategy>,
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

        let account = Account::new(market.clone(), exchange_api.clone()).await;

        let account = ArcMutex::new(account);

        let (strategy_tx, strategy_rx) = build_arc_channel::<SignalMessage>();

        let mut _self = Self {
            market,
            // stream_manager,
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
    ) -> String {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();
        let algo = MovingAverage::new();
        let strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            market,
            Box::new(algo),
        );

        let handle = strategy.start().await;
        let strategy_id = strategy.id.to_string();

        self.strategy_handles.insert(strategy_id.clone(), handle);
        self.strategies.insert(strategy_id.clone(), strategy);

        strategy_id.clone()
    }

    pub async fn stop_strategy(&mut self, strategy_id: &str) -> String {
        if let Some(handle) = self.strategy_handles.get(strategy_id) {
            handle.abort();

            self.strategy_handles.remove(&strategy_id.to_string());
            self.strategies.remove(strategy_id);
        }

        strategy_id.to_string()
    }

    pub async fn get_strategies(&mut self) -> Vec<String> {
        let mut strategies = vec![];
        for (strategy_id, _strategy) in self.strategies.iter() {
            strategies.push(strategy_id.to_string())
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
    ) -> BackTestResult {
        let market = self.market.clone();
        let strategy_tx = self.strategy_tx.clone();
        let algo = MovingAverage::new();

        let strategy = Strategy::new(
            strategy_name,
            symbol,
            interval,
            strategy_tx,
            market,
            Box::new(algo),
        );

        strategy.run_back_test(from_ts, to_ts).await
    }

    // ---
    // Private Methods
    // ---

    async fn init(&mut self) {
        let strategy_rx = self.strategy_rx.clone();

        tokio::spawn(async move {
            while let Some(signal) = strategy_rx.lock().await.recv().await {
                info!("{signal:?}");
            }
        });
    }
}

pub struct WsManager {
    receivers: HashMap<String, Receiver<String>>,
}

pub const INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct WsMessage {
    pub text: String,
}

impl ToString for Message {
    fn to_string(&self) -> String {
        self.int.to_string()
    }
}

impl WsManager {
    pub fn new() -> Self {
        Self {
            receivers: HashMap::new(),
        }
    }

    pub async fn get_ticker_stream(&self, symbol: &str) -> Receiver<String> {
        if let Some(rec) = self.receivers.get(symbol) {
            rec.clone()
        } else {
            let (sender, receiver) = channel::<String>("".to_string());

            // spawn new ticker thread
            self.spawn_ticker_thread(symbol, sender).await;
            receiver
        }
    }

    pub async fn spawn_ticker_thread(&self, symbol: &str, _sender: Sender<String>) {
        let _url = format!(
            "https://open-api.bingx.com/openApi/swap/v2/quote/ticker?symbol={}",
            symbol
        );

        // tokio::spawn(async move {
        //     let val = get_ticker(&url).await;
        //     sender.send(val);

        //     tokio::time::sleep(INTERVAL).await;
        // });
    }
}
pub struct AppState {
    pub bot: ArcMutex<RaderBot>,
    pub ws_manager: ArcMutex<WsManager>,
}

impl AppState {
    pub async fn get_account(&self) -> ArcMutex<Account> {
        self.bot.lock().await.account.clone()
    }

    // pub async fn get_bot(&self) -> ArcMutex<RaderBot> {
    //     self.bot.clone()
    // }

    pub async fn get_market(&self) -> ArcMutex<Market> {
        self.bot.lock().await.market.clone()
    }

    pub async fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>> {
        self.bot
            .lock()
            .await
            .exchange_api
            .get_stream_manager()
            .clone()
    }

    pub async fn get_exchange_api(&self) -> Arc<Box<dyn ExchangeApi>> {
        self.bot.lock().await.exchange_api.clone()
    }
}

pub async fn new_app_state() -> Data<AppState> {
    let bot = ArcMutex::new(RaderBot::new().await);
    let ws_manager = ArcMutex::new(WsManager::new());

    Data::new(AppState { bot, ws_manager })
}
