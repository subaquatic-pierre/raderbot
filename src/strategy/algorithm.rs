use std::time::Duration;

use log::warn;

use crate::{
    account::trade::OrderSide,
    algorithm::moving_average::MovingAverage,
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
    fn strategy_name(&self) -> String;
}

pub struct AlgorithmBuilder {}

impl AlgorithmBuilder {
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
}
