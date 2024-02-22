use std::sync::Arc;

use actix_web::web::Data;

use crate::{
    account::account::Account,
    bot::RaderBot,
    exchange::api::ExchangeApi,
    market::{market::Market, types::ArcMutex},
};

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
