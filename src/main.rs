use bot::new_app_state;
use dotenv::dotenv;
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
mod api;
mod bot;
mod exchange;
mod market;
mod storage;
mod strategy;
mod trade;
mod utils;

const SERVER_HOST: (&str, u16) = ("127.0.0.1", 3000);

#[derive(Debug)]
pub struct Message {
    pub int: String,
}

// Define the main entry point for the trading bot
#[actix_web::main]

async fn main() -> io::Result<()> {
    dotenv().ok();
    env_logger::init();

    println!(
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
