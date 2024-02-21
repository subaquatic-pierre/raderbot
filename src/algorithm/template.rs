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
        let custom_param = parse_usize_from_value("custom_param", algorithm_params.clone())
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

    fn strategy_name(&self) -> String {
        format!("CustomAlgorithm({})", self.custom_param)
    }
}
