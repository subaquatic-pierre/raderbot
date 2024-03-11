use log::info;
use serde_json::Value;
use ta::indicators::SimpleMovingAverage;
use ta::Next;
use uuid::serde;

use crate::account::trade::OrderSide;
use crate::market::interval::Interval;
use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::market::volume::{
    BucketVolume, PriceVolume, PriceVolumeData, TimeVolume, TimeVolumeData, TradeVolume,
};
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;
use crate::utils::time::{
    floor_mili_ts, generate_ts, string_to_timestamp, HOUR_AS_MILI, MIN_AS_MILI,
};

#[derive(Debug)]
pub struct LastPeriodData {
    high: f64,
    low: f64,
    period: AuctionPeriod,
    start_time: String,
    end_time: String,
    kline_avg_vol: BucketVolume,
    open_price: f64,
    close_price: f64,
    first_15_vol: BucketVolume,
    last_15_vol: BucketVolume,
    poc: Option<f64>,
}

pub struct VolumeContinuationReversal {
    klines: Vec<Kline>,
    // period_price_vol: PriceVolume,
    time_vol: TimeVolume,
    cur_period: AuctionPeriod,
    params: Value,
    sma: SimpleMovingAverage,
    last_period_data: Option<LastPeriodData>,
    last_result: Option<AlgoEvalResult>,
}

impl VolumeContinuationReversal {
    pub fn new(params: Value) -> Result<Self, AlgoError> {
        let sma = SimpleMovingAverage::new(10).unwrap();
        Ok(Self {
            klines: vec![],
            time_vol: TimeVolume::new(Interval::Min1),
            cur_period: AuctionPeriod::Unknown,
            params,
            last_result: None,
            sma,
            last_period_data: None,
        })
    }

    fn reset_data(&mut self) {
        self.klines = vec![];
        // self.period_price_vol.reset_volumes();
        self.time_vol.reset_volumes();
    }

    fn add_data_points(&mut self, kline: &Kline, _trades: &[Trade]) {
        self.klines.push(kline.clone());
        // self.period_price_vol.add_trades(trades);

        self.time_vol.add_trades(&kline.make_trades());
    }

    fn update_last_period_data(&mut self) {
        let last_time_vol = self.time_vol.result();

        let open_price = match last_time_vol.buckets.first_key_value() {
            Some((ts_key, _)) => {
                match self
                    .klines
                    .iter()
                    .find(|&k| k.open_time == string_to_timestamp(ts_key).unwrap())
                {
                    Some(k) => k.close,
                    None => 0.0,
                }
            }
            None => 0.0,
        };

        let close_price = match last_time_vol.buckets.last_key_value() {
            Some((ts_key, _)) => {
                match self
                    .klines
                    .iter()
                    .find(|&k| k.open_time == string_to_timestamp(ts_key).unwrap())
                {
                    Some(k) => k.close,
                    None => 0.0,
                }
            }
            None => 0.0,
        };

        let first_15_vol = self.time_vol.n_vol(false, 15);
        let last_15_vol = self.time_vol.n_vol(true, 15);

        let last_period_data = LastPeriodData {
            high: last_time_vol.max_price,
            low: last_time_vol.min_price,
            period: self.cur_period,
            start_time: last_time_vol.start_time.clone(),
            end_time: last_time_vol.end_time.clone(),
            kline_avg_vol: last_time_vol.average_volume,
            poc: None,
            first_15_vol,
            last_15_vol,
            open_price,
            close_price,
        };

        self.last_period_data = Some(last_period_data);
    }
}

impl Algorithm for VolumeContinuationReversal {
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        // get current period
        let new_period = determine_auction_period(&kline);
        let is_within_15 = is_within_15_min_of_next_period(&kline);

        // send closing signal if within 15min of next auction
        // currently in unknown period
        // and last result is some
        // update last result to None
        if self.cur_period == AuctionPeriod::Unknown && is_within_15 && self.last_result.is_some() {
            if let Some(last_result) = &self.last_result {
                match last_result {
                    AlgoEvalResult::Buy => {
                        self.last_result = None;
                        return AlgoEvalResult::Sell;
                    }
                    AlgoEvalResult::Sell => {
                        self.last_result = None;
                        return AlgoEvalResult::Buy;
                    }
                    _ => return AlgoEvalResult::Ignore,
                }
            }
        }

        // only add data if in known period
        if self.cur_period != AuctionPeriod::Unknown {
            self.add_data_points(&kline, trades)
        }

        // add data for the last auction period
        // first move into Unknown period
        if self.cur_period != AuctionPeriod::Unknown && new_period == AuctionPeriod::Unknown {
            self.update_last_period_data();
        }

        // main evaluation done in AuctionPeriod::Unknown
        if self.cur_period == AuctionPeriod::Unknown && !is_within_15 {
            let mut cur_vol = TimeVolume::new(kline.interval);
            cur_vol.add_trades(&kline.make_trades());

            if let Some(last_data) = &self.last_period_data {
                let LastPeriodData {
                    first_15_vol,
                    last_15_vol,
                    close_price,
                    open_price,
                    ..
                } = last_data;

                if self.last_result.is_none() {
                    // info!(
                    //     "NO LAST TRADE, EVALUATING IN UNKNOWN PERIOD: {kline:?}, VOL: {:?}",
                    //     cur_vol.result()
                    // );
                    if first_15_vol.total() > last_15_vol.total() && close_price < open_price {
                        self.last_result = Some(AlgoEvalResult::Sell);
                        return AlgoEvalResult::Sell;
                    } else if first_15_vol.total() < last_15_vol.total() && close_price > open_price
                    {
                        self.last_result = Some(AlgoEvalResult::Buy);
                        return AlgoEvalResult::Buy;
                    }
                }
            }
        }

        // reset data below if moving into new auction period
        // that is moving from Unknown to Known
        // new_period != self.cur_period means moving into new period
        if self.cur_period == AuctionPeriod::Unknown && new_period != AuctionPeriod::Unknown {
            // reset data
            self.reset_data();
        }

        // change into new period at end
        self.cur_period = new_period;

        // default Ignore
        AlgoEvalResult::Ignore
    }

    // ---
    // Trait helper methods
    // ---

    fn get_params(&self) -> &Value {
        &self.params
    }

    fn needs_trades(&self) -> bool {
        // true
        false
    }

    fn set_params(&mut self, _params: Value) -> Result<(), AlgoError> {
        Ok(())
    }

    fn data_points(&self) -> Vec<Kline> {
        self.klines.clone()
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

#[derive(Debug, PartialEq, Clone, Copy)]
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

fn is_within_15_min_of_next_period(kline: &Kline) -> bool {
    let mut updated_kline = kline.clone();
    updated_kline.close_time = kline.close_time + (5 * MIN_AS_MILI);

    // Calculate the start of the next auction period based on the current kline time
    let next_period_start = determine_auction_period(&updated_kline);

    next_period_start != AuctionPeriod::Unknown
}

#[derive(Debug, PartialEq)]
enum AuctionScenario {
    BearAuctionContinuation,
    BearAuctionReversal,
    BullAuctionContinuation,
    BullAuctionReversal,
    Undefined, // Used when none of the scenarios match
}

fn determine_auction_scenario(last_period_data: &LastPeriodData) -> AuctionScenario {
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
