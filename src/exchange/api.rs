use async_trait::async_trait;
use reqwest::{header::HeaderMap, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::{error::Error, fmt};

use crate::{
    account::trade::{OrderSide, Position, TradeTx},
    market::{kline::Kline, ticker::Ticker, types::ArcMutex},
};

use super::{
    stream::{build_stream_id, StreamManager, StreamMeta},
    types::{ApiResult, StreamType},
};

#[derive(Debug)]
pub struct ApiError;

impl Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

#[async_trait]
pub trait ExchangeApi: Send + Sync {
    // ---
    // Account methods
    // ---
    async fn get_account(&self) -> ApiResult<Value>;
    async fn get_account_balance(&self) -> ApiResult<f64>;
    async fn open_position(
        &self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
    ) -> ApiResult<Position>;
    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx>;
    async fn all_orders(&self) -> ApiResult<Value>;
    async fn list_open_orders(&self) -> ApiResult<Value>;

    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>>;

    async fn active_streams(&self) -> Vec<StreamMeta> {
        let stream_manager = self.get_stream_manager();
        let stream_manager = stream_manager.lock().await;
        stream_manager.active_streams().await
    }

    // ---
    // Exchange Methods
    // ---
    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline>;
    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker>;
    async fn info(&self) -> ApiResult<ExchangeInfo>;

    fn build_stream_url(
        &self,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) -> String;
}

pub struct QueryStr<'a> {
    params: Vec<(&'a str, &'a str)>,
}

impl<'a> QueryStr<'a> {
    pub fn new(params: Vec<(&'a str, &'a str)>) -> Self {
        Self { params }
    }
}

impl<'a> ToString for QueryStr<'a> {
    fn to_string(&self) -> String {
        let str_vec: Vec<String> = self
            .params
            .iter()
            .map(|(key, val)| format!("{key}={val}&"))
            .collect();

        let mut query_str = str_vec.join("");

        // remove last & from query_str
        query_str.pop();
        query_str
    }
}

#[derive(Serialize, Deserialize)]
pub struct ExchangeInfo {
    pub name: String,
}
