use std::collections::HashMap;
use std::path::Path;

use config::{get_exchange_config, CONFIG};
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use merovingian::candles::Candles;
use merovingian::minable_models::{Instrument, Margin, Trade};
use merovingian::non_minable_models::Fees;
use merovingian::order::{Order, OrderId};
use mouse::error::Result;
use mouse::ext::VecExt;
use mouse::log::*;
use mouse::num::{FromMaybeDecimal, IntoDecimal};
use num_traits::{One, Zero};
use rust_decimal::Decimal;

use super::INCEPTION_TIMESTAMP_S;
use crate::agents::network_agents::{Execution, FundingExecution, InstrumentConfig};

pub struct MockExchange {
    #[cfg(feature = "assert")]
    trades: (Vec<Trade>, usize),
    candles: HashMap<String, HashMap<u32, Candles>>,
    orders: Vec<Order>,
    executions: Vec<Execution>,
    funding_executions: Vec<FundingExecution>,
    fees: Fees,
    balance: Decimal,
    pnl: Decimal,
    position_amount: Decimal,

    #[cfg(not(feature = "assert"))]
    trades: HashMap<String, (Vec<Trade>, usize)>,
    #[cfg(not(feature = "assert"))]
    market_type: HashMap<String, bool>,
    #[cfg(not(feature = "assert"))]
    slippage: f32,
    #[cfg(not(feature = "assert"))]
    funding_time: u32,
}

impl MockExchange {
    #[cfg(not(feature = "assert"))]
    pub async fn new(config: &ExchangeConfig) -> Result<MockExchange> {
        let fees = Fees {
            maker: -0.00025,
            taker: 0.00075,
            // TODO: use real funding history
            // funding: 0.000378,
            funding: 0.0,
            funding_period: 60 * 60 * 8,
        };
        warn!("{:#?}", fees);
        let mut funding_time = u32::MAX;
        let mut trades = HashMap::new();
        let mut min_timeframes = HashMap::new();
        let mut candle_map: HashMap<String, HashMap<u32, Candles>> = HashMap::new();
        let mut market_type = HashMap::new();
        let data_path = CONFIG.data_path();
        let mut min_inception_timestamp = u32::MAX;
        for model_config in &config.models {
            market_type.insert(model_config.symbol.clone(), false);
            let timeframe = model_config.variable_values[0] as u32;
            min_timeframes
                .entry(model_config.symbol.clone())
                .and_modify(|x| {
                    if *x > timeframe {
                        *x = timeframe
                    }
                })
                .or_insert(timeframe);
            match candle_map.get_mut(&model_config.symbol) {
                Some(map) => {
                    if !map.contains_key(&timeframe) {
                        load_and_insert_candles(
                            timeframe,
                            data_path,
                            map,
                            &model_config.symbol,
                            &mut funding_time,
                            fees.funding_period,
                        )
                        .await?;
                    }
                }
                None => {
                    let mut map = HashMap::new();
                    load_and_insert_candles(
                        timeframe,
                        data_path,
                        &mut map,
                        &model_config.symbol,
                        &mut funding_time,
                        fees.funding_period,
                    )
                    .await?;
                    candle_map.insert(model_config.symbol.clone(), map);
                }
            }
        }

        for (market, timeframe) in min_timeframes {
            let candles = candle_map.get(&market).unwrap().get(&timeframe).unwrap();
            min_inception_timestamp.min_mut(candles.timestamp[0]);
            trades.insert(market, generate_trades(candles));
        }

        unsafe {
            *(&*INCEPTION_TIMESTAMP_S as *const _ as *mut u32) = min_inception_timestamp;
        }

        Ok(MockExchange {
            trades,
            candles: candle_map,
            market_type,
            orders: Vec::new(),
            executions: Vec::new(),
            position_amount: Decimal::zero(),
            fees,
            slippage: 1.,
            balance: Decimal::one(),
            pnl: Decimal::zero(),
            funding_time,
        })
    }

