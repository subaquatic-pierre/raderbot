use futures::StreamExt;

use log::info;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, sync::Arc};

// use tokio::time::{self, Duration};

use crate::exchange::api::ExchangeInfo;
use crate::exchange::stream::build_stream_id;
use crate::exchange::types::{ApiResult, StreamType};
use crate::utils::kline::{build_kline_key, build_ticker_key};
use crate::utils::time::{floor_mili_ts, interval_to_millis, MIN_AS_MILI, SEC_AS_MILI};
use crate::utils::trade::build_market_trade_key;
use crate::{
    exchange::{
        api::ExchangeApi,
        stream::{StreamManager, StreamMeta},
    },
    market::{
        kline::{Kline, KlineData, KlineMeta},
        messages::MarketMessage,
        ticker::{Ticker, TickerData, TickerMeta},
        types::ArcReceiver,
    },
    storage::manager::StorageManager,
    utils::time::generate_ts,
};

use super::trade::{Trade, TradeData, TradeDataMeta};
use super::types::ArcMutex;
use super::volume::MarketTradeVolume;

/// Represents the main market data structure for a trading application, managing market data streams, and integrating with exchange APIs.

pub struct Market {
    market_receiver: ArcReceiver<MarketMessage>,
    data: ArcMutex<MarketData>,
    exchange_api: Arc<Box<dyn ExchangeApi>>,
    needed_streams: ArcMutex<Vec<StreamMeta>>,
}

impl Market {
    /// Represents the main structure for managing market data and interactions with exchange APIs.
    ///
    /// This structure coordinates the reception of market messages, the management of data streams, and
    /// interaction with exchange APIs to fetch and process market data. It initializes data structures
    /// for storing kline and ticker information and manages streams to keep the market data updated.
    ///
    /// # Parameters
    ///
    /// - `market_receiver`: A receiver for market messages, including updates to klines and tickers.
    /// - `exchange_api`: A dynamic interface to the exchange API, allowing for fetching market data and
    ///   interacting with the exchange.
    /// - `storage_manager`: Manages the persistence of market data, ensuring data is saved and can be
    ///   retrieved for analysis.
    /// - `init_workers`: Indicates whether to initialize background tasks for processing market data
    ///   and managing streams upon creation of the market structure.
    ///
    /// # Returns
    ///
    /// An instance of `Market`, ready to process market data and interact with the exchange API.

    pub async fn new(
        market_receiver: ArcReceiver<MarketMessage>,
        exchange_api: Arc<Box<dyn ExchangeApi>>,
        storage_manager: Arc<Box<dyn StorageManager>>,
        init_workers: bool,
    ) -> Self {
        let mut _self = Self {
            data: ArcMutex::new(MarketData::new(storage_manager)),
            market_receiver,
            exchange_api,
            needed_streams: ArcMutex::new(vec![]),
        };

        if init_workers {
            _self.init().await;
        }

        _self
    }

    // ---
    // Data Methods
    // ---

    /// Fetches the latest price for a specified symbol, if available.
    ///
    /// This method attempts to retrieve the most recent price for a given symbol from the ticker data.
    /// It's a crucial function for strategies and analyses that rely on the latest market prices.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The trading symbol for which the latest price is requested.
    ///
    /// # Returns
    ///
    /// An `Option<f64>` representing the latest price of the symbol if available; otherwise, `None`.

    pub async fn last_price(&self, symbol: &str) -> Option<f64> {
        let ticker = self.last_ticker(symbol).await;

        ticker.map(|ticker| ticker.last_price)
    }

    /// Retrieves the most recent kline data for a specified symbol and interval.
    ///
    /// This method fetches the latest kline (candlestick) data, providing essential information for
    /// market analysis and decision-making processes.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The trading symbol for which kline data is requested.
    /// - `interval`: The time interval for the kline data.
    ///
    /// # Returns
    ///
    /// An `Option<Kline>` containing the most recent kline data if available; otherwise, `None`.

