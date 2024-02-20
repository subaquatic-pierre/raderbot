use async_trait::async_trait;

use futures_util::SinkExt;
use log::warn;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response};
// use reqwest::Client;

use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;

use std::time::Duration;
use tokio::task::JoinHandle;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::account::trade::{OrderSide, Position, PositionId, TradeTx};
use crate::exchange::api::{ExchangeApi, QueryStr};

use crate::market::messages::MarketMessage;
use crate::market::types::{ArcMutex, ArcSender};
use crate::market::{kline::Kline, ticker::Ticker};

use crate::utils::time::generate_ts;

use super::stream::build_stream_id;
use super::stream::{StreamManager, StreamMeta};
use super::types::{ApiResult, StreamType};

const BING_X_WS_HOST_URL: &str = "wss://open-api-swap.bingx.com/swap-market";
const BING_X_HOST_URL: &str = "https://open-api.bingx.com";

pub struct BingXApi {
    ws_host: String,
    host: String,
    client: Client,
    api_key: String,
    secret_key: String,
    stream_manager: ArcMutex<Box<dyn StreamManager>>,
}

impl BingXApi {
    pub fn new(api_key: &str, secret_key: &str, market_sender: ArcSender<MarketMessage>) -> Self {
        let ws_host = BING_X_WS_HOST_URL.to_string();
        let host = BING_X_HOST_URL.to_string();

        // Testnet hosts

        let stream_manager: ArcMutex<Box<dyn StreamManager>> =
            ArcMutex::new(Box::new(BingXStreamManager::new(market_sender)));

        Self {
            ws_host,
            host,
            client: Client::builder().build().unwrap(),
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            stream_manager,
        }
    }

    pub fn parse_kline(res_str: &str, symbol: &str, interval: &str) -> ApiResult<Kline> {
        let lookup: HashMap<String, Value> = serde_json::from_str(res_str).unwrap();

        // build kline from hashmap
        Kline::from_bingx_lookup(lookup, symbol, interval)
    }

    pub fn parse_ticker(res_str: &str) -> ApiResult<Ticker> {
        let lookup: HashMap<String, Value> = serde_json::from_str(res_str).unwrap();

        // build kline from hashmap
        Ticker::from_bingx_lookup(lookup)
    }

    fn build_headers(&self, json: bool) -> HeaderMap {
        let mut custom_headers = HeaderMap::new();

        // custom_headers.insert(USER_AGENT, HeaderValue::from_static("binance-rs"));
        if json {
            custom_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        custom_headers.insert(
            "X-MBX-APIKEY",
            HeaderValue::from_str(self.api_key.as_str()).expect("Unable to get API key"),
        );

        custom_headers
    }

    async fn get(
        &self,
        endpoint: &str,
        query_str: Option<&str>,
    ) -> Result<Response, reqwest::Error> {
        // let signature = self.sign_query_str(query_str);
        let url = match query_str {
            Some(qs) => format!("{}{}?{}", self.host, endpoint, qs),
            None => format!("{}{}", self.host, endpoint),
        };

        self.client
            .get(&url)
            .headers(self.build_headers(true))
            .send()
            .await
    }

    async fn post(&self, endpoint: &str, query_str: &str) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.host, endpoint);
        let body = query_str.to_string();

        self.client
            .post(&url)
            .headers(self.build_headers(true))
            .body(body)
            .send()
            .await
    }

    async fn handle_response(&self, response: Response) -> ApiResult<Value> {
        let data = match &response.headers().get("content-type") {
            Some(header) => {
                if header.to_str().unwrap().contains("text/html") {
                    json!({"text":response.text().await?})
                } else {
                    response.json::<serde_json::Value>().await?
                }
            }
            None => json!({"text":response.text().await?}),
        };

        Ok(data)
    }

    fn sign_query_str(&self, query_str: &str) -> String {
        // Create a new HMAC instance with SHA256
        let mut hmac =
            Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).expect("Invalid key length");

        // Update the HMAC with the data
        hmac.update(query_str.as_bytes());

        // Get the resulting HMAC value
        let result = hmac.finalize();

        // Convert the HMAC value to a string
        hex::encode(result.into_bytes())
    }
}

#[async_trait]
impl ExchangeApi for BingXApi {
    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }

    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline> {
        get_bingx_kline(symbol, interval).await
    }

    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker> {
        get_bingx_ticker(symbol).await
    }

    async fn open_position(
        &self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
    ) -> ApiResult<Position> {
        let quantity = (margin_usd * leverage as f64) / open_price;

        let endpoint = "/api/v3/order";

        // format qty to 8 decimals
        let _qty = format!("{:.1$}", quantity, 8);

        let ts = &generate_ts().to_string();
        let side = &order_side.to_string();
        let quote_qty = quantity.to_string();

        let request_body = QueryStr::new(vec![
            ("symbol", symbol),
            ("quoteOrderQty", &quote_qty),
            // ("quantity", &qty),
            ("type", "MARKET"),
            ("side", side),
            ("timestamp", ts),
        ]);

        let signature = self.sign_query_str(&request_body.to_string());

        let query_str = format!("{}&signature={signature}", request_body.to_string());

        let res = self.post(endpoint, &query_str).await?;

        match self.handle_response(res).await {
            Ok(_res) => {
                // parse response
                // build position from response
                Ok(Position::new(
                    symbol, open_price, order_side, margin_usd, leverage, None,
                ))
            }
            Err(e) => Err(e),
        }
    }

    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx> {
        // TODO: make api request to close position
        Ok(TradeTx::new(close_price, generate_ts(), position))
    }

    async fn get_account(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/account";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str)).await?;

        self.handle_response(res).await
    }

    async fn all_orders(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/allOrderList";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str)).await?;

        self.handle_response(res).await
    }

    async fn list_open_orders(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/openOrderList";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str)).await?;

        self.handle_response(res).await
    }

    // ---
    // Exchange Methods
    // ---
    async fn exchange_info(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/exchangeInfo";

        let res = self.get(endpoint, None).await?;

        self.handle_response(res).await
    }

    // ---
    // Stream Helper methods
    // ---

    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>> {
        self.stream_manager.clone()
    }

    fn build_stream_url(
        &self,
        _symbol: &str,
        _stream_type: StreamType,
        _interval: Option<&str>,
    ) -> String {
        self.ws_host.to_string()
    }
}

