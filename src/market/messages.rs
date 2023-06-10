use crate::market::{kline::Kline, ticker::Ticker};

#[derive(Debug)]
pub enum MarketMessage {
    UpdateTicker(Ticker),
    UpdateKline(Kline),
}
