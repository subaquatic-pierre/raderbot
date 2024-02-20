use async_trait::async_trait;

use futures_util::SinkExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response};
// use reqwest::Client;

use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;

use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::account::trade::{OrderSide, Position, TradeTx};
use crate::exchange::api::{ExchangeApi, QueryStr};
use crate::exchange::types::ArcEsStreamSync;
use crate::market::messages::MarketMessage;
use crate::market::types::{ArcMutex, ArcSender};
use crate::market::{kline::Kline, ticker::Ticker};
use crate::utils::time::generate_ts;

use super::stream::build_stream_id;
use super::stream::{StreamManager, StreamMeta};
use super::types::{ApiResult, StreamType};

pub struct BinanceApi {
    ws_host: String,
    host: String,
    client: Client,
    api_key: String,
    secret_key: String,
    stream_manager: ArcMutex<Box<dyn StreamManager>>,
}

impl BinanceApi {
    pub fn new(api_key: &str, secret_key: &str, market_sender: ArcSender<MarketMessage>) -> Self {
        let _ws_host = "wss://stream.binance.com".to_string();
        let _host = "https://api.binance.com".to_string();

        // Testnet hosts
        let host = "https://testnet.binance.vision".to_string();
        let ws_host = "wss://testnet.binance.vision".to_string();

        let stream_manager: ArcMutex<Box<dyn StreamManager>> =
            ArcMutex::new(Box::new(BinanceStreamManager::new(market_sender)));

        Self {
            ws_host,
            host,
            client: Client::builder().build().unwrap(),
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            stream_manager,
        }
    }

    pub fn parse_kline(res_str: &str) -> ApiResult<Kline> {
        let lookup: HashMap<String, Value> = serde_json::from_str(res_str).unwrap();

        // build kline from hashmap
        Kline::from_binance_lookup(lookup)
    }

