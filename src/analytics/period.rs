use log::info;
use serde_json::Value;
use ta::indicators::SimpleMovingAverage;
use ta::Next;
use uuid::serde;

use crate::account::trade::OrderSide;
use crate::market::interval::Interval;
use crate::market::kline::Kline;

use crate::analytics::volume::{
    BucketVolume, PriceVolume, PriceVolumeData, TimeVolume, TimeVolumeData, TradeVolume,
};
use crate::market::trade::Trade;
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;
use crate::utils::time::{
    floor_mili_ts, generate_ts, string_to_timestamp, HOUR_AS_MILI, MIN_AS_MILI,
};

#[derive(Debug)]
pub struct LastPeriodData {
    pub high: f64,
    pub low: f64,
    pub period: AuctionPeriod,
    pub start_time: String,
    pub end_time: String,
    pub kline_avg_vol: BucketVolume,
    pub open_price: f64,
    pub close_price: f64,
    pub first_15_vol: BucketVolume,
    pub last_15_vol: BucketVolume,
    pub poc: Option<f64>,
}

pub fn is_within_n_min_of_next_period(min: u64, kline: &Kline) -> bool {
    let mut updated_kline = kline.clone();
    updated_kline.close_time = kline.close_time + (min * MIN_AS_MILI);

    // Calculate the start of the next auction period based on the current kline time
    let next_period_start = determine_auction_period(&updated_kline);

    next_period_start != AuctionPeriod::Unknown
}

#[derive(Debug, PartialEq)]
pub enum AuctionScenario {
    BearAuctionContinuation,
    BearAuctionReversal,
    BullAuctionContinuation,
    BullAuctionReversal,
    Undefined, // Used when none of the scenarios match
}

pub fn determine_auction_scenario(last_period_data: &LastPeriodData) -> AuctionScenario {
    let is_bearish = last_period_data.close_price < last_period_data.open_price;
    let is_bullish = last_period_data.close_price > last_period_data.open_price;
    let last_15_vol_higher = last_period_data.last_15_vol.buy_volume
        + last_period_data.last_15_vol.sell_volume
        > last_period_data.first_15_vol.buy_volume + last_period_data.first_15_vol.sell_volume;

    match (is_bullish, is_bearish, last_15_vol_higher) {
        (false, true, true) => AuctionScenario::BearAuctionContinuation,
        (false, true, false) => AuctionScenario::BearAuctionReversal,
        (true, false, true) => AuctionScenario::BullAuctionContinuation,
        (true, false, false) => AuctionScenario::BullAuctionReversal,
        _ => AuctionScenario::Undefined,
    }
}

pub const ASIA_START: u64 = HOUR_AS_MILI * 1;
pub const ASIA_END: u64 = HOUR_AS_MILI * 5;

pub const EURO_START: u64 = HOUR_AS_MILI * 7;
pub const EURO_END: u64 = HOUR_AS_MILI * 10;

pub const NYAM_START: u64 = HOUR_AS_MILI * 13;
pub const NYAM_END: u64 = HOUR_AS_MILI * 15;

pub const NYMD_START: u64 = HOUR_AS_MILI * 17;
pub const NYMD_END: u64 = HOUR_AS_MILI * 18;

pub const NYPM_START: u64 = (HOUR_AS_MILI * 18) + MIN_AS_MILI * 30;
pub const NYPM_END: u64 = HOUR_AS_MILI * 21;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AuctionPeriod {
    Asia,
    Euro,
    NYAM,
    NYMD,
    NYPM,
    Unknown,
}

// ASIA 01:00 - 05:00
// EURO 07:00 - 10:00
// NYAM 13:00 - 15:00
// NYMD 17:00 - 18:00
// NYPM 18:30 - 21:00

fn calc_window(start_of_day: u64, window_time: u64) -> u64 {
    start_of_day + window_time
}

// Method to determine the auction period for a given kline
pub fn determine_auction_period(kline: &Kline) -> AuctionPeriod {
    let kline_time = kline.close_time; // Assuming close_time represents the time of the kline

    // Calculate the current day's starting UTC time
    let start_of_day = floor_mili_ts(kline.open_time, HOUR_AS_MILI * 24); // Round down to the start of the day

    // Check ASIA window
    if kline_time >= calc_window(start_of_day, ASIA_START)
        && kline_time <= calc_window(start_of_day, ASIA_END)
    {
        AuctionPeriod::Asia
    } else if kline_time >= calc_window(start_of_day, EURO_START)
        && kline_time <= calc_window(start_of_day, EURO_END)
    {
        AuctionPeriod::Euro
    } else if kline_time >= calc_window(start_of_day, NYAM_START)
        && kline_time <= calc_window(start_of_day, NYAM_END)
    {
        AuctionPeriod::NYAM
    } else if kline_time >= calc_window(start_of_day, NYMD_START)
        && kline_time <= calc_window(start_of_day, NYMD_END)
    {
        AuctionPeriod::NYMD
    } else if kline_time >= calc_window(start_of_day, NYPM_START)
        && kline_time <= calc_window(start_of_day, NYPM_END)
    {
        AuctionPeriod::NYPM
    } else {
        AuctionPeriod::Unknown
    }
}
