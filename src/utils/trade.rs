use chrono::Datelike;
use log::info;
use serde::Deserialize;

use std::{collections::HashMap, fs::File, time::SystemTime};

use crate::{
    account::trade::OrderSide,
    market::{
        kline::{BinanceKline, Kline},
        trade::MarketTrade,
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

pub fn load_binance_agg_trades(file_path: std::path::PathBuf, symbol: &str) -> Vec<MarketTrade> {
    let mut trades = vec![];
    let filepath_str = file_path.as_os_str().to_str().unwrap();
    info!("Loading Aggregate Trade Data from file: {filepath_str}");
    let mut data: HashMap<String, Vec<MarketTrade>> = HashMap::new();

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

            let market_trade = MarketTrade {
                id: row.agg_trade_id,
                timestamp: row.transact_time,
                price: row.price,
                symbol: symbol.to_string(),
                qty: row.quantity,
                order_side,
            };

            trades.push(market_trade);
        }
    }

    trades
}

pub fn save_trades(filename: std::path::PathBuf, trades: &[MarketTrade], _append: bool) {
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
