use async_trait::async_trait;

use futures_util::SinkExt;
use log::warn;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response};
// use reqwest::Client;

use serde_json::{json, Value};
use std::collections::HashMap;

use std::time::Duration;
use tokio::task::JoinHandle;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::account::trade::{OrderSide, Position, TradeTx};
use crate::exchange::api::{ExchangeApi, QueryStr};

use crate::market::messages::MarketMessage;
use crate::market::trade::MarketTrade;
use crate::market::types::{ArcMutex, ArcSender};
use crate::market::{kline::Kline, ticker::Ticker};

use crate::utils::time::generate_ts;

use super::api::ExchangeInfo;

use super::stream::{StreamManager, StreamMeta};
use super::types::{ApiResult, StreamType};

const BING_X_WS_HOST_URL: &str = "wss://open-api-swap.bingx.com/swap-market";
const BING_X_HOST_URL: &str = "https://open-api.bingx.com";
const API_VERSION: &str = "v3";

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

    /// Builds custom HTTP headers for API requests.
    ///
    /// # Arguments
    ///
    /// * `json` - A boolean indicating whether the "Content-Type" header should be set to "application/json".
    ///
    /// # Returns
    ///
    /// Returns a `HeaderMap` containing the constructed headers for the request.

    fn build_headers(&self, json: bool) -> HeaderMap {
        let mut custom_headers = HeaderMap::new();

        // custom_headers.insert(USER_AGENT, HeaderValue::from_static("binance-rs"));
        if json {
            custom_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        custom_headers.insert(
            "X-BX-APIKEY",
            HeaderValue::from_str(self.api_key.as_str()).expect("Unable to get API key"),
        );

        custom_headers
    }

    /// Performs an HTTP GET request to the specified endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - A string slice specifying the endpoint for the GET request.
    /// * `query_str` - An optional string slice containing the query string to be appended to the endpoint.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with the response `Response` object if the request is successful, or an error of type `reqwest::Error` otherwise.

    async fn get(
        &self,
        endpoint: &str,
        query_str: Option<&str>,
        body: Option<String>,
    ) -> Result<Response, reqwest::Error> {
        // let signature = self.sign_query_str(query_str);
        let url = match query_str {
            Some(qs) => format!("{}{}?{}", self.host, endpoint, qs),
            None => format!("{}{}", self.host, endpoint),
        };

        let body = match body {
            Some(b) => b.to_string(),
            None => "".to_string(),
        };

        self.client
            .get(&url)
            .headers(self.build_headers(true))
            .body(body)
            .send()
            .await
    }

    /// Performs an HTTP POST request to the specified endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - A string slice specifying the endpoint for the POST request.
    /// * `query_str` - A string slice containing the body of the POST request.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with the response `Response` object if the request is successful, or an error of type `reqwest::Error` otherwise.

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

    /// Processes the HTTP response, extracting the relevant data based on the content type.
    ///
    /// This method checks the content type of the response and accordingly parses the response body as either plain text or JSON. It is designed to handle different response formats gracefully, ensuring that the data is correctly extracted from various API endpoints.
    ///
    /// # Arguments
    ///
    /// * `response` - The `Response` object received from an HTTP request.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Value>`, which is a `Result` type that either contains the parsed data as a `serde_json::Value` or an error if the response processing fails.

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

    /// Signs a query string using the API secret key.
    ///
    /// This method is used to generate a signature for secured endpoints. The signature is generated using HMAC SHA256, based on the query string and the secret key.
    ///
    /// # Arguments
    ///
    /// * `query_str` - A string slice containing the query string to be signed.
    ///
    /// # Returns
    ///
    /// Returns a string representing the hexadecimal value of the signature.

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

    fn format_bingx_symbol(symbol: &str, lower_case: bool) -> String {
        let symbol: String = symbol.replace("USDT", "-USDT");

        if lower_case {
            return symbol.to_lowercase();
        }

        symbol
    }
}

