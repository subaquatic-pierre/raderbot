use std::collections::hash_map::Values;
use std::{collections::HashMap, sync::Arc};

use log::info;
use serde::{Deserialize, Serialize};

use crate::exchange::api::ExchangeInfo;
use crate::strategy::strategy::StrategyId;
use crate::{
    account::trade::{OrderSide, Position},
    exchange::api::ExchangeApi,
    strategy::signal::SignalMessage,
};

use super::trade::{PositionId, TradeTx};

/// Represents a trading account with positions, trades, and an exchange API.
pub struct Account {
    /// A hashmap containing positions associated with their IDs.
    positions: HashMap<PositionId, Position>,
    /// A vector containing trade transactions.
    trades: Vec<TradeTx>,
    /// A thread-safe reference to the exchange API.
    exchange_api: Arc<dyn ExchangeApi>,
    /// A flag indicating whether the account is in dry run mode.
    dry_run: bool,
    position_signals: HashMap<PositionId, Vec<SignalMessage>>,
}

impl Account {
    /// Creates a new instance of `Account`.
    ///
    /// # Parameters
    ///
    /// * `exchange_api` - A thread-safe reference to the exchange API.
    /// * `init_workers` - A flag indicating whether to initialize worker threads.
    /// * `dry_run` - A flag indicating whether the account operates in dry run mode.
    ///
    /// # Returns
    ///
    /// A new instance of `Account`.

    pub async fn new(
        exchange_api: Arc<dyn ExchangeApi>,
        init_workers: bool,
        dry_run: bool,
    ) -> Self {
        let _self = Self {
            exchange_api,
            positions: HashMap::new(),
            trades: vec![],
            dry_run,
            position_signals: HashMap::new(),
        };

        if init_workers {
            _self.init().await;
        }
        _self
    }

    /// Opens a position on the exchange.
    ///
    /// # Parameters
    ///
    /// * `symbol` - The symbol of the asset.
    /// * `margin_usd` - The margin allocated for the position in USD.
    /// * `leverage` - The leverage used for the position.
    /// * `order_side` - The side of the order (Buy or Sell).
    /// * `open_price` - The price at which the position is opened.
    /// * `strategy_id` - Optional strategy ID associated with the position.
    /// * `stop_loss` - Optional stop-loss price for the position.
    ///
    /// # Returns
    ///
    /// A mutable reference to the opened position if successful, otherwise `None`.

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
            .clone()
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

    /// Closes a position on the exchange.
    ///
    /// # Parameters
    ///
    /// * `position_id` - The ID of the position to close.
    /// * `close_price` - The price at which the position is closed.
    ///
    /// # Returns
    ///
    /// A reference to the trade transaction if successful, otherwise `None`.

    pub async fn close_position(
        &mut self,
        position_id: PositionId,
        close_price: f64,
    ) -> Option<&mut TradeTx> {
        if let Some(position) = self.positions.get(&position_id).cloned() {
            if let Ok(trade_tx) = self
                .exchange_api
                .close_position(position.clone(), close_price)
                .await
            {
                self.positions.remove(&position.id);

                let trade_tx_id = trade_tx.id;

                self.trades.push(trade_tx);

                if let Some(tx) = self.trades.iter_mut().find(|e| e.id == trade_tx_id) {
                    return Some(tx);
                }
            };
        };

        None
    }

    pub fn add_position_meta(&mut self, position_id: PositionId, signal: &SignalMessage) {
        let signals = self.position_signals.entry(position_id).or_insert(vec![]);
        signals.push(signal.clone())
    }

    pub fn get_position_meta(&mut self, position_id: PositionId) -> Option<Vec<SignalMessage>> {
        self.position_signals.get(&position_id).cloned()
    }

    /// Returns an iterator over the account's positions.
    ///
    /// # Returns
    ///
    /// An iterator yielding references to positions.

