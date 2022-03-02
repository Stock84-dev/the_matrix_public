use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ops::Sub;
use std::option::NoneError;
use std::time::Instant;

use async_std::task;
use async_trait::async_trait;
use bitmex::client::BitmexClient;
use bitmex::definitions::{ExecutionHistory, OrderBookL2};
use bitmex::error::BitmexWsError;
use bitmex::exchange::Bitmex;
use bitmex::models::*;
use bitmex::requests::*;
use bitmex::websocket::{Action, BitmexWebSocket, Command, Message, TableMessage, Topic};
use bitmex::*;
use chrono::{DateTime, Duration, Timelike, Utc};
use futures::future;
use futures::future::{FutureExt, *};
use futures::stream::StreamExt;
use lazy_static::lazy_static;
use merovingian::candles::Candles;
use merovingian::minable_models::{Margin, *};
use merovingian::order::Order;
use mouse::ext::AsPinned;
use mouse::log::*;
use mouse::num::{dec, FromMaybeDecimal};
use mouse::throw;
use mouse::time::{IntoDateTime, Timestamp};
use nebuchadnezzar_core::client::Client;
use nebuchadnezzar_core::error::NebError;
use nebuchadnezzar_core::paginators::{BasicPaginator, BasicPaginatorState};
use nebuchadnezzar_core::websocket::{tokio_tungstenite, WebSocket};
use nebuchadnezzar_core::{Credentials, Exchange};
use serde_json::{from_value, Value};
use sorted_vec::ReverseSortedVec;
use stream_flatten_iters::TryStreamExt as _;
use tokio::try_join;
use tungstenite::error::ProtocolError;

use crate::agents::network_agent::NetworkClient;
use crate::agents::network_agents::{Execution, *};
use crate::error::MatrixError;

pub struct BitmexAgent {
    state: NetworkAgentState<BitmexNetworkClient, BitmexWs>,
    active_bitmex_instruments: HashMap<String, BitmexInstrument>,
}

pub struct BitmexNetworkClient {
    client: BitmexClient,
}

#[async_trait]
impl NetworkClient for BitmexNetworkClient {
    type Exchange = Bitmex;

    fn exchange(&self) -> Self::Exchange {
        self.client.exchange()
    }

    async fn fetch_candles(
        &self,
        market: &str,
        timeframe: u32,
        start: u32,
        end: u32,
        candles: &mut Candles,
    ) -> Result<()> {
        let client = &self.client;
        let mut paginator =
            BasicPaginator::new(start, end, timeframe, |state: &BasicPaginatorState| {
                Ok(GetTradeBucketedRequest {
                    bin_size: match timeframe {
                        60 => BinSize::Minute1,
                        300 => BinSize::Minute5,
                        3600 => BinSize::Hour1,
                        86400 => BinSize::Day1,
                        _ => panic!("Unsupported timeframe."),
                    },
                    partial: Some(true),
                    symbol: market.into(),
                    count: Some(state.count),
                    start_time: Some(state.i.into_date_time()),
                    columns: Some(Value::String("open,high,low,close,volume".to_string())),
                    ..Default::default()
                })
            });
        let mut stream = client.paginate(paginator.as_pin_mut());
        while let Some(bins) = stream.next().await {
            for bin in bins? {
                candles.push(
                    bin.timestamp.timestamp_s(),
                    bin.open.unwrap().to_f32().unwrap(),
                    bin.high.unwrap().to_f32().unwrap(),
                    bin.low.unwrap().to_f32().unwrap(),
                    bin.close.unwrap().to_f32().unwrap(),
                    bin.volume.unwrap().to_f32().unwrap(),
                );
            }
        }
        // If there wasn't a trade then partial candle isn't generated.
        if *candles.timestamp.last().unwrap() < end {
            candles.rotate_left();
        }
        Ok(())
    }

