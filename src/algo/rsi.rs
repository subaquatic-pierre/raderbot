use crate::market::kline::Kline;
use crate::market::trade::Trade;
use crate::strategy::types::AlgoError;
use crate::strategy::{algorithm::Algorithm, types::AlgoEvalResult};
use fin_primitives::ta_compat::{indicators::RelativeStrengthIndex, Next};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct RsiParams {
    // Add fields that will be serialized from the params Value here
    rsi_period: Option<usize>,
}

pub struct Rsi {
    data_points: Vec<Kline>,
    params: Value,
    rsi_period: usize,
    rsi: f64, // Last calculated RSI value; NaN until `rsi_period + 1` klines are in
    indicator: RelativeStrengthIndex,
}

fn rsi_indicator(period: usize) -> Result<RelativeStrengthIndex, AlgoError> {
    RelativeStrengthIndex::new(period)
        .map_err(|e| AlgoError::InvalidParams(format!("invalid rsi_period {period}: {e}")))
}

impl Rsi {
    pub fn new(params: Value) -> Result<Self, AlgoError> {
        let rsi_params: RsiParams = serde_json::from_value(params.clone())?;

        let rsi_period = rsi_params.rsi_period.unwrap_or(14);

        Ok(Self {
            data_points: vec![],
            rsi_period,
            rsi: f64::NAN,
            params,
            indicator: rsi_indicator(rsi_period)?,
        })
    }

    /// Wilder's RSI (the one TA-Lib and charting platforms show), updated with
    /// the latest kline's close. NaN until `rsi_period + 1` closes are in, so
    /// the `< 30` / `> 70` checks below do not fire during warm-up.
    fn calculate_rsi(&mut self, close: f64) -> f64 {
        self.rsi = self.indicator.next(close);
        self.rsi
    }
}

impl Algorithm for Rsi {
    fn evaluate(&mut self, kline: Kline, trades: &[Trade]) -> AlgoEvalResult {
        let rsi = self.calculate_rsi(kline.close);
        self.data_points.push(kline);

        // Example RSI logic: Buy if RSI < 30 (oversold), Sell if RSI > 70 (overbought), else Ignore
        let result = if rsi < 30.0 {
            AlgoEvalResult::Buy
        } else if rsi > 70.0 {
            AlgoEvalResult::Sell
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
        let rsi_params: RsiParams = serde_json::from_value(params.clone())?;

        let rsi_period = rsi_params.rsi_period.unwrap_or(self.rsi_period);

        if rsi_period != self.rsi_period {
            // A new period starts a fresh average from the stored klines.
            let mut indicator = rsi_indicator(rsi_period)?;
            self.rsi = self
                .data_points
                .iter()
                .fold(f64::NAN, |_, k| indicator.next(k.close));
            self.indicator = indicator;
        }
        self.rsi_period = rsi_period;
        self.params = params;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rsi_matches_wilder_and_waits_for_a_full_window() {
        // Wilder's example closes; RSI(14) from TA-Lib on the same data.
        let closes = [
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03,
            45.61, 46.28, 46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
        ];
        let expected = [
            70.46413502109704,
            66.24961855355505,
            66.48094183471265,
            69.34685316290864,
            66.29471265892623,
            57.91502067008556,
        ];
        let mut rsi = Rsi::new(json!({ "rsi_period": 14 })).unwrap();
        for (i, close) in closes.iter().enumerate() {
            let value = rsi.calculate_rsi(*close);
            if i < 14 {
                // Before this change the first 14 klines read as RSI 0 (a Buy
                // signal), and the 14th indexed before the start of the slice.
                assert!(value.is_nan(), "kline {i}: {value}");
            } else {
                assert!(
                    (value - expected[i - 14]).abs() < 1e-9,
                    "kline {i}: {value}"
                );
            }
        }
    }
}
