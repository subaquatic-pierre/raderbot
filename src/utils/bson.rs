use crate::market::{kline::Kline, trade::Trade};

use super::{kline::build_kline_key, trade::build_market_trade_key};

pub fn build_bson_kline_meta(kline: &Kline) -> String {
    format!(
        "{}@{}",
        kline.open_time,
        build_kline_key(&kline.symbol, kline.interval)
    )
    .to_string()
}

pub fn build_bson_trade_meta(trade: &Trade) -> String {
    format!("{}@{}", trade.timestamp, trade.order_side).to_string()
}
