#![feature(async_stream)]

use bevy::prelude::*;
use bevy::utils::HashMap;
use bytecheck::CheckBytes;
use db::Db;
use mouse::ext::Extend;
use mouse::futures_util::{Stream, TryStreamExt};
use mouse::time::Utc;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use tokio::task::JoinHandle;

use crate::definitions::{
    Persistance, StaticEstimations, SystemKind, SystemLayout, SystemLayoutId, SystemLayoutWithId,
    ThreadUsage, TopicAccess, TopicConfig, TopicId, TopicLayout, TopicLayoutId, TopicLayoutWithId,
    TopicLifetime,
};
use crate::error::{ZionError, ZionResult};
use crate::{PluginLoader, Schedules, Stages, Zion, ZionPlug};

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct DbConnectedLabel;
pub struct DbPlugin;

impl ZionPlug for DbPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        let task = Db::connect().spawn();
        zion.set_stage(Stages::Main).add_startup_system(
            await_connection
                .config(|x| x.0 = Some(Some(task)))
                .label(DbConnectedLabel),
        )
    }
}

fn await_connection(mut task: Local<Option<JoinHandle<sqlx::Result<Db>>>>, mut commands: Commands) {
    let m: Option<Db> = task
        .take()
        .unwrap()
        .block()
        .unwrap()
        .log_context("failed to connect to database");
    commands.insert_resource(some!(m));
}

pub fn update_topic_layouts_modified_after_ts<'a>(
    db: &'a Db,
    timestamp_s: i64,
) -> impl Stream<Item = Result<(TopicLayoutId, TopicLayout), sqlx::Error>> + 'a {
    sqlx::query("SELECT * FROM zion.topic_layouts WHERE modified_s > ?")
        .bind(timestamp_s)
        .try_map(|row: MySqlRow| {
            let topic: DbTopic = DbTopic::from_row(&row)?;
            let topic: TopicLayoutWithId = topic.into();
            Ok((topic.id, topic.layout))
        })
        .fetch(db)
}

// pub fn update_system_layouts_modified_after_ts<'a>(
//    db: &'a Db,
//    timestamp_s: i64,
//) -> impl Stream<Item = Result<(SystemLayoutId, SystemLayout), sqlx::Error>> + 'a {
//    sqlx::query("SELECT * FROM zion.system_layouts WHERE modified_s > ?")
//        .bind(timestamp_s)
//        .try_map(|row: MySqlRow| {
//            let input_topics: &[u8] = row.try_get("input_topics")?;
//            let output_topics: &[u8] = row.try_get("output_topics")?;
//            let id: u64 = row.try_get("id")?;
//            let input_topics =
//                rkyv::check_archived_root::<Vec<DbTopic>>(input_topics).map_err(|_| {
//                    sqlx::error::Error::Decode(Box::new(ZionError::Other(anyhow!(
//                        "failed to decode input topic"
//                    ))))
//                })?;
//            let output_topics =
//                rkyv::check_archived_root::<Vec<DbTopic>>(output_topics).map_err(|_| {
//                    sqlx::error::Error::Decode(Box::new(ZionError::Other(anyhow!(
//                        "failed to decode output topic"
//                    ))))
//                })?;
//            let static_estimations = DbStaticEstimations::from_row(&row)?;
//            Ok((
//                SystemLayoutId(id),
//                SystemLayout {
//                    input_topics: input_topics.iter().map(|x| x.into()).collect(),
//                    output_topics: input_topics.iter().map(|x| x.into()).collect(),
//                    static_estimations: if static_estimations == Default::default() {
//                        None
//                    } else {
//                        Some(static_estimations.into())
//                    },
//                    kind: SystemKind::Bevy,
//                },
//            ))
//        })
//        .fetch(db)
//}

#[derive(sqlx::FromRow, rkyv::Archive, CheckBytes)]
#[archive_attr(derive(CheckBytes))]
struct DbTopic {
    id: u64,
    bits: u8,
}

macro_rules! impl_into_topic_layout {
    ($kind:ty) => {
        impl Into<TopicLayoutWithId> for $kind {
            fn into(self) -> TopicLayoutWithId {
                let access_mask: u8 = 1 << 7;
                let lifetime_mask: u8 = 1 << 6;
                let persistance_mask: u8 = 1 << 5;
                let config_mask: u8 = 0b00011111;
                TopicLayoutWithId {
                    layout: TopicLayout {
                        config: (self.bits & config_mask).into(),
                        lifetime: if self.bits & lifetime_mask != 0 {
                            TopicLifetime::Workflow
                        } else {
                            TopicLifetime::Global
                        },
                        access: if self.bits & access_mask != 0 {
                            TopicAccess::Public
                        } else {
                            TopicAccess::Private
                        },
                        persistance: if self.bits & persistance_mask != 0 {
                            Persistance::Storage
                        } else {
                            Persistance::RAM
                        },
                    },
                    id: TopicLayoutId(self.id),
                }
            }
        }
    };
}

impl_into_topic_layout!(&ArchivedDbTopic);
impl_into_topic_layout!(DbTopic);

#[derive(sqlx::FromRow, PartialEq)]
struct DbStaticEstimations {
    pub ram_usage_bytes: u64,
    // 0 = all threads
    pub thread_usage: u8,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub network_read_bytes: u64,
    pub network_write_bytes: u64,
}

impl Default for DbStaticEstimations {
    fn default() -> Self {
        Self {
            ram_usage_bytes: 0,
            thread_usage: ThreadUsage::SINGLE.0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            network_read_bytes: 0,
            network_write_bytes: 0,
        }
    }
}

impl Into<StaticEstimations> for DbStaticEstimations {
    fn into(self) -> StaticEstimations {
        StaticEstimations {
            ram_usage_bytes: self.ram_usage_bytes,
            thread_usage: ThreadUsage(self.thread_usage),
            io_read_bytes: self.io_read_bytes,
            io_write_bytes: self.io_write_bytes,
            network_read_bytes: self.network_read_bytes,
            network_write_bytes: self.network_write_bytes,
        }
    }
}
