use crate::{
    account::trade::OrderSide,
    market::{
        kline::{Kline, KlineData},
        ticker::TickerData,
    },
};

use super::types::{AlgorithmEvalResult, AlgorithmInputType};

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
}

impl Algorithm for MovingAverage {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline);
        // TODO: Loop over all data, run eval
        for point in &self.data_points {}

        if self.data_points.len() % 3 == 0 {
            AlgorithmEvalResult::Buy
        } else if self.data_points.len() % 3 == 1 {
            AlgorithmEvalResult::Sell
        } else {
            AlgorithmEvalResult::Ignore
        }
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }
}
