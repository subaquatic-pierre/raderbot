use crate::market::kline::Kline;
use crate::strategy::types::AlgorithmError;
use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};
use crate::utils::number::parse_usize_from_value;
use serde_json::Value;
use std::time::Duration;

pub struct Rsi {
    data_points: Vec<Kline>,
    interval: Duration,
    params: Value,
    rsi_period: usize,
    rsi: f64, // Optional: Store the last calculated RSI value
}

impl Rsi {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        let rsi_period = parse_usize_from_value("rsi_period", &params).unwrap_or(14); // Default to 14 if not specified
        Ok(Self {
            data_points: vec![],
            interval,
            rsi_period,
            rsi: 0.0,
            params,
        })
    }

    fn calculate_rsi(&mut self) -> f64 {
        if self.data_points.len() < self.rsi_period {
            return 0.0; // Not enough data to calculate RSI
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=self.rsi_period {
            let delta = self.data_points[self.data_points.len() - i].close
                - self.data_points[self.data_points.len() - i - 1].close;
            if delta > 0.0 {
                gains += delta;
            } else {
                losses -= delta; // Losses are positive numbers
            }
        }

        let avg_gain = gains / self.rsi_period as f64;
        let avg_loss = losses / self.rsi_period as f64;

        if avg_loss == 0.0 {
            return 100.0; // Prevent division by zero
        }

        let rs = avg_gain / avg_loss;
        let rsi = 100.0 - (100.0 / (1.0 + rs));

        self.rsi = rsi; // Store the calculated RSI value
        rsi
    }
}

impl Algorithm for Rsi {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline);

        let rsi = self.calculate_rsi();

        // Example RSI logic: Buy if RSI < 30 (oversold), Sell if RSI > 70 (overbought), else Ignore
        let result = if rsi < 30.0 {
            AlgorithmEvalResult::Buy
        } else if rsi > 70.0 {
            AlgorithmEvalResult::Sell
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
        let rsi_period = parse_usize_from_value("rsi_period", &params).unwrap_or(self.rsi_period);

        self.rsi_period = rsi_period;
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
