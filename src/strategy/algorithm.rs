use crate::{account::trade::OrderSide, market::ticker::TickerData};

pub trait Algorithm: Send + Sync {
    fn evaluate(&mut self, ticker_data: TickerData) -> AlgorithmEvalResult;
}

pub enum AlgorithmEvalResult {
    Buy,
    Sell,
    Ignore,
}

pub struct MovingAverage {
    data_points: Vec<TickerData>,
}

impl MovingAverage {
    pub fn new() -> Self {
        Self {
            data_points: vec![],
        }
    }
}

impl Algorithm for MovingAverage {
    fn evaluate(&mut self, ticker_data: TickerData) -> AlgorithmEvalResult {
        self.data_points.push(ticker_data);
        // TODO: Loop over all data, run eval
        // for point in &self.data_points {
        // }
        if self.data_points.len() % 3 == 0 {
            AlgorithmEvalResult::Buy
        } else if self.data_points.len() % 3 == 1 {
            AlgorithmEvalResult::Sell
        } else {
            AlgorithmEvalResult::Ignore
        }
    }
}
