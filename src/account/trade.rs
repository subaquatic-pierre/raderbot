use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{strategy::strategy::StrategyId, utils::time::generate_ts};

use uuid::Uuid;

pub type PositionId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => f.write_str("BUY"),
            OrderSide::Sell => f.write_str("SELL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            OrderSide::Buy => total_close_usd - total_open_usd,
            OrderSide::Sell => total_open_usd - total_close_usd,
        }
    }
}
