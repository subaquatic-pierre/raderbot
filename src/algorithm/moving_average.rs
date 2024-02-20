use std::time::Duration;

use crate::market::kline::Kline;

use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};

pub struct MovingAverage {
    data_points: Vec<Kline>,
    interval: Duration,
}

impl MovingAverage {
    pub fn new(interval: Duration) -> Self {
        Self {
            data_points: vec![],
            interval,
        }
    }

    fn calculate_moving_average(&self, period: usize) -> f64 {
        let start_index = self.data_points.len().saturating_sub(period); // Avoid index underflow

        let sum: f64 = self
            .data_points
            .iter()
            .rev()
            .skip(start_index)
            .map(|k| k.close)
            .sum();

        let divisor = usize::min(period, self.data_points.len()); // Ensure divisor is not zero

        sum / divisor as f64
    }
}

impl Algorithm for MovingAverage {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        // Set the moving average period
        let ma_period = 30;

        if self.data_points.len() >= ma_period {
            let ma = self.calculate_moving_average(ma_period);

            // Placeholder logic for buy/sell decision
            if kline.close > ma {
                AlgorithmEvalResult::Buy
            } else {
                AlgorithmEvalResult::Sell
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
        "MovingAverage".to_string()
    }
}