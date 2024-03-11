use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{
    strategy::{strategy::StrategyId, types::SignalMessage},
    utils::time::{generate_ts, timestamp_to_string},
};

use uuid::Uuid;

pub type PositionId = Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TradeTxMeta {
    pub signals: Vec<SignalMessage>,
}

/// Enum representing the side of an order (Buy or Sell).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, Copy)]
pub enum OrderSide {
    /// Represents a Buy order side.
    Buy,
    /// Represents a Sell order side.
    Sell,
}

impl Display for OrderSide {
    /// Formats the enum variant as a string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    ///
    /// # Returns
    ///
    /// A `std::fmt::Result`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => f.write_str("Buy"),
            OrderSide::Sell => f.write_str("Sell"),
        }
    }
}

/// Struct representing a trading position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    /// The unique identifier of the position.
    pub id: PositionId,
    /// The symbol associated with the position.
    pub symbol: String,
    /// The side of the order (Buy or Sell).
    pub order_side: OrderSide,
    /// The time when the position was opened.
    pub open_time: String,
    /// The price at which the position was opened.
    pub open_price: f64,
    /// The quantity of the asset in the position.
    pub quantity: f64,
    /// The margin used for the position in USD.
    pub margin_usd: f64,
    /// The leverage used for the position.
    pub leverage: u32,
    /// The optional strategy ID associated with the position.
    pub strategy_id: Option<StrategyId>,
    /// The optional stop loss price for the position.
    pub stop_loss: Option<f64>,
}

impl Position {
    /// Creates a new position with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol associated with the position.
    /// * `open_price` - The price at which the position was opened.
    /// * `order_side` - The side of the order (Buy or Sell).
    /// * `margin_usd` - The margin used for the position in USD.
    /// * `leverage` - The leverage used for the position.
    /// * `stop_loss` - The optional stop loss price for the position.
    ///
    /// # Returns
    ///
    /// A new `Position` instance.
    pub fn new(
        symbol: &str,
        open_price: f64,
        order_side: OrderSide,
        margin_usd: f64,
        leverage: u32,
        stop_loss: Option<f64>,
    ) -> Self {
        let total = margin_usd * leverage as f64;
        let qty = total / open_price;

        Self {
            id: Uuid::new_v4(),
            symbol: symbol.to_string(),
            order_side,
            open_price,
            stop_loss,
            quantity: qty,
            margin_usd,
            leverage,
            strategy_id: None,
            open_time: timestamp_to_string(generate_ts()),
        }
    }

    /// Sets the stop loss price for the position.
    ///
    /// # Arguments
    ///
    /// * `stop_loss` - The optional stop loss price for the position.

    pub fn set_stop_loss(&mut self, stop_loss: Option<f64>) {
        self.stop_loss = stop_loss
    }

    /// Sets the strategy ID associated with the position.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The optional strategy ID associated with the position.

    pub fn set_strategy_id(&mut self, strategy_id: Option<StrategyId>) {
        self.strategy_id = strategy_id
    }
}

/// Struct representing a trading transaction.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TradeTx {
    /// The unique identifier of the trade transaction.
    pub id: Uuid,
    pub profit: f64,
    /// The time when the position was closed.
    pub close_time: String,
    /// The price at which the position was closed.
    pub close_price: f64,
    /// The position associated with the trade transaction.
    pub position: Position,
    pub meta: Option<TradeTxMeta>,
}
impl TradeTx {
    /// Creates a new trade transaction with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `close_price` - The price at which the position was closed.
    /// * `close_time` - The time when the position was closed.
    /// * `position` - The position associated with the trade transaction.
    ///
    /// # Returns
    ///
    /// A new `TradeTx` instance.

    pub fn new(close_price: f64, close_time: u64, position: Position) -> Self {
        let profit = TradeTx::calc_profit(close_price, &position);
        Self {
            id: Uuid::new_v4(),
            close_price,
            profit,
            close_time: timestamp_to_string(close_time),
            position,
            meta: None,
        }
    }

    pub fn add_signal(&mut self, signal: &SignalMessage) {
        if let Some(meta) = &mut self.meta {
            meta.signals.push(signal.clone())
        } else {
            self.meta = Some(TradeTxMeta {
                signals: vec![signal.clone()],
            })
        }
    }

