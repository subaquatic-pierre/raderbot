use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::account::trade::OrderSide;

#[derive(Serialize, Deserialize, Debug)]
pub struct SignalMessage {
    pub strategy_id: String,
    pub order_side: OrderSide,
    pub symbol: String,
    pub price: f64,
}

// impl Display for SignalMessage {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str("{self:?}")
//     }
// }
