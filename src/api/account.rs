use std::sync::Arc;

use actix_web::{
    get,
    web::{self, scope, Json},
    HttpResponse, Responder, Scope,
};
use actix_web::{post, HttpRequest};

use log::info;
use serde::Deserialize;
use serde_json::json;

use crate::{
    account::trade::{OrderSide, Position, PositionId},
    exchange::mock::MockExchangeApi,
    strategy::strategy::StrategyId,
};
use crate::{app::AppState, exchange::api::ExchangeApi};

#[derive(Debug, Deserialize)]
pub struct ClosePosParams {
    position_id: PositionId,
}
#[post("/close-position")]
async fn close_position(
    app_data: web::Data<AppState>,
    body: Json<ClosePosParams>,
) -> impl Responder {
    let account = app_data.get_account().await;
    let market = app_data.get_market().await;
    let market = market.lock().await;
    let mut account = account.lock().await;

    let pos = account.get_position(&body.position_id);
    if pos.is_none() {
        let json_data =
            json!({ "error": "Unable to find position", "position_id":body.position_id });
        return HttpResponse::ExpectationFailed().json(json_data);
    };

    // SAFETY: None check above
    let position = pos.unwrap().clone();

    if let Some(last_price) = market.last_price(&position.symbol).await {
        let res = account.close_position(position.id, last_price).await;

        if let Some(trade) = res {
            let json_data = json!({ "success": "Position Closed", "trade": trade });
            HttpResponse::Ok().json(json_data)
        } else {
            let json_data = json!({ "error": "Unable to close position" });
            HttpResponse::ExpectationFailed().json(json_data)
        }
    } else {
        let json_data = json!({ "error": "Unable to close position, last price not found", "symbol": position.symbol });
        HttpResponse::ExpectationFailed().json(json_data)
    }
}

#[get("/close-all-positions")]
async fn close_all_positions(app_data: web::Data<AppState>) -> impl Responder {
    let account = app_data.get_account().await;
    let market = app_data.get_market().await;
    let market = market.lock().await;
    let mut account = account.lock().await;

    let mut trades = vec![];

    let positions: Vec<Position> = account.positions().map(|pos| pos.clone()).collect();

    for position in positions {
        if let Some(last_price) = market.last_price(&position.symbol).await {
            if let Some(trade) = account.close_position(position.id, last_price).await {
                trades.push(trade.clone())
            }
        } else {
            let json_data = json!({ "error": "Unable to close position, last price not found", "symbol": position.clone().symbol });
            return HttpResponse::ExpectationFailed().json(json_data);
        }
    }

    let json_data = json!({ "trades": trades });
    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct OpenPosParams {
    symbol: String,
    margin: f64,
    leverage: u32,
    order_side: OrderSide,
    stop_loss: Option<f64>,
    strategy_id: Option<StrategyId>,
}
#[post("/open-position")]
async fn open_position(app_data: web::Data<AppState>, body: Json<OpenPosParams>) -> impl Responder {
    let account = app_data.get_account().await;
    let market = app_data.get_market().await;

    let market = market.try_lock();
    let mut account = account.lock().await;

    if let Some(market) = market {
        if let Some(last_price) = market.last_price(&body.symbol).await {
            let res = account
                .open_position(
                    &body.symbol,
                    body.margin,
                    body.leverage,
                    body.order_side.clone(),
                    last_price,
                    body.strategy_id,
                    body.stop_loss,
                )
                .await;

            if let Some(res) = res {
                let json_data = json!({ "success": "Position Opened", "position": res });
                HttpResponse::Ok().json(json_data)
            } else {
                let json_data = json!({ "error": "Unable to open position" });
                HttpResponse::ExpectationFailed().json(json_data)
            }
        } else {
            let json_data = json!({ "error": "Unable to open position" });
            HttpResponse::ExpectationFailed().json(json_data)
        }
    } else {
        let json_data = json!({ "error": "Unable to get market lock" });
        HttpResponse::ExpectationFailed().json(json_data)
    }
}

#[get("/active-positions")]
async fn list_active_positions(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let account = app_data.get_account().await;
    let mut positions = vec![];

    for position in account.lock().await.positions() {
        positions.push(position.clone())
    }

    let json_data = json!({ "positions": positions });

    HttpResponse::Ok().json(json_data)
}

#[get("/trades")]
async fn list_trades(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let account = app_data.get_account().await;
    let mut trades = vec![];

    for trade in account.lock().await.trades() {
        trades.push(trade.clone())
    }

    let json_data = json!({ "trades": trades });

    HttpResponse::Ok().json(json_data)
}

#[get("/account-info")]
async fn account_info(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let account = app_data.get_account().await;
    let info = account.lock().await.info().await;

    let json_data = json!({ "account_info": info });

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
struct SetExchangeApiParams {
    exchange: String,
    dry_run: bool,
}
#[post("/set-exchange-api")]
async fn set_exchange_api(
    app_data: web::Data<AppState>,
    body: Json<SetExchangeApiParams>,
) -> impl Responder {
    let api = match body.exchange.as_str() {
        "Bing" => app_data.get_exchange_api().await,
        "Binance" => app_data.get_exchange_api().await,
        "Mock" => {
            let api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
            api
        }
        _ => {
            let json_data = json!({ "error": "Unknown exchange API" });
            return HttpResponse::Ok().json(json_data);
        }
    };

    let account = app_data.get_account().await;
    account.lock().await.set_exchange_api(api, body.dry_run);
    let info = account.lock().await.info().await;

    let json_data = json!({ "updated_account": info  });

    HttpResponse::Ok().json(json_data)
}

pub fn register_account_service() -> Scope {
    scope("/account")
        .service(account_info)
        .service(set_exchange_api)
        .service(open_position)
        .service(close_position)
        .service(close_all_positions)
        .service(list_active_positions)
        .service(list_trades)
}