    pub fn parse_ticker(res_str: &str) -> ApiResult<Ticker> {
        let lookup: HashMap<String, Value> = serde_json::from_str(res_str).unwrap();

        // build kline from hashmap
        Ticker::from_binance_lookup(lookup)
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
impl ExchangeApi for BinanceApi {
    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }

    async fn open_position(
        &self,
        symbol: &str,
        order_side: OrderSide,
        quantity: f64,
        open_price: f64,
    ) -> ApiResult<Position> {
        let endpoint = "/api/v3/order";

        // format qty to 8 decimals
        let _qty = format!("{:.1$}", quantity, 8);

        let ts = &generate_ts().to_string();
        let side = &order_side.to_string();
        let quote_qty = 50.to_string();

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

        println!("qry_str: {query_str}");

        let res = self.post(endpoint, &query_str).await?;

        match self.handle_response(res).await {
            Ok(_res) => {
                // parse response
                // build position from response
                Ok(Position::new(symbol, open_price, order_side, None, 0.0, 1))
            }
            Err(e) => Err(e),
        }
    }

    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx> {
        // TODO: make api request to close position
        let position = Position::new("SOME", 0.0, OrderSide::Buy, None, 0.0, 1);
        Ok(TradeTx::new(1.0, 0, position))
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

    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline> {
        let endpoint = format!("/api/v3/klines?symbol={symbol}&interval={interval}&limit=1");

        let res = self.get(&endpoint, None).await?;

        let data = self.handle_response(res).await?;

        // println!("{res:?}");

        let arr: Vec<Vec<Value>> = serde_json::from_value(data).unwrap();
        let open_time = arr[0][0].as_u64().unwrap();
        let open = arr[0][1].as_str().unwrap().parse::<f64>().unwrap();
        let high = arr[0][2].as_str().unwrap().parse::<f64>().unwrap();
        let low = arr[0][3].as_str().unwrap().parse::<f64>().unwrap();
        let close = arr[0][4].as_str().unwrap().parse::<f64>().unwrap();
        let volume = arr[0][5].as_str().unwrap().parse::<f64>().unwrap();
        let close_time = arr[0][6].as_u64().unwrap();

        Ok(Kline {
            interval: interval.to_string(),
            symbol: symbol.to_string(),
            open_time,
            open,
            high,
            low,
            close,
            volume,
            close_time,
        })
    }

    async fn get_ticker(&self, _symbol: &str) -> ApiResult<Ticker> {
        Ok(Ticker::default())
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
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) -> String {
        let url = match stream_type {
            StreamType::Kline => {
                format!(
                    "{}/ws/{}@kline_{}",
                    self.ws_host,
                    symbol.to_lowercase(),
                    interval.unwrap()
                )
            }
            StreamType::Ticker => {
                format!("{}/ws/{}@ticker", self.ws_host, symbol.to_lowercase(),)
            }
        };

        url
    }
}

pub struct BinanceStreamManager {
    streams: HashMap<String, ArcEsStreamSync>,
    market_sender: ArcSender<MarketMessage>,
    stream_metas: ArcMutex<HashMap<String, StreamMeta>>,
}

impl BinanceStreamManager {
    pub fn new(market_sender: ArcSender<MarketMessage>) -> Self {
        Self {
            streams: HashMap::new(),
            market_sender,
            stream_metas: ArcMutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl StreamManager for BinanceStreamManager {
    async fn open_stream(&mut self, stream_meta: StreamMeta) -> ApiResult<String> {
        let (ws_stream, _) = connect_async(stream_meta.url.to_string())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to create new kline stream for stream type: {} with symbol: {}",
                    stream_meta.stream_type, stream_meta.symbol
                )
            });

        // Split the Websocket to use sync to close connection
        let (sync, mut ws_stream) = ws_stream.split();

        let stream_metas = self.stream_metas();

        stream_metas
            .lock()
            .await
            .insert(stream_meta.id.to_string(), stream_meta.clone());

        let sync = ArcMutex::new(sync);
        self.streams.insert(stream_meta.id.clone(), sync);

        let market_sender = self.market_sender.clone();

        let thread_stream_id = stream_meta.id.clone();

        // Spawn client web socket to listen for kline
        tokio::spawn(async move {
            while let Some(result) = ws_stream.next().await {
                match result {
                    // Forward message to receiver
                    Ok(msg) => match msg {
                        // Handle received message
                        // If text message then can create new Kline
                        Message::Text(text) => {
                            if let Some(stream_meta) =
                                stream_metas.lock().await.get_mut(&thread_stream_id)
                            {
                                stream_meta.last_update = generate_ts();
                                match stream_meta.stream_type {
                                    StreamType::Kline => {
                                        let kline = BinanceApi::parse_kline(&text);

                                        if let Ok(kline) = kline {
                                            let _ = market_sender
                                                .send(MarketMessage::UpdateKline(kline));
                                        }
                                    }
                                    StreamType::Ticker => {
                                        let ticker = BinanceApi::parse_ticker(&text);

                                        if let Ok(ticker) = ticker {
                                            let _ = market_sender
                                                .send(MarketMessage::UpdateTicker(ticker));
                                        }
                                    }
                                }
                            };
                        }

                        Message::Close(_frame) => {
                            stream_metas.lock().await.remove(&thread_stream_id);
                        }

                        Message::Ping(_data) => {
                            // ignore Ping Pong Messages
                        }
                        Message::Pong(_data) => {
                            // ignore Ping Pong Messages
                        }
                        _ => {
                            println!("Received unexpected data: {:?}", msg);
                        }
                    },
                    Err(e) => {
                        // Handle error
                        eprintln!("Error receiving message: {:?}", e);
                    }
                }
            }
        });

        Ok(stream_meta.id.to_string())
    }

    async fn close_stream(&mut self, stream_id: &str) -> Option<StreamMeta> {
        let mut infos = self.stream_metas.lock().await;

        if let Some(stream_meta) = infos.get_mut(stream_id) {
            if let Some(sync) = self.streams.get(stream_id) {
                let _ = sync.lock().await.close().await;
            }
            return Some(stream_meta.clone());
        }

        None
    }

    // ---
    // Accessor methods for trait
    // ---
    fn stream_metas(&self) -> ArcMutex<HashMap<String, StreamMeta>> {
        self.stream_metas.clone()
    }
}