    pub async fn last_kline(&self, symbol: &str, interval: &str) -> Option<Kline> {
        let last_open_time = generate_ts() - interval_to_millis(interval);

        let kline = match self
            .data
            .lock()
            .await
            .kline_data(symbol, interval, Some(last_open_time), None, None)
            .await
        {
            Some(kline_data) => {
                // info!("Getting Kline from kline_data on on Market");
                kline_data.klines().last().cloned()
            }
            None => {
                // info!("Getting kline from remote API, kline_data doesn't exist on Market");
                let kline = match self.exchange_api.get_kline(symbol, interval).await {
                    Ok(kline) => Some(kline),
                    Err(_e) => None,
                };
                kline
            }
        };

        kline
    }

    /// Retrieves the most recent ticker data for a specified symbol.
    ///
    /// This method fetches the latest ticker data, which includes the last trade price among other
    /// information, crucial for real-time market analysis.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The trading symbol for which ticker data is requested.
    ///
    /// # Returns
    ///
    /// An `Option<Ticker>` containing the most recent ticker data if available; otherwise, `None`.

    pub async fn last_ticker(&self, symbol: &str) -> Option<Ticker> {
        // must be within the last second
        let last_sec = generate_ts() - SEC_AS_MILI;
        let ticker = match self.data.lock().await.ticker_data(symbol, last_sec) {
            Some(ticker_data) => {
                // info!("Getting Ticker from ticker_data on on Market");

                ticker_data.tickers().last().cloned()
            }
            None => {
                // info!("Getting Ticker remote API ticker_data older than 1 second or not found on Market");
                let ticker = match self.exchange_api.get_ticker(symbol).await {
                    Ok(ticker) => Some(ticker),
                    Err(_) => None,
                };
                ticker
            }
        };
        ticker
    }

    /// Fetches a range of Kline data for a specified symbol and interval, optionally filtered by timestamps and limited in size.
    ///
    /// This method retrieves Kline data from the internal market data structure based on the provided symbol and interval. It supports filtering the data by start and end timestamps (`from_ts` and `to_ts`) and limiting the number of Kline data points returned.
    ///
    /// # Parameters
    ///
    /// - `symbol`: A `&str` representing the trading pair or market symbol for which Kline data is requested.
    /// - `interval`: A `&str` indicating the time interval between each Kline.
    /// - `from_ts`: An `Option<u64>` specifying the start timestamp for filtering Kline data. If `None`, no start filter is applied.
    /// - `to_ts`: An `Option<u64>` specifying the end timestamp for filtering Kline data. If `None`, no end filter is applied.
    /// - `limit`: An `Option<usize>` limiting the number of Kline data points returned. If `None`, all matching Klines are returned.
    ///
    /// # Returns
    ///
    /// An `Option<KlineData>` containing the filtered range of Kline data, or `None` if no data matches the criteria.

