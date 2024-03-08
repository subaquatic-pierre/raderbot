use std::collections::HashMap;

use serde::Serialize;

use async_trait::async_trait;

use crate::{
    exchange::types::StreamType, market::interval::Interval, market::types::ArcMutex,
    utils::time::generate_ts,
};

use super::types::ApiResult;

/// Provides an interface for managing data streams in a concurrent environment.
///
/// This trait defines the essential functionalities for opening and closing streams, as well as
/// accessing active streams and their metadata. Implementors of this trait can manage streams
/// related to financial market data, such as price tickers and klines, in a real-time trading bot
/// or data aggregation system.

#[async_trait]
pub trait StreamManager: Send + Sync {
    /// Opens a stream with the provided metadata.
    ///
    /// # Arguments
    ///
    /// * `stream_meta` - Metadata for the stream to be opened.
    ///
    /// # Returns
    ///
    /// Returns the ID of the opened stream if successful, or an error message.

    async fn open_stream(&mut self, stream_meta: StreamMeta) -> ApiResult<String>;

    /// Closes the stream with the specified ID.
    ///
    /// # Arguments
    ///
    /// * `stream_id` - The ID of the stream to be closed.
    ///
    /// # Returns
    ///
    /// Returns metadata of the closed stream if successful, or `None` if the stream was not found.

    async fn close_stream(&mut self, stream_id: &str) -> Option<StreamMeta>;

    /// Retrieves metadata of all active streams.
    ///
    /// # Returns
    ///
    /// Returns metadata of all active streams as a vector of `StreamMeta` structs.

    async fn active_streams(&self) -> Vec<StreamMeta> {
        let metas = self.stream_metas();
        let stream_data = metas.lock().await;

        let mut metas = vec![];

        for (_, stream_meta) in stream_data.iter() {
            let meta = stream_meta.clone();
            metas.push(meta);
        }

        metas
    }

    /// Retrieves an Arc containing the mutex-protected HashMap of stream metadata.
    ///
    /// # Returns
    ///
    /// Returns metadata of all streams as an `ArcMutex<HashMap<String, StreamMeta>>`.
    fn stream_metas(&self) -> ArcMutex<HashMap<String, StreamMeta>>;
}

/// A struct representing metadata for a stream.
#[derive(Serialize, Clone, Debug)]
pub struct StreamMeta {
    /// The ID of the stream.
    pub id: String,
    /// The URL of the stream.
    pub url: String,
    /// The time when the stream was started.
    pub started_time: u64,
    /// The type of stream.
    pub stream_type: StreamType,
    /// The time when the stream was last updated.
    pub last_update: u64,
    /// The symbol associated with the stream.
    pub symbol: String,
    /// The interval of the stream, if applicable.
    pub interval: Option<Interval>,
}

impl StreamMeta {
    /// Creates a new `StreamMeta` instance.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the stream.
    /// * `url` - The URL of the stream.
    /// * `symbol` - The symbol associated with the stream.
    /// * `stream_type` - The type of stream.
    /// * `interval` - The interval of the stream, if applicable.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `StreamMeta`.
    pub fn new(
        id: &str,
        url: &str,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<Interval>,
    ) -> Self {
        Self {
            id: id.to_string(),
            url: url.to_string(),
            started_time: generate_ts(),
            stream_type,
            last_update: generate_ts(),
            symbol: symbol.to_string(),
            interval,
        }
    }
}

impl Default for StreamMeta {
    fn default() -> Self {
        Self {
            id: "unknown".to_string(),
            url: "unknown".to_string(),
            started_time: 42,
            stream_type: StreamType::Ticker,
            last_update: 123,
            symbol: "unknown".to_string(),
            interval: None,
        }
    }
}

/// Builds a stream ID based on the symbol and interval.
///
/// # Arguments
///
/// * `symbol` - The symbol associated with the stream.
/// * `interval` - The interval of the stream, if applicable.
///
/// # Returns
///
/// Returns the ID of the stream.
pub fn build_stream_id(
    symbol: &str,
    stream_type: StreamType,
    interval: Option<Interval>,
) -> String {
    match stream_type {
        StreamType::Kline => {
            if let Some(interval) = interval {
                format!("{}@kline_{}", symbol, interval)
            } else {
                format!("{}@ticker", symbol)
            }
        }
        StreamType::Ticker => {
            format!("{}@ticker", symbol)
        }
        StreamType::Trade => {
            format!("{}@trade", symbol)
        }
    }
}
