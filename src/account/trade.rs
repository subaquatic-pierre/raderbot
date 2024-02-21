use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{strategy::strategy::StrategyId, utils::time::generate_ts};

use uuid::Uuid;

pub type PositionId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Copy)]
pub enum OrderSide {
    Long,
    Short,
}

impl Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Long => f.write_str("Long"),
            OrderSide::Short => f.write_str("Short"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub id: PositionId,
    pub symbol: String,
    pub order_side: OrderSide,
    pub open_time: u64,
    pub open_price: f64,
    pub quantity: f64,
    pub margin_usd: f64,
    pub leverage: u32,
    pub strategy_id: Option<StrategyId>,
    pub stop_loss: Option<f64>,
}

impl Position {
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
            open_time: generate_ts(),
        }
    }

    pub fn set_stop_loss(&mut self, stop_loss: Option<f64>) {
        self.stop_loss = stop_loss
    }
    pub fn set_strategy_id(&mut self, strategy_id: Option<StrategyId>) {
        self.strategy_id = strategy_id
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TradeTx {
    pub id: Uuid,
    pub close_time: u64,
    pub close_price: f64,
    pub position: Position,
}

impl TradeTx {
    pub fn new(close_price: f64, close_time: u64, position: Position) -> Self {
        Self {
            id: Uuid::new_v4(),
            close_price,
            close_time,
            position,
        }
    }

    pub fn calc_profit(&self) -> f64 {
        let total_open_usd = self.position.open_price * self.position.quantity;
        let total_close_usd = self.close_price * self.position.quantity;
        match self.position.order_side {
            OrderSide::Long => total_close_usd - total_open_usd,
            OrderSide::Short => total_open_usd - total_close_usd,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::time::generate_ts;
    use tokio::test;

    #[test]
    async fn test_position_new() {
        let symbol = "BTCUSD";
        let open_price = 50000.0;
        let order_side = OrderSide::Long;
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
        assert!(position.open_time <= generate_ts());
    }

    #[test]
    async fn test_trade_tx_new() {
        let close_price = 51000.0;
        let close_time = generate_ts();

        let position = Position::new(
            "BTCUSD",
            50000.0,
            OrderSide::Long,
            1000.0,
            10,
            Some(49000.0),
        );

        let trade_tx = TradeTx::new(close_price, close_time, position.clone());

        assert_eq!(trade_tx.close_price, close_price);
        assert_eq!(trade_tx.close_time, close_time);
        assert_eq!(trade_tx.position, position);

        // Assert that trade_tx has a unique ID
        let another_trade_tx = TradeTx::new(52000.0, generate_ts(), position.clone());
        assert_ne!(trade_tx.id, another_trade_tx.id);

        // Assert that calc_profit calculates correctly
        let expected_profit = (close_price - position.open_price) * position.quantity;
        assert_eq!(trade_tx.calc_profit(), expected_profit);
    }
}
