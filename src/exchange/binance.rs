use async_trait::async_trait;

use futures_util::SinkExt;
use log::info;
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
use crate::market::trade::MarketTrade;
use crate::market::types::{ArcMutex, ArcSender};
use crate::market::{kline::Kline, ticker::Ticker};
use crate::utils::number::{parse_f64_from_lookup, parse_f64_from_value, parse_usize_from_value};
use crate::utils::time::generate_ts;

use super::api::ExchangeInfo;

use super::stream::{StreamManager, StreamMeta};
use super::types::{ApiResult, StreamType};

/// Represents the Binance API client for interacting with the Binance exchange.
///
/// This client provides methods for making API calls to Binance, handling requests and responses, and managing streams for real-time data. It encapsulates details such as the base URLs for REST and WebSocket endpoints, API keys for authentication, and a stream manager for handling data streams.

pub struct BinanceApi {
    ws_host: String,
    host: String,
    client: Client,
    api_key: String,
    secret_key: String,
    stream_manager: ArcMutex<Box<dyn StreamManager>>,
}

impl BinanceApi {
    /// Creates a new instance of `BinanceApi`.
    ///
    /// Initializes the API client with API keys for authentication, sets up the HTTP client for making requests, and prepares the stream manager for managing data streams.
    ///
    /// # Arguments
    ///
    /// * `api_key` - A string slice holding the Binance API key.
    /// * `secret_key` - A string slice holding the Binance secret key.
    /// * `market_sender` - An `ArcSender<MarketMessage>` for sending market-related messages through the system.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `BinanceApi`.

    pub fn new(
        api_key: &str,
        secret_key: &str,
        market_sender: ArcSender<MarketMessage>,
        test_net: bool,
    ) -> Self {
        let (ws_host, host) = if test_net {
            let host = "https://testnet.binancefuture.com".to_string();
            let ws_host = "wss://fstream.binancefuture.com".to_string();
            (ws_host, host)
        } else {
            let ws_host = "wss://fstream.binance.com".to_string();
            let host = "https://fapi.binance.com".to_string();
            (ws_host, host)
        };

        // Testnet hosts

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
            "X-MBX-APIKEY",
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

    fn format_binance_symbol(symbol: &str, lower_case: bool) -> String {
        if lower_case {
            return symbol.to_lowercase();
        }

        symbol.to_string()
    }
}

#[async_trait]
impl ExchangeApi for BinanceApi {
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
        let endpoint = "/api/v3/order";
        let quantity = (margin_usd * leverage as f64) / open_price;

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
        let endpoint = "/api/v3/account";
        let ts = generate_ts();

        let query_str = format!("timestamp={ts}");
        let signature = self.sign_query_str(&query_str);
        let query_str = format!("{}&signature={signature}", query_str);

        let res = self.get(endpoint, Some(&query_str)).await?;

        self.handle_response(res).await
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
        let format_symbol = BinanceApi::format_binance_symbol(symbol, false);
        let endpoint =
            format!("/fapi/v1/klines?symbol={format_symbol}&interval={interval}&limit=1");

        let res = self.get(&endpoint, None).await?;

        let data = self.handle_response(res).await?;

        // Response
        // [
        //     [
        //         1499040000000,      // Open time
        //         "0.01634790",       // Open
        //         "0.80000000",       // High
        //         "0.01575800",       // Low
        //         "0.01577100",       // Close
        //         "148976.11427815",  // Volume
        //         1499644799999,      // Close time
        //         "2434.19055334",    // Quote asset volume
        //         308,                // Number of trades
        //         "1756.87402397",    // Taker buy base asset volume
        //         "28.46694368",      // Taker buy quote asset volume
        //         "17928899.62484339" // Ignore.
        //     ]
        // ]

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
        let format_symbol = BinanceApi::format_binance_symbol(symbol, false);
        let endpoint = format!("/fapi/v1/ticker/24hr?symbol={format_symbol}");

        let res = self.get(&endpoint, None).await?;

        let data = self.handle_response(res).await?;

        // TODO: Handle error from Json values gracefully

        let high = parse_f64_from_value("highPrice", &data)?;
        let low = parse_f64_from_value("lowPrice", &data)?;
        let traded_vol = parse_f64_from_value("volume", &data)?;
        let last_price = parse_f64_from_value("lastPrice", &data)?;
        let open_price = parse_f64_from_value("openPrice", &data)?;

        let ticker = Ticker {
            time: generate_ts(),
            symbol: symbol.to_string(),
            high,
            low,
            traded_vol,
            last_price,
            open_price,
        };

        Ok(ticker)
        // Ok(Ticker::default())
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

