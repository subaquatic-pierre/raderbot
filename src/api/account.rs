use actix_web::{
    get,
    web::{self, scope, Json},
    HttpResponse, Responder, Scope,
};
use actix_web::{post, HttpRequest};

use serde::Deserialize;
use serde_json::json;

use crate::account::trade::OrderSide;
use crate::bot::AppState;

#[derive(Debug, Deserialize)]
pub struct ClosePosParams {
    position_id: u64,
}
#[post("/close-position")]
async fn close_position(
    _app_data: web::Data<AppState>,
    body: Json<ClosePosParams>,
) -> impl Responder {
    let json_data = json!({ "success": "Position Closed","position_id":body.position_id });

    HttpResponse::Ok().json(json_data)
}

#[derive(Debug, Deserialize)]
pub struct OpenPosParams {
    symbol: String,
    margin: f64,
    leverage: u32,
    order_side: OrderSide,
    stop_loss: Option<f64>,
}
#[post("/open-position")]
async fn open_position(app_data: web::Data<AppState>, body: Json<OpenPosParams>) -> impl Responder {
    let account = app_data.get_account().await;

    let res = account
        .lock()
        .await
        .open_position(
            &body.symbol,
            body.margin,
            body.leverage,
            body.order_side.clone(),
            body.stop_loss,
        )
        .await;

    if let Some(res) = res {
        let json_data = json!({ "success": "Position Opened","data":res });
        HttpResponse::Ok().json(json_data)
    } else {
        let json_data = json!({ "error": "Unable to open position" });
        HttpResponse::ExpectationFailed().json(json_data)
    }
}

#[get("/list-positions")]
async fn list_positions(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let account = app_data.get_account().await;
    let positions = account.lock().await.positions().await;

    let json_data = json!({ "positions": positions });

    HttpResponse::Ok().json(json_data)
}

#[get("/get-account")]
async fn get_account(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    // let account = app_data.get_account().await;
    // let positions = account.lock().await.positions().await;
    let exchange_api = app_data.get_exchange_api().await;

    let res = exchange_api.get_account().await;

    let value = match res {
        Ok(data) => data,
        Err(e) => {
            let msg = format!("{e:?}");
            json!({ "error": msg })
        }
    };

    let json_data = json!({ "response": value });

    HttpResponse::Ok().json(json_data)
}

pub fn register_account_service() -> Scope {
    scope("/account")
        .service(get_account)
        .service(open_position)
        .service(close_position)
        .service(list_positions)
}
