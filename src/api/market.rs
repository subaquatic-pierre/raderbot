use actix_web::post;
use actix_web::web::Json;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::Deserialize;
use serde_json::json;

use crate::exchange::types::StreamType;

use crate::app::AppState;
use crate::utils::time::string_to_timestamp;

#[derive(Debug, Deserialize)]
pub struct GetKlineDataParams {
    symbol: String,
    interval: String,
}
#[post("/kline-data")]
async fn get_kline_data(
    app_data: web::Data<AppState>,
    body: Json<GetKlineDataParams>,
) -> impl Responder {
    let market = app_data.get_market().await;

    let kline_data = market
        .lock()
        .await
        .kline_data(&body.symbol, &body.interval)
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

#[post("/ticker-data")]
async fn get_ticker_data(
    app_data: web::Data<AppState>,
    body: Json<GetTickerDataParams>,
) -> impl Responder {
    let market = app_data.get_market().await;

    let ticker_data = market.lock().await.ticker_data(&body.symbol).await;

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

#[derive(Debug, Deserialize)]
pub struct GetKlineDataRangeParams {
    symbol: String,
    interval: String,
    from_ts: Option<String>,
    to_ts: Option<String>,
    limit: Option<usize>,
}
#[post("/kline-data-range")]
async fn get_kline_data_range(
    app_data: web::Data<AppState>,
    body: Json<GetKlineDataRangeParams>,
) -> impl Responder {
    let market = app_data.get_market().await;

    let mut from_ts: Option<u64> = None;
    let mut to_ts: Option<u64> = None;

    if let Some(ts) = &body.to_ts {
        let _ts = string_to_timestamp(ts);
        if _ts.is_err() {
            let json_data = json!({ "error": "Unable to parse dates".to_string()});
            return HttpResponse::ExpectationFailed().json(json_data);
        }
        let _ts = _ts.unwrap();
        to_ts = Some(_ts);
    };

    if let Some(ts) = &body.from_ts {
        let _ts = string_to_timestamp(ts);
        if _ts.is_err() {
            let json_data = json!({ "error": "Unable to parse dates".to_string()});
            return HttpResponse::ExpectationFailed().json(json_data);
        }
        let _ts = _ts.unwrap();
        from_ts = Some(_ts);
    };

    let kline_data = market
        .lock()
        .await
        .kline_data_range(&body.symbol, &body.interval, from_ts, to_ts, body.limit)
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

#[post("/last-price")]
async fn last_price(
    app_data: web::Data<AppState>,
    body: Json<GetTickerDataParams>,
) -> impl Responder {
    let market = app_data.get_market().await;

    let last_price = market.lock().await.last_price(&body.symbol).await;

    if let Some(last_price) = last_price {
        // Return the stream data as JSON
        let json_data = json!({ "last_price": last_price,"symbol":body.symbol });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Last price not found","symbol":body.symbol });
        // Stream ID not found
        HttpResponse::Ok().json(json_data)
    }
}

#[get("/info")]
async fn market_info(app_data: web::Data<AppState>) -> impl Responder {
    let market = app_data.get_market().await;

    let info = market.lock().await.info().await;
    let json_data = json!({ "market_info": info });
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
#[post("/close-stream")]
async fn close_stream(
    app_data: web::Data<AppState>,
    body: Json<CloseStreamParams>,
) -> HttpResponse {
    let market = app_data.get_market().await;

    let stream_meta = market.lock().await.close_stream(&body.stream_id).await;

    // TODO: handle error
    match stream_meta {
        Some(meta) => {
            let json_data = json!({ "success": "Stream closed successfully","stream_meta":meta });
            HttpResponse::Ok().json(json_data)
        }
        None => {
            let json_data =
                json!({ "error": format!("Stream width ID {} not found", &body.stream_id) });
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
#[post("/open-stream")]
async fn open_stream(
    app_data: web::Data<AppState>,
    body: Json<OpenStreamParams>,
) -> impl Responder {
    let stream_type = body.stream_type.clone();
    let market = app_data.get_market().await;

    // TODO: handle errors

    let stream_id = match stream_type {
        StreamType::Kline => {
            let symbol = body.symbol.to_string();
            let interval = body.interval.clone().unwrap().to_string();
            market
                .lock()
                .await
                .open_stream(stream_type, &symbol, Some(&interval))
                .await
        }
        StreamType::Ticker => {
            let symbol = body.symbol.to_string();
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

pub fn register_market_service() -> Scope {
    scope("/market")
        .service(last_price)
        .service(close_stream)
        .service(open_stream)
        .service(get_kline_data)
        .service(get_kline_data_range)
        .service(market_info)
        .service(active_streams)
        .service(get_ticker_data)
}
