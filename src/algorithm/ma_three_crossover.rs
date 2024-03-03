use std::time::Duration;

use serde_json::Value;

use crate::market::kline::Kline;

use crate::strategy::types::AlgorithmError;
use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};
use crate::utils::number::parse_usize_from_value;

pub struct ThreeMaCrossover {
    data_points: Vec<Kline>,
    interval: Duration,
    short_period: usize,
    medium_period: usize,
    long_period: usize,
    params: Value,
}

impl ThreeMaCrossover {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        let short_period = parse_usize_from_value("short_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let medium_period = parse_usize_from_value("medium_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let long_period = parse_usize_from_value("long_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        Ok(Self {
            data_points: vec![],
            interval,
            short_period,
            medium_period,
            long_period,
            params,
        })
    }

    fn calculate_ma(&self, period: usize) -> f64 {
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

    fn calculate_short_ma(&self) -> f64 {
        self.calculate_ma(self.short_period)
    }

    fn calculate_medium_ma(&self) -> f64 {
        self.calculate_ma(self.medium_period)
    }

    fn calculate_long_ma(&self) -> f64 {
        self.calculate_ma(self.long_period)
    }
}

impl Algorithm for ThreeMaCrossover {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        let result = if self.data_points.len() >= self.long_period {
            let short_ma = self.calculate_short_ma();
            let medium_ma = self.calculate_medium_ma();
            let long_ma = self.calculate_long_ma();

            // Placeholder logic for buy/sell decision based on MA crossovers
            if short_ma > medium_ma && medium_ma > long_ma {
                AlgorithmEvalResult::Buy
            } else if short_ma < medium_ma && medium_ma < long_ma {
                AlgorithmEvalResult::Sell
            } else {
                AlgorithmEvalResult::Ignore
            }
        } else {
            AlgorithmEvalResult::Ignore
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

    fn set_params(&mut self, params: Value) -> Result<(), AlgorithmError> {
        let short_period = parse_usize_from_value("short_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let medium_period = parse_usize_from_value("medium_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let long_period = parse_usize_from_value("long_period", &params)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        self.params = params;
        self.long_period = long_period;
        self.short_period = short_period;
        self.medium_period = medium_period;

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
