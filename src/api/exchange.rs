use actix_web::web::Json;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};
use actix_web::{post, HttpRequest};

use serde::Deserialize;
use serde_json::json;

use crate::bot::AppState;

#[get("/info")]
async fn info(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange.info().await.expect("Unable to get exchange info");

    // Return the stream data as JSON
    HttpResponse::Ok().json(data)
}

#[get("/all-orders")]
async fn all_orders(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange
        .all_orders()
        .await
        .expect("Unable to get exchange info");

    // Return the stream data as JSON
    HttpResponse::Ok().json(data)
}

#[get("/list-open-orders")]
async fn list_list_open_orders(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange
        .list_open_orders()
        .await
        .expect("Unable to get exchange info");

    // Return the stream data as JSON
    HttpResponse::Ok().json(data)
}

#[derive(Debug, Deserialize)]
pub struct GetKlineParams {
    symbol: String,
    interval: String,
}
#[post("/get-kline")]
async fn get_kline(app_data: web::Data<AppState>, body: Json<GetKlineParams>) -> impl Responder {
    let exchange_api = app_data.get_exchange_api().await;

    let kline = exchange_api.get_kline(&body.symbol, &body.interval).await;

    if let Ok(kline) = kline {
        // Return the stream data as JSON
        let json_data = json!({ "kline_data": kline });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Ticker data not found" });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

#[derive(Debug, Deserialize)]
pub struct GetTickerParams {
    symbol: String,
}

#[post("/get-ticker")]
async fn get_ticker(app_data: web::Data<AppState>, body: Json<GetTickerParams>) -> impl Responder {
    let exchange_api = app_data.get_exchange_api().await;

    let ticker = exchange_api.get_ticker(&body.symbol).await;

    if let Ok(ticker) = ticker {
        // Return the stream data as JSON
        let json_data = json!({ "ticker_data": ticker });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Ticker data not found" });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

pub fn register_exchange_service() -> Scope {
    scope("/exchange")
        .service(info)
        .service(get_kline)
        .service(get_ticker)
        .service(list_list_open_orders)
        .service(all_orders)
}
