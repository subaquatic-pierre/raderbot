use log::info;
use serde_json::Value;

use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::market::volume::{PriceVolume, TradeVolume};
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;
use crate::utils::time::{floor_mili_ts, generate_ts, HOUR_AS_MILI, MIN_AS_MILI};

pub struct VolumeProfile {
    data_points: Vec<Kline>,
    market_volume: PriceVolume,
    last_auction_period: AuctionPeriod,
    params: Value,
}

impl VolumeProfile {
    pub fn new(params: Value) -> Result<Self, AlgoError> {
        Ok(Self {
            data_points: vec![],
            market_volume: PriceVolume::new(10.0, true),
            last_auction_period: AuctionPeriod::Unknown,
            params,
        })
    }

    // Add any custom methods specific to this algorithm here

    // Example method:
    // fn calculate_custom_value(&self) -> f64 {
    //     // Custom logic using self.custom_param
    //     // ...
    // }
}

impl Algorithm for VolumeProfile {
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        let is_start = self.data_points.len() == 0;

        self.data_points.push(kline.clone());
        self.market_volume.add_trades(trades);

        let new_period = determine_auction_period(&kline);

        if new_period != self.last_auction_period {
            info!(
                "MOVING INTO NEW AUCTION PERIOD: FROM: {:?} TO: {:?}",
                self.last_auction_period, new_period
            );
        }

        // match determine_auction_period(&kline) {
        //     AuctionPeriod::Asia => {
        //         info!("INSIDE Asia TIME WINDOW");
        //     }
        //     AuctionPeriod::Euro => {
        //         info!("INSIDE Euro TIME WINDOW");
        //     }
        //     AuctionPeriod::NYAM => {
        //         info!("INSIDE NYAM TIME WINDOW");
        //     }
        //     AuctionPeriod::NYMD => {
        //         info!("INSIDE NYMD TIME WINDOW");
        //     }
        //     AuctionPeriod::NYPM => {
        //         info!("INSIDE NYPM TIME WINDOW");
        //     }
        //     AuctionPeriod::Unknown => {
        //         if !is_start {
        //             info!(
        //                 "INSIDE UNKNOWN TIME WINDOW, PREVIOUS WINDOW: {:?}",
        //                 self.last_auction_period
        //             );
        //         }
        //     }
        // }

        self.last_auction_period = determine_auction_period(&kline);

        // Example logic using self.custom_param
        // ...

        self.clean_data_points();

        AlgoEvalResult::Ignore
    }

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
    let now = generate_ts();
    let start_of_day = floor_mili_ts(now, HOUR_AS_MILI * 24); // Round down to the start of the day

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
