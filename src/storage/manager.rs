use serde::{Deserialize, Serialize};
use std::io::{self};

use crate::market::kline::Kline;

pub trait StorageManager: Send + Sync {
    fn save_klines(&self, klines: &[Kline], kline_key: &str) -> io::Result<()>;

    fn load_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<Kline>;
}
