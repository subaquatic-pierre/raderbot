//! # RaderBot Application
//!
//! This is the main entry point for the RaderBot application. It initializes and runs
//! the web server, setting up all necessary services and routes for the application's API.
//! The application leverages Actix Web for its web server and API functionalities,
//! dotenv for environment variable management, and various internal modules to handle
//! different aspects of the trading bot's operations, such as account management,
//! market data processing, and executing trading strategies.

use app::new_app_state;
use dotenv::dotenv;
use log::info;
use std::io;

use actix_files::Files;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};

use api::{
    account::register_account_service, exchange::register_exchange_service,
    main::register_main_service, market::register_market_service,
    strategy::register_strategy_service, utils::register_utils_service,
};

#[allow(unused_must_use)]
mod account;
mod algo;
mod analytics;
mod api;
mod app;
mod bot;
mod exchange;
mod market;
mod storage;
mod strategy;
mod utils;

/// Server host configuration (IP address and port).
const SERVER_HOST: (&str, u16) = ("127.0.0.1", 3000);

/// The main function serves as the entry point of the application.
/// It performs initial setup, including loading environment variables, initializing logging,
/// creating application state, and starting the HTTP server with all the configured services.
///
/// # Errors
///
/// This function will return an `io::Error` if there's an issue binding the server to the specified address
/// or if any other issue occurs while starting the server.
///
/// # Examples
///
/// This function is called when the application starts:
/// ```
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     // Function body here
/// }
/// ```
// Define the main entry point for the trading bot
#[actix_web::main]

async fn main() -> io::Result<()> {
    dotenv().ok();
    env_logger::init();

    info!(
        "Server listening at {:}:{:}...",
        SERVER_HOST.0, SERVER_HOST.1
    );

    let app_state = new_app_state().await;

    // Make new HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(app_state.clone())
            .service(Files::new("/static", "./static"))
            .service(register_market_service())
            .service(register_exchange_service())
            .service(register_main_service())
            .service(register_utils_service())
            .service(register_account_service())
            .service(register_strategy_service())
    })
    // .listen(listener)?
    .bind(SERVER_HOST)?
    .run()
    .await
}
