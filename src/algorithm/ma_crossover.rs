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

pub struct EmaSmaCrossover {
    data_points: Vec<Kline>,
    interval: Duration,
    ema_period: usize,
    sma_period: usize,
    ema: ExponentialMovingAverage,
    sma: SimpleMovingAverage,
    params: Value,
}

impl EmaSmaCrossover {
    pub fn new(interval: Duration, params: Value) -> Result<Self, AlgorithmError> {
        let ema_period = parse_usize_from_value("ema_period", params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let sma_period = parse_usize_from_value("sma_period", params.clone())
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
            params,
        })
    }

    fn calculate_ema(&mut self, kline: Kline) -> f64 {
        self.ema.next(kline.close)
    }

    fn calculate_sma(&mut self, kline: Kline) -> f64 {
        self.sma.next(kline.close)
    }
}

impl Algorithm for EmaSmaCrossover {
    fn evaluate(&mut self, kline: Kline) -> AlgorithmEvalResult {
        self.data_points.push(kline.clone());

        if self.data_points.len() >= self.sma_period {
            let ema = self.calculate_ema(kline.clone());
            let sma = self.calculate_sma(kline.clone());

            // EMA crossover signal
            if ema > sma {
                return AlgorithmEvalResult::Long;
            } else if ema < sma {
                return AlgorithmEvalResult::Short;
            }

            // SMA crossover signal (additional signal for diversity)
            if kline.close > sma {
                return AlgorithmEvalResult::Long;
            } else if kline.close < sma {
                return AlgorithmEvalResult::Short;
            }
        }

        AlgorithmEvalResult::Ignore
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
        let ema_period = parse_usize_from_value("ema_period", params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let sma_period = parse_usize_from_value("sma_period", params.clone())
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        let ema = ExponentialMovingAverage::new(ema_period)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;
        let sma = SimpleMovingAverage::new(sma_period)
            .or_else(|e| Err(AlgorithmError::InvalidParams(e.to_string())))?;

        self.params = params;
        self.ema = ema;
        self.sma = sma;
        self.ema_period = ema_period;
        self.sma_period = sma_period;

        Ok(())
    }
}
