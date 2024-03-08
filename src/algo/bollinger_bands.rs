use crate::account::trade;
use crate::market::kline::Kline;
use crate::market::trade::Trade;
use crate::strategy::{
    algorithm::Algorithm,
    types::{AlgoError, AlgoEvalResult},
};
use crate::utils::number::parse_usize_from_value;
use serde_json::Value;
use std::time::Duration;

pub struct BollingerBands {
    data_points: Vec<Kline>,
    interval: Duration,
    params: Value,
    period: usize,
    multiplier: f64, // Typically, the multiplier is set to 2 for the standard deviation calculation.
}

impl BollingerBands {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgoError> {
        let period = parse_usize_from_value("period", &params).unwrap_or(20); // Default period is 20
        let multiplier = parse_usize_from_value("multiplier", &params).unwrap_or(2) as f64; // Default multiplier is 2

        Ok(Self {
            data_points: Vec::new(),
            interval,
            params,
            period,
            multiplier,
        })
    }

    fn calculate_sma(&self) -> f64 {
        if self.data_points.len() < self.period {
            return 0.0; // Not enough data
        }
        self.data_points
            .iter()
            .rev()
            .take(self.period)
            .map(|k| k.close)
            .sum::<f64>()
            / self.period as f64
    }

    fn calculate_std_dev(&self, sma: f64) -> f64 {
        if self.data_points.len() < self.period {
            return 0.0;
        }
        let variance: f64 = self
            .data_points
            .iter()
            .rev()
            .take(self.period)
            .map(|k| {
                let diff = k.close - sma;
                diff * diff
            })
            .sum::<f64>()
            / self.period as f64;

        variance.sqrt()
    }

    fn calculate_bollinger_bands(&self) -> (f64, f64, f64) {
        let sma = self.calculate_sma();
        let std_dev = self.calculate_std_dev(sma);
        let upper_band = sma + std_dev * self.multiplier;
        let lower_band = sma - std_dev * self.multiplier;

        (upper_band, sma, lower_band)
    }
}

impl Algorithm for BollingerBands {
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        self.data_points.push(kline.clone());

        let (upper_band, _middle_band, lower_band) = self.calculate_bollinger_bands();

        // Example trading logic based on Bollinger Bands
        let result = if kline.close > upper_band {
            // Price is above the upper band - potential sell signal (overbought condition)
            AlgoEvalResult::Sell
        } else if kline.close < lower_band {
            // Price is below the lower band - potential buy signal (oversold condition)
            AlgoEvalResult::Buy
        } else {
            // Price is within the bands - no clear signal
            AlgoEvalResult::Ignore
        };

        self.clean_data_points();

        result
    }

    // Implement the rest of the required methods from the Algo trait...
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
        if let Ok(period) = parse_usize_from_value("period", &params) {
            self.period = period
        }
        if let Ok(multiplier) = parse_usize_from_value("multiplier", &params) {
            self.multiplier = multiplier as f64;
        }

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
