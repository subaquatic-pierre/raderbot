use std::time::Duration;

use serde_json::Value;

use crate::{
    algo::{
        bollinger_bands::BollingerBands, ma_crossover::EmaSmaCrossover,
        ma_simple::SimpleMovingAverage, ma_three_crossover::ThreeMaCrossover, macd::Macd,
        macd_bollinger::MacdBollingerBands, rsi::Rsi,
    },
    market::{kline::Kline, trade::Trade},
    strategy::types::{AlgoError, AlgoEvalResult},
    utils::time::build_interval,
};

/// Defines a trait for algorithm implementations used in trading strategies.
///
/// This trait outlines the necessary functionality for any algorithm used to evaluate trading
/// signals based on historical k-line (candlestick) data. It includes methods for evaluating  
/// trading signals, setting and retrieving algorithm parameters, and managing historical data
/// points.

pub trait Algorithm: Send + Sync {
    /// Evaluates a single k-line (candlestick) data point to generate a trading signal.
    ///
    /// # Arguments
    ///
    /// * `kline` - A `Kline` struct representing the k-line data to evaluate.
    ///
    /// # Returns
    ///
    /// An `AlgoEvalResult` indicating the trading signal generated by the algorithm.

    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult;

    /// Sets the algorithm's parameters based on a JSON `Value`.
    ///
    /// # Arguments
    ///
    /// * `params` - A `Value` containing the algorithm's configuration parameters.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the parameters were successfully set or an error occurred.

    fn set_params(&mut self, params: Value) -> Result<(), AlgoError>;

    /// Retrieves the current parameters of the algorithm.
    ///
    /// # Returns
    ///
    /// A reference to a JSON `Value` containing the algorithm's current configuration parameters.

    fn get_params(&self) -> &Value;

    /// Provides access to the historical k-line data points the algorithm has evaluated.
    ///
    /// # Returns
    ///
    /// A vector of `Kline` structs representing the historical data points.

    // TODO: Create AlgoDataPointManager to handle data points
    // It will manage cleaning of data if data points length is too long,
    // to manage memory more efficiently as also prevent any bugs creeping
    // up that could occur when implementing a custom algorithm
    fn data_points(&self) -> Vec<Kline>;

    /// Cleans historical data points to manage memory usage efficiently.

    fn clean_data_points(&mut self);

    /// Indicates whether the algorithm requires historical trade data in addition to k-line data
    /// during evaluation.
    ///
    /// This method is called before the `evaluate` method to determine whether the algorithm
    /// needs access to historical trade data in order to generate trading signals based on k-line
    /// data. If this method returns `true`, the caller should provide historical trade data to
    /// the `evaluate` method.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the algorithm requires historical trade data (`true`) or
    /// not (`false`) during evaluation.
    /// Defaults to returning `false`

    fn needs_trades(&self) -> bool {
        false
    }
}
