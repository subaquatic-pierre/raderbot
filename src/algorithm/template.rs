use std::time::Duration;

use serde_json::Value;

use crate::market::kline::Kline;

use crate::strategy::types::AlgorithmError;
use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};
use crate::utils::number::parse_usize_from_value;

pub struct CustomAlgorithm {
    data_points: Vec<Kline>,
    interval: Duration,
    custom_param: usize,
}

impl CustomAlgorithm {
    pub fn new(interval: Duration, algorithm_params: Value) -> Result<Self, AlgorithmError> {
        let custom_param = parse_usize_from_value("custom_param", &algorithm_params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        Ok(Self {
            data_points: vec![],
            interval,
            custom_param,
        })
    }

    // Add any custom methods specific to this algorithm here

    // Example method:
    // fn calculate_custom_value(&self) -> f64 {
    //     // Custom logic using self.custom_param
    //     // ...
    // }
}

impl Algorithm for CustomAlgorithm {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        // Example logic using self.custom_param
        // ...

        AlgorithmEvalResult::Ignore
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    fn get_params(&self) -> &Value {
        unimplemented!()
    }

    fn set_params(&mut self, params: Value) -> Result<(), AlgorithmError> {
        unimplemented!()
    }
}

// pub enum AlgorithmEvalResult {
//     Long,
//     Short,
//     Ignore,
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct Kline {
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
