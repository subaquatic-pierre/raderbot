use actix_web::web::Json;
use actix_web::{
    get, post,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::account::trade::{Position, TradeTx};
use crate::app::AppState;
use crate::strategy::strategy::{StrategyId, StrategySettings};
use crate::utils::time::string_to_timestamp;

#[derive(Debug, Deserialize)]
pub struct NewStrategyParams {
    symbol: String,
    strategy_name: String,
    algorithm_params: Value,
    interval: String,
    margin: Option<f64>,
    leverage: Option<u32>,
}
#[post("/new-strategy")]
async fn new_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<NewStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let settings = StrategySettings {
        max_open_orders: 2,
        margin_usd: body.margin.unwrap_or(1000.0),
        leverage: body.leverage.unwrap_or(10),
        stop_loss: None,
    };

    let info = bot
        .lock()
        .await
        .start_strategy(
            &body.strategy_name,
            &body.symbol,
            &body.interval,
            settings,
            body.algorithm_params.clone(),
        )
        .await;

    match info {
        Ok(info) => {
            let json_data = json!({ "success": "Strategy started", "strategy_info": info });

            HttpResponse::Ok().json(json_data)
        }
        Err(e) => {
            let json_data = json!({ "error": e.to_string()});
            HttpResponse::ExpectationFailed().json(json_data)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetStrategyParams {
    strategy_id: StrategyId,
    close_positions: Option<bool>,
}
#[post("/stop-strategy")]
async fn stop_strategy(
    app_data: web::Data<AppState>,
    body: web::Json<GetStrategyParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let close_positions = body.close_positions.unwrap_or(true);

    let summary = bot
        .lock()
        .await
        .stop_strategy(body.strategy_id, close_positions)
        .await;

    let json_data = json!({ "success": "Strategy stopped","strategy_summary":summary });

    HttpResponse::Ok().json(json_data)
}

#[post("/list-positions")]
async fn list_strategy_positions(
    app_data: web::Data<AppState>,
    body: web::Json<GetStrategyParams>,
) -> impl Responder {
    let account = app_data.get_account().await;

    let positions: Vec<Position> = account
        .lock()
        .await
        .strategy_positions(body.strategy_id)
        .iter()
        .map(|&el| el.clone())
        .collect();

    let json_data = json!({ "strategy_positions": positions });

    HttpResponse::Ok().json(json_data)
}

#[post("/summary")]
async fn active_strategy_summary(
    app_data: web::Data<AppState>,
    body: web::Json<GetStrategyParams>,
) -> impl Responder {
    let mut bot = app_data.bot.lock().await;
    let account = bot.account.clone();

    if let Some(strategy) = bot.get_strategy(body.strategy_id) {
        let summary = strategy.summary(account).await;
        let json_data = json!({ "strategy_summary": summary });

        return HttpResponse::Ok().json(json_data);
    };

    let json_data = json!({ "error": "Unable to find strategy", "strategy_id": body.strategy_id });

    HttpResponse::ExpectationFailed().json(json_data)
}

#[post("/info")]
async fn strategy_info(
    app_data: web::Data<AppState>,
    body: web::Json<GetStrategyParams>,
) -> impl Responder {
    let mut bot = app_data.bot.lock().await;
    if let Some(strategy) = bot.get_strategy(body.strategy_id) {
        let info = strategy.info().await;
        let json_data = json!({ "strategy_info": info });

        return HttpResponse::Ok().json(json_data);
    };

    let json_data = json!({ "error": "Unable to find strategy" });

    HttpResponse::ExpectationFailed().json(json_data)
}

#[get("/active-strategies")]
async fn list_active_strategies(app_data: web::Data<AppState>) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategy_ids = bot.lock().await.get_active_strategy_ids();

    let mut infos = vec![];

    for id in strategy_ids {
        if let Some(strategy) = bot.lock().await.get_strategy(id) {
            infos.push(strategy.info().await.clone())
        }
    }

    let json_data = json!({ "strategy_infos": infos });

    HttpResponse::Ok().json(json_data)
}

#[get("/historical-strategies")]
async fn list_historical_strategies(app_data: web::Data<AppState>) -> impl Responder {
    let bot = app_data.bot.clone();

    let summaries = bot.lock().await.list_historical_strategies();

    let json_data = json!({ "strategy_summaries": summaries });

    HttpResponse::Ok().json(json_data)
}

#[post("/historical-summary")]
async fn historical_strategy_summary(
    app_data: web::Data<AppState>,
    body: Json<GetStrategyParams>,
) -> impl Responder {
    if let Some(summary) = app_data
        .bot
        .lock()
        .await
        .get_historical_strategy_summary(body.strategy_id)
    {
        let json_data = json!({ "strategy_summary": summary });

        HttpResponse::Ok().json(json_data)
    } else {
        let json_data =
            json!({ "error": "Historical data not found", "strategy_id": body.strategy_id });

        HttpResponse::Ok().json(json_data)
    }
}

#[derive(Serialize, Deserialize)]
struct StopAllStrategiesParams {
    close_positions: Option<bool>,
}
#[post("/stop-all-strategies")]
async fn stop_all_strategies(
    app_data: web::Data<AppState>,
    body: Json<StopAllStrategiesParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();

    let strategies = bot.lock().await.get_active_strategy_ids();

    let close_positions = body.close_positions.unwrap_or(true);

    for id in &strategies {
        bot.lock().await.stop_strategy(*id, close_positions).await;
    }

    let json_data = json!({ "strategies_stopped": strategies });

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct SetStrategyParams {
    strategy_id: StrategyId,
    params: Value,
}
#[post("/set-params")]
async fn set_strategy_params(
    app_data: web::Data<AppState>,
    body: Json<SetStrategyParams>,
) -> impl Responder {
    if let Some(strategy) = app_data.bot.lock().await.get_strategy(body.strategy_id) {
        if let Err(err) = strategy.set_algorithm_params(body.params.clone()).await {
            let json_data = json!({ "error": err.to_string() });
            HttpResponse::Ok().json(json_data)
        } else {
            let updated_params = strategy.get_algorithm_params().await;
            let json_data = json!({ "success": { "updated_params": updated_params } });
            HttpResponse::Ok().json(json_data)
        }
    } else {
        let json_data = json!({ "error": "Unable to find strategy" });
        HttpResponse::Ok().json(json_data)
    }
}

#[derive(Debug, Deserialize)]
pub struct ChangeSettingsParams {
    strategy_id: StrategyId,
    settings: StrategySettings,
}
#[post("/change-settings")]
async fn change_strategy_settings(
    app_data: web::Data<AppState>,
    body: Json<ChangeSettingsParams>,
) -> impl Responder {
    if let Some(strategy) = app_data.bot.lock().await.get_strategy(body.strategy_id) {
        strategy.change_settings(body.settings.clone());
        let json_data = json!({ "success": { "updated_settings": body.settings } });

        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Unable to find strategy" });
        HttpResponse::Ok().json(json_data)
    }
}

#[derive(Debug, Deserialize)]
pub struct RunBackTestParams {
    symbol: String,
    strategy_name: String,
    algorithm_params: Value,
    interval: String,
    margin: Option<f64>,
    leverage: Option<u32>,
    from_ts: String,
    to_ts: String,
}
#[post("/run-back-test")]
async fn run_back_test(
    app_data: web::Data<AppState>,
    body: Json<RunBackTestParams>,
) -> impl Responder {
    let bot = app_data.bot.clone();
    let settings = StrategySettings {
        max_open_orders: 2,
        margin_usd: body.margin.unwrap_or_else(|| 1000.0),
        leverage: body.leverage.unwrap_or_else(|| 10),
        stop_loss: None,
    };

    let from_ts = string_to_timestamp(&body.from_ts);
    let to_ts = string_to_timestamp(&body.to_ts);
    if from_ts.is_err() || to_ts.is_err() {
        let json_data = json!({ "error": "Unable to parse dates".to_string()});
        return HttpResponse::ExpectationFailed().json(json_data);
    }

    // SAFETY: Error check above
    let from_ts = from_ts.unwrap();
    let to_ts = to_ts.unwrap();

    let result = bot
        .lock()
        .await
        .run_back_test(
            &body.strategy_name,
            &body.symbol,
            &body.interval,
            from_ts,
            to_ts,
            settings,
            body.algorithm_params.clone(),
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
        .service(stop_all_strategies)
        .service(set_strategy_params)
        .service(change_strategy_settings)
        .service(list_active_strategies)
        .service(strategy_info)
        .service(list_strategy_positions)
        .service(active_strategy_summary)
        .service(list_historical_strategies)
        .service(historical_strategy_summary)
        .service(run_back_test)
}
