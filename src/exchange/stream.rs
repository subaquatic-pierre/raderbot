use std::collections::HashMap;

use serde::Serialize;

use async_trait::async_trait;

use crate::{exchange::types::StreamType, market::types::ArcMutex, utils::time::generate_ts};

use super::types::ApiResult;

#[async_trait]
pub trait StreamManager: Send + Sync {
    async fn open_stream(&mut self, stream_meta: StreamMeta) -> ApiResult<String>;

    async fn close_stream(&mut self, stream_id: &str) -> Option<StreamMeta>;

    //
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

    // Need trait method to get Arc of Stream Metas to be used in WebSocket threads
    fn stream_metas(&self) -> ArcMutex<HashMap<String, StreamMeta>>;
}

#[derive(Serialize, Clone, Debug)]
pub struct StreamMeta {
    pub id: String,
    pub url: String,
    pub started_time: u64,
    pub stream_type: StreamType,
    pub last_update: u64,
    pub symbol: String,
    pub interval: Option<String>,
}

impl StreamMeta {
    pub fn new(
        id: &str,
        url: &str,
        symbol: &str,
        stream_type: StreamType,
        interval: Option<String>,
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

pub fn build_stream_id(symbol: &str, interval: Option<&str>) -> String {
    if let Some(interval) = interval {
        format!("{}@kline_{}", symbol, interval)
    } else {
        format!("{}@ticker", symbol)
    }
}