    pub async fn kline_data_range(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Option<KlineData> {
        self.data
            .lock()
            .await
            .kline_data(symbol, interval, from_ts, to_ts, limit)
            .await
    }

    // TODO: docs
    pub async fn trade_data_range(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Option<TradeData> {
        self.data
            .lock()
            .await
            .trade_data(symbol, from_ts, to_ts, limit)
            .await
    }

    /// Provides a shared, thread-safe reference to the market data.
    ///
    /// This method grants access to the current state of market data, including Klines and tickers, managed within the Market instance.
    ///
    /// # Returns
    ///
    /// An `ArcMutex<MarketData>` encapsulating the market data, allowing for concurrent reads and writes.

    pub async fn market_data(&self) -> ArcMutex<MarketData> {
        self.data.clone()
    }

    // ---
    // Stream Methods
    // ---

    /// Retrieves a list of currently active streams within the market data instance.
    ///
    /// This method compiles a list of all streams that have been established and are actively being monitored or interacted with, providing visibility into the real-time data streams.
    ///
    /// # Returns
    ///
    /// A `Vec<StreamMeta>` containing metadata for each active stream, including their identifiers, symbols, types, and intervals.

    pub async fn active_streams(&self) -> Vec<StreamMeta> {
        self.exchange_api.active_streams().await
    }

    /// Initiates a new stream based on the specified parameters and adds it to the list of active streams.
    ///
    /// This method constructs a new stream URL and metadata for a given symbol, stream type, and optionally an interval, then requests the stream manager to open and monitor this stream.
    ///
    /// # Parameters
    ///
    /// - `stream_type`: The `StreamType` indicating the nature of the stream to be opened (e.g., Ticker, Kline).
    /// - `symbol`: A `&str` representing the trading pair or market symbol for which the stream is to be opened.
    /// - `interval`: An optional `&str` specifying the interval for Kline streams. Ignored for Ticker streams.
    ///
    /// # Returns
    ///
    /// An `ApiResult<String>` representing the outcome of the stream opening request, including success with the stream URL or an error message.

    pub async fn open_stream(
        &self,
        stream_type: StreamType,
        symbol: &str,
        interval: Option<&str>,
    ) -> ApiResult<String> {
        let url = self
            .exchange_api
            .build_stream_url(symbol, stream_type.clone(), interval);
        let stream_id = build_stream_id(symbol, stream_type, interval);

        let interval = interval.map(|s| s.to_owned());

        // create new StreamMeta
        let open_stream_meta = StreamMeta::new(&stream_id, &url, symbol, stream_type, interval);
        self.exchange_api
            .get_stream_manager()
            .lock()
            .await
            .open_stream(open_stream_meta)
            .await
    }

    /// Closes an active stream identified by its unique identifier.
    ///
    /// This method requests the stream manager to terminate a specific stream and remove it from the list of active streams, ceasing data flow and interactions with that stream.
    ///
    /// # Parameters
    ///
    /// - `stream_id`: A `&str` containing the unique identifier of the stream to be closed.
    ///
    /// # Returns
    ///
    /// An `Option<StreamMeta>` containing the metadata of the closed stream if successful, or `None` if the stream could not be found or closed.

    pub async fn close_stream(&self, stream_id: &str) -> Option<StreamMeta> {
        self.exchange_api
            .get_stream_manager()
            .lock()
            .await
            .close_stream(stream_id)
            .await
    }

    // ---
    // Init methods
    // ---

    /// Initializes background tasks for receiving and processing market messages and for monitoring
    /// active data streams.
    ///
    /// This method sets up asynchronous tasks to listen for incoming market messages (e.g., kline and
    /// ticker updates) and to ensure required data streams are active, reopening them as necessary.
    /// It's essential for maintaining an up-to-date view of the market.

    async fn init(&self) {
        // Add initial needed streams
        self.add_needed_stream("BTCUSDT", StreamType::Ticker, None)
            .await;
        self.add_needed_stream("BTCUSDT", StreamType::Trade, None)
            .await;
        self.add_needed_stream("BTCUSDT", StreamType::Kline, Some("1m"))
            .await;

        self.init_market_receivers().await;
        self.init_active_stream_monitor().await;
    }

    async fn init_market_receivers(&self) {
        let market_receiver = self.market_receiver.clone();
        let market_data = self.data.clone();

        // let active_streams = self.active_streams.clone();

        // spawn thread to handle stream_manager messages
        tokio::spawn(async move {
            while let Some(message) = market_receiver.lock().await.recv().await {
                // println!("{message:?}");

                match message {
                    MarketMessage::UpdateKline(kline) => {
                        market_data.lock().await.update_kline(kline).await;
                    }
                    MarketMessage::UpdateTicker(ticker) => {
                        market_data.lock().await.update_ticker(ticker).await;
                    }
                    MarketMessage::UpdateMarketTrade(mut trade) => {
                        market_data.lock().await.update_trade(&mut trade).await;
                    }
                }
            }
        });
    }

    async fn init_active_stream_monitor(&self) {
        let stream_manager = self.exchange_api.get_stream_manager();
        let exchange_api = self.exchange_api.clone();
        let needed_streams = self.needed_streams.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                let active_streams = stream_manager.lock().await.active_streams().await;
                for needed_stream_meta in needed_streams.lock().await.iter() {
                    let active_stream_meta = active_streams
                        .iter()
                        .find(|&meta| meta.symbol == needed_stream_meta.symbol);

                    match active_stream_meta {
                        Some(_meta) => {
                            continue;
                        }
                        None => {
                            let need_stream = needed_stream_meta.clone();

                            let _ = exchange_api
                                .get_stream_manager()
                                .lock()
                                .await
                                .open_stream(need_stream)
                                .await;
                        }
                    }
                }
            }
        });
    }

    /// Adds a specified stream to the list of necessary streams to be monitored or interacted with.
    ///
    /// This method queues a stream for opening based on the specified parameters. It constructs
    /// the stream metadata including its unique identifier, URL, symbol, and type, and then
    /// appends this metadata to the internal list of streams that need to be established.
    ///
    /// # Parameters
    ///
    /// - `symbol`: A `&str` specifying the trading pair or market symbol the stream is associated with.
    /// - `stream_type`: A `StreamType` indicating the type of stream to be opened (e.g., Ticker, Kline).
    /// - `interval`: An `Option<&str>` specifying the interval for Kline streams. This parameter is ignored for Ticker streams.

    pub async fn add_needed_stream(
        &self,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) {
        let mut needed_streams = self.needed_streams.lock().await;
        let url = self
            .exchange_api
            .build_stream_url(symbol, stream_type, interval);
        let stream_id = build_stream_id(symbol, stream_type, interval);
        let stream_meta = StreamMeta::new(&stream_id, &url, symbol, stream_type, None);

        needed_streams.push(stream_meta);
    }

    /// Removes a specified stream from the list of necessary streams.
    ///
    /// This method deletes the stream metadata based on the specified parameters from the internal list of streams that need to be monitored or interacted with. It ensures that no further actions or data processing occur for the removed stream.
    ///
    /// # Parameters
    ///
    /// - `symbol`: A `&str` specifying the trading pair or market symbol the stream is associated with.
    /// - `stream_type`: A `StreamType` indicating the type of stream to be removed. This parameter is currently not used but reserved for future functionality.
    /// - `interval`: An `Option<&str>` specifying the interval for Kline streams. This parameter helps identify the correct stream to remove and is ignored for Ticker streams.

    pub async fn remove_needed_stream(
        &self,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<&str>,
    ) {
        let mut needed_streams = self.needed_streams.lock().await;
        let stream_id = build_stream_id(symbol, stream_type, interval);

        needed_streams.retain(|x| x.id != stream_id);
    }

    /// Provides a summary of the current market status, including exchange information and stream details.
    ///
    /// This method compiles a comprehensive overview of the market, detailing active streams and
    /// exchange-specific information. It's useful for monitoring the market's health and connectivity.
    ///
    /// # Returns
    ///
    /// A `MarketInfo` structure containing details about the exchange and the number of active streams.

    pub async fn info(&self) -> MarketInfo {
        MarketInfo {
            exchange_info: self.exchange_api.info().await.ok(),
            num_active_streams: self.active_streams().await.len(),
        }
    }
}

