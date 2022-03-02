use std::collections::HashMap;
use std::convert::TryInto;

use itertools::Itertools;
use merovingian::minable_models::Maintenance;
use merovingian::model_snapshot::ModelSnapshot;
use merovingian::speedy::Readable;
use merovingian::Writable;

use super::schema::*;
use crate::mouse::num::traits::ToPrimitive;

#[derive(Queryable, Debug, Clone)]
#[from(config::ExchangeConfig)]
pub struct ExchangeConfig {
    #[from(with = "u16::MAX")]
    pub id: u16,
    pub use_testnet: bool,
    pub use_public_data_miner: bool,
    pub api_key: String,
    pub api_secret: String,
    #[from(with = "other.max_leverage")]
    pub max_leverage: f32,
    pub max_orders_per_m: f32,
}

#[derive(Queryable, Debug)]
pub struct ModelConfig {
    pub market_model_id: u32,
    pub market: String,
    pub target_leverage: f32,
    pub model_source_id: u16,
    pub serialized_variable_values: Vec<u8>,
}

impl ModelConfig {
    pub fn from_configs(configs: &Vec<config::ModelConfig>) -> Vec<Self> {
        let map: HashMap<_, _> = configs
            .iter()
            .unique_by(|x| &x.name)
            .enumerate()
            .map(|(i, x)| (&x.name, i as u16))
            .collect();
        configs
            .iter()
            .map(|x| ModelConfig {
                market_model_id: u32::MAX,
                market: x.market.clone(),
                target_leverage: x.target_leverage,
                model_source_id: *map.get(&x.name).unwrap(),
                serialized_variable_values: Writable::write_to_vec(&x.variable_values).unwrap(),
            })
            .collect()
    }
    pub fn variable_values(&self) -> Vec<f32> {
        Readable::read_from_buffer(&self.serialized_variable_values).unwrap()
    }
}

#[derive(Queryable)]
pub struct ModelSource {
    pub id: u16,
    pub name: String,
    pub source: Vec<u8>,
}

#[derive(Queryable)]
#[into(Maintenance)]
pub struct MaintenanceQuery {
    timestamp_s: u32,
    #[into(with = "self.mode.try_into().unwrap()")]
    mode: u8,
}

macro_rules! query_id {
    ($($kind:ty)+) => {
        paste! {
            $(
                #[derive(Queryable)]
                pub struct [<ID $kind>] {
                    pub id: $kind,
                }
            )+
        }
    }
}

query_id!(u8 u16 u32 u64 i8 i16 i32 i64);

#[derive(Insertable)]
#[table_name = "models_source"]
pub struct NewModelSource<'a> {
    pub name: &'a str,
    pub source: &'a [u8],
}

#[derive(Insertable)]
#[table_name = "models_values"]
pub struct NewModelValues<'a> {
    pub model_source_id: u16,
    pub variable_values: &'a [u8],
}

#[derive(Insertable)]
#[table_name = "market_models"]
pub struct NewMarketModel<'a> {
    pub exchange_id: u16,
    pub model_values_id: u32,
    pub market: &'a str,
}

#[derive(Insertable)]
#[table_name = "exchanges"]
pub struct NewExchange<'a> {
    pub name: &'a str,
}

#[derive(Insertable)]
#[table_name = "maintenances"]
pub struct NewMaintenance {
    pub timestamp_s: u32,
    pub exchange_id: u16,
    pub mode: u8,
}

#[derive(Insertable)]
#[table_name = "exchange_snapshots"]
pub struct NewExchangeSnapshot {
    pub timestamp_ns: u64,
    pub exchange_id: u16,
    pub balance: f32,
    pub leverage: f32,
}

#[derive(Insertable)]
#[table_name = "position_close_snapshots"]
pub struct NewPositionCloseSnapshot {
    pub timestamp_ns: u64,
    pub market_model_id: u32,
    pub exchange_id: u16,
    pub balance: f32,
    pub expected_amount: f32,
    pub actual_amount: f32,
    pub expected_price: f32,
    pub rounded_price: f32,
    pub actual_price: f32,
}

#[derive(Insertable)]
#[table_name = "position_open_snapshots"]
pub struct NewPositionOpenSnapshot {
    pub timestamp_ns: u64,
    pub market_model_id: u32,
    pub exchange_id: u16,
    pub balance: f32,
    pub expected_amount: f32,
    pub actual_amount: f32,
    pub expected_price: f32,
    pub rounded_price: f32,
    pub actual_price: f32,
}

#[derive(Insertable)]
#[table_name = "funding_snapshots"]
pub struct NewFundingSnapshot {
    pub timestamp_ns: u64,
    pub market_model_id: u32,
    pub exchange_id: u16,
    pub balance: f32,
}

impl NewPositionOpenSnapshot {
    pub fn new(s: &ModelSnapshot, exchange_id: u16) -> Self {
        NewPositionOpenSnapshot {
            timestamp_ns: s.timestamp_ns,
            market_model_id: s.market_model_id,
            exchange_id,
            balance: s.balance.to_f32().unwrap(),
            actual_amount: s.position_snapshot.execution_snapshot.actual_amount,
            expected_amount: s.position_snapshot.execution_snapshot.expected_amount,
            expected_price: s.position_snapshot.execution_snapshot.expected_price,
            rounded_price: s.position_snapshot.execution_snapshot.rounded_price,
            actual_price: s.position_snapshot.execution_snapshot.actual_price,
        }
    }
}

impl NewPositionCloseSnapshot {
    pub fn new(s: &ModelSnapshot, exchange_id: u16) -> Self {
        NewPositionCloseSnapshot {
            timestamp_ns: s.timestamp_ns,
            market_model_id: s.market_model_id,
            exchange_id,
            balance: s.balance.to_f32().unwrap(),
            actual_amount: s.position_snapshot.execution_snapshot.actual_amount,
            expected_amount: s.position_snapshot.execution_snapshot.expected_amount,
            expected_price: s.position_snapshot.execution_snapshot.expected_price,
            rounded_price: s.position_snapshot.execution_snapshot.rounded_price,
            actual_price: s.position_snapshot.execution_snapshot.actual_price,
        }
    }
}

impl NewFundingSnapshot {
    pub fn new(s: &ModelSnapshot, exchange_id: u16) -> Self {
        NewFundingSnapshot {
            timestamp_ns: s.timestamp_ns,
            market_model_id: s.market_model_id,
            exchange_id,
            balance: s.balance.to_f32().unwrap(),
        }
    }
}
