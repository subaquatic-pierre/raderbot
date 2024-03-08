use std::time::Duration;

use crate::market::kline::Kline;

use crate::market::trade::Trade;
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use crate::utils::number::parse_usize_from_value;
use ta::indicators::{ExponentialMovingAverage, SimpleMovingAverage};

// use indicators::exponential_moving_average::ExponentialMovingAverage;
// use indicators::simple_moving_average::SimpleMovingAverage;
use serde_json::Value;
use ta::Next;

// Assume the existence of the Kline struct and other necessary dependencies

pub struct EmaSmaCrossover {
    data_points: Vec<Kline>,
    ema_period: usize,
    sma_period: usize,
    ema: ExponentialMovingAverage,
    sma: SimpleMovingAverage,
    params: Value,
}

impl EmaSmaCrossover {
    pub fn new(params: Value) -> Result<Self, AlgoError> {
        let ema_period = parse_usize_from_value("ema_period", &params)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
        let sma_period = parse_usize_from_value("sma_period", &params)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;

        let ema = ExponentialMovingAverage::new(ema_period)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
        let sma = SimpleMovingAverage::new(sma_period)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;

        Ok(Self {
            data_points: vec![],
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
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        self.data_points.push(kline.clone());

        let result = if self.data_points.len() >= self.sma_period {
            let ema = self.calculate_ema(kline.clone());
            let sma = self.calculate_sma(kline.clone());

            // EMA crossover signal
            let result = if ema > sma {
                AlgoEvalResult::Buy
            } else if ema < sma {
                AlgoEvalResult::Sell
            } else if kline.close > sma {
                AlgoEvalResult::Buy
            } else if kline.close < sma {
                AlgoEvalResult::Sell
            } else {
                AlgoEvalResult::Ignore
            };
            result
        } else {
            AlgoEvalResult::Ignore
        };

        self.clean_data_points();

        result
    }

    fn data_points(&self) -> Vec<Kline> {
        self.data_points.clone()
    }

    fn get_params(&self) -> &Value {
        &self.params
    }

    fn set_params(&mut self, params: Value) -> Result<(), AlgoError> {
        let ema_period = parse_usize_from_value("ema_period", &params.clone())
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
        let sma_period = parse_usize_from_value("sma_period", &params.clone())
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;

        let ema = ExponentialMovingAverage::new(ema_period)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;
        let sma = SimpleMovingAverage::new(sma_period)
            .or_else(|e| Err(AlgoError::InvalidParams(e.to_string())))?;

        self.params = params;
        self.ema = ema;
        self.sma = sma;
        self.ema_period = ema_period;
        self.sma_period = sma_period;

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