        let res = self.get(endpoint, Some(&query_str)).await?;

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

        let res = self.get(endpoint, Some(&query_str)).await?;

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

        let _res = self.get(endpoint, None).await?;

        // self.handle_response(res).await

        Ok(ExchangeInfo {
            name: "Binance".to_string(),
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
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) -> String {
        let url = match stream_type {
            StreamType::Kline => {
                format!(
                    "{}/ws/{}@kline_{}",
                    self.ws_host,
                    BinanceApi::format_binance_symbol(symbol, true),
                    interval.unwrap()
                )
            }
            StreamType::Ticker => {
                format!(
                    "{}/ws/{}@ticker",
                    self.ws_host,
                    BinanceApi::format_binance_symbol(symbol, true)
                )
            }
            StreamType::MarketTrade => {
                format!(
                    "{}/ws/{}@aggTrade",
                    self.ws_host,
                    BinanceApi::format_binance_symbol(symbol, true)
                )
            }
        };

        url
    }
}

/// Represents a manager responsible for handling streams from Binance.
///
/// This struct is tasked with managing WebSocket streams for market data such as klines and tickers. It keeps track of active streams, dispatches market messages to a receiver, and manages the lifecycle of each stream.
///
/// # Fields
///
/// - `streams`: A collection of active WebSocket streams identified by their unique stream IDs.
/// - `market_sender`: A channel sender used to forward market messages (e.g., new klines or tickers) to a receiver for processing.
/// - `stream_metas`: A thread-safe container holding metadata about each stream, including its type, symbol, and last update timestamp.

pub struct BinanceStreamManager {
    streams: HashMap<String, ArcEsStreamSync>,
    market_sender: ArcSender<MarketMessage>,
    stream_metas: ArcMutex<HashMap<String, StreamMeta>>,
}

impl BinanceStreamManager {
    /// Constructs a new instance of the Binance stream manager.
    ///
    /// This constructor initializes the stream manager with an empty collection of active streams and a sender for market messages. It's responsible for managing websocket streams for market data updates.
    ///
    /// # Arguments
    ///
    /// * `market_sender` - An `ArcSender<MarketMessage>` used to send market updates to a receiver.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `BinanceStreamManager` with initialized fields.

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
    /// Opens a new stream based on the provided `StreamMeta` information.
    ///
    /// This method establishes a new websocket connection to the Binance API for the specified stream. It listens for messages on the websocket and forwards relevant market data to the `market_sender`.
    ///
    /// # Arguments
    ///
    /// * `stream_meta` - A `StreamMeta` object containing the details of the stream to open, such as the symbol, interval, and stream type.
    ///
    /// # Returns
    ///
    /// Returns an `ApiResult<String>` containing the stream ID if the stream is successfully opened, or an error in case of failure.

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
                                        let lookup: HashMap<String, Value> =
                                            serde_json::from_str(&text).unwrap();

                                        if let Ok(kline) = Kline::from_binance_lookup(lookup) {
                                            let _ = market_sender
                                                .send(MarketMessage::UpdateKline(kline));
                                        }
                                    }
                                    StreamType::Ticker => {
                                        let lookup: HashMap<String, Value> =
                                            serde_json::from_str(&text).unwrap();

                                        if let Ok(ticker) = Ticker::from_binance_lookup(lookup) {
                                            let _ = market_sender
                                                .send(MarketMessage::UpdateTicker(ticker));
                                        }
                                    }
                                    StreamType::MarketTrade => {
                                        let lookup: HashMap<String, Value> =
                                            serde_json::from_str(&text).unwrap();

                                        if let Ok(trade) = MarketTrade::from_binance_lookup(lookup)
                                        {
                                            let _ = market_sender
                                                .send(MarketMessage::UpdateMarketTrade(trade));
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

    /// Closes an active stream identified by its stream ID.
    ///
    /// This method shuts down the specified websocket connection and removes it from the manager's collection of active streams. It's used to stop receiving updates from a particular market data stream.
    ///
    /// # Arguments
    ///
    /// * `stream_id` - A string slice representing the ID of the stream to close.
    ///
    /// # Returns
    ///
    /// Returns an `Option<StreamMeta>` containing the metadata of the closed stream if found and successfully closed, or `None` if the stream ID does not match any active streams.

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_format_binance_symbol() {
        let symbol = "BTC-USDT";
        let formatted_symbol = BinanceApi::format_binance_symbol(symbol, true);
        assert_eq!(formatted_symbol, "btcusdt");
        let formatted_symbol = BinanceApi::format_binance_symbol(symbol, false);
        assert_eq!(formatted_symbol, "BTCUSDT");
    }
}
