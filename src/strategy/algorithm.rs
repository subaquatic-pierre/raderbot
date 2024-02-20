use std::time::Duration;

use log::warn;
use serde_json::Value;

use crate::{
    account::trade::OrderSide,
    algorithm::ma_crossover::MovingAverageCrossover,
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
        algorithm_params: Value,
    ) -> Result<Box<dyn Algorithm>, AlgorithmError> {
        let interval = match build_interval(interval) {
            Ok(interval) => interval,
            Err(e) => return Err(AlgorithmError::UnknownInterval(e.to_string())),
        };
        match algorithm_name {
            "MovingAverageCrossover" => {
                let algo = MovingAverageCrossover::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            _ => Err(AlgorithmError::UnkownName(
                format!("Strategy name {algorithm_name} is incorrect").to_string(),
            )),
        }
    }
}
