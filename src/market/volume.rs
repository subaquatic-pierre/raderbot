use crate::{
    account::trade::OrderSide,
    utils::time::{
        floor_mili_ts, generate_ts, timestamp_to_datetime, timestamp_to_string, HOUR_AS_MILI,
        MIN_AS_MILI,
    },
};

use super::trade::{Trade, TradeData};
use log::{info, warn};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize)]
pub struct TradePriceVolume {
    pub bucket_size: f64,
    pub buckets: BTreeMap<String, BucketVolume>,
    start_time: u64,
    end_time: u64,
    min_price: f64,
    max_price: f64,
    fixed_price: bool,
}

impl TradePriceVolume {
    pub fn new(bucket_size: f64, fixed_price: bool) -> Self {
        Self {
            bucket_size,
            buckets: BTreeMap::new(),
            min_price: 0.0,
            max_price: 0.0,
            start_time: u64::MAX,
            end_time: 0,
            fixed_price,
        }
    }

    pub fn add_trades(&mut self, trades: &[Trade]) {
        // produce warning if not fixed price bucket size
        // and adding more trades, a user should not be able
        // to add more trades to a variable price bucket TradeVolume
        // this doesn't make sense, it can only be calculated once
        // as variable sized buckets will differ with if more trades
        // are added, it is not a true reflection of trade volume
        // only a fixed size TradeVolume bucket can have trades added
        // to it
        if self.buckets.len() > 0 && !self.fixed_price {
            warn!("You shouldn't add more trades to non 'fixed_sized' TradeVolume");
        }

        self.update_min_max_price(trades);

        self.add_trade_by_price(trades);

        self.update_times(trades);
    }

    fn add_trade_by_price(&mut self, trades: &[Trade]) {
        for trade in trades {
            if trade.timestamp > self.end_time {
                let key = if self.fixed_price {
                    trade.floor_price(self.bucket_size)
                } else {
                    let bucket_index = ((trade.price - self.min_price) / self.bucket_size).floor();
                    let bucket_key = self.min_price + bucket_index * self.bucket_size;
                    bucket_key
                };

                let bucket_key_str = format!("{:.2}", key);

                let volume_entry = self
                    .buckets
                    .entry(bucket_key_str)
                    .or_insert_with(BucketVolume::default);

                if trade.order_side == OrderSide::Buy {
                    volume_entry.buy_volume += trade.qty;
                } else {
                    volume_entry.sell_volume += trade.qty;
                }
            }
        }
    }

    fn add_trade_by_time(&mut self, trades: &[Trade], time_interval: &str) {
        for trade in trades {
            let timestamp = match time_interval {
                "1m" => floor_mili_ts(trade.timestamp, 1 * MIN_AS_MILI),
                "5m" => floor_mili_ts(trade.timestamp, 5 * MIN_AS_MILI),
                "15m" => floor_mili_ts(trade.timestamp, 15 * MIN_AS_MILI),
                "1h" => floor_mili_ts(trade.timestamp, HOUR_AS_MILI),
                _ => floor_mili_ts(trade.timestamp, HOUR_AS_MILI),
            };

            let bucket_key_str = timestamp_to_string(timestamp);

            let volume_entry = self
                .buckets
                .entry(bucket_key_str)
                .or_insert_with(BucketVolume::default);
            if trade.order_side == OrderSide::Buy {
                volume_entry.buy_volume += trade.qty;
            } else {
                volume_entry.sell_volume += trade.qty;
            }
        }
    }

    fn update_min_max_price(&mut self, trades: &[Trade]) {
        let (min, max) = BucketedVolumeData::calc_min_max(trades);

        if min < self.min_price || self.buckets.len() == 0 {
            self.min_price = min
        }

        if max > self.max_price {
            self.max_price = max
        }
    }

    fn update_times(&mut self, trades: &[Trade]) {
        // update first time
        if let Some(trade) = trades.first() {
            if trade.timestamp < self.start_time {
                self.start_time = trade.timestamp
            }
        }

        // update last time
        if let Some(trade) = trades.last() {
            self.end_time = trade.timestamp
        }
    }

    fn poc(&self) -> f64 {
        let mut max_vol = 0.0;
        let mut poc_key = "0".to_string();

        // calculate bucket with greatest volume
        for (key, bucket) in &self.buckets {
            let bucket_total = bucket.buy_volume + bucket.sell_volume;
            if bucket_total > max_vol {
                max_vol = bucket_total;
                poc_key = key.to_string();
            }
        }
        // return the key
        poc_key.parse::<f64>().unwrap()
    }