    pub fn positions(&self) -> Values<'_, PositionId, Position> {
        self.positions.values()
    }

    /// Returns a clone of the list of trade transactions.
    ///
    /// # Returns
    ///
    /// A vector containing trade transactions.

    pub fn trades(&self) -> Vec<TradeTx> {
        self.trades.clone()
    }

    /// Returns positions and trades associated with a specific strategy ID.
    ///
    /// # Parameters
    ///
    /// * `strategy_id` - The ID of the strategy.
    ///
    /// # Returns
    ///
    /// A tuple containing vectors of positions and trade transactions associated with the strategy.

    pub fn strategy_positions_trades(
        &self,
        strategy_id: StrategyId,
    ) -> (Vec<Position>, Vec<TradeTx>) {
        // Get all positions associated with the strategy after
        // positions have been closed, this Vec should be empty
        let positions: Vec<Position> = self
            .strategy_positions(strategy_id)
            .iter()
            .map(|&p| p.clone())
            .collect();

        // Get all trades associated with this strategy
        // Used to calculate strategy summary
        let trades: Vec<TradeTx> = self
            .strategy_trades(strategy_id)
            .iter()
            .map(|&t| t.clone())
            .collect();

        (positions, trades)
    }

    /// Returns positions associated with a specific strategy ID.
    ///
    /// # Parameters
    ///
    /// * `strategy_id` - The ID of the strategy.
    ///
    /// # Returns
    ///
    /// A vector containing references to positions associated with the strategy.

    pub fn strategy_positions(&self, strategy_id: StrategyId) -> Vec<&Position> {
        let mut positions = vec![];
        for pos in self.positions.values() {
            if let Some(pos_strategy_id) = pos.strategy_id {
                if pos_strategy_id == strategy_id {
                    positions.push(pos)
                }
            }
        }
        positions
    }

    /// Returns trade transactions associated with a specific strategy ID.
    ///
    /// # Parameters
    ///
    /// * `strategy_id` - The ID of the strategy.
    ///
    /// # Returns
    ///
    /// A vector containing references to trade transactions associated with the strategy.

    pub fn strategy_trades(&self, _strategy_id: StrategyId) -> Vec<&TradeTx> {
        let mut trades = vec![];
        for trade in &self.trades {
            if let Some(strategy_id) = trade.position.strategy_id {
                if strategy_id == strategy_id {
                    trades.push(trade)
                }
            }
        }
        trades
    }

    /// Checks if the account is in dry run mode.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the account is in dry run mode.

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Sets the exchange API and dry run mode.
    ///
    /// # Parameters
    ///
    /// * `api` - A thread-safe reference to the exchange API.
    /// * `dry_run` - A flag indicating whether to operate in dry run mode.

    pub fn set_exchange_api(&mut self, api: Arc<dyn ExchangeApi>, dry_run: bool) {
        self.dry_run = dry_run;
        self.exchange_api = api;
    }

    /// Retrieves account information.
    ///
    /// # Returns
    ///
    /// Account information including positions, trades, and exchange API details.

    pub async fn info(&self) -> AccountInfo {
        let info = self.exchange_api.info().await.ok();
        AccountInfo {
            dry_run: self.dry_run,
            exchange_api: info,
            positions: self.positions.values().map(|el| el.clone()).collect(),
            trade_transactions: self.trades.clone(),
        }
    }

    /// Retrieves a position by its ID.
    ///
    /// # Parameters
    ///
    /// * `position_id` - The ID of the position to retrieve.
    ///
    /// # Returns
    ///
    /// A reference to the position if found, otherwise `None`.

    pub fn get_position(&self, position_id: &PositionId) -> Option<&Position> {
        self.positions.get(position_id)
    }

    // ---
    // Private Methods
    // ---

    /// Initializes worker threads for the account.
    async fn init(&self) {
        // start any worker threads for account
    }
}

