use crate::market::kline::Kline;
use crate::strategy::{
    algorithm::Algorithm,
    types::{AlgorithmError, AlgorithmEvalResult},
};
use crate::utils::number::parse_usize_from_value;
use serde_json::Value;
use std::time::Duration;

pub struct MacdBollingerBands {
    data_points: Vec<Kline>,
    interval: Duration,
    bollinger_period: usize,
    bollinger_multiplier: f64,
    short_ema_period: usize,
    long_ema_period: usize,
    signal_ema_period: usize,
    macd_line: Vec<f64>,
    signal_line: Vec<f64>,
    params: Value,
}

impl MacdBollingerBands {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        let bollinger_period = params
            .get("bollinger_period")
            .and_then(Value::as_u64)
            .unwrap_or(20) as usize;
        let bollinger_multiplier = params
            .get("bollinger_multiplier")
            .and_then(Value::as_f64)
            .unwrap_or(2.0);
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
            bollinger_period,
            bollinger_multiplier,
            short_ema_period,
            long_ema_period,
            signal_ema_period,
            macd_line: Vec::new(),
            signal_line: Vec::new(),
            params,
        })
    }

    // Assuming `prices` is a slice of the closing prices
    fn calculate_ema(&self, prices: &[f64], period: usize) -> f64 {
        if prices.is_empty() || period == 0 {
            return 0.0;
        }

        let k = 2.0 / (period as f64 + 1.0);
        prices.iter().fold(0.0, |acc, &price| {
            if acc == 0.0 {
                price
            } else {
                price * k + acc * (1.0 - k)
            }
        })
    }

    fn calculate_bollinger_bands(&self) -> (f64, f64, f64) {
        let prices: Vec<f64> = self.data_points.iter().map(|kline| kline.close).collect();
        let sma: f64 = prices.iter().sum::<f64>() / self.bollinger_period as f64;
        let _std_dev: f64 = prices
            .iter()
            .map(|price| (price - sma).powf(2.0))
            .sum::<f64>()
            / self.bollinger_period as f64;
        let std_dev = _std_dev.sqrt();

        let upper_band = sma + std_dev * self.bollinger_multiplier;
        let lower_band = sma - std_dev * self.bollinger_multiplier;

        (upper_band, sma, lower_band)
    }

    fn update_macd_and_signal_lines(&mut self) {
        let prices: Vec<f64> = self.data_points.iter().map(|kline| kline.close).collect();
        let short_ema = self.calculate_ema(&prices, self.short_ema_period);
        let long_ema = self.calculate_ema(&prices, self.long_ema_period);
        let macd_value = short_ema - long_ema;
        self.macd_line.push(macd_value);

        // Calculate Signal line: EMA of the MACD line
        let signal_value = if self.macd_line.len() >= self.signal_ema_period {
            self.calculate_ema(&self.macd_line, self.signal_ema_period)
        } else {
            0.0 // Not enough data to calculate the signal line
        };
        self.signal_line.push(signal_value);
    }
}

impl Algorithm for MacdBollingerBands {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        let (upper_band, _, lower_band) = self.calculate_bollinger_bands();
        self.update_macd_and_signal_lines();

        let result = if let (Some(&latest_macd), Some(&latest_signal)) =
            (self.macd_line.last(), self.signal_line.last())
        {
            let price = kline.close;

            if price < lower_band && latest_macd > latest_signal {
                // Buy signal: price below lower Bollinger Band and MACD crosses above signal line
                AlgorithmEvalResult::Buy
            } else if price > upper_band && latest_macd < latest_signal {
                // Sell signal: price above upper Bollinger Band and MACD crosses below signal line
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
        if let Ok(bollinger_period) = parse_usize_from_value("bollinger_period", &params) {
            self.bollinger_period = bollinger_period
        }
        if let Ok(bollinger_multiplier) = parse_usize_from_value("bollinger_multiplier", &params) {
            self.bollinger_multiplier = bollinger_multiplier as f64
        }
        if let Ok(short_ema_period) = parse_usize_from_value("short_ema_period", &params) {
            self.short_ema_period = short_ema_period
        }
        if let Ok(long_ema_period) = parse_usize_from_value("long_ema_period", &params) {
            self.long_ema_period = long_ema_period
        }
        if let Ok(signal_ema_period) = parse_usize_from_value("signal_ema_period", &params) {
            self.signal_ema_period = signal_ema_period
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
