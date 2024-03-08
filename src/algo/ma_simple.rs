use std::time::Duration;

use serde_json::Value;

use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;

pub struct SimpleMovingAverage {
    data_points: Vec<Kline>,
    interval: Duration,
    period: usize,
    params: Value,
}

impl SimpleMovingAverage {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgoError> {
        let period = parse_usize_from_value("sma_period", &params.clone())
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
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
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        self.data_points.push(kline.clone());

        let result = if self.data_points.len() >= self.period {
            let sma = self.calculate_sma(self.period);

            // Placeholder logic for buy/sell decision based on SMA
            if kline.close > sma {
                AlgoEvalResult::Buy
            } else {
                AlgoEvalResult::Sell
            }
        } else {
            AlgoEvalResult::Ignore
        };

        self.clean_data_points();

        result
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

    fn set_params(&mut self, params: Value) -> Result<(), AlgoError> {
        let period = parse_usize_from_value("sma_period", &params.clone())
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;

        self.period = period;
        self.params = params;
        Ok(())
    }

    fn clean_data_points(&mut self) {
        // TODO: Change length to be checked
        // based on individual algorithm
        let two_weeks_minutes = 10080 * 2;
        if self.data_points.len() > two_weeks_minutes {
            // reduce back to 1 week worth on data
            self.data_points.drain(0..10080);
        }
    }
}
