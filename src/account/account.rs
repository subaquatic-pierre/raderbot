use std::collections::hash_map::Values;
use std::{collections::HashMap, sync::Arc};

use crate::strategy::strategy::StrategyId;
use crate::{
    account::trade::{OrderSide, Position},
    exchange::api::ExchangeApi,
};

use super::trade::{PositionId, TradeTx};

pub struct Account {
    positions: HashMap<PositionId, Position>,
    trade_txs: Vec<TradeTx>,
    exchange_api: Arc<Box<dyn ExchangeApi>>,
}

impl Account {
    pub async fn new(exchange_api: Arc<Box<dyn ExchangeApi>>, init_workers: bool) -> Self {
        let _self = Self {
            exchange_api,
            positions: HashMap::new(),
            trade_txs: vec![],
        };

        if init_workers {
            _self.init().await;
        }
        _self
    }

    pub async fn open_position(
        &mut self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
        strategy_id: Option<StrategyId>,
        stop_loss: Option<f64>,
    ) -> Option<&mut Position> {
        if let Ok(mut position) = self
            .exchange_api
            .open_position(symbol, margin_usd, leverage, order_side, open_price)
            .await
        {
            position.set_stop_loss(stop_loss);
            position.set_strategy_id(strategy_id);
            let position_id = position.id;
            // insert new position into account positions
            self.positions.insert(position.id, position);

            return self.positions.get_mut(&position_id);
        };

        None
    }

    pub async fn close_position(
        &mut self,
        position_id: PositionId,
        close_price: f64,
    ) -> Option<&TradeTx> {
        if let Some(position) = self.positions.get(&position_id).cloned() {
            if let Ok(trade_tx) = self
                .exchange_api
                .close_position(position.clone(), close_price)
                .await
            {
                self.positions.remove(&position.id);

                let trade_tx_id = trade_tx.id;

                self.trade_txs.push(trade_tx);

                if let Some(tx) = self.trade_txs.iter().find(|e| e.id == trade_tx_id) {
                    return Some(tx);
                }
            };
        };

        None
    }

