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

fn aggregate_all_trades(trades: &mut Vec<Trade>) {
    let mut aggregated_trades_map: BTreeMap<(u64, OrderSide), Trade> = BTreeMap::new();

    for mut trade in trades.drain(..) {
        let floored_ts = floor_mili_ts(trade.timestamp, SEC_AS_MILI); // Floor to nearest second
        let key = (floored_ts, trade.order_side);

        if let Some(existing_trade) = aggregated_trades_map.get_mut(&key) {
            existing_trade.qty += trade.qty;
            existing_trade.price = (existing_trade.price + trade.price) / 2.0;
        } else {
            // update timestamp to be floored to second
            trade.timestamp = floored_ts;
            aggregated_trades_map.insert(key, trade);
        }
    }

    // Extract the aggregated trades and replace the original vector
    *trades = aggregated_trades_map.into_values().collect();
}

fn and_trade_and_aggregate(trades: &mut Vec<Trade>, new_trade: Trade) {
    let floored_ts = floor_mili_ts(new_trade.timestamp, SEC_AS_MILI);
    // Find the position where the new trade should be inserted or aggregated
    let position = trades.iter().position(|trade| {
        trade.timestamp == floored_ts && trade.order_side == new_trade.order_side
    });

    match position {
        Some(pos) => {
            // Aggregate the new trade with the existing one at the found position
            let existing_trade = &mut trades[pos];
            existing_trade.qty += new_trade.qty;
            // Assuming the price should be averaged
            existing_trade.price = (existing_trade.price + new_trade.price) / 2.0;
        }
        None => {
            // If no existing trade matches, add the new trade with the floored timestamp
            let mut trade_to_add = new_trade;
            trade_to_add.timestamp = floored_ts;
            trades.push(trade_to_add);
        }
    }
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
mod tests {
    use uuid::Uuid;

    use crate::utils::time::generate_ts;

    use super::*;

    fn generate_market_trades() -> Vec<Trade> {
        let ts = 1709464311000;
        let mut trades = Vec::new();
        for i in 0..20 {
            let order_side = if i % 2 == 0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            trades.push(Trade {
                symbol: "BTCUSD".to_string(),
                timestamp: ts + (i as u64) * 100, // Incrementing timestamp by 0.1 second for each trade
                qty: 1.0 + i as f64,              // Increasing quantity for variety
                price: 10000.0 + (i as f64) + 100.0, // Increasing price for variety
                order_side,
            });
        }
        trades
    }

    #[test]
    fn test_adding_new_trade() {
        let mut trades = Vec::new();
        let new_trade = Trade {
            symbol: "BTCUSD".to_string(),
            timestamp: 1609459201000, // Exact millisecond timestamp
            qty: 1.0,
            price: 10000.0,
            order_side: OrderSide::Buy,
        };

        and_trade_and_aggregate(&mut trades, new_trade.clone());

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0], new_trade);
    }

    #[test]
    fn test_aggregating_trade() {
        let ts = generate_ts();
        let mut trades = vec![];
        let trade = Trade {
            symbol: "BTCUSD".to_string(),
            timestamp: ts, // Floored to the nearest second
            qty: 1.0,
            price: 10000.0,
            order_side: OrderSide::Buy,
        };
        let new_trade = Trade {
            symbol: "BTCUSD".to_string(),
            timestamp: ts, // Within the same second, should be floored and aggregated
            qty: 2.0,
            price: 11000.0,
            order_side: OrderSide::Buy,
        };

        and_trade_and_aggregate(&mut trades, trade);
        and_trade_and_aggregate(&mut trades, new_trade);

        assert_eq!(trades.len(), 1); // Should still be one trade after aggregation
        assert_eq!(trades[0].qty, 3.0); // Quantities should be summed
        let avg_price = &format!("{:.2}", trades[0].price);
        assert!(avg_price == "10500.00"); // Price should be updated (averaged)
    }

    #[test]

    fn test_aggregation_with_multiple_trades() {
        let initial_trades = generate_market_trades(); // Initial set of trades

        let first_trade = &initial_trades[0].clone();

        let mut trades = vec![];

        for trade in initial_trades {
            and_trade_and_aggregate(&mut trades, trade);
        }

        // Verify the total number of trades
        // Since we're duplicating the trades for aggregation, we expect trades.len() to be equal to or less than the initial count if aggregation occurred
        assert!(
            trades.len() == 4,
            "Expected no more than 50 trades after aggregation"
        );

        // Verify that quantities are aggregated correctly for at least one pair of trades
        if let Some(aggregated_trade) = trades.iter().find(|&t| {
            t.timestamp == floor_mili_ts(first_trade.timestamp, SEC_AS_MILI)
                && t.order_side == first_trade.order_side
        }) {
            // qty: 25.0, price: 10106.125,
            assert_eq!(
                aggregated_trade.qty, 25.0,
                "Quantities not aggregated correctly."
            );

            // Verify the average price for the same trade
            assert_eq!(
                aggregated_trade.price, 10106.125,
                "Prices not averaged correctly."
            );
        } else {
            panic!("Expected aggregated trade not found.");
        }
    }
}
