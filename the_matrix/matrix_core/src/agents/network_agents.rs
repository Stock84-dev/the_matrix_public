mod bitmex_agent;

#[cfg(feature = "test")]
pub mod mock_network_agent;

use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
pub use bitmex_agent::*;
use chrono::{DateTime, Duration, Utc};
use downcast_rs::Downcast;
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use merovingian::candles::*;
use merovingian::minable_models::*;
use merovingian::order::{Order, OrderId};
use mouse::error::Result;
use mouse::log::*;
use mouse::num::traits::Zero;
use mouse::num::Decimal;
use num_traits::ToPrimitive;
use serde_json;
use tokio::sync::Mutex;

use super::network_agent::{NetworkAgentState, Ws};
use crate::agents::network_agent::NetworkClient;

extern crate multiqueue;

// If we get slow performance use tokio channels and wrap messages in Arc<Enum>
#[async_trait]
pub trait ExchangeListener: Downcast + Send + Sync + 'static {
    // Public
    // ---------------------------------------------------------------------------------------------
    async fn on_announcement(&mut self, _announcement: &Announcement) -> Result<()> {
        Ok(())
    }
    async fn on_public_trade(&mut self, _trade: &Trade, _symbol: &String) -> Result<()> {
        Ok(())
    }
    async fn on_chat_message(&mut self, _chat_message: &ChatMessage) -> Result<()> {
        Ok(())
    }
    async fn on_connected_users_changed(&mut self, _connected: &Connected) -> Result<()> {
        Ok(())
    }
    async fn on_funding(&mut self, _funding: &Funding, _symbol: &String) -> Result<()> {
        Ok(())
    }
    async fn on_instrument_changed(
        &mut self,
        _instrument: &Instrument,
        _symbol: &String,
        _config: &Option<&InstrumentConfig>,
    ) -> Result<()> {
        Ok(())
    }
    async fn on_insurance_updated(
        &mut self,
        _insurance: &Insurance,
        _symbol: &String,
    ) -> Result<()> {
        Ok(())
    }
    async fn on_public_liquidation(
        &mut self,
        _public_liquidation: &PublicLiquidation,
        _symbol: &String,
    ) -> Result<()> {
        Ok(())
    }
    async fn on_order_book_updated(
        &mut self,
        _order_book_update: &OrderBookUpdate,
        _symbol: &String,
    ) -> Result<()> {
        Ok(())
    }

    // Private
    // ---------------------------------------------------------------------------------------------
    async fn on_execution<'a>(
        &'a mut self,
        _execution: &'a Execution,
        _instruments: &'a HashMap<String, InstrumentConfig>,
    ) -> Result<()> {
        Ok(())
    }
    async fn on_funding_execution<'a>(
        &'a mut self,
        _execution: &'a FundingExecution,
        _instruments: &'a HashMap<String, InstrumentConfig>,
    ) -> Result<()> {
        Ok(())
    }
    async fn on_margin_changed(&mut self, _margin: &Margin) -> Result<()> {
        Ok(())
    }
    async fn on_position_changed(&mut self, _position: &Position) -> Result<()> {
        Ok(())
    }
    /// Gets called when orders are processed by exchange before they are executed.
    async fn on_orders_placed(&mut self, _orders: &Vec<Order>) -> Result<()> {
        Ok(())
    }
    /// Gets called each minute, last candle may be incomplete or already contains new candle.
    async fn on_new_candle<'a>(
        &'a mut self,
        _candles: &'a HashMap<String, HashMap<u32, Candles>>,
        _last_timestamp_s: u32,
        _active_instruments: &'a HashMap<String, InstrumentConfig>,
        _orders_to_open: &Arc<Mutex<Vec<Order>>>,
        _orders_to_cancel: &Arc<Mutex<Vec<OrderId>>>,
    ) -> Result<()> {
        Ok(())
    }

    // Utils
    // ---------------------------------------------------------------------------------------------
    fn required_candles(&self, _req: &mut HashMap<String, HashMap<u32, usize>>) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
    fn is_in_position(&self) -> bool {
        false
    }

    // Matrix Specific
    async fn on_maintenance(&mut self, _maintenance: &Maintenance) -> Result<()> {
        Ok(())
    }
}

impl_downcast!(ExchangeListener);

#[async_trait]
pub trait NetworkAgent: Send {
    type Websocket: Ws + Sync;
    type Client: NetworkClient;

    async fn build(
        exchange_config: ExchangeConfig,
        model_configs: Vec<ModelConfig>,
    ) -> Result<Self>
    where
        Self: Sized;
    async fn catch_up(
        &mut self,
        last_execution_time: DateTime<Utc>,
    ) -> Result<Vec<Result<Execution, FundingExecution>>>;
    fn new_client(use_testnet: bool, api_key: &str, api_secret: &str) -> Self::Client;
    async fn new_subscribed_web_socket(&mut self) -> Result<Self::Websocket>;
    async fn handle_message(
        &mut self,
        msg: <<Self as NetworkAgent>::Websocket as Ws>::Message,
    ) -> Result<()>;
    fn state_mut(&mut self) -> &mut NetworkAgentState<Self::Client, Self::Websocket>;
}

pub fn get_and_notify_time_difference(
    msg_timestamp: DateTime<Utc>,
    sent_timestamp: Instant,
    exchange_id: &str,
) -> Duration {
    let time_offset = (chrono::offset::Utc::now() - msg_timestamp).add(Duration::nanoseconds(
        sent_timestamp.elapsed().as_nanos() as i64,
    ));
    if time_offset.num_milliseconds() > 1000 {
        warn!(
            "{}: Our clock is {}ms ahead.",
            exchange_id,
            time_offset.num_milliseconds()
        );
    } else if time_offset.num_milliseconds() < -1000 {
        warn!(
            "{}: Our clock is {}ms behind.",
            exchange_id,
            -time_offset.num_milliseconds()
        );
    }
    time_offset
}

#[derive(Clone, Debug, Default)]
pub struct InstrumentConfig {
    pub base_currency: String,
    pub quote_currency: String,
    pub tick_size: Decimal,
    /// The the minimal amount that can be bought/sold.
    pub lot_size: Decimal,
    pub multiplier: f32,
    pub is_inverse: bool,
    pub taker_fee: Decimal,
    pub maker_fee: Decimal,
    pub funding_period: u32,
}

fn option_to_f32<T: ToPrimitive>(option: &Option<T>) -> f32 {
    match option {
        None => f32::NAN,
        Some(s) => s.to_f32().unwrap(),
    }
}

#[derive(Debug)]
pub struct Execution {
    pub market: String,
    pub order_id: OrderId,
    pub value: Decimal,
    pub amount: Decimal,
    pub amount_left: Decimal,
    pub fee_paid: Decimal,
    pub executed_price: Decimal,
    pub timestamp_ns: u64,
}

pub struct FundingExecution {
    pub market: String,
    pub fee_paid: Decimal,
    pub timestamp_ns: u64,
}

pub trait MyFrom<T>: Sized {
    fn ifrom(_: T) -> Self;
}

pub trait MyInto<T>: Sized {
    fn iinto(self) -> T;
}

impl<T, U> MyInto<U> for T
where
    U: MyFrom<T>,
{
    fn iinto(self) -> U {
        U::ifrom(self)
    }
}
