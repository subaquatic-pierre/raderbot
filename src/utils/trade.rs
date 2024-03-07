use chrono::Datelike;
use log::info;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::{collections::HashMap, fs::File, time::SystemTime};
use uuid::Uuid;

use crate::market::trade::{TradeData, TradeDataMeta};
use crate::utils::time::generate_ts;
use crate::{
    account::trade::OrderSide,
    market::{
        kline::{BinanceKline, Kline},
        trade::Trade,
    },
    utils::{
        csv::has_header,
        time::{timestamp_to_datetime, DAY_AS_MILI},
    },
};
use csv::Reader;

use super::time::{floor_mili_ts, string_to_timestamp, SEC_AS_MILI};

#[derive(Deserialize)]
struct BinanceAggTradeCsvRow {
    // row: u64,
    agg_trade_id: u64,
    price: f64,
    quantity: f64,
    first_trade_id: u64,
    last_trade_id: u64,
    transact_time: u64,
    is_buyer_maker: bool,
}

pub fn load_binance_agg_trades(file_path: std::path::PathBuf, symbol: &str) -> Vec<Trade> {
    let mut aggregated_trades_map: BTreeMap<(u64, OrderSide), Trade> = BTreeMap::new();
    let filepath_str = file_path.as_os_str().to_str().unwrap();
    info!("Loading Aggregate Trade Data from file: {filepath_str}");

    if let Err(e) = File::open(file_path.clone()) {
        info!("{e}")
    }

    let file = File::open(file_path.clone())
        .unwrap_or_else(|_| panic!("Unable to open file {}", filepath_str));

    let headers = [
        "agg_trade_id",
        "price",
        "quantity",
        "first_trade_id",
        "last_trade_id",
        "transact_time",
        "is_buyer_maker",
    ];

    let has_header = has_header(filepath_str, &headers).unwrap();

    let mut reader = if has_header {
        Reader::from_reader(file)
    } else {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(file)
    };

    let mut market_data = TradeData::new(symbol);

    for result in reader.deserialize::<BinanceAggTradeCsvRow>() {
        if let Err(e) = result {
            info!("{e}")
        } else {
            let row: BinanceAggTradeCsvRow =
                result.unwrap_or_else(|_| panic!("Unable to read Kline in file: {}", filepath_str));

            let order_side = if row.is_buyer_maker {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            };

            let mut market_trade = Trade {
                timestamp: floor_mili_ts(row.transact_time, SEC_AS_MILI),
                price: row.price,
                symbol: symbol.to_string(),
                qty: row.quantity,
                order_side,
            };

            market_data.add_trade(&mut market_trade);

            // let key = (market_trade.timestamp, market_trade.order_side);

            // if let Some(existing_trade) = aggregated_trades_map.get_mut(&key) {
            //     existing_trade.qty += market_trade.qty;
            //     existing_trade.price = (existing_trade.price + market_trade.price) / 2.0;
            // } else {
            //     aggregated_trades_map.insert(key, market_trade);
            // }
        }
    }

    market_data.trades()
}

pub fn is_same_ts_and_order_side(left: &Trade, right: &Trade) -> bool {
    left.order_side == right.order_side && left.timestamp == right.timestamp
}

pub fn save_trades(filename: std::path::PathBuf, trades: &[Trade], _append: bool) {
    let str_filename = filename.as_os_str().to_string_lossy();
    info!("Saving Trades to new file: {str_filename}");

    let file = File::create(filename.clone()).expect("Unable to create file");

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(&file);

    for trade in trades {
        // Serialize and write all trades to the file
        writer.serialize(trade).unwrap_or_else(|_| {
            panic!("Unable to save kline: {:?} to file:{}", trade, str_filename)
        });
    }
}

pub fn build_market_trade_key(symbol: &str) -> String {
    format!("{}@trade", symbol)
}

pub fn build_market_trade_filename(trade_key: &str, timestamp: u64) -> String {
    let time = timestamp_to_datetime(timestamp);
    let date_str = time.format("%Y-%m-%d").to_string();
    format!("{trade_key}-{date_str}.csv")
}

pub fn generate_trade_filenames_in_range(trade_key: &str, from_ts: u64, to_ts: u64) -> Vec<String> {
    let start_day = floor_mili_ts(from_ts, DAY_AS_MILI);
    let end_day = floor_mili_ts(to_ts, DAY_AS_MILI);
    let mut filenames = Vec::new();

    let mut current_ts = start_day;
    while current_ts <= end_day {
        let filename = build_market_trade_filename(trade_key, current_ts);
        filenames.push(filename);

        current_ts += DAY_AS_MILI;
    }

    filenames
}

#[cfg(test)]
mod tests {}
