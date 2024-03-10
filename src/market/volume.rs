use crate::{
    account::trade::OrderSide,
    market::trade::Trade,
    utils::{
        time::{floor_mili_ts, generate_ts, timestamp_to_string, HOUR_AS_MILI, MIN_AS_MILI},
        trade::{calc_min_max, calc_total_volume},
    },
};

use log::{info, warn};
use serde::Serialize;
use std::collections::BTreeMap;

pub trait TradeVolume {
    fn add_trades(&mut self, trades: &[Trade]);
    fn result(&self) -> impl Serialize;
}

#[derive(Serialize, Debug)]
pub struct PriceVolume {
    pub bucket_size: f64,
    pub buckets: BTreeMap<String, BucketVolume>,
    start_time: u64,
    end_time: u64,
    min_price: f64,
    max_price: f64,
    fixed_price: bool,
}

impl PriceVolume {
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

    pub fn reset_volumes(&mut self) {
        self.buckets = BTreeMap::new();
        self.min_price = 0.0;
        self.max_price = 0.0;
        self.start_time = u64::MAX;
        self.end_time = 0;
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

    fn update_min_max_price(&mut self, trades: &[Trade]) {
        let (min, max) = calc_min_max(trades);

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
}

impl TradeVolume for PriceVolume {
    fn add_trades(&mut self, trades: &[Trade]) {
        // produce warning if not fixed price bucket size
        // and adding more trades, a user should not be able
        // to add more trades to a variable price bucket TradeVolume
        // this doesn't make sense, it can only be calculated once
        // as variable sized buckets will differ with if more trades
        // are added, it is not a true reflection of trade volume
        // only a fixed size TradeVolume bucket can have trades added
        // to it
        if self.buckets.len() > 0 && !self.fixed_price {
            warn!("You shouldn't add more trades to non 'fixed_price' buckets in PriceVolume");
        }

        self.update_min_max_price(trades);

        self.add_trade_by_price(trades);

        self.update_times(trades);
    }

    fn result(&self) -> PriceVolumeData {
        let total_volume = calc_total_volume(&self.buckets);

        PriceVolumeData {
            num_buckets: self.buckets.len(),
            buckets: self.buckets.clone(),
            end_time: timestamp_to_string(self.end_time),
            start_time: timestamp_to_string(self.start_time),
            total_volume,
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
pub struct PriceVolumeData {
    pub num_buckets: usize,
    pub start_time: String,
    pub end_time: String,
    pub total_volume: BucketVolume,
    pub min_price: f64,
    pub max_price: f64,
    pub price_range: f64,
    pub poc: f64,
    pub buckets: BTreeMap<String, BucketVolume>,
}

impl Default for PriceVolumeData {
    fn default() -> Self {
        Self {
            num_buckets: 0,
            buckets: BTreeMap::new(),
            start_time: timestamp_to_string(generate_ts()),
            end_time: timestamp_to_string(generate_ts()),
            total_volume: BucketVolume::default(),
            min_price: 0.0,
            max_price: 0.0,
            poc: 0.0,
            price_range: 0.0,
        }
    }
}

pub struct TimeVolume {
    pub interval: String,
    pub buckets: BTreeMap<String, BucketVolume>,
    start_time: u64,
    end_time: u64,
}

impl TimeVolume {
    pub fn new(interval: &str) -> Self {
        Self {
            interval: interval.to_string(),
            buckets: BTreeMap::new(),
            start_time: u64::MAX,
            end_time: 0,
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

    fn add_trade_by_time(&mut self, trades: &[Trade]) {
        for trade in trades {
            let timestamp = match self.interval.as_str() {
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
}

impl TradeVolume for TimeVolume {
    fn add_trades(&mut self, trades: &[Trade]) {
        self.add_trade_by_time(trades);
        self.update_times(trades)
    }

    fn result(&self) -> TimeVolumeData {
        let total_volume = calc_total_volume(&self.buckets);

        TimeVolumeData {
            num_buckets: self.buckets.len(),
            start_time: timestamp_to_string(self.start_time),
            end_time: timestamp_to_string(self.end_time),
            total_volume,
            buckets: self.buckets.clone(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct TimeVolumeData {
    pub num_buckets: usize,
    pub start_time: String,
    pub end_time: String,
    pub total_volume: BucketVolume,
    pub buckets: BTreeMap<String, BucketVolume>,
}