#[async_trait]
impl ExchangeApi for BingXApi {
    /// Initiates an asynchronous request to retrieve the balance of the account.
    ///
    /// This method asynchronously queries the exchange to fetch the current balance of the trading account. It encapsulates the necessary API call, handling any authentication and request formatting internally.
    ///
    /// # Returns
    ///
    /// An `ApiResult<f64>` representing the successful retrieval of the account balance as a floating-point number. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }

    /// Fetches the latest k-line (candlestick) data for a specified symbol and interval.
    ///
    /// This method queries the exchange for the most recent k-line data of the given trading pair and interval. It's useful for strategies that require up-to-date market data to make informed decisions.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The market symbol for the trading pair.
    /// * `interval` - The interval between k-lines, such as "1m" for one minute.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Kline>`, encapsulating the latest k-line data. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn get_kline(&self, symbol: &str, interval: &str) -> ApiResult<Kline> {
        get_bingx_kline(symbol, interval).await
    }

    /// Retrieves the current ticker information for a specified symbol.
    ///
    /// This method queries the exchange for the latest market ticker of the given trading pair. The ticker includes price changes, high, low, and other relevant market data.
    ///
    /// # Arguments
    ///
    /// * `_symbol` - The market symbol for the trading pair.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Ticker>`, providing the current market ticker data. If the operation fails, it returns an error within `ApiResult`.

    async fn get_ticker(&self, symbol: &str) -> ApiResult<Ticker> {
        get_bingx_ticker(symbol).await
    }

    /// Opens a new trading position on the exchange with specified parameters.
    ///
    /// This method places an order to open a new trading position based on the symbol, margin used, leverage, order side (buy/sell), and the specified opening price. It constructs the request, signs it, and sends it to the exchange.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The market symbol for the trading pair.
    /// * `margin_usd` - The amount of margin in USD to be used for this position.
    /// * `leverage` - The leverage to apply to the position.
    /// * `order_side` - The side of the order, either `OrderSide::Buy` or `OrderSide::Sell`.
    /// * `open_price` - The price at which to attempt to open the position.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Position>` indicating the successful creation of a trading position, or an error if the operation fails.

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

    /// Closes an existing trading position on the exchange.
    ///
    /// This method sends a request to the exchange to close a specific trading position at the specified price. It handles the necessary calculations to close the position based on its current state.
    ///
    /// # Arguments
    ///
    /// * `position` - The `Position` object representing the trading position to close.
    /// * `close_price` - The price at which the position should be closed.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<TradeTx>` representing the transaction details of the closed position, or an error if the operation fails.

    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx> {
        // TODO: make api request to close position
        Ok(TradeTx::new(close_price, generate_ts(), position))
    }

    /// Retrieves the account information from the exchange.
    ///
    /// This asynchronous method sends a request to the exchange to get detailed information about the trading account, including balances for each asset.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Value>`, where `Value` is a JSON representation of the account information. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn get_account(&self) -> ApiResult<Value> {
        let endpoint = "/openApi/swap/v2/user/balance";
        // let endpoint = "/openApi/spot/v1/account/balance";
        let ts = generate_ts().to_string();

        let query_str = QueryStr::new(vec![("timestamp", &ts)]);

        let signature = self.sign_query_str(&query_str.to_string());

        let query_str = QueryStr::new(vec![("timestamp", &ts), ("signature", &signature)]);

        // let body = json!({
        //     "timestamp": &ts,
        //     "signature": &signature
        // });

        let res = self
            .get(endpoint, Some(&query_str.to_string()), None)
            .await?;

        self.handle_response(res).await
    }

    /// Lists all orders associated with the account, including historical orders.
    ///
    /// This asynchronous method sends a request to the exchange to retrieve a comprehensive list of all orders placed by the account, allowing for a complete audit trail of trading activity.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<Value>`, where `Value` is a JSON array of orders. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn all_orders(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/allOrderList";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str), None).await?;

        self.handle_response(res).await
    }

    /// Retrieves a list of all open (active) orders for the account.
    ///
    /// This method queries the exchange for any orders that have been placed but not yet filled or canceled. It's essential for managing and monitoring current market positions.
    ///
    /// # Returns
    ///
    /// An `ApiResult<Value>` that contains a JSON array of open orders. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn list_open_orders(&self) -> ApiResult<Value> {
        let endpoint = "/api/v3/openOrderList";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str), None).await?;

        self.handle_response(res).await
    }

    // ---
    // Exchange Methods
    // ---

    /// Provides general information about the exchange, such as supported symbols and limits.
    ///
    /// This method sends an asynchronous request to fetch metadata about the exchange, including the names of supported trading pairs, rate limits, and other relevant data.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<ExchangeInfo>`, encapsulating various pieces of information about the exchange. In case of an error, it returns an appropriate error encapsulated within `ApiResult`.

    async fn info(&self) -> ApiResult<ExchangeInfo> {
        let endpoint = "/api/v3/exchangeInfo";

        let _res = self.get(endpoint, None, None).await?;

        // self.handle_response(res).await

        Ok(ExchangeInfo {
            name: "BingX".to_string(),
        })
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

/// Manages streaming connections for market data from BingX, specifically handling ticker and kline data streams.
///
/// This struct maintains separate collections for ticker and kline streams, each identified by a unique key. It orchestrates the setup, management, and teardown of these streams, ensuring that market data is continuously processed and relayed.
///
/// # Fields
///
/// - `ticker_streams`: A map holding active ticker streams, where each stream is identified by a symbol and associated with a task handle for asynchronous operation.
/// - `kline_streams`: Similar to `ticker_streams`, but specifically for kline (candlestick data) streams, facilitating the tracking and management of multiple kline data feeds.
/// - `market_sender`: A channel sender used to dispatch market data messages (e.g., new klines or tickers) to a designated receiver for further processing.
/// - `stream_metas`: A thread-safe structure storing metadata for each stream, including details like the stream's symbol, type, and last update time.

pub struct BingXStreamManager {
    ticker_streams: HashMap<String, JoinHandle<()>>,
    kline_streams: HashMap<String, JoinHandle<()>>,
    market_sender: ArcSender<MarketMessage>,
    stream_metas: ArcMutex<HashMap<String, StreamMeta>>,
}

impl BingXStreamManager {
    /// Initializes a new instance of `BingXStreamManager` with a given market message sender.
    ///
    /// # Arguments
    ///
    /// * `market_sender`: An `ArcSender` for `MarketMessage` used to send market data updates.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `BingXStreamManager`, ready to manage streaming connections for both ticker and kline data from BingX.

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
    /// Opens a new stream based on the provided `StreamMeta` configuration, either for ticker or kline data.
    ///
    /// For ticker streams, it periodically fetches the latest ticker information at a fixed interval and sends it through `market_sender`. For kline streams, it subscribes to a websocket endpoint for real-time updates.
    ///
    /// # Arguments
    ///
    /// * `stream_meta`: The metadata defining the stream's symbol, type (ticker or kline), and other relevant details.
    ///
    /// # Returns
    ///
    /// Returns the unique ID of the opened stream as a `String` wrapped in an `ApiResult`.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot be opened or if there's an issue with fetching or sending the data.

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
            StreamType::MarketTrade => {
                let market_sender = self.market_sender.clone();

                let thread_handle = tokio::spawn(async move {
                    loop {
                        // TODO: Implement get market trade
                        let trade = MarketTrade::default();
                        let _ = market_sender.send(MarketMessage::UpdateMarketTrade(trade));

                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                });

                self.kline_streams
                    .insert(stream_meta.id.clone(), thread_handle);
            }
        };

        Ok(stream_meta.id.to_string())
    }

    /// Closes an active stream identified by its unique ID.
    ///
    /// This method terminates the corresponding streaming task for either ticker or kline data and removes its metadata from tracking.
    ///
    /// # Arguments
    ///
    /// * `stream_id`: A `&str` representing the unique ID of the stream to be closed.
    ///
    /// # Returns
    ///
    /// Returns an `Option<StreamMeta>` containing the metadata of the closed stream if it exists, or `None` if the stream could not be found.

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

    /// Provides access to the internal storage of stream metadata.
    ///
    /// This method is primarily used within the `BingXStreamManager` to query or modify metadata about active streams.
    ///
    /// # Returns
    ///
    /// Returns an `ArcMutex<HashMap<String, StreamMeta>>`, a thread-safe reference to the map holding stream metadata.

    fn stream_metas(&self) -> ArcMutex<HashMap<String, StreamMeta>> {
        self.stream_metas.clone()
    }
}

