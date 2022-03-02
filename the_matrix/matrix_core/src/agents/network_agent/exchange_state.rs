use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::sync::Arc;

use chrono::Utc;
use config::{get_exchange_config, CONFIG};
use futures::FutureExt;
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use iaas::mysql::{load_exchange_config, load_last_maintenance};
use merovingian::candles::Candles;
use merovingian::candles_builder::CandlesBuilder;
use merovingian::minable_models::*;
use merovingian::order;
use merovingian::order::{Order, OrderId};
use mouse::error::*;
use mouse::log::*;
use mouse::num::traits::Zero;
use mouse::num::{Decimal, IntoDecimal, NumExt};
use mouse::time::Timestamp;
use nebuchadnezzar_core::Exchange;
use sorted_vec::ReverseSortedVec;
use tokio::fs::{create_dir_all, metadata, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use zion::{Command, Zion};

use super::client::NetworkClient;
use super::websocket::Ws;
use crate::agents::data_agents::{DbAgent, ExchangeDataAgent};
use crate::agents::network_agents::{
    ExchangeListener, Execution, FundingExecution, InstrumentConfig,
};
use crate::agents::trade_guard::TradeGuard;
use crate::error::MatrixError;
use crate::event::Listeners;

enum MaintenanceState {
    Normal,
    SafeReload,
    SafeShutdown,
}

pub struct NetworkAgentState<C: NetworkClient, WS: Ws> {
    #[cfg(not(feature = "test"))]
    zion: Arc<Mutex<Zion>>,
    #[cfg(feature = "test")]
    pub listeners: Listeners<dyn ExchangeListener>,
    #[cfg(not(feature = "test"))]
    listeners: Listeners<dyn ExchangeListener>,
    client: C,
    pub(super) ws: Option<WS>,
    candles_builder: CandlesBuilder,
    active_instruments: HashMap<String, InstrumentConfig>,
    /// Market orders that have been sent to the exchange but haven't been executed.
    // executing_market_orders: Arc<Mutex<HashMap<Uuid, Vec<ExecutingMarketOrder>>>>,
    open_orders: HashMap<OrderId, OpenedOrder>,
    maintenance_state: MaintenanceState,
    tmp_sub_orders_to_open: Arc<Mutex<Vec<Order>>>,
    tmp_sub_orders_to_cancel: Arc<Mutex<Vec<OrderId>>>,
    tmp_orders_to_open: Vec<Order>,
}

impl<C: NetworkClient, WS: Ws> NetworkAgentState<C, WS> {
    pub async fn new(
        config: ExchangeConfig,
        model_configs: Vec<ModelConfig>,
        client: C,
        ws: WS,
        instrument_configs: HashMap<String, InstrumentConfig>,
        instruments: Vec<(String, Instrument)>,
        balance: Decimal,
        leverage: Decimal,
        supported_timeframes: &ReverseSortedVec<u32>,
        max_candles_fetched_at_once: usize,
    ) -> Result<NetworkAgentState<C, WS>> {
        #[cfg(feature = "test")]
        println!("test feature enabled");
        #[cfg(feature = "assert")]
        println!("assert feature enabled");
        let mut listeners = Listeners::<dyn ExchangeListener>::new();
        listeners.push(Box::new(TradeGuard::new(
            model_configs,
            config.max_leverage.to_decimal().unwrap(),
            config.max_orders_per_m,
            config.id,
            client.exchange().name().to_string(),
            leverage,
            balance,
            &instrument_configs,
        )?));
        push_non_essential_listeners(
            &mut listeners,
            config.use_public_data_miner,
            client.exchange().name(),
            config.id,
        );
        let mut state = NetworkAgentState {
            #[cfg(not(feature = "test"))]
            zion: Zion::new(),
            listeners,
            client,
            ws: Some(ws),
            candles_builder: CandlesBuilder::new(),
            active_instruments: instrument_configs,
            open_orders: Default::default(),
            tmp_sub_orders_to_cancel: Arc::new(Mutex::new(Vec::new())),
            tmp_sub_orders_to_open: Arc::new(Default::default()),
            tmp_orders_to_open: vec![],
            maintenance_state: MaintenanceState::Normal,
        };
        state
            .init(
                instruments,
                supported_timeframes,
                max_candles_fetched_at_once,
            )
            .await?;
        Ok(state)
    }

    async fn init(
        &mut self,
        instruments: Vec<(String, Instrument)>,
        supported_timeframes: &ReverseSortedVec<u32>,
        max_candles_fetched_at_once: usize,
    ) -> Result<()> {
        let exchange_path = CONFIG.data_dir.join(self.client.exchange().name());
        if !metadata(&exchange_path).await.is_ok() {
            create_dir_all(&exchange_path).await?;
        }
        // There could be network delays where we would send orders but instruments haven't been
        // initialized. It actually happened.
        for (market, instrument) in instruments {
            self.on_instrument_changed(instrument, &market).await?;
        }

        self.load_required_candles(supported_timeframes, max_candles_fetched_at_once)
            .await?;

        // In order for test to be valid we need to call all models on first candle.
        // The reason why we don't do this live is if we have long timeframe then we would be very
        // late to open a trade.
        #[cfg(feature = "test")]
        {
            let mut timestamps = HashSet::new();
            for (_market, timeframe_map) in self.candles_builder.candles() {
                for (_timeframe, candles) in timeframe_map {
                    timestamps.insert(candles.timestamp[candles.len() - 1]);
                }
            }

            for timestamp in timestamps {
                self.on_new_candle(timestamp).await?;
            }
        }

        Ok(())
    }

    pub fn active_instruments(&self) -> &HashMap<String, InstrumentConfig> {
        &self.active_instruments
    }

    pub fn client(&self) -> &C {
        &self.client
    }

    pub async fn on_funding_execution(
        &mut self,
        funding_execution: FundingExecution,
    ) -> Result<()> {
        let instruments = &self.active_instruments;
        self.listeners
            .broadcast_async(|x| x.on_funding_execution(&funding_execution, &instruments))
            .await?;
        // Doing sequentially to ensure if error happens we can still process orders on next boot
        save_execution_time(
            self.client.exchange().name(),
            funding_execution.timestamp_ns,
        )
        .await?;
        Ok(())
    }

    pub async fn on_execution(&mut self, execution: Execution) -> Result<()> {
        let partial_execution = match self.open_orders.get_mut(&execution.order_id) {
            Some(partial_execution) => partial_execution,
            None => {
                error!("Unknown order executed, ignoring it.");
                return Ok(());
            }
        };
        partial_execution.amount += execution.amount;
        partial_execution.value += execution.value;
        partial_execution.fee_paid += execution.fee_paid;
        if partial_execution.amount == partial_execution.max_amount {
            self.on_order_filled(&execution).await?;
        }
        Ok(())
    }

    /// Calls 'on_instrument_changed'.
    pub async fn on_new_instrument(
        &mut self,
        config: InstrumentConfig,
        instrument: Instrument,
        market: &String,
    ) -> Result<()> {
        // Order matters
        self.active_instruments.insert(market.clone(), config);
        self.on_instrument_changed(instrument, &market).await
    }

    pub async fn on_instrument_changed(
        &mut self,
        instrument: Instrument,
        market: &String,
    ) -> Result<()> {
        let config = self.active_instruments.get(market);
        broadcast_async!(self, on_instrument_changed, instrument, market, config);
        Ok(())
    }

    pub(super) async fn catch_up(
        &mut self,
        mut executions: Vec<Result<Execution, FundingExecution>>,
    ) -> Result<()> {
        executions.sort_by_key(|x| match x {
            Ok(e) => e.timestamp_ns,
            Err(f) => f.timestamp_ns,
        });
        // First we process orders because there could be known executions
        for execution in executions {
            match execution {
                Ok(execution) => self.on_execution(execution).await?,
                Err(funding) => self.on_funding_execution(funding).await?,
            }
        }
        if get_exchange_config().is_none() {
            if let Some(Maintenance {
                mode: MaintenanceMode::Crash,
                ..
            }) = load_last_maintenance(self.trade_guard_mut().exchange_id()).unwrap_or(None)
            {
                warn!("Recovering from system failure.");
                self.trade_guard_mut().reset()?;
            }
        } else {
            warn!("Skipping loading from database because exchange config found.");
        }
        self.handle_maintenance(MaintenanceMode::Boot).await?;
        Ok(())
    }

    pub(super) async fn reconnect(&mut self) -> Result<()> {
        warn!("Reconnecting");
        // TODO: fetch candles again
        self.handle_maintenance(MaintenanceMode::Reconnect).await?;
        Ok(())
    }
}

impl<C: NetworkClient, WS: Ws> NetworkAgentState<C, WS> {
    pub async fn on_announcement(&mut self, announcement: Announcement) -> Result<()> {
        broadcast_async!(self, on_announcement, announcement);
        Ok(())
    }
    pub async fn on_public_trade(&mut self, trade: Trade, symbol: String) -> Result<()> {
        broadcast_async!(self, on_public_trade, trade, symbol);
        let completed_timestamp_s = self.candles_builder.tick(
            &symbol,
            ((trade.timestamp_ns - 1) / 1_000_000_000) as u32,
            trade.price,
            trade.amount.abs(),
        );
        if completed_timestamp_s != 0 {
            self.on_new_candle(completed_timestamp_s).await?;
        }
        Ok(())
    }
    pub async fn on_chat_message(&mut self, chat_message: ChatMessage) -> Result<()> {
        broadcast_async!(self, on_chat_message, chat_message);
        Ok(())
    }
    pub async fn on_connected_users_changed(&mut self, connected: Connected) -> Result<()> {
        broadcast_async!(self, on_connected_users_changed, connected);
        Ok(())
    }
    pub async fn on_funding(&mut self, funding: Funding, symbol: String) -> Result<()> {
        broadcast_async!(self, on_funding, funding, symbol);
        Ok(())
    }
    pub async fn on_insurance_changed(
        &mut self,
        insurance: Insurance,
        market: String,
    ) -> Result<()> {
        broadcast_async!(self, on_insurance_updated, insurance, market);
        Ok(())
    }
    pub async fn on_public_liquidation(
        &mut self,
        public_liquidation: PublicLiquidation,
        symbol: String,
    ) -> Result<()> {
        broadcast_async!(self, on_public_liquidation, public_liquidation, symbol);
        Ok(())
    }
    pub async fn on_order_book_updated(
        &mut self,
        order_book_update: OrderBookUpdate,
        symbol: String,
    ) -> Result<()> {
        broadcast_async!(self, on_order_book_updated, order_book_update, symbol);
        Ok(())
    }
    pub async fn on_margin_changed(&mut self, margin: Margin) -> Result<()> {
        broadcast_async!(self, on_margin_changed, margin);
        Ok(())
    }
    pub async fn on_position_changed(&mut self, position: Position) -> Result<()> {
        broadcast_async!(self, on_position_changed, position);
        Ok(())
    }
    pub async fn on_shutdown(&mut self) -> Result<()> {
        broadcast_async!(self, on_shutdown);
        Ok(())
    }
}

impl<C: NetworkClient, WS: Ws> NetworkAgentState<C, WS> {
    pub(super) async fn tick_candles_on_all_markets(&mut self, timestamp_s: u32) -> Result<()> {
        let completed_timestamp_s = self.candles_builder.tick_empty_all_markets(timestamp_s);
        if completed_timestamp_s != 0 {
            self.on_new_candle(completed_timestamp_s).await?;
        }
        Ok(())
    }

    pub(super) fn min_timeframe(&self) -> u32 {
        self.candles_builder.min_timeframe()
    }

    #[cfg(not(feature = "test"))]
    pub(super) async fn check_for_zion_message(&mut self) -> Result<()> {
        let mut zion = self.zion.lock().await;
        if let Some(command) = zion.try_next().await? {
            drop(zion);
            match command {
                Command::Maintenance(maintenance) => {
                    info!("Maintenance received {:?}", maintenance);
                    if let MaintenanceMode::Crash = maintenance {
                        self.on_shutdown().await?;
                        panic!("Received crash signal.")
                    }
                    self.handle_maintenance(maintenance).await?;
                    self.maybe_go_under_maintenance().await?;
                }
            }
        }
        Ok(())
    }

    pub(super) async fn kill(&mut self) -> Result<()> {
        self.client.kill().await?;
        self.handle_maintenance(MaintenanceMode::Crash).await?;
        Ok(())
    }
}

impl<C: NetworkClient, WS: Ws> NetworkAgentState<C, WS> {
    fn trade_guard_mut(&mut self) -> &mut TradeGuard {
        self.listeners
            .iter_mut()
            .next()
            .unwrap()
            .downcast_mut()
            .unwrap()
    }

    async fn on_new_candle(&mut self, completed_candle_timestamp_s: u32) -> Result<()> {
        let candles = self.candles_builder.candles();
        let instruments = &self.active_instruments;
        let tmp_sub_orders_to_open = &self.tmp_sub_orders_to_open;
        let tmp_sub_orders_to_cancel = &self.tmp_sub_orders_to_cancel;

        self.listeners
            .broadcast_async(|x| {
                x.on_new_candle(
                    candles,
                    completed_candle_timestamp_s,
                    instruments,
                    tmp_sub_orders_to_open,
                    tmp_sub_orders_to_cancel,
                )
            })
            .await?;
        self.open_orders().await?;
        Ok(())
    }

    async fn load_required_candles(
        &mut self,
        supported_timeframes: &ReverseSortedVec<u32>,
        max_candles_fetched_at_once: usize,
    ) -> Result<()> {
        // TODO: if n_requests > 60 panic because of ratelimit
        let mut req = HashMap::<String, HashMap<u32, usize>>::new();
        self.listeners
            .broadcast_result(|x| x.required_candles(&mut req))?;
        let mut total_fetches = 0usize;

        for supported_timeframe in supported_timeframes.iter() {
            for (market, map) in &mut req {
                // Get maximum candles count for specific market and timeframe that can be used to
                // construct other timeframes.
                let mut max_len = 0;
                let mut max_ratio = 1;
                for (timeframe, len) in &*map {
                    if *timeframe < 60 {
                        unimplemented!("Subminute timeframes aren't implemented.");
                    }
                    if timeframe % supported_timeframe != 0 {
                        continue;
                    }
                    let ratio = *timeframe as usize / (*supported_timeframe) as usize;
                    let count = ratio * len;
                    max_len = max_len.max(count);
                    max_ratio.max_mut(ratio);
                }
                if max_len == 0 {
                    continue;
                }
                // For partial candle
                max_len += 1 * max_ratio;
                // Fetching candles.
                let mut candles = Candles::with_capacity(
                    self.client.exchange().name().into(),
                    market.clone(),
                    max_len,
                );
                let now = Utc::now().timestamp_s();
                let end = now - now % supported_timeframe + supported_timeframe;
                let period = max_len as u32 * supported_timeframe;
                let mut current = end - period;
                while current < end {
                    let mut count = ((end - current) / supported_timeframe) as usize;
                    count = count.min(max_candles_fetched_at_once);
                    // TODO: load from cache.
                    trace!(
                        "Fetching {} candles {};{};{}",
                        count,
                        self.client.exchange().name(),
                        market,
                        supported_timeframe
                    );
                    self.client
                        .fetch_candles(&market, *supported_timeframe, current, end, &mut candles)
                        .await?;
                    current += count as u32 * supported_timeframe;
                    total_fetches += 1;
                }
                // Fixing candles.
                candles.fix_integrity(*supported_timeframe);
                // Applying candles.
                assert_eq!(candles.len(), max_len);
                for (timeframe, len) in &*map {
                    if timeframe % supported_timeframe != 0 {
                        continue;
                    }
                    let ratio = *timeframe as usize / (*supported_timeframe) as usize;
                    if ratio != 1 {
                        let mut constructed_candles = Candles::with_default_value(
                            self.client.exchange().name().into(),
                            market.clone(),
                            *len + 1,
                        );
                        let count =
                            candles.increase_timeframe(&mut constructed_candles, *timeframe, true);
                        if count != *len + 1 {
                            // Candles didn't contain partial candles so adding manually.
                            constructed_candles.set_candle_partial(constructed_candles.len() - 1);
                        }
                        self.candles_builder.insert(&market, constructed_candles);
                    } else {
                        self.candles_builder.insert(&market, candles.clone());
                    }
                }
                // Removing already processed timeframes.
                map.retain(|timeframe, _| timeframe % supported_timeframe != 0);
            }
        }

        // Checking if some of the candles are outdated.
        // By now there could be outdated candles if there were large amount of requests.
        let now = Utc::now().timestamp_s();
        loop {
            let mut fetched = None;
            let mut fetched_market = String::new();
            let mut fetched_timeframe = 0;
            let mut fetched_start = 0;
            for (market, map) in self.candles_builder.candles() {
                let mut min_timeframe = u32::MAX;
                let mut start = 0;
                for (timeframe, candles) in map {
                    let possible_start = *candles.timestamp.last().unwrap();
                    if possible_start + timeframe > now {
                        continue;
                    }
                    if min_timeframe > *timeframe {
                        min_timeframe = *timeframe;
                        start = possible_start;
                    }
                }
                if start == 0 {
                    continue;
                }
                for supported_timeframe in supported_timeframes.iter() {
                    if min_timeframe % supported_timeframe == 0 {
                        min_timeframe = *supported_timeframe;
                        break;
                    }
                }
                let count = ((now - now % min_timeframe - start) / min_timeframe) as usize;
                let new_candles = Candles::with_capacity(
                    self.client.exchange().name().into(),
                    market.clone(),
                    count,
                );
                fetched = Some(new_candles);
                fetched_start = start;
                fetched_timeframe = min_timeframe;
                fetched_market = market.clone();
                break;
            }
            if !cfg!(feature = "test") && fetched_timeframe != 0 {
                let candles = fetched.as_mut().unwrap();
                warn!("New candles available, fetching them.");
                trace!(
                    "Fetching {} candles {};{};{}",
                    candles.timestamp.capacity(),
                    self.client.exchange().name(),
                    fetched_market,
                    fetched_timeframe
                );
                self.client
                    .fetch_candles(
                        &fetched_market,
                        fetched_timeframe,
                        fetched_start,
                        fetched_start + candles.timestamp.capacity() as u32 * fetched_timeframe,
                        candles,
                    )
                    .await?;
                total_fetches += 1;
                for i in 0..candles.len() {
                    // If timestamp is the same as shift timestamp then candles would shift thus
                    // reducing by 1;
                    let t = *candles.timestamp.last().unwrap() - 1;
                    let market = &candles.market;
                    self.candles_builder.tick(market, t, candles.open[i], 0.);
                    self.candles_builder.tick(market, t, candles.high[i], 0.);
                    self.candles_builder.tick(market, t, candles.low[i], 0.);
                    self.candles_builder
                        .tick(market, t, candles.close[i], candles.volume[i]);
                }
            } else {
                break;
            }
        }

        info!(
            "Successfully fetched candles, total_fetches: {}",
            total_fetches
        );
        Ok(())
    }

    async fn on_order_filled(&mut self, execution: &Execution) -> Result<()> {
        let is_inverse = self
            .active_instruments
            .get(&execution.market)
            .unwrap()
            .is_inverse;
        let instruments = &self.active_instruments;
        // Market orders from models are bundled into one.
        // Models are still notified that their market orders have been executed.
        // Preventing recording of bundled market order.
        let open_order = self.open_orders.remove(&execution.order_id).unwrap();
        let absolute_amount = open_order.max_amount.abs();
        let mark_price = order::executed_price(open_order.amount, open_order.value, is_inverse);

        for sub_order in open_order.open_sub_orders {
            let weight = sub_order.amount.abs() / absolute_amount;
            let sub_execution = Execution {
                market: execution.market.clone(),
                order_id: sub_order.id,
                value: order::value(mark_price, sub_order.amount, is_inverse),
                amount: sub_order.amount,
                amount_left: Decimal::zero(),
                fee_paid: open_order.fee_paid * weight,
                executed_price: mark_price,
                timestamp_ns: execution.timestamp_ns,
            };
            broadcast_async!(self, on_execution, sub_execution, instruments);
        }
        // Doing sequentially to ensure if error happens we can still process orders on next boot
        save_execution_time(self.client.exchange().name(), execution.timestamp_ns).await?;
        // If websocket doesn't provide many messages or if next websocket message is new candle we
        // might open a position but we want to go under maintenance.
        self.maybe_go_under_maintenance().await?;
        Ok(())
    }

    async fn open_orders(&mut self) -> Result<()> {
        let mut tmp_sub_orders_to_open = self.tmp_sub_orders_to_open.lock().await;
        let mut tmp_sub_orders_to_cancel = self.tmp_sub_orders_to_cancel.lock().await;
        loop {
            if tmp_sub_orders_to_open.is_empty() {
                break;
            }
            let mut order = tmp_sub_orders_to_open.pop().unwrap();
            if order.is_canceled() {
                tmp_sub_orders_to_cancel.push(order.id);
            } else if order.is_market() {
                let amount = tmp_sub_orders_to_open
                    .iter()
                    .filter(|x| {
                        x.market == order.market
                            && x.trigger_price == order.trigger_price
                            && x.limit == order.limit
                    })
                    .map(|x| x.amount)
                    .sum::<Decimal>()
                    + order.amount;
                if amount.is_zero() {
                    let sub_orders = tmp_sub_orders_to_open
                        .drain_filter(|x| {
                            x.market == order.market
                                && x.trigger_price == order.trigger_price
                                && x.limit == order.limit
                        })
                        .collect();
                    broadcast_async!(self, on_orders_placed, sub_orders);
                    for sub_order in sub_orders {
                        let execution = Execution {
                            market: sub_order.market.clone(),
                            order_id: sub_order.id,
                            value: sub_order.value.unwrap(),
                            amount: sub_order.amount,
                            amount_left: Decimal::zero(),
                            fee_paid: Decimal::zero(),
                            executed_price: sub_order.predicted_price.to_decimal().unwrap(),
                            timestamp_ns: Utc::now().timestamp_ns(),
                        };
                        let instruments = &self.active_instruments;
                        self.listeners
                            .broadcast_async(|x| x.on_execution(&execution, instruments))
                            .await?;
                    }
                } else {
                    order.amount = amount;
                    let mut open_sub_orders = vec![OpenedSubOrder::from(&order)];
                    tmp_sub_orders_to_open.retain(|o| {
                        if o.market == order.market
                            && o.trigger_price == order.trigger_price
                            && o.limit == order.limit
                        {
                            open_sub_orders.push(o.into());
                            false
                        } else {
                            true
                        }
                    });
                    let open_order = OpenedOrder::new(order.amount, open_sub_orders);
                    self.open_orders.insert(order.id, open_order);
                    self.tmp_orders_to_open.push(order);
                }
            } else {
                let open_sub_orders = vec![OpenedSubOrder::from(&order)];
                let open_order = OpenedOrder::new(order.amount, open_sub_orders);
                self.open_orders.insert(order.id, open_order);
                self.tmp_orders_to_open.push(order);
            }
        }
        let cancel_fut = if tmp_sub_orders_to_cancel.is_empty() {
            async { Ok(()) }.boxed()
        } else {
            self.client.cancel_orders(&tmp_sub_orders_to_cancel)
        };
        let post_fut = if !self.tmp_orders_to_open.is_empty() {
            self.client.post_orders(&self.tmp_orders_to_open)
        } else {
            async { Ok(()) }.boxed()
        };
        try_join!(cancel_fut, post_fut)?;
        let orders_to_open = &self.tmp_orders_to_open;
        broadcast_async!(self, on_orders_placed, orders_to_open);
        for id in &*tmp_sub_orders_to_cancel {
            self.open_orders.remove(id);
        }
        self.tmp_orders_to_open.clear();
        tmp_sub_orders_to_open.clear();
        tmp_sub_orders_to_cancel.clear();

        Ok(())
    }

    async fn handle_maintenance(&mut self, maintenance: MaintenanceMode) -> Result<()> {
        let maintenance = Maintenance {
            mode: maintenance,
            timestamp_s: Utc::now().timestamp_s(),
        };
        broadcast_async!(self, on_maintenance, maintenance);
        match maintenance.mode {
            MaintenanceMode::Boot | MaintenanceMode::Reconnect | MaintenanceMode::Crash => {}
            MaintenanceMode::ReloadSafe => self.maintenance_state = MaintenanceState::SafeReload,
            MaintenanceMode::Reload => {
                broadcast_async!(self, on_shutdown);
                info!("Reloading...");
                return Err(MatrixError::Reload.into());
            }
            MaintenanceMode::ShutdownSafe => {
                self.maintenance_state = MaintenanceState::SafeShutdown
            }
            MaintenanceMode::Shutdown => {
                info!("Shutting down...");
                broadcast_async!(self, on_shutdown);
                return Err(MatrixError::Shutdown.into());
            }
        }
        Ok(())
    }

    async fn maybe_go_under_maintenance(&mut self) -> Result<()> {
        match self.maintenance_state {
            MaintenanceState::Normal => {}
            MaintenanceState::SafeReload => {
                if self.open_orders.is_empty() && !self.trade_guard_mut().is_in_position() {
                    self.handle_maintenance(MaintenanceMode::Reload).await?;
                }
            }
            MaintenanceState::SafeShutdown => {
                if self.open_orders.is_empty() && !self.trade_guard_mut().is_in_position() {
                    self.handle_maintenance(MaintenanceMode::Shutdown).await?;
                }
            }
        }
        Ok(())
    }
}

pub async fn build_and_kill(
    client: &impl NetworkClient,
    exchange_id: u16,
    use_public_data_miner: bool,
) -> Result<()> {
    client.kill().await?;
    let mut listeners = Listeners::new();
    push_non_essential_listeners(
        &mut listeners,
        use_public_data_miner,
        client.exchange().name(),
        exchange_id,
    );
    let maintenance = Maintenance {
        mode: MaintenanceMode::Crash,
        timestamp_s: Utc::now().timestamp_s(),
    };
    listeners
        .broadcast_async(|x| x.on_maintenance(&maintenance))
        .await?;
    listeners.broadcast_async(|x| x.on_shutdown()).await?;
    Ok(())
}

pub(super) async fn load_last_execution_time(exchange_name: &str) -> Result<u64> {
    match File::open(CONFIG.data_dir.join(exchange_name).join("state.bin")).await {
        Ok(mut file) => Ok(file.read_u64_le().await?),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => Ok(u64::MAX),
            _ => Err(e.into()),
        },
    }
}

