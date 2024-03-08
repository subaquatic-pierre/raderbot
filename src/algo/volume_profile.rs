use std::time::Duration;

use serde_json::Value;

use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;

pub struct VolumeProfile {
    data_points: Vec<Kline>,
    interval: Duration,
    custom_param: usize,
    params: Value,
}

impl VolumeProfile {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgoError> {
        let custom_param = parse_usize_from_value("custom_param", &params)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
        Ok(Self {
            data_points: vec![],
            interval,
            custom_param,
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
        self.data_points.push(kline.clone());

        // Example logic using self.custom_param
        // ...

        self.clean_data_points();

        AlgoEvalResult::Ignore
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    fn get_params(&self) -> &Value {
        &self.params
    }

    fn set_params(&mut self, _params: Value) -> Result<(), AlgoError> {
        unimplemented!()
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }
    fn clean_data_points(&mut self) {
        unimplemented!()
    }
}

// ---
// Data structures used in algorithm
// Examples below
// ---

// enum AlgoEvalResult {
//     Buy,
//     Sell,
//     Ignore,
// }

// struct Kline {
//     pub symbol: String,
//     pub interval: String,
//     pub open: f64,
//     pub high: f64,
//     pub low: f64,
//     pub close: f64,
//     pub volume: f64,
//     pub open_time: u64,
//     pub close_time: u64,
// }