    async fn post_orders(&self, orders: &Vec<Order>) -> Result<()> {
        let mut bulk = Vec::new();
        let mut individual = Vec::new();
        for order in orders {
            let client = &self.client;
            if order.is_market() {
                individual.push(async move {
                    let request = order_to_post_request(order);
                    handle_overload(|| client.request(request.clone())).await
                });
            } else {
                bulk.push(order_to_post_request(order));
            }
        }
        let bulk_fut = if !bulk.is_empty() {
            async {
                let req = PostOrderBulkRequest { orders: Some(bulk) };
                handle_overload(|| self.client.request(req.clone())).await?;
                Ok(())
            }
            .boxed()
        } else {
            async { Ok(()) }.boxed()
        };
        let individual_fut = futures::future::try_join_all(individual);
        try_join!(bulk_fut, individual_fut)?;

        Ok(())
    }

    async fn cancel_orders(&self, orders_to_cancel: &Vec<OrderId>) -> Result<()> {
        self.client
            .request(DeleteOrderRequest {
                order_id: None,
                cl_ord_id: Some(Value::Array(
                    orders_to_cancel
                        .iter()
                        .map(|x| Value::String(x.to_string()))
                        .collect(),
                )),
                text: None,
            })
            .await?;
        Ok(())
    }

    async fn kill(&self) -> Result<()> {
        trace!("Cancelling all open orders.");
        self.client
            .request(DeleteOrderAllRequest {
                ..Default::default()
            })
            .await?;
        info!("Orders cancelled!");
        trace!("Fetching open positions.");
        let positions = self
            .client
            .request(GetPositionRequest {
                ..Default::default()
            })
            .await?;
        let n_positions = positions.iter().filter(|x| x.is_open.unwrap()).count();
        for (i, position) in positions
            .into_iter()
            .filter(|x| x.is_open.unwrap())
            .enumerate()
        {
            trace!("Closing position {}/{}.", i + 1, n_positions);
            self.client
                .request(PostOrderRequest {
                    symbol: position.symbol,
                    side: None,
                    simple_order_qty: None,
                    order_qty: None,
                    price: None,
                    display_qty: None,
                    stop_px: None,
                    cl_ord_id: None,
                    cl_ord_link_id: None,
                    peg_offset_value: None,
                    peg_price_type: None,
                    ord_type: None,
                    time_in_force: None,
                    exec_inst: Some(ExecInst::Close),
                    contingency_type: None,
                    text: None,
                })
                .await?;
        }
        info!("Positions closed successfully!");

        Ok(())
    }
}

pub struct BitmexWs {
    inner: BitmexWebSocket,
}

#[async_trait]
impl Ws for BitmexWs {
    type Message = Option<Result<Message, BitmexWsError>>;

    async fn next(&mut self) -> Self::Message {
        self.inner.next().await
    }

    async fn close(&mut self) -> Result<()> {
        Ok(self.inner.close().await?)
    }
}

#[async_trait]
impl NetworkAgent for BitmexAgent {
    type Websocket = BitmexWs;
    type Client = BitmexNetworkClient;

    async fn build(exchange_config: ExchangeConfig, model_configs: Vec<ModelConfig>) -> Result<Self>
    where
        Self: Sized,
    {
        let bitmex = Bitmex::new(exchange_config.use_testnet);
        let mut client = bitmex.new_client();
        let credentials = Credentials::new(&exchange_config.api_key, &exchange_config.api_secret);
        client.authenticate(credentials)?;
        let ws = new_subscribed_websocket(&client).await?;
        let mut instrument_configs = HashMap::new();
        let mut active_bitmex_instruments = HashMap::new();
        let (active_instruments, margin) = try_join!(
            get_instrument_configs(
                &client,
                &mut active_bitmex_instruments,
                Some(&mut instrument_configs)
            ),
            client.request(GetUserMarginRequest {
                currency: Some("XBt".into())
            })
        )?;
        let instruments = active_instruments
            .into_iter()
            .map(|x| {
                let instrument = Instrument::ifrom(&x);
                (x.symbol, instrument)
            })
            .collect();

        let bitmex_agent = BitmexAgent {
            state: NetworkAgentState::new(
                exchange_config,
                model_configs,
                BitmexNetworkClient { client },
                ws,
                instrument_configs,
                instruments,
                // Balance is in satoshis, converting to bitcoin.
                Decimal::new(margin.margin_balance.unwrap(), 8),
                margin.margin_leverage.unwrap(),
                &*SUPPORTED_TIMEFRAMES,
                1000,
            )
            .await?,
            active_bitmex_instruments,
        };
        info!("Successfully initialized BitmexAgent.");
        Ok(bitmex_agent)
    }

