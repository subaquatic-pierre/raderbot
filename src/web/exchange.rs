use actix_web::HttpRequest;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::Deserialize;
use serde_json::json;

use crate::app::AppState;

#[get("/exchange-info")]
async fn exchange_info(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange
        .exchange_info()
        .await
        .expect("Unable to get exchange info");

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
#[get("/get-kline")]
async fn get_kline(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let exchange_api = app_data.get_exchange_api().await;
    let params = web::Query::<GetKlineParams>::from_query(req.query_string()).unwrap();

    let kline = exchange_api
        .get_kline(&params.symbol, &params.interval)
        .await;

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

#[get("/get-ticker")]
async fn get_ticker(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let exchange_api = app_data.get_exchange_api().await;
    let params = web::Query::<GetTickerParams>::from_query(req.query_string()).unwrap();

    let ticker = exchange_api.get_ticker(&params.symbol).await;

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
        .service(exchange_info)
        .service(get_kline)
        .service(get_ticker)
        .service(list_list_open_orders)
        .service(all_orders)
}
