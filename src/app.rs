use actix_web::web::Data;
use dotenv_codegen::dotenv;

use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    account::account::Account,
    exchange::{api::ExchangeApi, bingx::BingXApi, stream::StreamManager},
    market::{market::Market, messages::MarketMessage, types::ArcMutex},
    storage::manager::StorageManager,
    utils::channel::build_arc_channel,
};

use tokio::sync::watch::{channel, Receiver, Sender};

use crate::Message;

pub struct RaderBot {
    pub market: ArcMutex<Market>,
    // TODO: remove stream manager
    // only handle streams through the market field
    // pub stream_manager: ArcMutex<StreamManager>,
    pub account: ArcMutex<Account>,
    pub exchange_api: Arc<Box<dyn ExchangeApi>>,
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

        Self {
            market,
            // stream_manager,
            account,
            exchange_api: exchange_api.clone(),
        }
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

    pub fn get_ticker_stream(&self, symbol: &str) -> Receiver<String> {
        if let Some(rec) = self.receivers.get(symbol) {
            rec.clone()
        } else {
            let (sender, receiver) = channel::<String>("".to_string());

            // spawn new ticker thread
            self.spawn_ticker_thread(symbol, sender);
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

    pub async fn get_bot(&self) -> ArcMutex<RaderBot> {
        self.bot.clone()
    }

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
