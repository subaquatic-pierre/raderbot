use crate::market::{kline::Kline, ticker::Ticker};

use super::trade::Trade;

/// Represents a message for market data updates within the trading system. This enum encapsulates the different types of market data updates that can occur, specifically updates to tickers and klines. It is used as a communication medium between different components of the system to synchronize market data changes.
///
/// # Variants
///
/// - UpdateTicker(Ticker): Carries a Ticker instance representing the latest ticker information to be updated in the market data.
///
/// - UpdateKline(Kline): Contains a Kline instance representing a new or updated kline data point to be incorporated into the market data.

#[derive(Debug)]
pub enum MarketMessage {
    UpdateTicker(Ticker),
    UpdateKline(Kline),
    UpdateMarketTrade(Trade),
}
