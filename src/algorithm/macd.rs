use crate::market::kline::Kline;
use crate::strategy::{
    algorithm::Algorithm,
    types::{AlgorithmError, AlgorithmEvalResult},
};
use crate::utils::number::parse_usize_from_value;
use serde_json::Value;
use std::time::Duration;

pub struct Macd {
    data_points: Vec<Kline>,
    interval: Duration,
    short_ema_period: usize,
    long_ema_period: usize,
    signal_ema_period: usize,
    macd_line: Vec<f64>,   // MACD values for each data point
    signal_line: Vec<f64>, // Signal line values for each data point\\
    params: Value,
}

impl Macd {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        // Extract parameters or set to default values
        let short_ema_period = params
            .get("short_ema_period")
            .and_then(Value::as_u64)
            .unwrap_or(12) as usize;
        let long_ema_period = params
            .get("long_ema_period")
            .and_then(Value::as_u64)
            .unwrap_or(26) as usize;
        let signal_ema_period = params
            .get("signal_ema_period")
            .and_then(Value::as_u64)
            .unwrap_or(9) as usize;

        Ok(Self {
            data_points: Vec::new(),
            interval,
            short_ema_period,
            long_ema_period,
            signal_ema_period,
            macd_line: Vec::new(),
            signal_line: Vec::new(),
            params,
        })
    }

    fn calculate_ema(&self, prices: &[f64], period: usize) -> f64 {
        if prices.len() < period {
            return 0.0;
        }

        let k = 2.0 / (period as f64 + 1.0);
        prices.iter().rev().fold(0.0, |ema, &price| {
            if ema == 0.0 {
                price
            } else {
                price * k + ema * (1.0 - k)
            }
        })
    }

    fn update_macd_and_signal_lines(&mut self) {
        let prices: Vec<f64> = self.data_points.iter().map(|kline| kline.close).collect();
        let short_ema = self.calculate_ema(&prices, self.short_ema_period);
        let long_ema = self.calculate_ema(&prices, self.long_ema_period);

        let macd_value = short_ema - long_ema;
        self.macd_line.push(macd_value);

        // Use the MACD line values for the signal line calculation
        let signal_value = if self.macd_line.len() >= self.signal_ema_period {
            self.calculate_ema(&self.macd_line, self.signal_ema_period)
        } else {
            0.0
        };
        self.signal_line.push(signal_value);
    }
}

impl Algorithm for Macd {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline);
        self.update_macd_and_signal_lines();

        if let (Some(&latest_macd), Some(&latest_signal)) =
            (self.macd_line.last(), self.signal_line.last())
        {
            if latest_macd > latest_signal {
                // MACD line crosses above the signal line, potential buy signal
                AlgorithmEvalResult::Long
            } else if latest_macd < latest_signal {
                // MACD line crosses below the signal line, potential sell signal
                AlgorithmEvalResult::Short
            } else {
                AlgorithmEvalResult::Ignore
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
        if let Ok(short_ema_period) = parse_usize_from_value("short_ema_period", &params) {
            self.short_ema_period = short_ema_period
        }
        if let Ok(long_ema_period) = parse_usize_from_value("long_ema_period", &params) {
            self.long_ema_period = long_ema_period
        }
        if let Ok(signal_ema_period) = parse_usize_from_value("signal_ema_period", &params) {
            self.signal_ema_period = signal_ema_period
        }
        // Update parameters logic...
        self.params = params;

        Ok(())
    }
}
