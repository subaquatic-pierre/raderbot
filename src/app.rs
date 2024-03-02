use std::sync::Arc;

use actix_web::web::Data;

use crate::{
    account::account::Account,
    bot::RaderBot,
    exchange::api::ExchangeApi,
    market::{market::Market, types::ArcMutex},
    storage::manager::StorageManager,
};

/// Represents the shared state of the application.
///
/// `AppState` holds references to the core components of the application, including the trading bot,
/// account management, market data, and exchange API interfaces. It provides methods to access
/// each component, ensuring that they can be shared safely across asynchronous tasks.
pub struct AppState {
    /// A thread-safe, mutable reference to the `RaderBot` instance.
    pub bot: ArcMutex<RaderBot>,
}

impl AppState {
    /// Retrieves a shared, thread-safe reference to the `Account` component.
    ///
    /// This method allows other parts of the application to interact with the account management
    /// functionalities, such as querying account balance or open positions.
    ///
    /// # Returns
    ///
    /// An `ArcMutex<Account>` allowing safe, concurrent access to the `Account`.
    pub async fn get_account(&self) -> ArcMutex<Account> {
        self.bot.lock().await.account.clone()
    }

    /// Retrieves a shared, thread-safe reference to the `Market` component.
    ///
    /// This method provides access to market data, enabling features like fetching current prices,
    /// historical data, or subscribing to market updates.
    ///
    /// # Returns
    ///
    /// An `ArcMutex<Market>` allowing safe, concurrent access to market data.
    pub async fn get_market(&self) -> ArcMutex<Market> {
        self.bot.lock().await.market.clone()
    }

    pub async fn get_storage_manager(&self) -> Arc<Box<dyn StorageManager>> {
        self.bot.lock().await.storage_manager.clone()
    }

    /// Retrieves a shared, thread-safe reference to the `ExchangeApi` component.
    ///
    /// This method facilitates interaction with the exchange, including executing trades, fetching
    /// order book data, and managing orders.
    ///
    /// # Returns
    ///
    /// An `Arc<Box<dyn ExchangeApi>>` providing a polymorphic interface to the exchange API,
    /// allowing for flexibility in supporting multiple exchanges.
    pub async fn get_exchange_api(&self) -> Arc<Box<dyn ExchangeApi>> {
        self.bot.lock().await.exchange_api.clone()
    }
}

/// Creates and initializes a new application state.
///
/// This function constructs a new `RaderBot` instance and wraps it in a shared state object,
/// making it accessible throughout the application.
///
/// # Returns
///
/// A `Data<AppState>` wrapper around the initialized application state, ready for integration
/// into an Actix web application.
pub async fn new_app_state() -> Data<AppState> {
    let bot = ArcMutex::new(RaderBot::new().await);

    Data::new(AppState { bot })
}