    async fn catch_up(
        &mut self,
        last_execution_time: DateTime<Utc>,
    ) -> Result<Vec<Result<Execution, FundingExecution>>> {
        let mut history = Vec::new();
        for (market, _) in &self.active_bitmex_instruments {
            let _start = usize::MAX;
            let mut ts = Utc::now();
            loop {
                // Must be here
                if ts < last_execution_time {
                    break;
                }
                let batch = self
                    .state
                    .client()
                    .client
                    .request(GetUserExecutionHistoryRequest {
                        symbol: market.clone(),
                        timestamp: ts,
                    })
                    .await?;
                history.extend(
                    batch
                        .into_iter()
                        .filter(|x| x.timestamp > last_execution_time)
                        .map(|x| convert_execution_history(x)),
                );
                ts = ts.sub(Duration::days(1));
            }
        }
        Ok(history)
    }

    async fn handle_message(
        &mut self,
        msg: <<Self as NetworkAgent>::Websocket as Ws>::Message,
    ) -> Result<()> {
        match msg {
            None => {
                error!("Connection closed!");
                return Err(MatrixError::WebsocketExhausted.into());
            }
            Some(Ok(msg)) => match msg {
                Message::Table(t) => {
                    match t.table.as_str() {
                        "announcement" => self.handle_announcement_message(*t).await?,
                        "trade" => self.handle_trade_message(*t).await?,
                        "chat" => self.handle_chat_message(*t).await?,
                        "connected" => self.handle_connected_users_message(*t).await?,
                        "funding" => self.handle_funding_message(*t).await?,
                        "instrument" => self.handle_instrument_message(*t).await?,
                        "insurance" => self.handle_insurance_message(*t).await?,
                        "liquidation" => self.handle_public_liquidation_message(*t).await?,
                        "orderBookL2" => self.handle_order_book_message(*t).await?,
                        "margin" => self.handle_margin_message(*t).await?,
                        "tradeBin1m" => {}
                        "execution" => self.handle_execution_message(*t).await?,
                        "order" => {}
                        "position" => self.handle_position_message(*t).await?,
                        _ => println!("Other table message {:?}", t),
                    };
                }
                Message::Success(_) => {}
                Message::Error(e) => {
                    error!("BitMEX websocket error: {:?}", e);
                    panic!("BitMEX websocket error: {:?}", e);
                }
                _ => println!("Other ws message: {:?}", msg),
            },
            Some(Err(e)) => match e {
                BitmexWsError::Tungstenite(tokio_tungstenite::tungstenite::Error::Protocol(
                    ProtocolError::ResetWithoutClosingHandshake,
                )) => return Err(MatrixError::WebsocketExhausted.into()),
                e => return Err(e.into()),
            },
        }
        Ok(())
    }

    fn new_client(use_testnet: bool, api_key: &str, api_secret: &str) -> Self::Client {
        let mut client = Bitmex::new(use_testnet).new_client();
        // Authentication never fails on bitmex client because it doesen't make network request
        // immediately
        client
            .authenticate(Credentials::new(api_key, api_secret))
            .unwrap();
        BitmexNetworkClient { client }
    }

    async fn new_subscribed_web_socket(&mut self) -> Result<Self::Websocket> {
        let ws = new_subscribed_websocket(&self.state.client().client).await?;
        Ok(ws)
    }

    fn state_mut(&mut self) -> &mut NetworkAgentState<BitmexNetworkClient, Self::Websocket> {
        &mut self.state
    }
}

