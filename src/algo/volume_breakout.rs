use log::info;
use serde_json::Value;
use uuid::serde;

use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::market::volume::{PriceVolume, PriceVolumeData, TradeVolume};
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;
use crate::utils::time::{floor_mili_ts, generate_ts, HOUR_AS_MILI, MIN_AS_MILI};

pub struct VolumeBreakout {
    data_points: Vec<Kline>,
    market_volume: PriceVolume,
    last_auction_period: AuctionPeriod,
    params: Value,
    // recent_high: f64,
    // recent_low: f64,
}

impl VolumeBreakout {
    pub fn new(params: Value) -> Result<Self, AlgoError> {
        Ok(Self {
            data_points: vec![],
            market_volume: PriceVolume::new(10.0, true),
            last_auction_period: AuctionPeriod::Unknown,
            params,
            // recent_high: 0.0,
            // recent_low: std::f64::MAX,
        })
    }

    fn reset_data(&mut self) {
        self.data_points = vec![];
        self.market_volume.reset_volumes();
    }
}

impl Algorithm for VolumeBreakout {
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        // add data to strategy
        // self.data_points.push(kline.clone());
        self.market_volume.add_trades(trades);

        // get volume within this kline

        // Update the recent high/low for the last auction period
        let new_period = determine_auction_period(&kline);

        // moving out of known period
        if self.last_auction_period != AuctionPeriod::Unknown
            && new_period == AuctionPeriod::Unknown
        {
            let vol_res: PriceVolumeData = self.market_volume.result().into();
            info!(
                "LAST PERIOD: {:?}, LAST_HIGH: {}, LAST_LOW: {}, LAST_POC: {}",
                self.last_auction_period, vol_res.max_price, vol_res.min_price, vol_res.poc
            );
        }

        // moving into new period
        if self.last_auction_period == AuctionPeriod::Unknown
            && new_period != self.last_auction_period
        {
            self.reset_data();
            // info!(
            //     "MOVING INTO NEW AUCTION PERIOD: FROM: {:?} TO: {:?}",
            //     self.last_auction_period, new_period
            // );
        }
        self.last_auction_period = new_period;

        // Example: Just return Ignore for now
        AlgoEvalResult::Ignore
    }

    // ---
    // Trait helper methods
    // ---

    fn get_params(&self) -> &Value {
        &self.params
    }

    fn needs_trades(&self) -> bool {
        true
    }

    fn set_params(&mut self, _params: Value) -> Result<(), AlgoError> {
        Ok(())
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }

    fn clean_data_points(&mut self) {
        // unimplemented!()
    }
}

// ASIA 01:00 - 05:00
// EURO 07:00 - 10:00
// NYAM 13:00 - 15:00
// NYMD 17:00 - 18:00
// NYPM 18:30 - 21:00

#[derive(Debug, PartialEq)]
enum AuctionPeriod {
    Asia,
    Euro,
    NYAM,
    NYMD,
    NYPM,
    Unknown,
}

const ASIA_START: u64 = HOUR_AS_MILI * 1;
const ASIA_END: u64 = HOUR_AS_MILI * 5;

const EURO_START: u64 = HOUR_AS_MILI * 7;
const EURO_END: u64 = HOUR_AS_MILI * 10;

const NYAM_START: u64 = HOUR_AS_MILI * 13;
const NYAM_END: u64 = HOUR_AS_MILI * 15;

const NYMD_START: u64 = HOUR_AS_MILI * 17;
const NYMD_END: u64 = HOUR_AS_MILI * 18;

const NYPM_START: u64 = (HOUR_AS_MILI * 18) + MIN_AS_MILI * 30;
const NYPM_END: u64 = HOUR_AS_MILI * 21;

fn calc_window(start_of_day: u64, window_time: u64) -> u64 {
    start_of_day + window_time
}

// Method to determine the auction period for a given kline
fn determine_auction_period(kline: &Kline) -> AuctionPeriod {
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
