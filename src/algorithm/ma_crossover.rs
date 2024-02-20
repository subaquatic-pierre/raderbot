use std::time::Duration;

use crate::market::kline::Kline;

use crate::strategy::types::AlgorithmError;
use crate::strategy::{algorithm::Algorithm, types::AlgorithmEvalResult};
use crate::utils::number::parse_usize_from_value;
use ta::indicators::{ExponentialMovingAverage, SimpleMovingAverage};

// use indicators::exponential_moving_average::ExponentialMovingAverage;
// use indicators::simple_moving_average::SimpleMovingAverage;
use serde_json::Value;
use ta::Next;

// Assume the existence of the Kline struct and other necessary dependencies

pub struct MovingAverageCrossover {
    data_points: Vec<Kline>,
    interval: Duration,
    ema_period: usize,
    sma_period: usize,
    ema: ExponentialMovingAverage,
    sma: SimpleMovingAverage,
}

impl MovingAverageCrossover {
    pub fn new(interval: Duration, algorithm_params: Value) -> Result<Self, AlgorithmError> {
        let ema_period = parse_usize_from_value("ema_period", algorithm_params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let sma_period = parse_usize_from_value("sma_period", algorithm_params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        let ema = ExponentialMovingAverage::new(ema_period)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let sma = SimpleMovingAverage::new(sma_period)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        Ok(Self {
            data_points: vec![],
            interval,
            ema_period,
            sma_period,
            ema,
            sma,
        })
    }

    fn calculate_ema(&mut self, kline: Kline) -> f64 {
        self.ema.next(kline.close)
    }

    fn calculate_sma(&mut self, kline: Kline) -> f64 {
        self.sma.next(kline.close)
    }
}

impl Algorithm for MovingAverageCrossover {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        if self.data_points.len() >= self.sma_period {
            let ema = self.calculate_ema(kline.clone());
            let sma = self.calculate_sma(kline.clone());

            // EMA crossover signal
            if ema > sma {
                return AlgorithmEvalResult::Buy;
            } else if ema < sma {
                return AlgorithmEvalResult::Sell;
            }

            // SMA crossover signal (additional signal for diversity)
            if kline.close > sma {
                return AlgorithmEvalResult::Buy;
            } else if kline.close < sma {
                return AlgorithmEvalResult::Sell;
            }
        }

        AlgorithmEvalResult::Ignore
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }

    fn interval(&self) -> Duration {
        // Set your desired interval
        self.interval
    }

    fn strategy_name(&self) -> String {
        format!(
            "MovingAverageCrossover(EMA:{}, SMA:{})",
            self.ema_period, self.sma_period
        )
    }
}