    #[cfg(feature = "assert")]
    pub async fn new(
        _config: &ExchangeConfig,
        model_configs: &Vec<ModelConfig>,
    ) -> Result<MockExchange> {
        let fees = Fees {
            maker: -0.00025,
            taker: 0.00075 + 0.0006728571,
            // TODO: use real funding history
            // average XBTUSD funding
            funding: 0.000077490909,
            funding_period: 60 * 60 * 8,
        };
        warn!("{:#?}", fees);

        let mut _funding_time = 0;

        let mut candles_map = HashMap::new();
        let mut timeframe_map = HashMap::new();
        load_and_insert_candles(
            model_configs[0].variable_values()[0] as u32,
            &CONFIG.data_dir,
            &mut timeframe_map,
            &model_configs[0].market,
            &mut _funding_time,
            fees.funding_period,
        )
        .await?;
        let candles = timeframe_map.iter().next().unwrap().1;
        let trades = generate_trades(candles);
        unsafe {
            *(&*INCEPTION_TIMESTAMP_S as *const _ as *mut u32) = candles.timestamp[0];
        }
        candles_map.insert(model_configs[0].market.clone(), timeframe_map);

        Ok(MockExchange {
            trades,
            candles: candles_map,
            orders: Vec::new(),
            executions: Vec::new(),
            position_amount: Decimal::zero(),
            fees,
            balance: Decimal::one(),
            pnl: Decimal::zero(),
            funding_executions: vec![],
        })
    }

    #[cfg(feature = "assert")]
    pub fn init(&mut self, _: &HashMap<String, InstrumentConfig>) {}

    #[cfg(not(feature = "assert"))]
    pub fn init(&mut self, instrument_configs: &HashMap<String, InstrumentConfig>) {
        for (market, is_inverse) in &mut self.market_type {
            *is_inverse = instrument_configs.get(market).unwrap().is_inverse;
        }
    }

    pub fn candles(&self) -> &HashMap<String, HashMap<u32, Candles>> {
        &self.candles
    }

    #[cfg(not(feature = "assert"))]
    pub fn get_instruments(&self) -> Vec<(String, Instrument)> {
        self.trades
            .iter()
            .map(|(market, (trades, _))| {
                (
                    market.into(),
                    Instrument {
                        fair_price: trades[0].price,
                        mark_price: trades[0].price,
                        timestamp_ns: trades[0].timestamp_ns,
                    },
                )
            })
            .collect()
    }

    #[cfg(feature = "assert")]
    pub fn get_instruments(&self) -> Vec<(String, Instrument)> {
        self.candles
            .iter()
            .map(|(market, map)| {
                let candles = map.iter().next().unwrap().1;
                (
                    market.into(),
                    Instrument {
                        fair_price: candles.open[0],
                        mark_price: candles.open[0],
                        timestamp_ns: candles.timestamp[0] as u64 * 1_000_000_000,
                    },
                )
            })
            .collect()
    }

    #[cfg(not(feature = "assert"))]
    pub fn get_margin(&mut self) -> Margin {
        if self.position_amount.is_zero() {
            return Margin {
                balance: self.balance,
                leverage: Decimal::zero(),
                timestamp_ns: 0,
            };
        }
        let is_inverse = self.market_type.iter().next().unwrap().1;
        let (trades, i) = self.trades.iter().next().unwrap().1;
        let price = trades[*i].price.to_decimal().unwrap();
        let open_value = self.pnl;
        let value = order::value(price, self.position_amount, *is_inverse);
        let mut diff = value - open_value;
        // Copy-pasted from model_state.rs
        if *is_inverse {
            if self.position_amount.is_sign_positive() {
                diff *= dec!(-1.);
            } else {
            }
        } else {
            if self.position_amount.is_sign_positive() {
            } else {
                diff *= dec!(-1.);
            }
        }
        let margin = self.balance + diff;
        let leverage = value.abs() / margin;

        Margin {
            balance: margin,
            leverage,
            timestamp_ns: 0,
        }
    }

