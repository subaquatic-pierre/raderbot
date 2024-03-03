use chrono::Datelike;
use log::info;

use std::fs::File;

use crate::{
    market::kline::{BinanceKline, Kline},
    utils::{csv::has_header, time::timestamp_to_datetime},
};
use csv::Reader;

/// Loads k-line (candlestick) data for Binance trading pairs from a CSV file.
///
/// # Arguments
///
/// * `file_path` - The file path to the CSV file containing k-line data.
/// * `symbol` - The symbol for which k-line data is being loaded, e.g., "BTCUSDT".
/// * `interval` - The interval for k-line data, e.g., "1m" for one minute.
///
/// # Returns
///
/// A vector of `Kline` structs representing the k-line data from the file.
///
/// # Panics
///
/// Panics if the file cannot be opened or if there is an error reading from the file.

pub fn load_binance_klines(
    file_path: std::path::PathBuf,
    symbol: &str,
    interval: &str,
) -> Vec<Kline> {
    let filepath_str = file_path.as_os_str().to_str().unwrap();
    info!("Loading klines from file: {filepath_str}");
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

/// Parses the filename to extract the symbol and interval for k-line data.
///
/// # Arguments
///
/// * `filename` - The name of the file containing k-line data.
///
/// # Returns
///
/// A tuple containing the symbol and interval as `String`.
pub fn interval_symbol_from_binance_filename(filename: &str) -> (String, String) {
    let parts = filename.split('-');
    let collection: Vec<&str> = parts.collect();
    (collection[0].to_string(), collection[1].to_string())
}

/// Saves k-line data to a specified CSV file.
///
/// # Arguments
///
/// * `filename` - The file path where the k-line data should be saved.
/// * `klines` - A slice of `Kline` structs to be saved to the file.
/// * `_append` - Whether to append the k-lines to the file if it already exists.
///
/// # Panics
///
/// Panics if the file cannot be created or if there is an error writing to the file.
pub fn save_klines(filename: std::path::PathBuf, klines: &[Kline], _append: bool) {
    let str_filename = filename.as_os_str().to_string_lossy();

    info!("Saving klines to file: {str_filename}");

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

/// Generates filenames for saving k-line data based on a key, from and to timestamps.
///
/// # Arguments
///
/// * `kline_key` - A key representing the k-line data set.
/// * `from_ts` - The starting UNIX timestamp for the k-line data.
/// * `to_ts` - The ending UNIX timestamp for the k-line data.
///
/// # Returns
///
/// A vector of filenames as `String`.
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
        let filename = build_kline_filename_from_year_month(kline_key, current_year, current_month);
        filenames.push(filename);

        current_month += 1;
        if current_month > 12 {
            current_month = 1;
            current_year += 1;
        }
    }

    filenames
}

pub fn get_min_max_open_time(klines: &[Kline]) -> (u64, u64) {
    let min_time = klines
        .iter()
        .map(|t| t.open_time)
        .min_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap();
    let max_time = klines
        .iter()
        .map(|t| t.open_time)
        .max_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap();
    (min_time, max_time)
}

pub fn build_kline_key(symbol: &str, interval: &str) -> String {
    format!("{}@kline_{}", symbol, interval)
}

pub fn build_ticker_key(symbol: &str) -> String {
    format!("{}@ticker", symbol)
}

pub fn build_kline_filename(kline_key: &str, timestamp: u64) -> String {
    let month_str = build_kline_month_string(timestamp);
    format!("{kline_key}-{month_str}.csv")
}

pub fn build_kline_filename_from_year_month(kline_key: &str, year: u32, month: u32) -> String {
    format!("{kline_key}-{:04}-{:02}.csv", year, month)
}

pub fn build_kline_month_string(timestamp: u64) -> String {
    let timestamp = timestamp_to_datetime(timestamp);
    timestamp.format("%Y-%m").to_string()
}

#[cfg(test)]
mod tests {
    // TODO: Write tests
}
