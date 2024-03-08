use std::time::Duration;

use serde_json::Value;

use crate::{
    algo::{
        bollinger_bands::BollingerBands, ma_crossover::EmaSmaCrossover,
        ma_simple::SimpleMovingAverage, ma_three_crossover::ThreeMaCrossover, macd::Macd,
        macd_bollinger::MacdBollingerBands, rsi::Rsi,
    },
    market::{kline::Kline, trade::Trade},
    strategy::{
        algorithm::Algorithm,
        types::{AlgoError, AlgoEvalResult},
    },
    utils::time::build_interval,
};

use super::volume_profile::VolumeProfile;

/// A builder for constructing instances of algorithms based on their names and parameters.
///
/// This struct provides a method to build various trading algorithm instances dynamically
/// based on the algorithm's name, the desired interval for evaluation, and any specific
/// parameters required by the algorithm.

pub struct AlgoBuilder {}

impl AlgoBuilder {
    /// Constructs a new algorithm instance based on provided specifications.
    ///
    /// # Arguments
    ///
    /// * `algorithm_name` - A string slice representing the name of the algorithm to construct.
    /// * `interval` - A string slice representing the interval between k-lines for the algorithm's operation.
    /// * `algorithm_params` - A `Value` containing any specific parameters required by the algorithm.
    ///
    /// # Returns
    ///
    /// A `Result` containing the constructed algorithm boxed as a `dyn Algorithm` if successful,
    /// or an `AlgoError` if an error occurs during construction.

    pub fn build_algorithm(
        algorithm_name: &str,
        interval: &str,
        algorithm_params: Value,
    ) -> Result<Box<dyn Algorithm>, AlgoError> {
        let interval = match build_interval(interval) {
            Ok(interval) => interval,
            Err(e) => return Err(AlgoError::UnknownInterval(e.to_string())),
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
            "Rsi" => {
                let algo = Rsi::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "RsiEmaSma" => {
                let algo = Rsi::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "BollingerBands" => {
                let algo = BollingerBands::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "Macd" => {
                let algo = Macd::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "MacdBollingerBands" => {
                let algo = MacdBollingerBands::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            "VolumeProfile" => {
                let algo = VolumeProfile::new(interval, algorithm_params)?;
                Ok(Box::new(algo))
            }
            _ => Err(AlgoError::UnkownName(
                format!("Strategy name {algorithm_name} is incorrect").to_string(),
            )),
        }
    }
}
