// use bevy::utils::HashSet;
//
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct SystemLayoutId(u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct TopicId(u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct TopicLayoutId(u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct WorkflowLayoutId(u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct WorkflowId(u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
// pub struct SystemId(usize);
//
// pub enum WorkflowTopicId {
//    Global(TopicId),
//    Temp(TopicKind, usize),
//}
//
//// pub struct BevyTopicConfig {
////    pub layout_id: TopicLayoutId,
//// }
//
//#[derive(PartialEq, Debug)]
// pub enum TopicKind {
//    Bevy,
//    Async,
//}
//
//#[non_exhaustive]
// pub enum TopicConfig {
//    Bevy,
//}
// impl TopicConfig {
//    pub fn kind(&self) -> TopicKind {
//        match self {
//            TopicConfig::Bevy => TopicKind::Bevy,
//        }
//    }
//}
// pub struct ReaderConfig {
//    pub topic_id: WorkflowTopicId,
//    pub topic_layout: TopicLayoutWithId,
//}
// pub struct TopicLayoutWithId {
//    pub id: TopicLayoutId,
//    pub layout: TopicLayout,
//}
// pub struct TopicLayout {
//    pub config: TopicConfig,
//    pub lifetime: TopicLifetime,
//    pub access: TopicAccess,
//}
//
//#[derive(Eq, PartialEq)]
//#[non_exhaustive]
// pub enum TopicLifetime {
//    Workflow,
//}
//
//#[derive(Eq, PartialEq)]
// pub enum TopicAccess {
//    Private,
//    Public,
//}
// pub struct WriterConfig {
//    pub topic_id: WorkflowTopicId,
//    pub topic_layout: TopicLayoutWithId,
//}
// pub struct SystemLayout {
//    pub input_topics: Vec<TopicLayoutWithId>,
//    pub output_topics: Vec<TopicLayoutWithId>,
//    pub static_estimations: Option<StaticEstimations>,
//    pub kind: SystemKind,
//}
// pub enum SystemDeterminism {
//    /// Non deterministic: with same inputs provides different same result
//    NonDeterministic,
//    /// Deterministic: with same inputs provides same result
//    /// Without side effects: Impacts only output topics, must not impact other systems, clients,
//    /// databases...
//    DeterministicWithoutSideEffects,
//}
//
//#[non_exhaustive]
// pub enum SystemKind {
//    Bevy,
//}
// pub struct SystemSpawnConfig {
//    pub id: SystemLayoutId,
//    pub consts: Vec<u8>,
//    pub reader_topics: Vec<ReaderConfig>,
//    pub writer_topics: Vec<WriterConfig>,
//    /*    pub scale_factor: SpawnCondition,
//     * pub target_machine: Option<u64>, */
//}
//
//// pub enum SpawnCondition {
////    Cluster(u64),
////    Topic,
////    Machine(u64),
////    Graph,
////    Edge,
////    Event,
//// }
// pub struct WorkflowLayout {
//    pub systems: Vec<SystemSpawnConfig>,
//}
// pub enum CpuUsage {
//    SingleThreadedNs,
//    MultiThreadedNs { n_threads: u64 },
//    AllThreadedNs,
//}
// pub struct StaticEstimations {
//    pub ram_usage_bytes: u64,
//    pub cpu_time_ns: CpuUsage,
//    pub io_read_bytes: u64,
//    pub io_write_bytes: u64,
//    pub network_read_bytes: u64,
//    pub network_write_bytes: u64,
//}
//
//// pub struct TempTopicsConfig {
////    pub bevy: HashSet<TopicLayoutId>,
//// }

use bevy::prelude::*;
use bevy::utils::HashSet;
use num_enum::FromPrimitive;

use crate::Stages;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct SystemLayoutId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct TopicId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct TopicLayoutId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct WorkflowLayoutId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct WorkflowId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct SystemId(pub usize);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct Ids {
    pub layout: u64,
    pub id: u64,
    pub entity: Entity,
}

// pub enum WorkflowTopicId {
//    Global(TopicId),
//    Temp(usize),
//}

// pub struct BevyTopicConfig {
//    pub layout_id: TopicLayoutId,
//}

#[derive(PartialEq, Debug)]
pub enum TopicKind {
    Bevy,
    Async,
}

#[repr(u8)]
#[derive(FromPrimitive)]
#[non_exhaustive]
#[derive(Debug)]
pub enum TopicConfig {
    #[default]
    Bevy,
}

impl TopicConfig {
    pub fn kind(&self) -> TopicKind {
        match self {
            TopicConfig::Bevy => TopicKind::Bevy,
        }
    }
}

pub struct ReaderConfig {
    pub topic_id: TopicId,
    pub topic_layout_id: TopicLayoutId,
}

#[derive(Debug, Clone)]
pub struct SystemTopicConfig {
    pub topic_id: TopicId,
    pub topic_layout_id: TopicLayoutId,
}

#[derive(Debug)]
pub struct TopicLayoutWithId {
    pub id: TopicLayoutId,
    pub layout: TopicLayout,
}

#[derive(Debug)]
pub struct TopicLayout {
    pub config: TopicConfig,
    pub lifetime: TopicLifetime,
    pub access: TopicAccess,
    pub persistance: Persistance,
}

#[derive(Debug)]
pub enum Persistance {
    RAM,
    Storage,
}

#[repr(u8)]
#[derive(Eq, PartialEq, FromPrimitive, Debug)]
#[non_exhaustive]
pub enum TopicLifetime {
    #[default]
    Global,
    Workflow,
}

#[repr(u8)]
#[derive(Eq, PartialEq, FromPrimitive, Debug)]
pub enum TopicAccess {
    #[default]
    Private,
    Public,
}

pub struct WriterConfig {
    pub topic_id: TopicId,
    pub topic_layout: TopicLayoutWithId,
}

pub struct SystemLayoutWithId {
    pub layout: SystemLayout,
    pub id: SystemLayoutId,
}

pub struct SystemLayout {
    pub stage: Stages,
    pub input_topics: Vec<TopicLayoutWithId>,
    pub output_topics: Vec<TopicLayoutWithId>,
    pub static_estimations: Option<StaticEstimations>,
    pub kind: SystemKind,
}

pub enum SystemDeterminism {
    /// Non deterministic: with same inputs provides different same result
    NonDeterministic,
    /// Deterministic: with same inputs provides same result
    /// Without side effects: Impacts only output topics, must not impact other systems, clients,
    /// databases...
    DeterministicWithoutSideEffects,
}

#[non_exhaustive]
pub enum SystemKind {
    Bevy,
}

pub struct SystemSpawnConfig {
    pub id: SystemLayoutId,
    pub consts: Vec<u8>,
    pub reader_topics: Vec<ReaderConfig>,
    pub writer_topics: Vec<WriterConfig>,
    /*    pub scale_factor: SpawnCondition,
     * pub target_machine: Option<u64>, */
}

// pub enum SpawnCondition {
//    Cluster(u64),
//    Topic,
//    Machine(u64),
//    Graph,
//    Edge,
//    Event,
//}

pub struct WorkflowLayout {
    pub systems: Vec<SystemSpawnConfig>,
}

pub struct StaticEstimations {
    pub ram_usage_bytes: u64,
    pub thread_usage: ThreadUsage,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub network_read_bytes: u64,
    pub network_write_bytes: u64,
}

#[derive(new)]
pub struct ThreadUsage(pub u8);

impl ThreadUsage {
    pub const ALL: ThreadUsage = ThreadUsage(0);
    pub const SINGLE: ThreadUsage = ThreadUsage(1);
}

impl Default for ThreadUsage {
    fn default() -> Self {
        ThreadUsage::SINGLE
    }
}

// pub struct TempTopicsConfig {
//    pub bevy: HashSet<TopicLayoutId>,
//}