    pub fn get_next_trades(&mut self, market: &String) -> Option<&[Trade]> {
        #[cfg(feature = "assert")]
        let (trades, id) = &mut self.trades;
        #[cfg(not(feature = "assert"))]
        let (trades, id) = self.trades.get_mut(market).unwrap();
        let i = *id;
        if i + 4 > trades.len() {
            return None;
        }
        // Intentionally lagging due to how candle builder works. On new candle gets called when a
        // trade with timestamp that is bigger than close timestamp of a candle. Which means that
        // exchange receives orders when public trades are already sent.
        let prev_trades = &trades[i - 4..i];
        if (i / 4) % 1000 == 0 {
            trace!(
                "{}/{}, {}%",
                i,
                trades.len(),
                i as f32 / trades.len() as f32 * 100.
            );
        }
        // if i == 148 * 4 {
        //     println!("{:#?}", &prev_trades[i..i + 4 * 2]);
        //     println!("{:#?}", self.orders);
        // }
        *id += 4;

        // Cannot borrow self in closure while part of it (orders) are being borrowed too.
        let position_amount = &mut self.position_amount;
        let messages = &mut self.executions;
        let fees = &self.fees;
        #[cfg(feature = "assert")]
        let slippage = 1.;
        #[cfg(not(feature = "assert"))]
        let slippage = self.slippage;
        let balance = &mut self.balance;
        let pnl = &mut self.pnl;
        #[cfg(feature = "assert")]
        let is_inverse = false;
        #[cfg(not(feature = "assert"))]
        let is_inverse = *self.market_type.get(market).unwrap();

        #[cfg(not(feature = "assert"))]
        {
            panic!();
            // First we pay funding because we open/close positions after rounded time.
            let trade_timestamp = (prev_trades[0].timestamp_ns / 1_000_000_000) as u32;
            if trade_timestamp > self.funding_time {
                self.funding_time = trade_timestamp + self.fees.funding_period;
                new_funding(
                    &self.fees,
                    self.position_amount,
                    &mut self.balance,
                    &mut self.executions,
                    prev_trades[0].price,
                    prev_trades[0].timestamp_ns,
                    market,
                    is_inverse,
                );
            }
        }
        self.orders.keep(|order| {
            let trigger_price = order.trigger_price.to_f32();
            if order.is_market() {
                order.timestamp_ns = prev_trades[0].timestamp_ns;
                new_trade(
                    market,
                    messages,
                    slippage,
                    fees,
                    order,
                    prev_trades[0].price,
                    balance,
                    position_amount,
                    pnl,
                    is_inverse,
                );
                return false;
            } else if order.is_stop_market() {
                if order.amount.is_sign_positive() {
                    if prev_trades[1].price > trigger_price {
                        //                        println!("risk short @ {:.6}", order.trigger_price);
                        order.timestamp_ns = prev_trades[0].timestamp_ns;
                        new_trade(
                            market,
                            messages,
                            slippage,
                            fees,
                            order,
                            trigger_price * 1.0006728571,
                            balance,
                            position_amount,
                            pnl,
                            is_inverse,
                        );
                        return false;
                    }
                } else if prev_trades[2].price < trigger_price {
                    //                    println!("risk short @ {:.6}", order.trigger_price);
                    order.timestamp_ns = prev_trades[0].timestamp_ns;
                    new_trade(
                        market,
                        messages,
                        slippage,
                        fees,
                        order,
                        trigger_price * (1. - 0.0006728571),
                        balance,
                        position_amount,
                        pnl,
                        is_inverse,
                    );
                    return false;
                }
            }
            true
        });

        Some(&trades[i..i + 4])
    }

