use std::time::Duration;

use serde_json::Value;

use crate::{
    algo::{
        bollinger_bands::BollingerBands, ma_crossover::EmaSmaCrossover,
        ma_simple::SimpleMovingAverage, ma_three_crossover::ThreeMaCrossover, macd::Macd,
        macd_bollinger::MacdBollingerBands, rsi::Rsi,
    },
    market::{interval::Interval, kline::Kline, trade::Trade},
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
        algorithm_params: Value,
    ) -> Result<Box<dyn Algorithm>, AlgoError> {
        match algorithm_name {
            "EmaSmaCrossover" => {
                let algo = EmaSmaCrossover::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "SimpleMovingAverage" => {
                let algo = SimpleMovingAverage::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "ThreeMaCrossover" => {
                let algo = ThreeMaCrossover::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "Rsi" => {
                let algo = Rsi::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "RsiEmaSma" => {
                let algo = Rsi::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "BollingerBands" => {
                let algo = BollingerBands::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "Macd" => {
                let algo = Macd::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "MacdBollingerBands" => {
                let algo = MacdBollingerBands::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            "VolumeProfile" => {
                let algo = VolumeProfile::new(algorithm_params)?;
                Ok(Box::new(algo))
            }
            _ => Err(AlgoError::UnkownName(
                format!("Strategy name {algorithm_name} is incorrect").to_string(),
            )),
        }
    }
}
