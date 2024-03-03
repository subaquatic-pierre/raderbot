use actix_web::HttpRequest;
use actix_web::{
    get,
    web::{self, scope},
    HttpResponse, Responder, Scope,
};

use crate::app::AppState;

#[get("/account")]
async fn account(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange
        .get_account()
        .await
        .expect("Unable to get account info");

    // Return the stream data as JSON
    HttpResponse::Ok().json(data)
}

#[get("/info")]
async fn info(app_data: web::Data<AppState>, _req: HttpRequest) -> impl Responder {
    let exchange = app_data.get_exchange_api().await;

    let data = exchange.info().await.expect("Unable to get exchange info");

    // Return the stream data as JSON
    HttpResponse::Ok().json(data)
}

pub fn register_exchange_service() -> Scope {
    scope("/exchange").service(info).service(account)
}
