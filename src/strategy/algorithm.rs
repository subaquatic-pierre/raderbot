use crate::{
    account::trade::OrderSide,
    market::{
        kline::{Kline, KlineData},
        ticker::TickerData,
    },
};

use super::types::AlgorithmEvalResult;

pub trait Algorithm: Send + Sync {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult;
    fn data_points(&self) -> Vec<Kline>;
}

pub struct MovingAverage {
    data_points: Vec<Kline>,
}

impl MovingAverage {
    pub fn new() -> Self {
        Self {
            data_points: vec![],
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
}

pub fn build_algorithm(algorithm_name: &str) -> Option<Box<dyn Algorithm>> {
    match algorithm_name {
        "MovingAverage" => Some(Box::new(MovingAverage::new())),
        _ => None,
    }
}
