pub mod models;
pub mod schema;

use config::{get_exchange_config, CONFIG};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::{DatabaseErrorKind, Error};
use diesel::{insert_into, ExpressionMethods, MysqlConnection, QueryDsl, RunQueryDsl};
use itertools::Itertools;
use merovingian::minable_models::{Maintenance, Margin};
use merovingian::model_snapshot::ModelSnapshot;
use models::*;
use mouse::error::Result;
use mouse::log::*;
use mouse::num::traits::ToPrimitive;
use tokio::task::spawn_blocking;

use crate::diesel::mysql::Mysql;
use crate::mysql::models::{ExchangeConfig, ModelConfig, NewMaintenance, NewModelSource};

lazy_static::lazy_static! {
    static ref POOL: Pool<ConnectionManager<MysqlConnection>> = {
        let manager = ConnectionManager::new(&CONFIG.iaas.as_ref().expect("no iaas in config").the_matrix_db_url);
        let pool = Pool::builder().max_size(1).build(manager).unwrap();
        pool
    };
}

pub fn con() -> PooledConnection<ConnectionManager<MysqlConnection>> {
    POOL.get().unwrap()
}

pub fn get_model_source_id(model_name: &str) -> Result<u16> {
    use schema::models_source::*;
    Ok(*table
        .filter(name.eq(model_name))
        .select(id)
        .load::<u16>(&con())?
        .first()
        .unwrap())
}

pub fn load_configs(exchange_name: &str) -> Result<(ExchangeConfig, Vec<ModelConfig>)> {
    match get_exchange_config() {
        None => {
            let exchange_config = load_exchange_config(exchange_name)?;
            let model_configs = load_model_configs(exchange_config.id)?;
            Ok((exchange_config, model_configs))
        }
        Some(exchange) => Ok((
            exchange.clone().into(),
            ModelConfig::from_configs(&exchange.models),
        )),
    }
}

pub fn load_exchange_config(exchange_name: &str) -> Result<ExchangeConfig> {
    use schema::exchanges::*;
    Ok(table
        .filter(name.eq(exchange_name))
        .limit(1)
        .select((
            id,
            use_testnet,
            use_public_data_miner,
            api_key,
            api_secret,
            max_leverage,
            max_orders_per_m,
        ))
        .load::<ExchangeConfig>(&con())?
        .pop()
        .unwrap())
}

pub fn load_model_configs(exchange_id_: u16) -> Result<Vec<ModelConfig>> {
    use schema::market_models::*;
    use schema::models_values;
    Ok(table
        .filter(exchange_id.eq(exchange_id_))
        .inner_join(models_values::table)
        .select((
            id,
            market,
            target_leverage,
            models_values::model_source_id,
            models_values::variable_values,
        ))
        .load::<ModelConfig>(&con())?)
}

pub fn load_source(source_name: &str) -> Result<String> {
    use schema::models_source::*;
    Ok(String::from_utf8(
        table
            .filter(name.eq(source_name))
            .select(source)
            .load::<Vec<u8>>(&con())?
            .pop()
            .unwrap(),
    )?)
}

pub fn load_models_source(model_source_ids: impl Iterator<Item = u16>) -> Result<Vec<ModelSource>> {
    trace!("Querying model source...");
    use schema::models_source::*;
    let mut query = table.into_boxed::<Mysql>();
    let mut count = 0;
    for source_id in model_source_ids.unique() {
        count += 1;
        query = query.or_filter(id.eq(source_id));
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    Ok(query.load::<ModelSource>(&con())?)
}

pub fn load_all_models_source() -> Result<Vec<ModelSource>> {
    use schema::models_source::*;
    Ok(table.load::<ModelSource>(&con())?)
}

pub fn load_last_maintenance(exchange_id_: u16) -> Result<Option<Maintenance>> {
    use schema::maintenances::*;
    Ok(table
        .filter(exchange_id.eq(exchange_id_))
        .select((timestamp_s, mode))
        .order(timestamp_s.desc())
        .limit(1)
        .load::<MaintenanceQuery>(&con())?
        .into_iter()
        .next()
        .map(|x| {
            let maintenance: Maintenance = x.into();
            maintenance
        }))
}

pub fn insert_model_source(name: &str, source: &str) -> Result<()> {
    use schema::models_source::table;
    let s = NewModelSource {
        name,
        source: source.as_bytes(),
    };
    insert_into(table).values(&s).execute(&con())?;
    Ok(())
}

pub fn insert_model_values(model_source_id: u16, variable_values: &[u8]) -> Result<()> {
    use schema::models_values::table;
    let s = NewModelValues {
        model_source_id,
        variable_values,
    };
    insert_into(table).values(&s).execute(&con())?;
    Ok(())
}

pub async fn insert_position_open_snapshot(s: &ModelSnapshot, exchange_id: u16) -> Result<()> {
    use schema::position_open_snapshots::table;
    let s = NewPositionOpenSnapshot::new(s, exchange_id);
    spawn_blocking(move || {
        insert_into(table).values(&s).execute(&con())?;
        Ok(())
    })
    .await?
}

pub async fn insert_position_close_snapshot(s: &ModelSnapshot, exchange_id: u16) -> Result<()> {
    use schema::position_close_snapshots::table;
    let s = NewPositionCloseSnapshot::new(s, exchange_id);
    spawn_blocking(move || {
        insert_into(table).values(&s).execute(&con())?;
        Ok(())
    })
    .await?
}

pub async fn insert_funding_snapshot(s: &ModelSnapshot, exchange_id: u16) -> Result<()> {
    use schema::funding_snapshots::table;
    let s = NewFundingSnapshot::new(s, exchange_id);
    spawn_blocking(move || {
        insert_into(table).values(&s).execute(&con())?;
        Ok(())
    })
    .await?
}

pub async fn insert_maintenance_snapshot(s: &Maintenance, exchange_id: u16) -> Result<()> {
    use schema::maintenances::table;
    // If shutdown safe gets received we first send that we have received shutdown safe then we send
    // shutdown signal if conditions are met which could result in duplicate primary key.
    // Could also happen if error happens when writing maintenance to disk but disk is full but
    // entry is already in db. When kill switch detects an error we insert another entry.
    // Could also happen if we receive reload, when exiting we write and when booting we write again.
    // Diesel doesn't support tables without primary keys.
    let mut maintenance = NewMaintenance {
        timestamp_s: s.timestamp_s,
        exchange_id,
        mode: s.mode as u8,
    };
    spawn_blocking(move || loop {
        match insert_into(table).values(&maintenance).execute(&con()) {
            Ok(_) => return Ok(()),
            Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                maintenance.timestamp_s += 1;
            }
            Err(e) => return Err(e.into()),
        }
    })
    .await?
}

/// This function ignores if a row with primary key is already set.
/// Every time we connect with websocket we receive margin update with a timestamp of a last update
/// which means that when we insert it we already have the same one in db.
pub async fn insert_notunique_exchange_snapshot(margin: &Margin, exchange_id: u16) -> Result<()> {
    use schema::exchange_snapshots::table;
    let s = NewExchangeSnapshot {
        timestamp_ns: margin.timestamp_ns,
        exchange_id,
        balance: margin.balance.to_f32().unwrap(),
        leverage: margin.leverage.to_f32().unwrap(),
    };
    spawn_blocking(move || {
        match insert_into(table).values(&s).execute(&con()) {
            Ok(_) | Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {}
            Err(e) => return Err(e.into()),
        }
        Ok(())
    })
    .await?
}
