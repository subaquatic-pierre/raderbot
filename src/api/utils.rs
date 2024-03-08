use std::fs;

use actix_web::web::Json;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};
use actix_web::{post, HttpRequest};
use directories::UserDirs;
use log::info;
use serde::Deserialize;
use serde_json::json;

use crate::app::AppState;
use crate::market::interval::Interval;
use crate::utils::crypt::sign_hmac;
use crate::utils::kline::{
    build_kline_filename, build_kline_key, interval_symbol_from_binance_filename,
    load_binance_klines, save_klines,
};
use crate::utils::time::{calculate_kline_open_time, get_time_difference};
use crate::utils::time::{generate_ts, year_month_day_to_ts};
use crate::utils::trade::{build_market_trade_key, load_binance_agg_trades, save_trades};

#[get("/timestamp")]
async fn get_ts(_app_data: web::Data<AppState>) -> HttpResponse {
    let ts = generate_ts();
    let json_data = json!({ "timestamp": ts });
    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct DateToTsParams {
    year: u32,
    month: u32,
    day: u32,
}

#[post("/date-to-timestamp")]
async fn date_to_timestamp(body: Json<DateToTsParams>) -> impl Responder {
    let (year, month, day) = (body.year, body.month, body.day);

    let timestamp = year_month_day_to_ts(year, month, day);

    let json_data = match timestamp {
        Some(timestamp) => {
            json!({ "timestamp": timestamp })
        }
        None => json!({ "error": "Unable to create timestamp" }),
    };

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
struct BootstrapKlinesParams {
    filename: String,
    symbol: String,
    interval: String,
}
#[post("/bootstrap-historical-klines")]
async fn load_klines(
    app_data: web::Data<AppState>,
    _body: Json<BootstrapKlinesParams>,
) -> impl Responder {
    let storage_manager = app_data.get_storage_manager().await;

    let user_dirs = UserDirs::new().expect("Failed to get user directories");
    let home_dir = user_dirs.home_dir();
    let data_dir = home_dir.join("Projects/BinanceData/Kline");

    let entries = fs::read_dir(data_dir).unwrap();

    // Loop over filenames in from directory
    for entry in entries.flatten() {
        if entry.file_type().unwrap().is_file() {
            let file_name = entry
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let (symbol, str_interval) = interval_symbol_from_binance_filename(&file_name);

            let interval = str_interval.try_into().unwrap_or(Interval::Invalid);

            let kline_key = build_kline_key(&symbol, interval);

            let klines = load_binance_klines(entry.path(), &symbol, interval);

            if let Err(e) = storage_manager.save_klines(&klines, &kline_key, true).await {
                info!("Unable to save klines: {e}");
            }
        }
    }

    // Return the stream data as JSON
    let json_data = json!({ "success": "Klines loaded" });
    HttpResponse::Ok().json(json_data)
}

#[derive(Deserialize)]
struct BootstrapTradesParams {
    filename: String,
    symbol: String,
}
#[post("/bootstrap-historical-trades")]
async fn bootstrap_historical_trades(
    app_data: web::Data<AppState>,
    _body: Json<BootstrapTradesParams>,
) -> impl Responder {
    let storage_manager = app_data.get_storage_manager().await;

    let user_dirs = UserDirs::new().expect("Failed to get user directories");
    let home_dir = user_dirs.home_dir();
    let data_dir = home_dir.join("Projects/BinanceData/Trade");

    let org_entries = fs::read_dir(data_dir).unwrap();

    // Loop over filenames in from directory
    for entry in org_entries.flatten() {
        if entry.file_type().unwrap().is_file() {
            let file_name = entry
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            // get symbol from filename
            let symbol: String = file_name.split("-").collect::<Vec<&str>>()[0].to_string();

            let agg_trades = load_binance_agg_trades(entry.path(), &symbol);

            info!("Aggregate trade length: {}", agg_trades.len());

            let trade_key = build_market_trade_key(&symbol);

            if let Err(e) = storage_manager
                .save_trades(&agg_trades, &trade_key, true)
                .await
            {
                info!("Unable to save trades: {e}");
            }
        }
    }

    // Return the stream data as JSON
    let json_data = json!({ "success": "Trade Aggregates loaded" });
    HttpResponse::Ok().json(json_data)
}

#[get("/sign-hmac")]
async fn get_sign_hmac(_app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let secret_key = "";
    let data = "timestamp=1578963600000";
    let signature = sign_hmac(secret_key, data);
    // Return the stream data as JSON
    let json_data = json!({ "signature": signature });
    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
struct TimeDifParams {
    from_ts: String,
    to_ts: String,
}
#[post("/time-difference")]
async fn time_difference(
    _app_data: web::Data<AppState>,
    body: Json<TimeDifParams>,
) -> impl Responder {
    let difference = get_time_difference(
        body.from_ts.parse::<u64>().unwrap(),
        body.to_ts.parse::<u64>().unwrap(),
    );

    // Return the stream data as JSON
    let json_data = json!({ "difference": difference });
    HttpResponse::Ok().json(json_data)
}
#[derive(Debug, Deserialize)]
struct CalculateOpenTimeParams {
    close_time: String,
    interval: Interval,
}
#[post("/calculate-open-time")]
async fn calculate_open_time(
    _app_data: web::Data<AppState>,
    body: Json<CalculateOpenTimeParams>,
) -> impl Responder {
    // let params = web::Query::<CalculateOpenTimeParams>::from_query(req.query_string()).unwrap();

    let open_time =
        calculate_kline_open_time(body.close_time.parse::<u64>().unwrap(), body.interval);

    // Return the stream data as JSON
    let json_data = json!({ "open_time": open_time });
    HttpResponse::Ok().json(json_data)
}

pub fn register_utils_service() -> Scope {
    scope("/utils")
        .service(get_ts)
        .service(calculate_open_time)
        .service(time_difference)
        .service(load_klines)
        .service(date_to_timestamp)
        .service(get_sign_hmac)
        .service(bootstrap_historical_trades)
}