/// Fetches the latest Kline data for a given symbol and interval from BingX's open API.
///
/// This function adjusts the interval format to match BingX API requirements, constructs the query string, and sends a GET request to the BingX kline endpoint.
///
/// # Arguments
///
/// * `symbol` - A string slice representing the trading symbol (e.g., "BTC-USDT").
/// * `interval` - A string slice representing the candlestick chart interval (e.g., "1min", "5min").
///
/// # Returns
///
/// Returns an `ApiResult<Kline>`, which is either the latest Kline data for the symbol and interval if successful, or an error message if the request fails or data is incomplete.

pub async fn get_bingx_kline(symbol: &str, interval: &str) -> ApiResult<Kline> {
    let symbol = BingXApi::format_bingx_symbol(symbol, false);
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
    let ts = generate_ts().to_string();

    let client = reqwest::Client::new();
    let query_str = QueryStr::new(vec![
        ("symbol", &symbol),
        ("interval", &_interval),
        ("timestamp", &ts),
        ("limit", "1"),
    ]);

    let url: String = format!(
        "{}/openApi/swap/v3/quote/klines?{}",
        BING_X_HOST_URL,
        query_str.to_string()
    );

    let res = client.get(url).send().await?;

    let kline_str = res.json::<Value>().await?.to_string();

    // build kline from hashmap
    let lookup: HashMap<String, Value> = serde_json::from_str(&kline_str).unwrap();

    let data = lookup.get("data").ok_or_else(|| {
        // Create an error message or construct an error type
        "Missing 'data' key from data kline lookup".to_string()
    })?;

    let data: Vec<Value> = serde_json::from_value(data.to_owned())?;
    let data = data[0].clone();
    let data: HashMap<String, Value> = serde_json::from_value(data.to_owned())?;

    let kline = Kline::from_bingx_lookup(data, &symbol, interval)?;

    Ok(kline)
}