    fn total_volume(&self) -> BucketVolume {
        let mut total_buy_volume = 0.0;
        let mut total_sell_volume = 0.0;

        for bucket in self.buckets.values() {
            total_buy_volume += bucket.buy_volume;
            total_sell_volume += bucket.sell_volume;
        }

        BucketVolume {
            buy_volume: total_buy_volume,
            sell_volume: total_sell_volume,
        }
    }

    pub fn result(&self) -> BucketedVolumeData {
        let total_volume = self.total_volume();

        BucketedVolumeData {
            num_buckets: self.buckets.len(),
            buckets: self.buckets.clone(),
            end_time: timestamp_to_string(self.end_time),
            start_time: timestamp_to_string(self.start_time),
            total_buy_volume: total_volume.buy_volume,
            total_sell_volume: total_volume.sell_volume,
            min_price: self.min_price,
            max_price: self.max_price,
            poc: self.poc(),
            price_range: self.max_price - self.min_price,
        }
    }
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct BucketVolume {
    pub buy_volume: f64,
    pub sell_volume: f64,
}

#[derive(Serialize, Debug)]
pub struct BucketedVolumeData {
    // Using HashMap to map bucket keys to volumes
    pub num_buckets: usize,
    pub start_time: String,
    pub end_time: String,
    pub total_sell_volume: f64,
    pub total_buy_volume: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub price_range: f64,
    pub poc: f64,
    pub buckets: BTreeMap<String, BucketVolume>,
}

impl Default for BucketedVolumeData {
    fn default() -> Self {
        Self {
            num_buckets: 0,
            buckets: BTreeMap::new(),
            start_time: timestamp_to_string(generate_ts()),
            end_time: timestamp_to_string(generate_ts()),
            total_buy_volume: 0.0,
            total_sell_volume: 0.0,
            min_price: 0.0,
            max_price: 0.0,
            poc: 0.0,
            price_range: 0.0,
        }
    }
}

impl BucketedVolumeData {
    // ---
    // Static Methods
    // ---
    // Method to calculate bucketed volumes with dynamic bucket sizes based on price_granularity
    pub fn calc_volume_buckets(
        trades: &[Trade],
        price_granularity: usize,
        time_interval: &str,
    ) -> BucketedVolumeData {
        let (start_time, end_time) = BucketedVolumeData::calc_start_end_time(trades);
        let (min_price, max_price) = BucketedVolumeData::calc_min_max(trades);

        if trades.is_empty() || price_granularity == 0 {
            return BucketedVolumeData::default();
        }

        let price_range = max_price - min_price;

        let buckets = BucketedVolumeData::calc_price_buckets(trades, price_granularity);
        // let volume_by_time_bucket = BucketedVolumeData::calc_time_buckets(trades, time_interval);

        let total_vol = BucketedVolumeData::calc_total_vol(&buckets);

        let poc = BucketedVolumeData::get_poc(&buckets);

        BucketedVolumeData {
            num_buckets: buckets.len(),
            buckets,
            end_time: timestamp_to_string(end_time),
            start_time: timestamp_to_string(start_time),
            total_buy_volume: total_vol.buy_volume,
            total_sell_volume: total_vol.sell_volume,
            min_price,
            max_price,
            poc,
            price_range,
        }
    }

    pub fn get_poc(volume_by_price_bucket: &BTreeMap<String, BucketVolume>) -> f64 {
        let mut max_vol = 0.0;
        let mut poc_key = "0".to_string();
        for (key, bucket) in volume_by_price_bucket {
            let bucket_total = bucket.buy_volume + bucket.sell_volume;
            if bucket_total > max_vol {
                max_vol = bucket_total;
                poc_key = key.to_string();
            }
        }
        // calculate bucket with greatest volume
        // return the key
        poc_key.parse::<f64>().unwrap()
    }

    pub fn calc_price_buckets(
        trades: &[Trade],
        price_granularity: usize,
    ) -> BTreeMap<String, BucketVolume> {
        let mut volume_by_price_bucket = BTreeMap::new();

        let (min_price, max_price) = BucketedVolumeData::calc_min_max(trades);
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
        trades: &[Trade],
        time_interval: &str,
    ) -> BTreeMap<String, BucketVolume> {
        let mut volume_by_time_bucket = BTreeMap::new();

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

    pub fn calc_min_max(trades: &[Trade]) -> (f64, f64) {
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

    pub fn calc_start_end_time(trades: &[Trade]) -> (u64, u64) {
        let start_time = if let Some(trade) = trades.first() {
            trade.timestamp
        } else {
            generate_ts()
        };
        let end_time = if let Some(trade) = trades.last() {
            trade.timestamp
        } else {
            generate_ts()
        };

        (start_time, end_time)
    }

    pub fn calc_total_vol(bucketed_vols: &BTreeMap<String, BucketVolume>) -> BucketVolume {
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
