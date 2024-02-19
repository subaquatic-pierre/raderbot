use std::time::Duration;

use log::warn;

use crate::{
    account::trade::OrderSide,
    market::{
        kline::{Kline, KlineData},
        ticker::TickerData,
    },
    utils::time::build_interval,
};

use super::types::{AlgorithmError, AlgorithmEvalResult};

pub trait Algorithm: Send + Sync {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult;
    fn data_points(&self) -> Vec<Kline>;
    fn interval(&self) -> Duration;
}

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
}

pub fn build_algorithm(
    algorithm_name: &str,
    interval: &str,
) -> Result<Box<dyn Algorithm>, AlgorithmError> {
    let interval = match build_interval(interval) {
        Some(interval) => interval,
        None => {
            return Err(AlgorithmError::UnknownInterval(
                format!("Interval {interval} is incorrect").to_string(),
            ))
        }
    };
    match algorithm_name {
        "MovingAverage" => Ok(Box::new(MovingAverage::new(interval))),
        _ => Err(AlgorithmError::UnkownName(
            format!("Strategy name {algorithm_name} is incorrect").to_string(),
        )),
    }
}
