use std::collections::HashMap;
use std::stream::Stream;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use bitflags::_core::pin::Pin;
use bitflags::_core::task::{Context, Poll};
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use lazy_static::lazy_static;
use merovingian::candles::Candles;
use merovingian::candles_builder::CandleAppender;
use merovingian::order::{Order, OrderId};
use mock_exchange::MockExchange;
use mouse::error::Result;
use mouse::num::dec;
use nebuchadnezzar_core::chrono::{DateTime, Utc};
use nebuchadnezzar_core::client::NotClient;
use nebuchadnezzar_core::error::NebError;
use nebuchadnezzar_core::websocket::{NotWebSocket, WebSocket};
use nebuchadnezzar_core::Exchange;
use residual_self_image::backtest_report::BacktestReport;
use rust_decimal::prelude::{One, Zero};
use rust_decimal::Decimal;
use sorted_vec::ReverseSortedVec;
use tokio::sync::Mutex;

use crate::agents::network_agent::{NetworkAgentState, NetworkClient, Ws};
use crate::agents::network_agents::{Execution, FundingExecution, InstrumentConfig, NetworkAgent};
use crate::agents::trade_guard::{ModelState, TradeGuard};

mod mock_exchange;

lazy_static! {
    pub static ref INCEPTION_TIMESTAMP_S: u32 = 0;
}

pub struct MockNetworkAgent {
    state: NetworkAgentState<MockClient, MockWebsocket>,
    report: Option<BacktestReport>,
    markets: Vec<String>,
}

pub struct MockClient {
    exchange: Arc<Mutex<MockExchange>>,
}

pub struct MockNebExchange;

#[async_trait]
impl Exchange for MockNebExchange {
    type Client = NotClient<Self>;
    type WebSocket = NotWebSocket<Self>;

    fn name(&self) -> &'static str {
        "Mock Exchange"
    }

    fn api_version(&self) -> &'static str {
        todo!()
    }

    fn site_url(&self) -> &'static str {
        todo!()
    }

    fn api_url(&self) -> &'static str {
        todo!()
    }

    fn ws_api_url(&self) -> &'static str {
        todo!()
    }

    fn api_doc_url(&self) -> &'static str {
        todo!()
    }

    fn is_demo(&self) -> bool {
        todo!()
    }

    fn new_client(&self) -> Self::Client {
        todo!()
    }

    async fn new_web_socket(
        &self,
    ) -> Result<Self::WebSocket, <Self::WebSocket as WebSocket>::Error> {
        todo!()
    }
}

#[async_trait]
impl NetworkClient for MockClient {
    type Exchange = MockNebExchange;

    async fn fetch_candles(
        &self,
        market: &str,
        timeframe: u32,
        start: u32,
        end: u32,
        candles: &mut Candles,
    ) -> Result<()> {
        let mut builder = CandleAppender::new(timeframe);
        let lock = self.exchange.lock().await;
        let count = (end - start) / timeframe;
        for i in 0..count {
            let candle = lock
                .candles()
                .get(market)
                .unwrap()
                .get(&timeframe)
                .unwrap()
                .get_candle(i as usize)
                .unwrap();
            builder.tick(candle.timestamp - 1, candle.open, 0.);
            builder.tick(candle.timestamp - 1, candle.high, 0.);
            builder.tick(candle.timestamp - 1, candle.low, 0.);
            builder.tick(candle.timestamp - 1, candle.close, candle.volume);
        }
        // Building last candle
        // builder.tick(lock.candles().timestamp[count as usize] - 1, 0., 0.);
        for candle in builder.candles() {
            candles.push_candle(candle);
        }
        Ok(())
    }

    async fn post_orders(&self, orders: &Vec<Order>) -> Result<()> {
        self.exchange.lock().await.post_orders(orders);
        Ok(())
    }

    async fn cancel_orders(&self, orders_to_cancel: &Vec<OrderId>) -> Result<()> {
        self.exchange.lock().await.cancel_orders(orders_to_cancel);
        Ok(())
    }

    fn exchange(&self) -> Self::Exchange {
        MockNebExchange {}
    }

    async fn kill(&self) -> Result<()> {
        unimplemented!()
    }
}

// on_order_fill
// on_public_trade

pub struct MockWebsocket {}
impl Stream for MockWebsocket {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unimplemented!()
    }
}
#[async_trait]
impl Ws for MockWebsocket {
    type Message = ();