/// Fetches the latest ticker information for a given symbol from BingX's open API.
///
/// Constructs the query string and sends a GET request to the BingX ticker endpoint. Parses the JSON response into a `Ticker` struct.
///
/// # Arguments
///
/// * `symbol` - A string slice representing the trading symbol (e.g., "BTC-USDT").
///
/// # Returns
///
/// Returns an `ApiResult<Ticker>`, which is either the latest ticker data for the symbol if successful, or an error message if the request fails or data is incomplete.

pub async fn get_bingx_ticker(symbol: &str) -> ApiResult<Ticker> {
    let client = reqwest::Client::new();
    let ts = generate_ts().to_string();
    let symbol = BingXApi::format_bingx_symbol(symbol, false);
    let query_str = QueryStr::new(vec![("symbol", &symbol), ("timestamp", &ts)]);
    let url = format!(
        "{}/openApi/swap/v2/quote/ticker?{}",
        BING_X_HOST_URL,
        query_str.to_string()
    );

    let res = client.get(url).send().await?;

    let ticker_str = res.json::<Value>().await?.to_string();

    let lookup: HashMap<String, Value> = serde_json::from_str(&ticker_str).unwrap();
    let data = lookup.get("data").ok_or_else(|| {
        // Create an error message or construct an error type
        "Missing 'data' key from data ticker lookup".to_string()
    })?;
    let data: HashMap<String, Value> = serde_json::from_value(data.to_owned()).unwrap();

    // build kline from hashmap
    let ticker = Ticker::from_bingx_lookup(data)?;

    Ok(ticker)
}