/// Represents aggregated information about the market, including exchange details and the number of active streams.
///
/// This struct is used to encapsulate general information about the market state, such as which exchange is
/// currently in use and how many data streams are actively providing market data.

#[derive(Serialize, Deserialize)]
pub struct MarketInfo {
    exchange_info: Option<ExchangeInfo>,
    num_active_streams: usize,
}

/// A trait defining a common interface for market data symbols.
///
/// This trait allows for polymorphic treatment of different market data types that are identified by a symbol,
/// ensuring that any type implementing this trait can be queried for its symbol.
pub trait MarketDataSymbol {
    fn symbol(&self) -> String;
}

/// Manages and stores market data, including klines and tickers, for various symbols.
///
/// `MarketData` serves as a central repository for storing time series data and ticker information
/// fetched from an exchange. It supports adding new data, retrieving data ranges, and periodic backup
/// to durable storage via a provided `StorageManager`.

pub struct MarketData {
    all_klines: HashMap<String, KlineData>,
    all_tickers: HashMap<String, TickerData>,
    all_trades: HashMap<String, TradeData>,
    storage_manager: Arc<Box<dyn StorageManager>>,
    last_backup: u64,
}

/// Specifies the interval in seconds between consecutive backups of market data.
const BACKUP_INTERVAL_SECS: u64 = MIN_AS_MILI * 1; // 5min

impl MarketData {
    /// Initializes a new instance of MarketData, creating a central repository for both kline and ticker data managed throughout the application lifecycle.
    ///
    /// This constructor sets up the necessary data structures to store market data, including historical klines and real-time tickers. It also initializes the last backup timestamp to the current system time, preparing the system for periodic data persistence.
    ///
    /// # Parameters
    ///
    /// - storage_manager: A shared reference to a storage manager implementing the StorageManager trait, responsible for data persistence and retrieval operations.
    ///
    /// # Returns
    ///
    /// Returns an instance of MarketData, fully initialized and ready for data ingestion and querying.