    async fn next(&mut self) -> Self::Message {}

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl NetworkAgent for MockNetworkAgent {
    type Websocket = MockWebsocket;
    type Client = MockClient;

    async fn build(
        _exchange_config: ExchangeConfig,
        _model_configs: Vec<ModelConfig>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    async fn catch_up(
        &mut self,
        _last_execution_time: DateTime<Utc>,
    ) -> Result<Vec<Result<Execution, FundingExecution>>> {
        todo!()
    }

    fn new_client(use_testnet: bool, api_key: &str, api_secret: &str) -> Self::Client {
        unimplemented!()
    }

    async fn new_subscribed_web_socket(&mut self) -> Result<Self::Websocket> {
        unimplemented!()
    }

    async fn handle_message(&mut self, _msg: ()) -> Result<()> {
        unimplemented!()
    }

    fn state_mut(&mut self) -> &mut NetworkAgentState<Self::Client, Self::Websocket> {
        unimplemented!()
    }
}

impl MockNetworkAgent {
    pub async fn new(
        config: ExchangeConfig,
        model_configs: Vec<ModelConfig>,
    ) -> Result<MockNetworkAgent> {
        #[cfg(feature = "test")]
        println!("'test' feature enabled");
        #[cfg(feature = "assert")]
        println!("'assert' feature enabled");
        let client = MockClient {
            exchange: Arc::new(Mutex::new(
                MockExchange::new(&config, &model_configs).await?,
            )),
        };

        let mut markets = Vec::new();
        for (market, _) in client.exchange.lock().await.candles() {
            markets.push(market.clone());
        }

        let mut min_timeframe = u32::MAX;
        for model_config in &model_configs {
            let timeframe = model_config.variable_values()[0] as u32;
            if timeframe < min_timeframe {
                min_timeframe = timeframe;
            }
        }

        // TODO: make this dynamic
        let instrument_config = InstrumentConfig {
            base_currency: "XBT".to_string(),
            quote_currency: "USD".to_string(),
            tick_size: dec!(0.5),
            lot_size: dec!(0.00000001),
            multiplier: 0.0,
            // MUST be false if we are comparing with construct model
            is_inverse: false,
            taker_fee: dec!(0.00075),
            maker_fee: Decimal::zero(),
            funding_period: 0,
        };
        let mut active_instruments = HashMap::new();
        active_instruments.insert("XBTUSD".into(), instrument_config);
        let instruments;
        {
            let mut exchange = client.exchange.lock().await;
            exchange.init(&active_instruments);
            instruments = exchange.get_instruments();
        }
        let supported_timeframes = ReverseSortedVec::from_unsorted(vec![min_timeframe]);
        let agent = MockNetworkAgent {
            state: NetworkAgentState::new(
                config,
                model_configs,
                client,
                MockWebsocket {},
                active_instruments,
                instruments,
                Decimal::one(),
                Decimal::zero(),
                &supported_timeframes,
                usize::MAX,
            )
            .await?,
            report: None,
            markets,
        };
        Ok(agent)
    }

    pub async fn test(&mut self) -> Result<ModelState> {
        let _instant = Instant::now();
        let exchange = self.state.client().exchange.clone();
        'main: loop {
            for market in &self.markets {
                let mut exchange_guard = exchange.lock().await;
                let (t1, t2, t3, t4) = match exchange_guard.get_next_trades(market) {
                    None => break 'main,
                    Some(trades) => (
                        trades[0].clone(),
                        trades[1].clone(),
                        trades[2].clone(),
                        trades[3].clone(),
                    ),
                };
                // First we pay funding because we open/close positions after rounded time.
                while let Some(funding) = exchange_guard.get_funding_executions().pop() {
                    self.state.on_funding_execution(funding).await?;
                }
                while let Some(execution) = exchange_guard.get_executions().pop() {
                    self.state.on_execution(execution).await?;
                }
                drop(exchange_guard);
                #[cfg(not(feature = "assert"))]
                self.state
                    .on_margin_changed(exchange.lock().await.get_margin())
                    .await?;
                self.state.on_public_trade(t1, market.clone()).await?;
                self.state.on_public_trade(t2, market.clone()).await?;
                self.state.on_public_trade(t3, market.clone()).await?;
                self.state.on_public_trade(t4, market.clone()).await?;
            }
        }
        self.state.on_shutdown().await?;
        let model_state = self
            .state
            .listeners
            .iter()
            .next()
            .unwrap()
            .downcast_ref::<TradeGuard>()
            .unwrap()
            .trading_agent
            .models
            .first()
            .unwrap()
            .state
            .clone();
        Ok(model_state)
    }
}
