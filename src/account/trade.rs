use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub symbol: String,
    pub status: String,
    pub order_side: OrderSide,
    pub entry_price: f64,
    pub stop_loss: Option<f64>,
    pub quantity: f64,
    pub margin: f64,
    pub leverage: u32,
    pub last_price: f64,
    pub order_id: Option<String>,
}

impl Position {
    pub fn new(
        symbol: &str,
        last_price: f64,
        order_side: OrderSide,
        stop_loss: Option<f64>,
        margin: f64,
        leverage: u32,
    ) -> Self {
        let total = margin * leverage as f64;
        let qty = total / last_price;

        Self {
            symbol: symbol.to_string(),
            status: "open".to_string(),
            order_side,
            entry_price: last_price,
            stop_loss,
            quantity: qty,
            margin,
            leverage,
            last_price,
            order_id: None,
        }
    }

    pub fn _set_id(&mut self, id: &str) {
        self.order_id = Some(id.to_string());
    }
}
