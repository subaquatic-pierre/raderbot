use crate::account::trade::{OrderSide, Position, TradeTx};
use crate::exchange::api::ExchangeApi;
use crate::exchange::stream::StreamManager;
use crate::exchange::types::{ApiResult, StreamType};
use crate::market::kline::Kline;
use crate::market::ticker::Ticker;
use crate::market::types::ArcMutex;
use crate::utils::time::generate_ts;
use async_trait::async_trait;
use serde_json::Value;

pub struct MockExchangeApi {}

#[async_trait]
impl ExchangeApi for MockExchangeApi {
    async fn open_position(
        &self,
        symbol: &str,
        margin_usd: f64,
        leverage: u32,
        order_side: OrderSide,
        open_price: f64,
    ) -> ApiResult<Position> {
        let position = Position::new(symbol, open_price, order_side, margin_usd, leverage, None);
        Ok(position)
    }
    async fn close_position(&self, position: Position, close_price: f64) -> ApiResult<TradeTx> {
        let trade_tx = TradeTx::new(close_price, generate_ts(), position);
        Ok(trade_tx)
    }

    // ---
    // All Other methods not used on this mock MockExchangeApi
    // Will fail if called
    // ---
    async fn get_account(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn get_account_balance(&self) -> ApiResult<f64> {
        unimplemented!()
    }
    async fn all_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    async fn list_open_orders(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    fn get_stream_manager(&self) -> ArcMutex<Box<dyn StreamManager>> {
        unimplemented!()
    }
    async fn get_kline(&self, _symbol: &str, _interval: &str) -> ApiResult<Kline> {
        unimplemented!()
    }
    async fn get_ticker(&self, _symbol: &str) -> ApiResult<Ticker> {
        unimplemented!()
    }

    async fn exchange_info(&self) -> ApiResult<Value> {
        unimplemented!()
    }
    fn build_stream_url(
        &self,
        _symbol: &str,
        _stream_type: StreamType,
        _interval: Option<&str>,
    ) -> String {
        todo!()
    }
}

impl Default for MockExchangeApi {
    fn default() -> Self {
        Self {}
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::account::trade::OrderSide;
    use crate::utils::time::generate_ts;
    use tokio::test;

    #[test]
    async fn test_mock_open_position() {
        let api = MockExchangeApi::default();
        let symbol = "BTCUSD";
        let margin_usd = 1000.0;
        let leverage = 10;
        let order_side = OrderSide::Long;
        let open_price = 50000.0;

        let result = api
            .open_position(symbol, margin_usd, leverage, order_side, open_price)
            .await;

        assert!(result.is_ok());
        let position = result.unwrap();

        assert_eq!(position.symbol, symbol);
        assert_eq!(position.margin_usd, margin_usd);
        assert_eq!(position.leverage, leverage);
        assert_eq!(position.order_side, order_side);
        assert_eq!(position.open_price, open_price);
    }

    #[test]
    async fn test_mock_close_position() {
        let api = MockExchangeApi::default();
        let symbol = "BTCUSD";
        let margin_usd = 1000.0;
        let leverage = 10;
        let order_side = OrderSide::Long;
        let open_price = 50000.0;

        let position = Position::new(symbol, open_price, order_side, margin_usd, leverage, None);

        let close_price = 55000.0;

        let result = api.close_position(position.clone(), close_price).await;

        assert!(result.is_ok());
        let trade_tx = result.unwrap();

        assert_eq!(trade_tx.close_price, close_price);
        assert_eq!(trade_tx.position.id, position.id);
    }
}