    pub fn new(storage_manager: Arc<Box<dyn StorageManager>>) -> Self {
        Self {
            storage_manager,
            all_klines: HashMap::new(),
            all_tickers: HashMap::new(),
            all_trades: HashMap::new(),
            last_backup: generate_ts(),
        }
    }

    /// Adds a new kline to the market data repository. This method intelligently handles the insertion of klines, updating existing entries with new data if the kline's open time matches an existing entry, or appending it to the collection otherwise.
    ///
    /// This method also triggers a backup operation to persist klines to disk based on a predefined interval, ensuring data durability and recoverability.
    ///
    /// # Parameters
    ///
    /// - kline: The Kline instance representing the new market data to be added.
    ///
    pub async fn update_kline(&mut self, kline: Kline) {
        // get kline key eg. BTCUSDT@kline_1m
        let kline_key = build_kline_key(&kline.symbol, &kline.interval);

        // add new kline to data if key found for kline symbol
        if let Some(kline_data) = self.all_klines.get_mut(&kline_key) {
            kline_data.add_kline(kline);
        } else {
            let mut new_kline_data = KlineData::new(&kline.symbol, &kline.interval);
            new_kline_data.add_kline(kline);
            self.all_klines
                .insert(kline_key.to_string(), new_kline_data);
        }

        self.handle_data_backup().await;
    }

    /// Updates the latest ticker data for a given symbol. If an entry for the symbol exists, it updates the existing data; otherwise, it creates a new entry with the provided ticker information. This method is crucial for maintaining up-to-date market prices and other relevant ticker information.
    ///
    /// # Parameters
    ///
    /// - ticker: The Ticker instance containing the latest market data for a specific symbol.
    ///
    pub async fn update_ticker(&mut self, ticker: Ticker) {
        let ticker_key = build_ticker_key(&ticker.symbol);

        if let Some(ticker_data) = self.all_tickers.get_mut(&ticker_key) {
            ticker_data.add_ticker(ticker);
        } else {
            let mut new_ticker_data = TickerData::new(&ticker.symbol);
            new_ticker_data.add_ticker(ticker);

            self.all_tickers
                .insert(ticker_key.to_string(), new_ticker_data);
        }

        self.handle_data_backup().await;
    }

    // TODO: write docs
    pub async fn update_trade(&mut self, trade: &mut Trade) {
        let trade_key = build_market_trade_key(&trade.symbol);

        if let Some(trade_data) = self.all_trades.get_mut(&trade_key) {
            trade_data.add_trade(trade);
        } else {
            // create new TradeData for given symbol
            let mut new_trade_data = TradeData::new(&trade.symbol);
            // add new trade
            new_trade_data.add_trade(trade);
            self.all_trades
                .insert(trade_key.to_string(), new_trade_data);
        }

        self.handle_data_backup().await;
    }

