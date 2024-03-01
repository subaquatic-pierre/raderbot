use chrono::Datelike;
use log::info;
use serde::Deserialize;

use std::{collections::HashMap, fs::File, time::SystemTime};

use crate::{
    account::trade::OrderSide,
    market::{
        kline::{BinanceKline, Kline},
        trade::AggTrade,
    },
    utils::{
        csv::has_header,
        time::{timestamp_to_datetime, HOUR_24_MILI_SEC},
    },
};
use csv::Reader;

use super::time::string_to_timestamp;

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

pub fn load_binance_agg_trades(
    file_path: std::path::PathBuf,
    symbol: &str,
) -> HashMap<String, Vec<AggTrade>> {
    let filepath_str = file_path.as_os_str().to_str().unwrap();
    info!("Loading Aggregate Trade Data from file: {filepath_str}");
    let mut data: HashMap<String, Vec<AggTrade>> = HashMap::new();

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

    // Create TS of beginning of time
    // let time = timestamp_to_datetime(SystemTime::UNIX_EPOCH);
    let mut last_day_ts: u64 = 0;

    for result in reader.deserialize::<BinanceAggTradeCsvRow>() {
        if let Err(e) = result {
            info!("{e}")
        } else {
            let row: BinanceAggTradeCsvRow =
                result.unwrap_or_else(|_| panic!("Unable to read Kline in file: {}", filepath_str));

            let date_str = if row.transact_time > last_day_ts {
                let time = timestamp_to_datetime(row.transact_time);
                let last_day_str = time.format("%Y-%m-%dT00:00:00Z").to_string();
                last_day_ts = string_to_timestamp(&last_day_str).unwrap();
                time.format("%Y-%m-%d").to_string()
            } else {
                let time = timestamp_to_datetime(last_day_ts);

                time.format("%Y-%m-%d").to_string()
            };

            let order_side = if row.is_buyer_maker {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            };

            let agg_trade = AggTrade {
                ts: row.transact_time,
                price: row.price,
                symbol: symbol.to_string(),
                qty: row.quantity,
                order_side,
            };

            let filename = format!("{symbol}@aggTrades-{date_str}.csv");

            if let Some(agg_trades) = data.get_mut(&filename) {
                agg_trades.push(agg_trade);
            } else {
                data.insert(filename.clone(), vec![agg_trade]);
            };
        }
    }

    data
}

pub fn save_agg_trades(filename: std::path::PathBuf, trades: &[AggTrade], _append: bool) {
    let str_filename = filename.as_os_str().to_string_lossy();
    info!("Saving AggTrades to new file: {str_filename}");

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