#[derive(Serialize, Deserialize)]
pub struct AccountInfo {
    dry_run: bool,
    exchange_api: Option<ExchangeInfo>,
    positions: Vec<Position>,
    trade_transactions: Vec<TradeTx>,
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
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi {});
        let mut account = Account::new(exchange_api.clone(), false, true).await;

        // Open a position
        let position = account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Buy, 50000.0, None, None)
            .await
            .unwrap();

        assert_eq!(position.symbol, "BTCUSD");
        assert_eq!(position.margin_usd, 1000.0);
        assert_eq!(position.leverage, 10);
        assert_eq!(position.order_side, OrderSide::Buy);

        assert_eq!(account.positions.len(), 1);
    }

    #[test]
    async fn test_close_position() {
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi {});
        let mut account = Account::new(exchange_api.clone(), false, true).await;

        // Open a position
        let position = account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Buy, 50000.0, None, None)
            .await
            .unwrap();

        let position = position.clone();

        let trade_tx = account.close_position(position.id, 55000.0).await.unwrap();
        let trade_tx = trade_tx.clone();

        assert_eq!(trade_tx.close_price, 55000.0);
        assert_eq!(account.positions.len(), 0);
        assert_eq!(account.trades.len(), 1);
        assert_eq!(account.trades[0].id, trade_tx.id);
        // Close the opened position
    }

    #[test]
    async fn test_close_multiple_positions() {
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi {});
        let mut account = Account::new(exchange_api.clone(), false, true).await;

        const NUM_POSITIONS: usize = 10; // Change this to the desired number of positions for testing

        let mut positions = Vec::new();
        let mut trades = Vec::new();

        // Open multiple positions
        for _ in 0..NUM_POSITIONS {
            let symbol = "BTCUSD";
            let margin_usd = rand::random::<f64>() * 1000.0;
            let leverage = rand::random::<u32>() % 10 + 1;
            let order_side = if rand::random::<bool>() {
                OrderSide::Buy
            } else {
                OrderSide::Sell
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
                trades.push(trade_tx.clone());
            };
        }

        let order_long: Vec<Position> = positions
            .iter()
            .filter(|e| e.order_side == OrderSide::Buy)
            .map(|e| e.clone())
            .collect();
        let order_short: Vec<Position> = positions
            .iter()
            .filter(|e| e.order_side == OrderSide::Sell)
            .map(|e| e.clone())
            .collect();

        let tx_long: Vec<TradeTx> = trades
            .iter()
            .filter(|e| e.position.order_side == OrderSide::Buy)
            .map(|e| e.clone())
            .collect();
        let tx_short: Vec<TradeTx> = trades
            .iter()
            .filter(|e| e.position.order_side == OrderSide::Sell)
            .map(|e| e.clone())
            .collect();

        assert_eq!(order_long.len(), tx_long.len());
        assert_eq!(order_short.len(), tx_short.len());
        assert_eq!(positions.len(), trades.len());

        // Close the opened position
    }

    #[test]
    async fn test_open_positions() {
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi {});
        let mut account = Account::new(exchange_api.clone(), false, true).await;

        // Open a position
        account
            .open_position("BTCUSD", 1000.0, 10, OrderSide::Buy, 50000.0, None, None)
            .await
            .unwrap();

        // Check the open positions
        let positions = account.positions().collect::<Vec<_>>();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "BTCUSD");
        assert_eq!(positions[0].margin_usd, 1000.0);
        assert_eq!(positions[0].leverage, 10);
        assert_eq!(positions[0].order_side, OrderSide::Buy);
    }

    #[test]
    async fn test_strategy_open_positions() {
        let exchange_api: Arc<dyn ExchangeApi> = Arc::new(MockExchangeApi {});
        let mut account = Account::new(exchange_api.clone(), false, true).await;

        let strategy_id_1 = Uuid::new_v4();
        let strategy_id_2 = Uuid::new_v4();

        // Open positions for different strategies
        let position_1 = account
            .open_position(
                "BTCUSD",
                1000.0,
                10,
                OrderSide::Buy,
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
                OrderSide::Sell,
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
                OrderSide::Buy,
                48000.0,
                Some(strategy_id_2),
                None,
            )
            .await
            .unwrap();

        let position_3_id = position_3.id;

        // Close one position to test if it doesn't appear in the strategy_positions
        account
            .close_position(position_1_id, 51000.0)
            .await
            .unwrap();

        // Fetch open positions for each strategy
        let open_positions_strategy_1: Vec<PositionId> = account
            .strategy_positions(strategy_id_1)
            .iter()
            .map(|el| el.id)
            .collect();

        let open_positions_strategy_2: Vec<PositionId> = account
            .strategy_positions(strategy_id_2)
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