async fn save_execution_time(exchange_name: &str, timestamp_ns: u64) -> Result<()> {
    let mut file = File::create(CONFIG.data_dir.join(exchange_name).join("state.bin")).await?;
    file.write_u64_le(timestamp_ns).await?;

    Ok(())
}

fn push_non_essential_listeners(
    listeners: &mut Listeners<dyn ExchangeListener>,
    use_public_data_miner: bool,
    exchange_name: &str,
    exchange_id: u16,
) {
    if use_public_data_miner {
        listeners.push(Box::new(ExchangeDataAgent::new(
            &CONFIG.data_dir,
            exchange_name,
        )));
    }
    if get_exchange_config().is_none() {
        listeners.push(Box::new(DbAgent::new(exchange_id)));
    }
}

#[derive(Debug)]
struct OpenedOrder {
    value: Decimal,
    fee_paid: Decimal,
    amount: Decimal,
    max_amount: Decimal,
    open_sub_orders: Vec<OpenedSubOrder>,
}

impl OpenedOrder {
    fn new(max_amount: Decimal, open_sub_orders: Vec<OpenedSubOrder>) -> OpenedOrder {
        OpenedOrder {
            value: Decimal::zero(),
            fee_paid: Decimal::zero(),
            amount: Decimal::zero(),
            max_amount,
            open_sub_orders,
        }
    }
}

#[derive(Debug)]
pub struct OpenedSubOrder {
    amount: Decimal,
    id: OrderId,
}

impl From<&Order> for OpenedSubOrder {
    fn from(order: &Order) -> Self {
        OpenedSubOrder {
            amount: order.amount,
            id: order.id,
        }
    }
}
