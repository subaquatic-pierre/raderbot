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
    params: Value,
}

impl SimpleMovingAverage {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        let period = parse_usize_from_value("sma_period", &params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        Ok(Self {
            data_points: vec![],
            interval,
            period,
            params,
        })
    }

    fn calculate_sma(&self, period: usize) -> f64 {
        if self.data_points.len() < period {
            return 0.0;
        }

        let sum: f64 = self
            .data_points
            .iter()
            .rev()
            .take(period)
            .map(|k| k.close)
            .sum();

        sum / period as f64
    }
}

impl Algorithm for SimpleMovingAverage {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        if self.data_points.len() >= self.period {
            let sma = self.calculate_sma(self.period);

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

    fn get_params(&self) -> &Value {
        &self.params
    }

    fn set_params(&mut self, params: Value) -> Result<(), AlgorithmError> {
        let period = parse_usize_from_value("sma_period", &params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        self.period = period;
        self.params = params;
        Ok(())
    }
}
