use actix_web::HttpRequest;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::Deserialize;
use serde_json::json;

use crate::exchange::types::StreamType;

use crate::app::AppState;

#[get("/meta")]
async fn get_market_meta(_app_data: web::Data<AppState>) -> impl Responder {
    let json_data = json!({ "success": "Market meta data" });
    // Stream ID not found
    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct GetKlineDataParams {
    symbol: String,
    interval: String,
    from_ts: Option<u64>,
    to_ts: Option<u64>,
    limit: Option<usize>,
}
#[get("/kline-data")]
async fn get_kline_data(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let params = web::Query::<GetKlineDataParams>::from_query(req.query_string()).unwrap();

    let market = app_data.get_market().await;

    let kline_data = market
        .lock()
        .await
        .kline_data(
            &params.symbol,
            &params.interval,
            params.from_ts,
            params.to_ts,
            params.limit,
        )
        .await;

    if let Some(kline_data) = kline_data {
        // Return the stream data as JSON
        let json_data = json!({ "kline_data": kline_data });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Kline data not found" });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

#[derive(Debug, Deserialize)]
pub struct GetTickerDataParams {
    symbol: String,
}

#[get("/last-price")]
async fn last_price(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let params = web::Query::<GetTickerDataParams>::from_query(req.query_string()).unwrap();

    let market = app_data.get_market().await;

    let last_price = market.lock().await.last_price(&params.symbol).await;

    if let Some(last_price) = last_price {
        // Return the stream data as JSON
        let json_data = json!({ "last_price": last_price,"symbol":params.symbol });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Last price not found","symbol":params.symbol });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

#[get("/ticker-data")]
async fn get_ticker_data(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let params = web::Query::<GetTickerDataParams>::from_query(req.query_string()).unwrap();

    let market = app_data.get_market().await;

    let ticker_data = market.lock().await.ticker_data(&params.symbol).await;

    if let Some(ticker_data) = ticker_data {
        // Return the stream data as JSON
        let json_data = json!({ "ticker_data": ticker_data });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Ticker data not found" });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

#[get("/market-data")]
async fn get_market_data(app_data: web::Data<AppState>) -> impl Responder {
    let market = app_data.get_market().await;

    let market_data = market.lock().await.market_data().await;
    // Return the stream data as JSON
    let json_data = json!({ "active_streams": market_data });
    HttpResponse::Ok().json(json_data)
}

#[get("/active-streams")]
async fn active_streams(app_data: web::Data<AppState>) -> impl Responder {
    let market = app_data.get_market().await;
    let active_streams = market.lock().await.active_streams().await;
    // Return the stream data as JSON
    let json_data = json!({ "active_streams": active_streams });
    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct CloseStreamParams {
    stream_id: String,
}
#[get("/close-stream")]
async fn close_stream(app_data: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let params = web::Query::<CloseStreamParams>::from_query(req.query_string()).unwrap();

    let market = app_data.get_market().await;

    let stream_meta = market.lock().await.close_stream(&params.stream_id).await;

    // TODO: handle error
    match stream_meta {
        Some(meta) => {
            let json_data = json!({ "success": "Stream closed successfully","stream_meta":meta });
            HttpResponse::Ok().json(json_data)
        }
        None => {
            let json_data =
                json!({ "error": format!("Stream width ID {} not found", &params.stream_id) });
            // Stream ID not found
            HttpResponse::Ok().json(json_data)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OpenStreamParams {
    stream_type: StreamType,
    symbol: String,
    interval: Option<String>,
}
#[get("/open-stream")]
async fn open_stream(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let params = web::Query::<OpenStreamParams>::from_query(req.query_string()).unwrap();

    let stream_type = params.stream_type.clone();
    let market = app_data.get_market().await;

    // TODO: handle errors

    let stream_id = match stream_type {
        StreamType::Kline => {
            let symbol = params.symbol.to_string();
            let interval = params.interval.clone().unwrap().to_string();
            market
                .lock()
                .await
                .open_stream(stream_type, &symbol, Some(&interval))
                .await
        }
        StreamType::Ticker => {
            let symbol = params.symbol.to_string();
            market
                .lock()
                .await
                .open_stream(stream_type, &symbol, None)
                .await
        }
    };

    let data = match stream_id {
        Ok(stream_id) => {
            json!({ "success": "Stream created","stream_id":stream_id })
        }
        Err(e) => {
            json!({ "error": "Unable to open stream","msg":e.to_string() })
        }
    };

    HttpResponse::Ok().json(data)
}

#[get("/remote-kline")]
async fn get_remote_kline(app_data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let exchange_api = app_data.get_exchange_api().await;
    let params = web::Query::<GetKlineDataParams>::from_query(req.query_string()).unwrap();

    let kline = exchange_api
        .get_kline(&params.symbol, &params.interval)
        .await;

    if let Ok(kline) = kline {
        // Return the stream data as JSON
        let json_data = json!({ "ticker_data": kline });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Ticker data not found" });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

pub fn register_market_service() -> Scope {
    scope("/market")
        .service(get_remote_kline)
        .service(last_price)
        .service(close_stream)
        .service(open_stream)
        .service(get_market_meta)
        .service(get_kline_data)
        .service(get_market_data)
        .service(active_streams)
        .service(get_ticker_data)
}
