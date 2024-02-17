use actix_web::HttpRequest;
use actix_web::{
    get, post,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::Deserialize;
use serde_json::json;

use crate::account::trade::OrderSide;
use crate::app::AppState;
use crate::strategy::strategy::Strategy;

#[derive(Debug, Deserialize)]
pub struct NewStrategyParams {
    symbol: String,
}
#[post("/new-strategy")]
async fn new_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<NewStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategy_id = bot.lock().await.add_strategy(&body.symbol).await;

    let json_data = json!({ "success": "Strategy started","strategy_id":strategy_id });

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct StopStrategyParams {
    strategy_id: String,
}
#[post("/stop-strategy")]
async fn stop_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<StopStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategy_id = bot.lock().await.stop_strategy(&body.strategy_id).await;

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
        bot.lock().await.stop_strategy(id).await;
    }

    let json_data = json!({ "strategies_stopped": strategies });

    HttpResponse::Ok().json(json_data)
}

pub fn register_strategy_service() -> Scope {
    scope("/strategy")
        .service(new_strategy)
        .service(stop_strategy)
        .service(get_strategies)
        .service(stop_all_strategies)
}