    pub fn open_positions(&self) -> Values<'_, PositionId, Position> {
        self.positions.values()
    }

    pub fn trade_txs(&self) -> Vec<TradeTx> {
        self.trade_txs.clone()
    }

    pub fn strategy_open_positions(&self, strategy_id: StrategyId) -> Vec<Position> {
        let mut positions = vec![];
        for pos in self.positions.values() {
            if let Some(pos_strategy_id) = pos.strategy_id {
                if pos_strategy_id == strategy_id {
                    positions.push(pos.clone())
                }
            }
        }
        positions
    }

    // ---
    // Private Methods
    // ---

    async fn init(&self) {
        // start any worker threads for account
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::number::generate_random_id;
    use crate::{
        account::trade::OrderSide,
        exchange::{api::ExchangeApi, mock::MockExchangeApi},
    };
    use tokio::test;
    use uuid::Uuid;

    #[test]
    async fn test_open_position() {
        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
        let mut account = Account::new(exchange_api.clone(), false).await;

        // Open a position
        let position = account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Long, 50000.0, None, None)
            .await
            .unwrap();

        assert_eq!(position.symbol, "BTCUSD");
        assert_eq!(position.margin_usd, 1000.0);
        assert_eq!(position.leverage, 10);
        assert_eq!(position.order_side, OrderSide::Long);

        assert_eq!(account.positions.len(), 1);
        // Close the opened position
        // let trade_tx = account.close_position(position.id, 55000.0).await.unwrap();

        // assert_eq!(trade_tx.close_price, 55000.0);

        // // Ensure the positions and trade transactions are updated accordingly
        // assert_eq!(account.trade_txs.len(), 1);
        // assert_eq!(account.trade_txs[0].id, trade_tx.id);
    }

    #[test]
    async fn test_close_position() {
        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
        let mut account = Account::new(exchange_api.clone(), false).await;

        // Open a position
        let position = account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Long, 50000.0, None, None)
            .await
            .unwrap();

        let position = position.clone();

        let trade_tx = account.close_position(position.id, 55000.0).await.unwrap();
        let trade_tx = trade_tx.clone();

        assert_eq!(trade_tx.close_price, 55000.0);
        assert_eq!(account.positions.len(), 0);
        assert_eq!(account.trade_txs.len(), 1);
        assert_eq!(account.trade_txs[0].id, trade_tx.id);
        // Close the opened position
    }

    #[test]
    async fn test_close_multiple_positions() {
        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
        let mut account = Account::new(exchange_api.clone(), false).await;

        const NUM_POSITIONS: usize = 10; // Change this to the desired number of positions for testing

        let mut positions = Vec::new();
        let mut trade_txs = Vec::new();

        // Open multiple positions
        for _ in 0..NUM_POSITIONS {
            let symbol = "BTCUSD";
            let margin_usd = rand::random::<f64>() * 1000.0;
            let leverage = rand::random::<u32>() % 10 + 1;
            let order_side = if rand::random::<bool>() {
                OrderSide::Long
            } else {
                OrderSide::Short
            };
            let open_price = rand::random::<f64>() * 50000.0;

            let position = account
                .open_position(
                    symbol, margin_usd, leverage, order_side, open_price, None, None,
                )
                .await
                .unwrap();
            positions.push(position.clone());
        }

        for pos in &positions {
            if let Some(trade_tx) = account.close_position(pos.id, pos.open_price).await {
                trade_txs.push(trade_tx.clone());
            };
        }

        let order_long: Vec<Position> = positions
            .iter()
            .filter(|e| e.order_side == OrderSide::Long)
            .map(|e| e.clone())
            .collect();
        let order_short: Vec<Position> = positions
            .iter()
            .filter(|e| e.order_side == OrderSide::Short)
            .map(|e| e.clone())
            .collect();

        let tx_long: Vec<TradeTx> = trade_txs
            .iter()
            .filter(|e| e.position.order_side == OrderSide::Long)
            .map(|e| e.clone())
            .collect();
        let tx_short: Vec<TradeTx> = trade_txs
            .iter()
            .filter(|e| e.position.order_side == OrderSide::Short)
            .map(|e| e.clone())
            .collect();

        assert_eq!(order_long.len(), tx_long.len());
        assert_eq!(order_short.len(), tx_short.len());
        assert_eq!(positions.len(), trade_txs.len());

        // Close the opened position
    }

    #[test]
    async fn test_open_positions() {
        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
        let mut account = Account::new(exchange_api.clone(), false).await;

        // Open a position
        account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Long, 50000.0, None, None)
            .await
            .unwrap();

        // Check the open positions
        let open_positions = account.open_positions().collect::<Vec<_>>();

        assert_eq!(open_positions.len(), 1);
        assert_eq!(open_positions[0].symbol, "BTCUSD");
        assert_eq!(open_positions[0].margin_usd, 1000.0);
        assert_eq!(open_positions[0].leverage, 10);
        assert_eq!(open_positions[0].order_side, OrderSide::Long);
    }

    #[test]
    async fn test_strategy_open_positions() {
        let exchange_api: Arc<Box<dyn ExchangeApi>> = Arc::new(Box::new(MockExchangeApi {}));
        let mut account = Account::new(exchange_api.clone(), false).await;

        let strategy_id_1 = generate_random_id();
        let strategy_id_2 = generate_random_id();

        // Open positions for different strategies
        let position_1 = account
            .open_position(
                "BTCUSD",
                1000.0,
                10,
                OrderSide::Long,
                50000.0,
                Some(strategy_id_1),
                None,
            )
            .await
            .unwrap();
        let position_1_id = position_1.id;

        let position_2 = account
            .open_position(
                "ETHUSD",
                500.0,
                5,
                OrderSide::Short,
                2000.0,
                Some(strategy_id_1),
                None,
            )
            .await
            .unwrap();

        let position_2_id = position_2.id;

        let position_3 = account
            .open_position(
                "BTCUSD",
                200.0,
                2,
                OrderSide::Long,
                48000.0,
                Some(strategy_id_2),
                None,
            )
            .await
            .unwrap();

        let position_3_id = position_3.id;

        // Close one position to test if it doesn't appear in the strategy_open_positions
        account
            .close_position(position_1_id, 51000.0)
            .await
            .unwrap();

        // Fetch open positions for each strategy
        let open_positions_strategy_1: Vec<PositionId> = account
            .strategy_open_positions(strategy_id_1)
            .iter()
            .map(|el| el.id)
            .collect();

        let open_positions_strategy_2: Vec<PositionId> = account
            .strategy_open_positions(strategy_id_2)
            .iter()
            .map(|el| el.id)
            .collect();

        // Assert that open positions match the expected count for each strategy
        assert_eq!(open_positions_strategy_1.len(), 1);
        assert_eq!(open_positions_strategy_2.len(), 1);

        // // Assert that the closed position is not in the open positions for strategy 1
        assert!(!open_positions_strategy_1.contains(&position_1_id));
        assert!(open_positions_strategy_1.contains(&position_2_id));
        assert!(open_positions_strategy_2.contains(&position_3_id));
    }
}
