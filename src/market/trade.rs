use serde::{Deserialize, Serialize};

use crate::account::trade::OrderSide;

#[derive(Serialize, Deserialize, Debug)]
pub struct AggTrade {
    pub symbol: String,
    pub ts: u64,
    pub qty: f64,
    pub price: f64,
    pub order_side: OrderSide,
}