    /// Retrieves a range of kline data for a specific symbol and interval, optionally filtered by a start and end timestamp, with a limit on the number of klines returned. This method aggregates data from both in-memory storage and persistent storage, providing a comprehensive view of historical market data.
    ///
    /// # Parameters
    ///
    /// - symbol: The market symbol for which to retrieve kline data.
    /// - interval: The interval or timeframe for the kline data.
    /// - from_ts: An optional start timestamp for filtering the data.
    /// - to_ts: An optional end timestamp for filtering the data.
    /// - limit: An optional maximum number of kline entries to return.
    ///
    /// # Returns
    ///
    /// Returns an Option<KlineData> containing the requested kline data, or None if no data is available.
    pub async fn kline_data(
        &mut self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Option<KlineData> {
        let kline_key = build_kline_key(symbol, interval);

        let mut kline_data = KlineData::new(symbol, interval);

        let in_mem_kline = match self.all_klines.get(&kline_key) {
            Some(kline_data) => kline_data.klines(),
            None => vec![],
        };

        let mut filtered_klines = self
            .storage_manager
            .get_klines(symbol, interval, from_ts, to_ts)
            .await;
        filtered_klines.extend_from_slice(&in_mem_kline);

        // filtered by from_ts and to_ts
        if let Some(from_ts) = from_ts {
            filtered_klines.retain(|kline| kline.open_time >= from_ts);
            if let Some(to_ts) = to_ts {
                filtered_klines.retain(|kline| kline.open_time <= to_ts);
            }
        }

        // Sort the klines by open_time in descending order
        filtered_klines.sort_by(|a, b| a.open_time.cmp(&b.open_time));

        // Limit the number of data points returned
        if let Some(limit) = limit {
            filtered_klines = filtered_klines[..limit].to_vec();
        }

        // Create a new KlineData object to hold the filtered klines

        filtered_klines.iter().for_each(|k| {
            kline_data.add_kline(k.clone());
        });

        if kline_data.meta.len == 0 {
            None
        } else {
            Some(kline_data)
        }
    }

    /// Provides a snapshot of the latest ticker data for a given symbol. This method retrieves the most recent ticker information, offering insights into current market conditions such as the latest price, volume, and price changes.
    ///
    /// # Parameters
    ///
    /// - symbol: The market symbol for which to retrieve the latest ticker data.
    ///
    /// # Returns
    ///
    /// Returns an Option<TickerData> containing the latest ticker information for the specified symbol, or None if the data is unavailable.
    /// return last 20 seconds of tickers for given symbol
    // TODO: implement getting ticker data from storage
    // this is to be able to get ticker data in range
    pub fn ticker_data(&self, symbol: &str, from_ts: u64) -> Option<TickerData> {
        let ticker_key = build_ticker_key(symbol);
        if let Some(ticker_data) = self.all_tickers.get(&ticker_key) {
            // ensure returning data newer that from_ts
            if ticker_data.meta.last_update > from_ts {
                return Some(ticker_data.clone());
            } else {
                return None;
            }
        }

        None
    }

    // TODO: docs
    pub async fn trade_data(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        _limit: Option<usize>,
    ) -> Option<TradeData> {
        let trade_key = build_market_trade_key(symbol);

        let mut market_data = TradeData::new(symbol);

        let in_mem_trades = match self.all_trades.get(&trade_key) {
            Some(trade_data) => trade_data.trades(),
            None => vec![],
        };

        if from_ts.is_none() && to_ts.is_none() {
            in_mem_trades
                .iter()
                .for_each(|t| market_data.add_trade(&mut t.clone()));

            return Some(market_data);
        }

        let mut filtered_trades = self
            .storage_manager
            .get_trades(symbol, from_ts, to_ts)
            .await;
        filtered_trades.extend_from_slice(&in_mem_trades);

        // filtered by from_ts and to_ts
        if let Some(from_ts) = from_ts {
            filtered_trades.retain(|trade| trade.timestamp >= from_ts);
            if let Some(to_ts) = to_ts {
                filtered_trades.retain(|trade| trade.timestamp <= to_ts);
            }
        }

        filtered_trades.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        filtered_trades.iter().for_each(|t| {
            market_data.add_trade(&mut t.clone());
        });

        if market_data.meta.len == 0 {
            None
        } else {
            Some(market_data)
        }
    }

    // ---
    // Private methods
    // ---

    async fn handle_data_backup(&mut self) {
        let now = generate_ts();

        if self.last_backup + BACKUP_INTERVAL_SECS < now {
            // clear all klines
            for (key, kline_data) in self.all_klines.iter_mut() {
                let klines = kline_data.drain_klines(self.last_backup);
                if klines.len() > 0 {
                    self.storage_manager
                        .save_klines(&klines, key, false)
                        .await
                        .expect("Unable to save Klines");
                }
            }

            // Clear trade_data
            for (key, trade_data) in self.all_trades.iter_mut() {
                let trades = trade_data.drain_trades(self.last_backup);
                if trades.len() > 0 {
                    self.storage_manager
                        .save_trades(&trades, key, false)
                        .await
                        .expect("Unable to save trades");
                }
            }

            // Clear ticker_data
            for (key, ticker_data) in self.all_tickers.iter_mut() {
                let tickers = ticker_data.drain_tickers(self.last_backup);
                // TODO: write tickers to storage
                // self.storage_manager
                //     .save_trades(&trades, key, false)
                //     .await
                //     .expect("Unable to save Klines");
            }

            // Update the last backup time
            self.last_backup = now;
        }
    }
}
