use std::time::Duration;

use serde_json::Value;

use crate::market::kline::Kline;

use crate::strategy::types::AlgorithmError;
use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};
use crate::utils::number::parse_usize_from_value;

pub struct SimpleMovingAverage {
    data_points: Vec<Kline>,
    interval: Duration,
    period: usize,
}

impl SimpleMovingAverage {
    pub fn new(interval: Duration, algorithm_params: Value) -> Result<Self, AlgorithmError> {
        let period = parse_usize_from_value("sma_period", algorithm_params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        Ok(Self {
            data_points: vec![],
            interval,
            period,
        })
    }

    fn calculate_sma(&self) -> f64 {
        let start_index = self.data_points.len().saturating_sub(self.period); // Avoid index underflow

        let sum: f64 = self
            .data_points
            .iter()
            .rev()
            .skip(start_index)
            .map(|k| k.close)
            .sum();

        let divisor = usize::min(self.period, self.data_points.len()); // Ensure divisor is not zero

        sum / divisor as f64
    }
}

impl Algorithm for SimpleMovingAverage {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        if self.data_points.len() >= self.period {
            let sma = self.calculate_sma();

            // Placeholder logic for buy/sell decision based on SMA
            if kline.close > sma {
                AlgorithmEvalResult::Long
            } else {
                AlgorithmEvalResult::Short
            }
        } else {
            AlgorithmEvalResult::Ignore
        }
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    fn strategy_name(&self) -> String {
        format!("SimpleMovingAverage({})", self.period)
    }
}
