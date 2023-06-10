use chrono::Datelike;

use serde::{Deserialize, Serialize};

use std::fs::File;

use std::io::BufRead;

use crate::{
    market::{
        kline::{BinanceKline, Kline},
        market::MarketData,
    },
    utils::{csv::has_header, time::timestamp_to_datetime},
};
use csv::Reader;

pub fn load_binance_klines(
    file_path: std::path::PathBuf,
    symbol: &str,
    interval: &str,
) -> Vec<Kline> {
    let filepath_str = file_path.as_os_str().to_str().unwrap();
    println!("filepath: {filepath_str}");
    let file = File::open(file_path.clone())
        .unwrap_or_else(|_| panic!("Unable to open file {}", filepath_str));

    let headers = [
        "open_time",
        "open",
        "high",
        "low",
        "close",
        "volume",
        "close_time",
        "quote_volume",
        "count",
        "taker_buy_volume",
        "taker_buy_quote_volume",
        "ignore",
    ];

    let has_header = has_header(filepath_str, &headers).unwrap();

    let mut reader = if has_header {
        Reader::from_reader(file)
    } else {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(file)
    };

    let mut klines = vec![];

    for result in reader.deserialize::<BinanceKline>() {
        let binance_kline: BinanceKline =
            result.unwrap_or_else(|_| panic!("Unable to read Kline in file: {}", filepath_str));

        let symbol = symbol.replace("USDT", "-USDT");

        let kline = Kline {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            open_time: binance_kline.open_time,
            open: binance_kline.open,
            high: binance_kline.high,
            low: binance_kline.low,
            close: binance_kline.close,
            volume: binance_kline.volume,
            close_time: binance_kline.close_time,
        };
        klines.push(kline);
    }

    klines
}

pub fn interval_symbol_from_binance_filename(filename: &str) -> (String, String) {
    let parts = filename.split('-');
    let collection: Vec<&str> = parts.collect();
    (collection[0].to_string(), collection[1].to_string())
}

pub fn save_klines(filename: std::path::PathBuf, klines: &[Kline], _append: bool) {
    let str_filename = filename.as_os_str().to_string_lossy();

    let file = File::create(filename.clone()).expect("Unable to create file");

    // let file = OpenOptions::new()
    //     .append(append)
    //     .create(true)
    //     .open(&filename)
    //     .expect(&format!("unable to open file: {}", str_filename));

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(&file);

    for kline in klines {
        // Serialize and write all klines to the file
        writer.serialize(kline.clone()).unwrap_or_else(|_| {
            panic!("Unable to save kline: {:?} to file:{}", kline, str_filename)
        });
    }
}

pub fn generate_kline_filenames_in_range(kline_key: &str, from_ts: u64, to_ts: u64) -> Vec<String> {
    let from_date = timestamp_to_datetime(from_ts);
    let to_date = timestamp_to_datetime(to_ts);

    let from_year = from_date.year() as u32;
    let from_month = from_date.month();
    let to_year = to_date.year() as u32;
    let to_month = to_date.month();

    let mut filenames = Vec::new();

    let mut current_year = from_year;
    let mut current_month = from_month;
    while current_year < to_year || (current_year == to_year && current_month <= to_month) {
        let filename = MarketData::build_kline_filename_from_year_month(
            kline_key,
            current_year,
            current_month,
        );
        filenames.push(filename);

        current_month += 1;
        if current_month > 12 {
            current_month = 1;
            current_year += 1;
        }
    }

    filenames
}