impl BitmexAgent {
    async fn handle_announcement_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for datum in table.data {
            let msg: bitmex::definitions::Announcement = from_value(datum)?;
            let announcement = Announcement {
                link: msg.link.unwrap(),
                title: msg.title.unwrap(),
                content: msg.content.unwrap(),
                timestamp_s: msg.date.unwrap().timestamp_s(),
            };
            self.state.on_announcement(announcement).await?;
        }
        Ok(())
    }

    async fn handle_trade_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        //        println!("{:?}", table);
        for msg in table.data {
            let bitmex_trade: bitmex::definitions::Trade = from_value(msg)?;
            let timestamp_ns = bitmex_trade.timestamp.timestamp_ns();
            let _timestamp_s = bitmex_trade.timestamp.timestamp_s();

            let price = match bitmex_trade.price {
                Some(p) => p.to_f32().unwrap(),
                None => {
                    warn!("No price in trade, ignoring it.");
                    return Ok(());
                }
            };
            let volume = match bitmex_trade.amount {
                Some(s) => match bitmex_trade.side.unwrap() {
                    Side::Buy => s as f32,
                    Side::Sell => -s as f32,
                    Side::Unknown => {
                        // This actually happened
                        warn!("Unknown trade side, ignoring it.");
                        return Ok(());
                    }
                },
                None => {
                    warn!("No volume in trade, ignoring it");
                    return Ok(());
                }
            };
            let trade = Trade {
                timestamp_ns,
                price,
                amount: volume,
            };
            let symbol = bitmex_trade.symbol;
            self.state.on_public_trade(trade, symbol).await?;
        }

        Ok(())
    }

    async fn handle_chat_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for datum in table.data {
            let msg: bitmex::definitions::Chat = from_value(datum)?;
            let chat_message = ChatMessage {
                channel_id: msg.id.unwrap() as u8,
                from_bot: match msg.from_bot.unwrap() {
                    true => 1,
                    false => 0,
                },
                timestamp_ns: msg.date.timestamp_ns(),
                message: msg.message,
                user: msg.user,
            };
            self.state.on_chat_message(chat_message).await?;
        }
        Ok(())
    }

    async fn handle_connected_users_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for datum in table.data {
            let msg: bitmex::definitions::ConnectedUsers = from_value(datum)?;
            let connected = Connected {
                bots: msg.bots.unwrap() as u32,
                users: msg.users.unwrap() as u32,
                timestamp_s: Utc::now().timestamp_s(),
            };
            self.state.on_connected_users_changed(connected).await?;
        }
        Ok(())
    }

    async fn handle_funding_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for datum in table.data {
            let msg: bitmex::definitions::Funding = from_value(datum)?;
            let funding = Funding {
                rate: msg.funding_rate.unwrap().to_f32().unwrap(),
                daily_rate: msg.funding_rate_daily.unwrap().to_f32().unwrap(),
                timestamp_s: msg.timestamp.timestamp_s(),
            };
            self.state.on_funding(funding, msg.symbol).await?;
        }
        Ok(())
    }

    async fn handle_execution_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        //        trace!("handling execution {:#?}", table);
        for datum in table.data {
            let msg: bitmex::definitions::Execution = from_value(datum)?;
            if let Some(status) = msg.ord_status {
                match status {
                    OrdStatus::Filled => match convert_execution(msg) {
                        Ok(execution) => self.state.on_execution(execution).await?,
                        Err(funding) => self.state.on_funding_execution(funding).await?,
                    },
                    OrdStatus::PartiallyFilled => match convert_execution(msg) {
                        Ok(execution) => self.state.on_execution(execution).await?,
                        Err(_) => unreachable!("funding partially filled"),
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_instrument_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        match table.action {
            Action::Delete => {
                for datum in table.data {
                    let msg: bitmex::definitions::Instrument = from_value(datum)?;
                    self.active_bitmex_instruments.remove(&msg.symbol);
                }
            }
            Action::Insert => {
                for datum in table.data {
                    let msg: bitmex::definitions::Instrument = from_value(datum)?;
                    self.active_bitmex_instruments.insert(
                        msg.symbol.clone(),
                        BitmexInstrument {
                            id: self
                                .active_bitmex_instruments
                                .iter()
                                .map(|x| x.1.id)
                                .max()
                                .unwrap()
                                + 1,
                            legacy_tick_size: msg.tick_size.unwrap(),
                        },
                    );
                    match InstrumentConfig::try_from(&msg) {
                        Ok(instrumnet) => {
                            self.state
                                .on_new_instrument(instrumnet, (&msg).iinto(), &msg.symbol)
                                .await?;
                        }
                        Err(_) => {
                            warn!("could not convert {:#?} while inserting, ignoring it", msg)
                        }
                    }
                }
            }
            Action::Partial | Action::Update => {
                for datum in table.data {
                    let msg: bitmex::definitions::Instrument = from_value(datum)?;
                    self.state
                        .on_instrument_changed((&msg).iinto(), &msg.symbol)
                        .await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_insurance_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for datum in table.data {
            let msg: bitmex::definitions::Insurance = from_value(datum)?;
            let mut insurance = Insurance {
                balance: match msg.wallet_balance {
                    Some(b) => b as u64,
                    None => return Ok(()),
                },
                timestamp_s: msg.timestamp.timestamp_s(),
            };
            match msg.currency.as_str() {
                "XBT" => insurance.balance *= 100_000_000,
                // Currency is already in satoshis
                "XBt" => {}
                "μXBT" => insurance.balance *= 100,
                "mXBT" => insurance.balance *= 100_000,
                _ => panic!("Unknown currency."),
            }
            self.state
                .on_insurance_changed(insurance, "XBTUSD".to_string())
                .await?;
        }
        Ok(())
    }

    async fn handle_public_liquidation_message(
        &mut self,
        table: TableMessage<Value>,
    ) -> Result<()> {
        let timestamp_ns = Utc::now().timestamp_ns();
        for datum in table.data {
            let msg: bitmex::definitions::Liquidation = from_value(datum)?;
            let liquidation = PublicLiquidation {
                order_id: msg.order_id,
                price: match msg.price {
                    Some(p) => p.to_f32().unwrap(),
                    None => std::f32::NAN,
                },
                amount: match msg.leaves_qty {
                    Some(a) => a as f32,
                    None => std::f32::NAN,
                },
                timestamp_ns,
            };
            self.state
                .on_public_liquidation(liquidation, msg.symbol.unwrap())
                .await?;
        }
        Ok(())
    }

    async fn handle_order_book_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        fn create_update(
            active_instrument: &BitmexInstrument,
            msg: &OrderBookL2,
        ) -> Result<OrderBookUpdate, ()> {
            // Getting price form id.
            // See https://www.bitmex.com/app/wsAPI#OrderBookL2
            let price = ((dec!(100_000_000) * Decimal::from(active_instrument.id))
                - Decimal::from(msg.id))
                * active_instrument.legacy_tick_size;
            let size = match msg.size {
                Some(s) => s as f32,
                None => 0.,
            };
            let size = match msg.side {
                Side::Buy => size,
                Side::Sell => -size,
                _ => {
                    warn!("Unknown side in order_book_message {:#?}, ignoring it", msg);
                    return Err(());
                }
            };
            Ok(OrderBookUpdate {
                size,
                price: price.to_f32().unwrap(),
                timestamp_ns: Utc::now().timestamp_ns(),
            })
        }
        for datum in table.data {
            let msg: bitmex::definitions::OrderBookL2 = from_value(datum)?;
            let order_book_update = match self.active_bitmex_instruments.get(&msg.symbol) {
                Some(i) => match create_update(i, &msg) {
                    Ok(u) => u,
                    Err(_) => return Ok(()),
                },
                // reload tick sizes and ids
                None => {
                    self.active_bitmex_instruments.clear();
                    get_instrument_configs(
                        &self.state.client().client,
                        &mut self.active_bitmex_instruments,
                        None,
                    )
                    .await?;
                    let instrument = self
                        .active_bitmex_instruments
                        .get(&msg.symbol)
                        .expect("Instrument not found");
                    match create_update(instrument, &msg) {
                        Ok(u) => u,
                        Err(_) => return Ok(()),
                    }
                }
            };
            self.state
                .on_order_book_updated(order_book_update, msg.symbol)
                .await?;
        }
        Ok(())
    }

    async fn handle_margin_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for margin in table.data {
            let msg: bitmex::definitions::Margin = from_value(margin)?;
            let leverage = match msg.margin_leverage {
                None => continue,
                Some(l) => l,
            };
            let balance = match msg.margin_balance {
                None => continue,
                Some(b) => Decimal::new(b, 8),
            };
            let margin = Margin {
                // Converting balance to bitcoin from satoshi.
                balance,
                leverage,
                timestamp_ns: msg.timestamp.unwrap().timestamp_ns(),
            };
            self.state.on_margin_changed(margin).await?;
        }
        Ok(())
    }

    async fn handle_position_message(&mut self, table: TableMessage<Value>) -> Result<()> {
        for position in table.data {
            let position: bitmex::definitions::Position = from_value(position)?;
            let amount = match position.current_qty {
                None => continue,
                Some(q) => q.into(),
            };
            let position = Position {
                market: position.symbol,
                amount,
                timestamp_ns: position.timestamp.unwrap().timestamp_ns(),
            };
            self.state.on_position_changed(position).await?;
        }
        Ok(())
    }
}

async fn new_subscribed_websocket(client: &BitmexClient) -> Result<BitmexWs> {
    let exchange = client.exchange();
    let watch = Instant::now();
    let mut ws = exchange.new_web_socket().await?;
    match WebSocket::next(&mut ws).await.unwrap()? {
        Message::Info(msg) => {
            get_and_notify_time_difference(msg.timestamp, watch, exchange.name());
        }
        _ => panic!("Expected info message while synchronizing clock."),
    }
    ws.authenticate_raw(client.credential().as_ref().unwrap())
        .await?;
    ws.send(Command::Subscribe(vec![
        Topic::Announcement,
        Topic::Chat,
        Topic::Connected,
        Topic::Funding,
        Topic::Instrument,
        Topic::Insurance,
        Topic::Liquidation,
        Topic::OrderBookL2(None), // Subscribed when Instruments get received.
        Topic::PublicNotifications,
        //          NOTE: There is significant delay when new 1min candles are received (20s or so).
        Topic::Trade(None),
        Topic::Order, // live order status from user
        Topic::Execution,
        Topic::Margin,
        Topic::Position,
    ]))
    .await?;
    Ok(BitmexWs { inner: ws })
}

async fn get_instrument_configs(
    client: &BitmexClient,
    bitmex_instruments: &mut HashMap<String, BitmexInstrument>,
    mut instrument_configs: Option<&mut HashMap<String, InstrumentConfig>>,
) -> Result<Vec<bitmex::definitions::Instrument>> {
    let mut active_instruments = Vec::new();
    let mut paginator = BasicPaginator::new(0, u32::MAX, 1, |state: &BasicPaginatorState| {
        Ok(GetInstrumentRequest {
            count: Some(state.count as i32),
            start: Some(state.i as i32),
            ..Default::default()
        })
    });
    let mut stream = client
        .paginate(paginator.as_pin_mut())
        .take_while(|x| {
            future::ready(match x {
                Ok(vec) => !vec.is_empty(),
                Err(_) => true,
            })
        })
        .try_flatten_iters()
        .enumerate()
        .filter(|x| {
            future::ready(if let Ok(x) = &x.1 {
                x.state.as_ref().unwrap() == "Open"
            } else {
                true
            })
        });
    while let Some((i, instrument)) = stream.next().await {
        let instrument = instrument?;
        let mut tick_size = instrument.tick_size.unwrap();
        apply_legacy_tick_size(&instrument.symbol, &mut tick_size);
        bitmex_instruments.insert(
            instrument.symbol.clone(),
            BitmexInstrument {
                id: i,
                legacy_tick_size: tick_size,
            },
        );
        if let Some(configs) = instrument_configs.as_mut() {
            configs.insert(instrument.symbol.clone(), (&instrument).try_into().unwrap());
        }
        active_instruments.push(instrument);
    }
    Ok(active_instruments)
}

async fn handle_overload<R, T>(mut coroutine: impl FnMut() -> R) -> Result<T>
where
    R: Future<Output = Result<T>>,
{
    loop {
        match coroutine().await {
            Ok(success) => return Ok(success),
            Err(e) => {
                match e.downcast::<NebError>() {
                    // Error type we are looking for.
                    Ok(err) => {
                        match err {
                            NebError::RemoteError(response) => {
                                let value: Value = serde_json::from_str(&response.text?)?;
                                let obj = value.as_object().unwrap();

                                if let Some(msg) = obj.get("message") {
                                    if msg == "The system is currently overloaded. Please try again later." {
                                    warn!("BitMEX server overload. Retrying in 500ms.");
                                    task::sleep(std::time::Duration::from_millis(500)).await;
                                }
                                } else {
                                    error!("{:#?}", obj);
                                    throw!("Error while handling overload: {:#?}", obj);
                                }
                            }
                            _ => {
                                return Err(err.into());
                            }
                        }
                    }
                    // Different kind of error.
                    Err(error) => return Err(error),
                }
            }
        }
        trace!("looping");
    }
}

fn apply_legacy_tick_size(symbol: &String, tick_size: &mut Decimal) {
    match symbol.as_str() {
        "XBTUSD" => *tick_size = Decimal::new(1, 2),
        _ => {}
    }
}

fn order_to_post_request(order: &Order) -> PostOrderRequest {
    let mut request = PostOrderRequest {
        symbol: order.market.clone(),
        side: None,
        simple_order_qty: None,
        order_qty: Some(order.amount.to_i32().unwrap()),
        price: order.limit,
        display_qty: None,
        stop_px: order.trigger_price,
        cl_ord_id: None,
        cl_ord_link_id: None,
        peg_offset_value: None,
        peg_price_type: None,
        ord_type: None,
        time_in_force: None,
        exec_inst: None,
        contingency_type: None,
        text: None,
    };
    request.cl_ord_id = Some(order.id.to_string());
    if order.amount.is_sign_positive() {
        request.side = Some(Side::Buy)
    } else if order.amount.is_sign_negative() {
        request.side = Some(Side::Sell)
    }
    if order.trigger_price.is_some() {
        if order.limit.is_some() {
            request.ord_type = Some(OrdType::StopLimit);
        } else {
            request.ord_type = Some(OrdType::Stop);
        }
    } else if order.limit.is_some() {
        request.ord_type = Some(OrdType::Limit);
    } else {
        request.ord_type = Some(OrdType::Market);
    }

    request
}

fn convert_execution_history(
    mut execution: ExecutionHistory,
) -> Result<Execution, FundingExecution> {
    let market = execution.symbol;
    execution.exec_comm.rescale(8);
    let timestamp_ns = execution.timestamp.timestamp_ns();
    match execution.exec_type {
        ExecType::Trade | ExecType::TriggeredOrActivatedBySystem => {
            let amount = fix_amount(&None, &Some(execution.side), &Some(execution.last_qty));
            let amount_left = fix_amount(&None, &Some(execution.side), &Some(execution.leaves_qty));
            execution.exec_cost.rescale(8);
            execution
                .exec_cost
                .set_sign_positive(amount.is_sign_positive());
            Ok(Execution {
                market,
                order_id: OrderId::from_str(&execution.cl_ord_id),
                value: execution.exec_cost,
                amount,
                amount_left,
                fee_paid: execution.exec_comm,
                executed_price: execution.price,
                timestamp_ns,
            })
        }
        ExecType::Funding => Err(FundingExecution {
            market,
            fee_paid: execution.exec_comm,
            timestamp_ns,
        }),
        _ => panic!("Other execution kind."),
    }
}

pub fn convert_execution(execution: definitions::Execution) -> Result<Execution, FundingExecution> {
    let market = execution.symbol.unwrap();
    let fee_paid = Decimal::new(execution.exec_comm.unwrap(), 8);
    let timestamp_ns = execution.timestamp.unwrap().timestamp_ns();
    match execution.exec_type.unwrap() {
        ExecType::Trade | ExecType::TriggeredOrActivatedBySystem => {
            let amount = fix_amount(&execution.ord_status, &execution.side, &execution.last_qty);
            let amount_left = fix_amount(
                &execution.ord_status,
                &execution.side,
                &execution.leaves_qty,
            );
            let mut value = Decimal::new(execution.exec_cost.unwrap(), 8);
            value.set_sign_positive(amount.is_sign_positive());
            Ok(Execution {
                market,
                order_id: OrderId::from_str(execution.cl_ord_id.as_ref().unwrap()),
                value,
                amount,
                amount_left,
                fee_paid,
                executed_price: execution.price.unwrap(),
                timestamp_ns,
            })
        }
        ExecType::Funding => Err(FundingExecution {
            market,
            fee_paid,
            timestamp_ns,
        }),
        _ => panic!("Other execution kind."),
    }
}

impl From<bitmex::definitions::Instrument> for InstrumentConfig {
    fn from(instrument: bitmex::definitions::Instrument) -> Self {
        InstrumentConfig {
            base_currency: instrument.underlying_symbol.unwrap(),
            tick_size: instrument.tick_size.unwrap(),
            lot_size: instrument.lot_size.unwrap().into(),
            multiplier: instrument.multiplier.unwrap() as f32,
            quote_currency: instrument.quote_currency.unwrap(),
            is_inverse: instrument.is_inverse.unwrap(),
            taker_fee: instrument.taker_fee.unwrap(),
            maker_fee: instrument.maker_fee.unwrap(),
            funding_period: match instrument.funding_interval {
                Some(period) => period.timestamp_s(),
                None => 60 * 60 * 8,
            },
        }
    }
}

impl MyFrom<&bitmex::definitions::Instrument> for Instrument {
    fn ifrom(instrument: &bitmex::definitions::Instrument) -> Self {
        Instrument {
            fair_price: instrument.fair_price.to_f32(),
            mark_price: instrument.mark_price.to_f32(),
            timestamp_ns: instrument.timestamp.unwrap().timestamp_ns(),
        }
    }
}

impl TryFrom<&bitmex::definitions::Instrument> for InstrumentConfig {
    type Error = NoneError;
    fn try_from(instrument: &bitmex::definitions::Instrument) -> Result<Self, NoneError> {
        Ok(InstrumentConfig {
            base_currency: instrument.underlying_symbol.as_ref()?.clone(),
            tick_size: instrument.tick_size?,
            lot_size: instrument.lot_size?.into(),
            multiplier: instrument.multiplier? as f32,
            quote_currency: instrument.quote_currency.as_ref()?.clone(),
            is_inverse: instrument.is_inverse?,
            taker_fee: instrument.taker_fee?, // TODO: sometimes this is null
            maker_fee: instrument.maker_fee?,
            funding_period: match instrument.funding_interval {
                Some(period) => period.timestamp_s(),
                None => 60 * 60 * 8,
            },
        })
    }
}

fn fix_amount(
    status: &Option<OrdStatus>,
    side: &Option<Side>,
    amount: &Option<Decimal>,
) -> Decimal {
    match status {
        None => {
            return Decimal::zero();
        }
        Some(status) => {
            if let OrdStatus::Canceled = status {
                return Decimal::zero();
            }
        }
    }
    match side.as_ref().unwrap() {
        Side::Buy => Decimal::from(amount.unwrap()),
        Side::Sell => Decimal::from(-amount.unwrap()),
        Side::Unknown => panic!("Unknown order side."),
    }
}

struct BitmexInstrument {
    id: usize,
    legacy_tick_size: Decimal,
}

lazy_static! {
    pub static ref SUPPORTED_TIMEFRAMES: ReverseSortedVec<u32> =
        ReverseSortedVec::from_unsorted(vec![60, 5 * 60, 60 * 60, 60 * 60 * 24]);
}
