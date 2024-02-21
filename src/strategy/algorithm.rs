use std::time::Duration;

use log::warn;
use serde_json::Value;

use crate::{
    account::trade::OrderSide,
    algorithm::{
        ma_crossover::EmaSmaCrossover, ma_simple::SimpleMovingAverage,
        ma_three_crossover::ThreeMaCrossover,
    },
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
    fn set_params(&mut self, params: Value) -> Result<(), AlgorithmError>;
    fn get_params(&self) -> &Value;
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
            "EmaSmaCrossover" => {
                let algo = EmaSmaCrossover::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "SimpleMovingAverage" => {
                let algo = SimpleMovingAverage::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "ThreeMaCrossover" => {
                let algo = ThreeMaCrossover::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            _ => Err(AlgorithmError::UnkownName(
                format!("Strategy name {algorithm_name} is incorrect").to_string(),
            )),
        }
    }
}
