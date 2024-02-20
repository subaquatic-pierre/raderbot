use actix_web::web::Json;
use actix_web::{
    get, post,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bot::AppState;

#[derive(Debug, Deserialize)]
pub struct NewStrategyParams {
    symbol: String,
    strategy_name: String,
    interval: String,
}
#[post("/new-strategy")]
async fn new_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<NewStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategy_id = bot
        .lock()
        .await
        .add_strategy(&body.strategy_name, &body.symbol, &body.interval)
        .await;

    match strategy_id {
        Ok(strategy_id) => {
            let json_data = json!({ "success": "Strategy started","strategy_id":strategy_id });

            HttpResponse::Ok().json(json_data)
        }
        Err(_e) => {
            let json_data = json!({ "error": "Unable to find strategy_name"});
            HttpResponse::ExpectationFailed().json(json_data)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StopStrategyParams {
    strategy_id: u32,
}
#[post("/stop-strategy")]
async fn stop_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<StopStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategy_id = bot.lock().await.stop_strategy(body.strategy_id).await;

    let json_data = json!({ "success": "Strategy stopped","strategy_id":strategy_id });

    HttpResponse::Ok().json(json_data)
}

#[get("/active-strategies")]
async fn get_strategies(app_data: web::Data<AppState>) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategies = bot.lock().await.get_strategies().await;

    let json_data = json!({ "strategies": strategies });

    HttpResponse::Ok().json(json_data)
}

#[get("/stop-all-strategies")]
async fn stop_all_strategies(app_data: web::Data<AppState>) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategies = bot.lock().await.get_strategies().await;

    for id in &strategies {
        bot.lock().await.stop_strategy(*id).await;
    }

    let json_data = json!({ "strategies_stopped": strategies });

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct RunBackTestParams {
    symbol: String,
    strategy_name: String,
    interval: String,
    from_ts: u64,
    to_ts: u64,
}
#[post("/run-back-test")]
async fn run_back_test(
    app_data: web::Data<AppState>,
    body: Json<RunBackTestParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let result = bot
        .lock()
        .await
        .run_back_test(
            &body.strategy_name,
            &body.symbol,
            &body.interval,
            body.from_ts,
            body.to_ts,
        )
        .await;

    match result {
        Ok(result) => {
            let json_data = json!({ "result": result });

            HttpResponse::Ok().json(json_data)
        }
        Err(e) => {
            let json_data = json!({ "error": e.to_string()});
            HttpResponse::ExpectationFailed().json(json_data)
        }
    }
}

pub fn register_strategy_service() -> Scope {
    scope("/strategy")
        .service(new_strategy)
        .service(stop_strategy)
        .service(get_strategies)
        .service(stop_all_strategies)
        .service(run_back_test)
}