    /// Calculates the profit of the trade transaction.
    ///
    /// # Returns
    ///
    /// The profit of the trade transaction.

    // pub fn calc_profit(&self) -> f64 {
    //     let total_open_usd = self.position.open_price * self.position.quantity;
    //     let total_close_usd = self.close_price * self.position.quantity;
    //     match self.position.order_side {
    //         OrderSide::Buy => total_close_usd - total_open_usd,
    //         OrderSide::Sell => total_open_usd - total_close_usd,
    //     }
    // }

    pub fn calc_profit(close_price: f64, position: &Position) -> f64 {
        let total_open_usd = position.open_price * position.quantity;
        let total_close_usd = close_price * position.quantity;
        match position.order_side {
            OrderSide::Buy => total_close_usd - total_open_usd,
            OrderSide::Sell => total_open_usd - total_close_usd,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::time::{generate_ts, string_to_timestamp};
    use tokio::test;

    #[test]
    async fn test_position_new() {
        let symbol = "BTCUSD";
        let open_price = 50000.0;
        let order_side = OrderSide::Buy;
        let margin_usd = 1000.0;
        let leverage = 10;
        let stop_loss = Some(49000.0);

        let position = Position::new(
            symbol, open_price, order_side, margin_usd, leverage, stop_loss,
        );

        assert_eq!(position.symbol, symbol);
        assert_eq!(position.open_price, open_price);
        assert_eq!(position.order_side, order_side);
        assert_eq!(position.margin_usd, margin_usd);
        assert_eq!(position.leverage, leverage);
        assert_eq!(position.stop_loss, stop_loss);

        // Assert that quantity is calculated correctly
        let expected_quantity = margin_usd * leverage as f64 / open_price;
        assert_eq!(position.quantity, expected_quantity);

        // Assert that other fields have default values
        assert!(position.strategy_id.is_none());
        assert!(string_to_timestamp(&position.open_time).unwrap() <= generate_ts());
    }

    #[test]
    async fn position_set_stop_loss() {
        let mut position = Position::new("ETHUSD", 2000.0, OrderSide::Sell, 500.0, 5, None);
        position.set_stop_loss(Some(1900.0));
        assert_eq!(position.stop_loss, Some(1900.0));
    }

    #[test]
    async fn position_set_strategy_id() {
        let mut position = Position::new("ETHUSD", 2000.0, OrderSide::Sell, 500.0, 5, None);
        let strategy_id = StrategyId::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
        position.set_strategy_id(Some(strategy_id));
        assert_eq!(position.strategy_id, Some(strategy_id));
    }

    #[test]
    async fn test_trade_tx_new() {
        let close_price = 51000.0;
        let close_time = generate_ts();

        let position = Position::new("BTCUSD", 50000.0, OrderSide::Buy, 1000.0, 10, Some(49000.0));

        let trade_tx = TradeTx::new(close_price, close_time, position.clone());

        assert_eq!(trade_tx.close_price, close_price);
        assert_eq!(trade_tx.close_time, timestamp_to_string(close_time));
        assert_eq!(trade_tx.position, position);

        // Assert that trade_tx has a unique ID
        let another_trade_tx = TradeTx::new(52000.0, generate_ts(), position.clone());
        assert_ne!(trade_tx.id, another_trade_tx.id);

        // Assert that calc_profit calculates correctly
        let expected_profit = (close_price - position.open_price) * position.quantity;
        assert_eq!(
            TradeTx::calc_profit(close_price, &position),
            expected_profit
        );
    }

    #[test]
    async fn calc_profit_edge_cases() {
        let position_zero_qty = Position {
            id: Uuid::new_v4(),
            symbol: "BTCUSD".to_string(),
            order_side: OrderSide::Buy,
            open_time: "2021-01-01T00:00:00Z".to_string(),
            open_price: 50000.0,
            quantity: 0.0,
            margin_usd: 0.0,
            leverage: 10,
            strategy_id: None,
            stop_loss: None,
        };
        let trade_tx_zero_qty = TradeTx::new(51000.0, generate_ts(), position_zero_qty);
        assert_eq!(trade_tx_zero_qty.profit, 0.0);
    }
}
