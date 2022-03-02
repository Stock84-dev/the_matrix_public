use bevy::prelude::*;

use crate::definitions::{SystemLayoutId, TopicKind, TopicLayoutId};
use crate::SpawnSystem;

pub type ZionResult<T> = Result<T, ZionError>;

#[derive(Error, Debug)]
#[error("{0:#?}")]
pub enum ZionError {
    #[error("failed to spawn system from {1:#?}, caused by {0:#?}")]
    FailedToSpawnSystem(Box<ZionError>, SpawnSystem),

    #[error("system already exists")]
    SystemAlreadyExists,
    #[error("unregistered system id {0:#?}")]
    UnknownSystem(SystemLayoutId),
    #[error("unregistered topic id {0:#?}")]
    UnknownTopicLayout(TopicLayoutId),
    #[error(
        "system expected TopicKind::{:#?}, config provided TopicKind::{:#?}",
        system,
        config
    )]
    TopicReaderLayoutError {
        system: TopicKind,
        config: TopicKind,
    },
    #[error("not enough readers provided (provided: {0:})")]
    NotEnoughTopicReaders(usize),
    #[error(
        "system expected TopicKind::{:#?}, config provided TopicKind::{:#?}",
        system,
        config
    )]
    TopicWriterLayoutError {
        system: TopicKind,
        config: TopicKind,
    },
    #[error("not enough writers provided (provided: {0:})")]
    NotEnoughTopicWriters(usize),
    #[error("transaction rolled back successfully: {0:#?}")]
    TransactionRolledBackSuccessfully(Box<ZionError>),
    #[error("failed to spawn topics {0:#?}")]
    FailedToSpawnTopics(Box<ZionError>),
    #[error("failed to spawn topics {0:#?}")]
    FailedToSpawnSystems(Box<ZionError>),
    #[error("transaction failed to roll back: {transaction_error:#?}, rollback cause: {cause:#?}")]
    TransactionFailedToRollback {
        transaction_error: Box<ZionError>,
        cause: Box<ZionError>,
    },
    Critical(Box<ZionError>),
    Other(#[from] bevy::prelude::Error),
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("system needs to shutdown")]
    ShutdownRequested,
}
