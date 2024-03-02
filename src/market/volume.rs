use crate::{
    account::trade::OrderSide,
    utils::time::{
        floor_mili_ts, generate_ts, timestamp_to_datetime, timestamp_to_string, HOUR_AS_MILI,
        MIN_AS_MILI,
    },
};

use super::trade::{MarketTrade, MarketTradeData};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct MarketTradeVolume {
    // pub trade_data: MarketTradeData,
    // Use String as the key type to avoid f64 precision issues
}

impl MarketTradeVolume {
    pub fn new() -> Self {
        Self {
            // trade_data,
        }
    }

    // Method to calculate bucketed volumes with dynamic bucket sizes based on price_granularity
    pub fn calc_volume_buckets(
        &self,
        trades: &[MarketTrade],
        price_granularity: usize,
        time_interval: &str,
    ) -> BucketedVolumeData {
        let (start_time, end_time) = self.calc_start_end_time(trades);
        let (min_price, max_price) = self.calc_min_max(trades);

        if trades.is_empty() || price_granularity == 0 {
            return BucketedVolumeData {
                volume_by_price_bucket: HashMap::new(),
                volume_by_time_bucket: HashMap::new(),
                start_time,
                end_time,
                total_buy_volume: 0.0,
                total_sell_volume: 0.0,
                price_granularity: 0,
                min_price,
                max_price,
                poc: 0.0,
                price_range: 0.0,
            };
        }

        let price_range = max_price - min_price;

        let volume_by_price_bucket = self.calc_price_buckets(trades, price_granularity);
        let volume_by_time_bucket = self.calc_time_buckets(trades, time_interval);

        let total_vol = self.calc_total_vol(&volume_by_price_bucket);

        let poc = self.get_poc(&volume_by_price_bucket);

        BucketedVolumeData {
            volume_by_price_bucket,
            volume_by_time_bucket,
            end_time,
            start_time,
            total_buy_volume: total_vol.buy_volume,
            total_sell_volume: total_vol.sell_volume,
            price_granularity,
            min_price,
            max_price,
            poc,
            price_range,
        }
    }

    pub fn get_poc(&self, volume_by_price_bucket: &HashMap<String, BucketVolume>) -> f64 {
        let mut max_vol = "0".to_string();
        for (key, bucket) in volume_by_price_bucket {
            let bucket_total = bucket.buy_volume + bucket.sell_volume;
            if bucket_total > max_vol.parse::<f64>().unwrap() {
                max_vol = key.to_string();
            }
        }
        // calculate bucket with greatest volume
        // return the key
        max_vol.parse::<f64>().unwrap()
    }

    pub fn calc_price_buckets(
        &self,
        trades: &[MarketTrade],
        price_granularity: usize,
    ) -> HashMap<String, BucketVolume> {
        let mut volume_by_price_bucket = HashMap::new();

        let (min_price, max_price) = self.calc_min_max(trades);
        let price_range = max_price - min_price;
        let bucket_size = price_range / price_granularity as f64;

        for trade in trades {
            let bucket_index = ((trade.price - min_price) / bucket_size).floor();
            let bucket_key = min_price + bucket_index * bucket_size;
            let bucket_key_str = format!("{:.2}", bucket_key);

            let volume_entry = volume_by_price_bucket
                .entry(bucket_key_str)
                .or_insert_with(BucketVolume::default);
            if trade.order_side == OrderSide::Buy {
                volume_entry.buy_volume += trade.qty;
            } else {
                volume_entry.sell_volume += trade.qty;
            }
        }
        volume_by_price_bucket
    }

    pub fn calc_time_buckets(
        &self,
        trades: &[MarketTrade],
        time_interval: &str,
    ) -> HashMap<String, BucketVolume> {
        let mut volume_by_time_bucket = HashMap::new();

        for trade in trades {
            let timestamp = match time_interval {
                "1m" => floor_mili_ts(trade.timestamp, 1 * MIN_AS_MILI),
                "5m" => floor_mili_ts(trade.timestamp, 5 * MIN_AS_MILI),
                "15m" => floor_mili_ts(trade.timestamp, 15 * MIN_AS_MILI),
                "1h" => floor_mili_ts(trade.timestamp, HOUR_AS_MILI),
                _ => floor_mili_ts(trade.timestamp, HOUR_AS_MILI),
            };
            let bucket_key_str = timestamp_to_string(timestamp);

            let volume_entry = volume_by_time_bucket
                .entry(bucket_key_str)
                .or_insert_with(BucketVolume::default);
            if trade.order_side == OrderSide::Buy {
                volume_entry.buy_volume += trade.qty;
            } else {
                volume_entry.sell_volume += trade.qty;
            }
        }

        volume_by_time_bucket
    }

    pub fn calc_min_max(&self, trades: &[MarketTrade]) -> (f64, f64) {
        let min_price = trades
            .iter()
            .map(|t| t.price)
            .min_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap();
        let max_price = trades
            .iter()
            .map(|t| t.price)
            .max_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap();
        (min_price, max_price)
    }

    pub fn calc_start_end_time(&self, trades: &[MarketTrade]) -> (String, String) {
        let start_time = if let Some(trade) = trades.first() {
            timestamp_to_string(trade.timestamp)
        } else {
            timestamp_to_string(generate_ts())
        };
        let end_time = if let Some(trade) = trades.last() {
            timestamp_to_string(trade.timestamp)
        } else {
            timestamp_to_string(generate_ts())
        };

        (start_time, end_time)
    }

    pub fn calc_total_vol(&self, bucketed_vols: &HashMap<String, BucketVolume>) -> BucketVolume {
        let mut total_buy_volume = 0.0;
        let mut total_sell_volume = 0.0;

        for bucket in bucketed_vols.values() {
            total_buy_volume += bucket.buy_volume;
            total_sell_volume = bucket.sell_volume;
        }

        BucketVolume {
            buy_volume: total_buy_volume,
            sell_volume: total_sell_volume,
        }
    }
}

#[derive(Serialize, Default, Clone)]
pub struct BucketVolume {
    pub buy_volume: f64,
    pub sell_volume: f64,
}

#[derive(Serialize)]
pub struct BucketedVolumeData {
    // Using HashMap to map bucket keys to volumes
    pub volume_by_price_bucket: HashMap<String, BucketVolume>,
    pub volume_by_time_bucket: HashMap<String, BucketVolume>,
    pub start_time: String,
    pub end_time: String,
    pub total_sell_volume: f64,
    pub total_buy_volume: f64,
    pub price_granularity: usize,
    pub min_price: f64,
    pub max_price: f64,
    pub price_range: f64,
    pub poc: f64,
}
