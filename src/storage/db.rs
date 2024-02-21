use super::manager::StorageManager;
use crate::market::kline::Kline;

pub struct DbStorageManager {}

impl StorageManager for DbStorageManager {
    fn load_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<Kline> {
        unimplemented!()
    }

    fn save_klines(&self, klines: &[Kline], kline_key: &str) -> std::io::Result<()> {
        unimplemented!()
    }
}
