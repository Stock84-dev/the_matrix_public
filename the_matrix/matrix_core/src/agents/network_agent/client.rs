use async_trait::async_trait;
use merovingian::candles::Candles;
use merovingian::order::{Order, OrderId};
use mouse::error::Result;
use nebuchadnezzar_core::Exchange;

#[async_trait]
pub trait NetworkClient: Send + Sync {
    type Exchange: Exchange + Send + Sync;

    fn exchange(&self) -> Self::Exchange;
    /// Fetches candles between start and including end timestamp, implementors also need to auto
    /// paginate and rate limit.
    async fn fetch_candles(
        &self,
        market: &str,
        timeframe: u32,
        start: u32,
        end: u32,
        candles: &mut Candles,
    ) -> Result<()>;

    async fn post_orders(&self, orders: &Vec<Order>) -> Result<()>;
    async fn cancel_orders(&self, orders: &Vec<OrderId>) -> Result<()>;
    /// Closes all positions and cancels all orders.
    async fn kill(&self) -> Result<()>;
}