pub struct BingXStreamManager {
    ticker_streams: HashMap<String, JoinHandle<()>>,
    kline_streams: HashMap<String, JoinHandle<()>>,
    market_sender: ArcSender<MarketMessage>,
    stream_metas: ArcMutex<HashMap<String, StreamMeta>>,
}

impl BingXStreamManager {
    pub fn new(market_sender: ArcSender<MarketMessage>) -> Self {
        Self {
            ticker_streams: HashMap::new(),
            kline_streams: HashMap::new(),
            market_sender,
            stream_metas: ArcMutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl StreamManager for BingXStreamManager {
    async fn open_stream(&mut self, stream_meta: StreamMeta) -> ApiResult<String> {
        let stream_metas = self.stream_metas();

        stream_metas
            .lock()
            .await
            .insert(stream_meta.id.to_string(), stream_meta.clone());

        // if stream type is ticker, start thread to call http request every 1 second
        // if stream type is kline, subscribe to normal web socket endpoint
        match stream_meta.stream_type {
            StreamType::Ticker => {
                let market_sender = self.market_sender.clone();

                let thread_handle = tokio::spawn(async move {
                    loop {
                        let ticker = get_bingx_ticker(&stream_meta.symbol).await;

                        if let Ok(ticker) = ticker {
                            let _ = market_sender.send(MarketMessage::UpdateTicker(ticker));
                        } else {
                            warn!("Unable to get ticker from BingX API");
                        }

                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                });

                self.ticker_streams
                    .insert(stream_meta.id.clone(), thread_handle);
            }
            StreamType::Kline => {
                let market_sender = self.market_sender.clone();

                let thread_handle = tokio::spawn(async move {
                    loop {
                        let kline = get_bingx_kline(
                            &stream_meta.symbol,
                            &stream_meta
                                .interval
                                .clone()
                                .unwrap_or_else(|| "UNKNOWN".to_string()),
                        )
                        .await;

                        if let Ok(kline) = kline {
                            // let ticker = BingXApi::parse_ticker(&ticker_str);
                            let _ = market_sender.send(MarketMessage::UpdateKline(kline));
                        } else {
                            warn!("Unable to get kline from BingX API");
                        }

                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                });

                self.kline_streams
                    .insert(stream_meta.id.clone(), thread_handle);
            }
        };

        Ok(stream_meta.id.to_string())
    }

    async fn close_stream(&mut self, stream_id: &str) -> Option<StreamMeta> {
        // check if stream_id in ticker streams
        if let Some(sync) = self.ticker_streams.get(stream_id) {
            let _ = sync.abort();
        }

        // check if stream_id in kline streams
        if let Some(sync) = self.kline_streams.get(stream_id) {
            let _ = sync.abort();
        }

        let mut infos = self.stream_metas.lock().await;

        let meta = infos.get(stream_id).cloned();

        infos.remove(stream_id);

        meta
    }

    fn stream_metas(&self) -> ArcMutex<HashMap<String, StreamMeta>> {
        self.stream_metas.clone()
    }
}

pub async fn get_bingx_kline(symbol: &str, interval: &str) -> ApiResult<Kline> {
    // remove last two letters from interval if interval is {number}min
    // api accepts interval as {number}m
    let _interval = if interval.ends_with('n') {
        let mut interval_copy = interval.to_string();
        interval_copy.pop();
        interval_copy.pop();
        interval_copy
    } else {
        interval.to_string()
    };

    let client = reqwest::Client::new();
    let query_str = QueryStr::new(vec![("symbol", symbol), ("interval", &_interval)]);
    let url: String = format!(
        "{}/openApi/swap/v2/quote/klines?{}",
        BING_X_HOST_URL,
        query_str.to_string()
    );

    let res = client.get(url).send().await?;

    let kline_str = res.json::<Value>().await?.to_string();

    let kline = BingXApi::parse_kline(&kline_str, symbol, interval)?;

    Ok(kline)
}

pub async fn get_bingx_ticker(symbol: &str) -> ApiResult<Ticker> {
    let client = reqwest::Client::new();
    let query_str = QueryStr::new(vec![("symbol", symbol)]);
    let url = format!(
        "{}/openApi/swap/v2/quote/ticker?{}",
        BING_X_HOST_URL,
        query_str.to_string()
    );

    let res = client.get(url).send().await?;

    let ticker_str = res.json::<Value>().await?.to_string();

    let ticker = BingXApi::parse_ticker(&ticker_str)?;

    Ok(ticker)
}