    pub fn cancel_orders(&mut self, ids: &Vec<OrderId>) {
        for id in ids {
            trace!("Order canceled {:?}", id);
            if self.orders.keep(|open_order| open_order.id != *id) {
                error!(
                    r#"Canceling order that doesn't exist. (Is the model so bad that has lost money and placed an order to with value of 0?)
Existing orders: {:#?}, faulty order: {:#?}"#,
                    self.orders, ids
                );
                panic!("Fatal error.");
            }
        }
    }

    pub fn post_orders(&mut self, orders: &Vec<Order>) {
        trace!("Orders placed {:?}", orders);
        // First we process market orders then we porcess stop orders
        // There could be a scenario that on the same candle we enter and also get stopped out
        self.orders
            .extend(orders.iter().filter(|x| x.trigger_price.is_none()).cloned());
        self.orders
            .extend(orders.iter().filter(|x| x.trigger_price.is_some()).cloned());
    }

    pub fn get_executions(&mut self) -> &mut Vec<Execution> {
        &mut self.executions
    }

    pub fn get_funding_executions(&mut self) -> &mut Vec<FundingExecution> {
        &mut self.funding_executions
    }
}

fn generate_trades(candles: &Candles) -> (Vec<Trade>, usize) {
    fn construct_trade(i: usize, candles: &Candles, price: f32) -> Trade {
        Trade {
            // if trade timestamp is the same as candle timestamp then that trade is counted towards
            // new candle
            timestamp_ns: candles.timestamp[i] as u64 * 1_000_000_000 - 1,
            price,
            amount: candles.volume[i] / 4.,
        }
    }

    // Makes execution timestamp more accurate against real world execution timestamp
    // if trade timestamp is the same as candle timestamp then that trade is counted towards
    // new candle
    let offset = candles.timeframe_step() as u64 * 1_000_000_000 - 1;
    let mut trades = Vec::with_capacity(candles.len() * 4);
    for i in 0..candles.len() {
        trades.push(Trade {
            timestamp_ns: candles.timestamp[i] as u64 * 1_000_000_000 - offset,
            price: candles.open[i],
            amount: candles.volume[i] / 4.,
        });
        trades.push(Trade {
            timestamp_ns: candles.timestamp[i] as u64 * 1_000_000_000 - offset,
            price: candles.high[i],
            amount: candles.volume[i] / 4.,
        });
        trades.push(Trade {
            timestamp_ns: candles.timestamp[i] as u64 * 1_000_000_000 - offset,
            price: candles.low[i],
            amount: candles.volume[i] / 4.,
        });
        trades.push(Trade {
            timestamp_ns: candles.timestamp[i] as u64 * 1_000_000_000 - offset,
            price: candles.close[i],
            amount: candles.volume[i] / 4.,
        });
    }
    (trades, 4)
}

fn new_trade(
    market: &String,
    executions: &mut Vec<Execution>,
    _slippage: f32,
    fees: &Fees,
    order: &Order,
    executed_price: f32,
    balance: &mut Decimal,
    position_amount: &mut Decimal,
    pnl: &mut Decimal,
    is_inverse: bool,
) {
    let value = merovingian::order::value(
        executed_price.to_decimal().unwrap(),
        order.amount,
        is_inverse,
    );
    let execution = Execution {
        market: market.clone(),
        order_id: order.id,
        value,
        amount: order.amount,
        amount_left: Decimal::zero(),
        fee_paid: (value.abs() * fees.taker.to_decimal().unwrap()),
        executed_price: executed_price.to_decimal().unwrap(),
        timestamp_ns: order.timestamp_ns,
    };
    *balance -= execution.fee_paid;
    if position_amount.is_zero() {
        *pnl = value;
    } else if *position_amount > Decimal::zero() {
        *balance += value.abs() - pnl.abs();
    } else {
        *balance += pnl.abs() - value.abs();
    }
    let _margin = Margin {
        balance: *balance,
        leverage: Decimal::one(),
        timestamp_ns: 0,
    };
    info!(
        "Order executed {:?}, value: {}, balance: {}, fee_paid: {}",
        order, value, *balance, execution.fee_paid
    );
    executions.push(execution);
    // executions.push(Message::Balance(margin));
    // *BALANCE.lock().unwrap() = *balance;
    *position_amount += order.amount;
}

fn new_funding(
    fees: &Fees,
    position_amount: Decimal,
    balance: &mut Decimal,
    funding_executions: &mut Vec<FundingExecution>,
    price: f32,
    timestamp_ns: u64,
    market: &String,
    is_inverse: bool,
) {
    if position_amount.is_zero() {
        return;
    }
    let price = price.to_decimal().unwrap();
    let value = merovingian::order::value(price, position_amount, is_inverse);
    let fee_paid = value.abs() * fees.funding.to_decimal().unwrap();
    *balance -= fee_paid;
    trace!("funding, value: {} fee_paid: {}", value, fee_paid);
    funding_executions.push(FundingExecution {
        market: market.clone(),
        fee_paid,
        timestamp_ns,
    });
}

async fn load_and_insert_candles(
    timeframe: u32,
    data_path: impl AsRef<Path>,
    map: &mut HashMap<u32, Candles>,
    market: &str,
    funding_time: &mut u32,
    funding_period: u32,
) -> Result<()> {
    let path = data_path
        .as_ref()
        .join(&get_exchange_config().unwrap().name)
        .join("candles")
        .join(&market);
    let mut candles = Candles::read(path).await?;
    // candles.trim(
    //     candles.timeframe_step() * candles.len() as u32 / 2 + candles.timestamp[0],
    //     *candles.timestamp.last().unwrap(),
    // );
    // candles.trim(
    //     1622488001,
    //     // candles.timeframe_step() * candles.len() as u32 / 2 + candles.timestamp[0],
    //     *candles.timestamp.last().unwrap(),
    // );
    if candles.timestamp[0] < *funding_time {
        *funding_time =
            candles.timestamp[0] - candles.timestamp[0] % funding_period + funding_period;
    }
    let mut candles2 = candles.clone();
    if candles.timeframe_step() == timeframe {
        map.insert(timeframe, candles2);
    } else {
        let len = candles.increase_timeframe(&mut candles2, timeframe, false);
        candles2.truncate(len + 1);
        // Adding partial candle
        candles2.set_candle_partial(len);
        map.insert(timeframe, candles2);
    }
    Ok(())
}
