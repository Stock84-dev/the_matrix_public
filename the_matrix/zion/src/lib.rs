#![feature(async_stream)]
#![feature(map_try_insert)]
#![feature(const_trait_impl)]
#![feature(option_result_unwrap_unchecked)]
#![feature(try_blocks)]

// pub mod listener;
#[macro_use]
mod macros;
mod error;
mod reactor;
#[macro_use]
mod system;
mod definitions;
mod schedule;
// mod error;
mod db;
mod hello;
mod topic;
mod transmitter;
// mod workflow;

mod prelude {
    pub use bevy::prelude::*;
    // If schedule is rebuild then these trackers could trigger systems thus making them
    // private
    use bevy::prelude::{Added, Changed, RemovedComponents};
}

// use std::collections::{HashMap, VecDeque};
// use std::marker::PhantomData;
// use std::ops::Deref;
// use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
// use std::sync::Arc;
//
// use bevy::prelude::*;
//// bytecheck can be used to validate your data if you want
// use bytecheck::CheckBytes;
// use futures_util::future::{BoxFuture, FutureExt};
// pub use merovingian::minable_models::MaintenanceMode;
// use merovingian::non_minable_models::MatrixId;
// use merovingian::speedy;
// use merovingian::speedy::{Readable, Writable};
// use mouse::error::UnsafeSendSyncError;
// use mouse::prelude::*;
// use mouse::sync::{AsyncMutex, Mutex, RwLock, RwLockReadGuard};
// use rkyv::validation::validators::DefaultValidator;
// use rkyv::{Archive, Deserialize, Serialize};
// use serde::de::DeserializeOwned;
// use tokio::sync::Notify;
// use crate::db::definitions::{StaticEstimations, SystemSpawnConfig};
// use crate::db::error::{ZionError, ZionResult};

//
//// use crate::definitions::StaticEstimations;
//// use crate::error::{SystemError, ZionError, ZionResult};
// use crate::topic::tmp::RamTopic;
//
// pub struct Zion {
//    //    system_defs: HashMap<u64, SystemDef>,
//}
//
//// impl Zion {
////    pub fn system<S: System>(&mut self) -> ZionResult<&mut Self> {
////        if self
////            .system_defs
////            .try_insert(S::ID, SystemDef::new::<S>())
////            .is_err()
////        {
////            return Err(ZionError::SystemAlreadyExists);
////        }
////        Ok(self)
////    }
//// }
////
// pub struct TopicConfig {
//    max_retention_ms: u64,
//    max_retention_size: u64,
//    max_retention_items: u64,
//    in_memory_storage: bool,
//    local: bool,
//}
// pub struct ReaderConfig {
//    topic_id: u64,
//    kind: ReaderKind,
//}
// pub struct WriterConfig {
//    topic_id: u64,
//    kind: ReaderKind,
//}
// pub struct SystemConfig {
//    consts: Vec<u8>,
//    input_topics: Vec<ReaderConfig>,
//    onput_topics: Vec<WriterConfig>,
//    scale_factor: SpawnCondition,
//    target_machine: Option<u64>,
//}
// impl SystemConfig {
//    pub fn validate_consts_rkyv<'a, T: Archive>(&'a self) -> Result<&'a T::Archived>
//    where
//        T::Archived: CheckBytes<DefaultValidator<'a>>,
//    {
//        Ok(rkyv::check_archived_root::<T>(&self.consts)
//            .map_err(|e| unsafe { UnsafeSendSyncError::new(e) })?)
//    }
//}
// pub enum SpawnCondition {
//    Cluster(u64),
//    Topic,
//    Machine(u64),
//    Graph,
//    Edge,
//    Event,
//}
// pub struct WorkflowConfig {
//    tmp_topics: HashMap<u64, TopicConfig>,
//    systems: HashMap<u64, SystemConfig>,
//}
// pub struct Zion {
//    app: RwLock<bevy::app::App>,
//}
// impl Zion {}
//
//#[derive(Deref)]
// pub struct SystemExecutors(RwLock<Vec<SystemExecutor>>);
//#[derive(Deref)]
// pub struct SystemConfigs(RwLock<HashMap<u64, SystemConfigs>>);
//
// struct ZionPlugin;
//
// impl Plug for ZionPlugin {
//    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
//        loader
//    }
//
//    fn load<'a>(app: &'a mut bevy::app::App) -> &mut bevy::app::App {}
//}
// struct SpawnSystem {
//    pub id: SystemId,
//    pub config: SystemConfig,
//}
// fn system_spawner(
//    map: Res<SystemIdToEntity>,
//    reader: PortalReader<SpawnSystem>,
//    system_defs: Query<&SystemDef>,
//) {
//    for e in reader.read().iter() {}
//}
// pub type ZionArc = Arc<Mutex<Zion>>;
//
// struct SystemExecutor {
//    despawn: Arc<AtomicBool>,
//    /*        id: u64,
//     * static_estimations: Option<StaticEstimations>, */
//}
// impl SystemExecutor {
//    fn spawn(zion: &ZionArc, system_id: u64, config: &SystemConfig) -> ZionResult<SystemExecutor>
// {        let guard = zion.lock();
//        if !guard.app.system_defs.contains_key(&system_id) {
//            return Err(ZionError::UnknownSystem(system_id));
//        }
//        let executor = SystemExecutor {
//            despawn: Default::default(),
//        };
//        Self::execute(zion.clone(), executor.despawn.clone()).spawn();
//        Ok(executor)
//    }
//
//    async fn execute(zion: ZionArc, should_despawn: Arc<AtomicBool>) {
//        let mut system = self.system.take().unwrap();
//        let executor = self.clone();
//        executor.execute(system).spawn();
//        loop {
//            match system.run().await {
//                Ok(_) => {}
//                Err(e) => match e.downcast_ref::<SystemError>() {
//                    None => {
//                        unimplemented!()
//                    }
//                    Some(SystemError::ShutdownRequested) => {
//                        break;
//                    }
//                },
//            }
//        }
//        system
//            .despawn()
//            .await
//            .log_context("failed to despawn system");
//    }
//}
// struct SystemId(u64);
//
// struct SystemIdToEntity(HashMap<SystemId, Entity>);
//
//#[derive(Component)]
// struct SystemDef {
//    static_estimations: Option<StaticEstimations>,
//    validate_fn: fn(&SystemConfig) -> Result<()>,
//    new_fn: for<'z> fn(&'z mut SystemBuilder<'z>) -> BoxFuture<'z, Result<Box<dyn DynSystem>>>,
//}
// impl SystemDef {
//    fn new<S: System>() -> SystemDef {
//        fn wrap_new<'z, S: System>(
//            builder: &'z mut SystemBuilder<'z>,
//        ) -> BoxFuture<'z, Result<Box<dyn DynSystem>>> {
//            async {
//                let b: Box<dyn DynSystem> = Box::new(S::new(builder).await?);
//                Ok(b)
//            }
//            .boxed()
//        }
//
//        SystemDef {
//            static_estimations: S::STATIC_ESTIMATIONS,
//            validate_fn: S::validate,
//            new_fn: wrap_new::<S>,
//        }
//    }
//
//    fn validate(&self, config: &SystemConfig) -> Result<()> {
//        (self.validate_fn)(config)
//    }
//
//    async fn spawn(&self, config: &SystemConfig) -> Result<SystemExecutor> {
//        let mut builder = SystemBuilder::new(config);
//        let system = (self.new_fn)(&mut builder).await?;
//        let mut executor = SystemExecutor::new(system);
//        executor.spawn();
//        unimplemented!();
//        //            Ok(executor)
//    }
//}

// pub struct SystemBuilder<'z> {
//    config: &'z SystemSpawnConfig,
//    current_input_topic: usize,
//    current_output_topic: usize,
//}
// impl<'z> SystemBuilder<'z> {
//    pub fn new(config: &'z SystemSpawnConfig) -> Self {
//        Self {
//            config,
//            current_input_topic: 0,
//            current_output_topic: 0,
//        }
//    }
//}
// pub enum ReaderKind {
//    Local,
//}
// impl<'z> SystemBuilder<'z> {
//    pub fn read_consts_rkyv<'a, T: Archive>(&'a self) -> &'a T::Archived {
//        unsafe { rkyv::archived_root::<T>(&self.config.consts) }
//    }
//
//    //        pub fn local_reader<T>(&mut self) -> LocalReader<T> {}
//}
//
//#[async_trait]
// pub trait System: Sized + Send + Sync + 'static {
//    const ID: u64;
//    const STATIC_ESTIMATIONS: Option<StaticEstimations>;
//    fn validate(config: &SystemSpawnConfig) -> Result<()>;
//    async fn new(config: &mut SystemBuilder) -> Result<Self>;
//    // if system returns early reactor will get a chance to inspect system usage
//    async fn run(&mut self) -> Result<()>;
//    async fn despawn(self) -> Result<()>;
//    fn name() -> &'static str {
//        std::any::type_name::<Self>()
//    }
//}
//
//#[async_trait]
// pub trait DynSystem: Send + Sync {
//    // if system returns early reactor will get a chance to inspect system usage
//    async fn run(&mut self) -> Result<()>;
//    async fn despawn(self: Box<Self>) -> Result<()>;
//}
//
//#[async_trait]
// impl<S: System> DynSystem for S {
//    async fn run(&mut self) -> Result<()> {
//        System::run(self).await
//    }
//
//    async fn despawn(self: Box<Self>) -> Result<()> {
//        System::despawn(*self).await
//    }
//}

//#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
//// This will generate a PartialEq impl between our unarchived and archived types
//#[archive(compare(PartialEq))]
//// To use the safe API, you have to derive CheckBytes for the archived type
//#[archive_attr(derive(CheckBytes, Debug))]
// pub struct StageConfig {
//    id: u64,
//}
// pub struct StageSystem {
//    id: u64,
//}
//
////    #[async_trait]
////    impl System for StageSystem {
////        const ID: u64 = 1;
////
////        fn validate(config: &SystemConfig) -> Result<()> {
////            config.validate_consts_rkyv::<StageConfig>()?;
////            Ok(())
////        }
////
////        async fn new(config: &SystemBuilder) -> Result<Self> {
////            todo!()
////        }
////
////        async fn run(&mut self) -> Result<()> {
////            todo!()
////        }
////
////        async fn despawn(self) -> Result<()> {
////            todo!()
////        }
////    }
// pub struct MailBoxConfig {
//    kind: MailBoxKind,
//}
// pub enum MailBoxKind {
//    Local,
//    Remote,
//}
// struct Pipeline {
//    id: u64,
//    outputs: Vec<Vec<u64>>,
//}
// pub enum MailBoxID {
//    Local(u64),
//    Remote(u64),
//}
// pub struct Message {
//    data: Vec<u8>,
//    pipeline_id: u64,
//    // we could have multiple mailboxes with same id
//    node_id: u64,
//}
// lazy_static! {
//    pub static ref MAILBOXES: AsyncMutex<HashMap<u64, Vec<Message>>> = Default::default();
//}
// trait MailBox {
//    const ID: u64;
//}
// pub struct Semaphore {
//    // if 0 then block
//    capacity: u64,
//}
// pub struct SemaphoreSystem;

use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter, Pointer, Write};
use std::future::Future;
use std::hash::Hasher;
use std::mem::swap;
use std::ops::Add;
use std::pin::Pin;
use std::sync::Arc;

use bevy::ecs::schedule::{IntoSystemDescriptor, RunOnce};
use bevy::log::{LogPlugin, LogSettings};
use bevy::prelude::*;
use bevy::utils::label::{DynEq, DynHash};
use bevy::utils::{HashMap, HashSet};
use mouse::smallbox::space::S1;
use mouse::smallbox::SmallBox;
use mouse::sync::RwLock;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::db::DbPlugin;
use crate::definitions::{SystemId, SystemLayout};
use crate::error::ZionResult;
use crate::hello::{Hello, HelloPlugin};
use crate::system::{
    AddSystem, SpawnSystem, SpawnSystemInner, SystemBuilder, SystemComponent, SystemData,
    SystemDefs, SystemFactory, SystemFactoryContainer, SystemPlugin,
};
use crate::topic::mem::{MemTopic, RawTopicReader, ResTopicReader};
use crate::topic::{TopicPlugin, TopicSystems};

pub enum Schedules {
    Pre = 0,
    Main = 1,
    Post = 2,
}

#[derive(Clone, Copy)]
pub enum Stages {
    /// Name of app stage responsible for performing setup before an update. Runs before MAIN.
    PreMain,
    /// Name of app stage responsible for doing most app logic. Systems should be registered here
    /// by default.
    Main,
    /// Name of app stage responsible for processing the results of MAIN. Runs after MAIN.
    PostMain,
}

impl Stages {
    fn into_startup(&self) -> StageLabelContainer {
        match self {
            Stages::PreMain => StageLabelContainer::new(StartupStage::PreStartup),
            Stages::Main => StageLabelContainer::new(StartupStage::Startup),
            Stages::PostMain => StageLabelContainer::new(StartupStage::PostStartup),
        }
    }

    fn into_update(&self) -> StageLabelContainer {
        match self {
            Stages::PreMain => StageLabelContainer::new(CoreStage::PreUpdate),
            Stages::Main => StageLabelContainer::new(CoreStage::Update),
            Stages::PostMain => StageLabelContainer::new(CoreStage::PostUpdate),
        }
    }
}

pub struct GlobalEntity(pub Entity);

pub struct Zion {
    schedules: [Schedule; 3],
    current_stage: Stages,
    current_schedule: usize,
    loaded_plugins: HashSet<TypeId>,
    world: World,
}

impl Zion {
    pub fn new() -> Self {
        let mut zion = Self {
            schedules: [Default::default(), Default::default(), Default::default()],
            current_stage: Stages::Main,
            current_schedule: Schedules::Post as usize,
            loaded_plugins: Default::default(),
            world: Default::default(),
        };
        let id = zion.world.spawn().id();
        zion.world.insert_resource(GlobalEntity(id));
        for schedule in &mut zion.schedules {
            add_default_stages(schedule);
        }
        let mut plugin_loader = PluginLoader { zion: &mut zion };
        plugin_loader
            .load(DbPlugin)
            .load(TopicPlugin)
            .load(SystemPlugin);
        zion
    }

    pub fn add_plugin(&mut self, plugin: impl ZionPlug) -> &mut Self {
        let mut plugin_loader = PluginLoader { zion: self };
        plugin_loader.load(plugin);
        self
    }

    pub fn add_legacy_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        let mut app = App::empty();
        swap(&mut app.world, &mut self.world);
        swap(
            &mut app.schedule,
            &mut self.schedules[Schedules::Post as usize],
        );
        app.add_plugin(plugin);
        swap(&mut app.world, &mut self.world);
        swap(
            &mut app.schedule,
            &mut self.schedules[Schedules::Post as usize],
        );
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_local_topic<T: Resource>(&mut self) -> &mut Self {
        let id = self.world.get_resource::<GlobalEntity>().unwrap().0;
        let mut topic_systems = self.world.get_resource_mut::<TopicSystems>().unwrap();
        let system = MemTopic::<T>::update_system;
        if topic_systems.0.insert(system as usize) {
            self.schedules[Schedules::Post as usize]
                .add_system_to_stage(StageLabelContainer::new(CoreStage::Last), system);
        }
        let topic = MemTopic::<T>::new();
        self.world.entity_mut(id).insert(topic);
        self
    }

    pub fn init_resource<R: FromWorld + Send + Sync + 'static>(&mut self) -> &mut Self {
        let resource = R::from_world(&mut self.world);
        self.world.insert_resource(resource);
        self
    }

    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    //    pub fn add_raw_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
    //        self.inner.add_plugin(plugin);
    //        self
    //    }

    pub fn set_schedule(&mut self, schedule: Schedules) -> &mut Self {
        self.current_schedule = schedule as usize;
        self
    }

    pub fn set_stage(&mut self, stage: Stages) -> &mut Self {
        self.current_stage = stage;
        self
    }

    pub fn register_system<S: SystemFactory>(&mut self) -> &mut Self {
        let mut defs = self.world.get_resource_mut::<SystemDefs>().unwrap();
        let name = S::struct_name();
        if defs
            .0
            .try_insert(
                name,
                SystemFactoryContainer {
                    new: S::new,
                    layout: S::layout(),
                },
            )
            .is_err()
        {
            panic!("system `{}` already registered", name);
        }
        self
    }

    pub fn add_system<Params>(&mut self, desc: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.schedules[self.current_schedule].stage(
            self.current_stage.into_update(),
            |stage: &mut SystemStage| stage.add_system(desc),
        );
        self
    }

    pub fn add_startup_system<Params>(
        &mut self,
        desc: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        let stage = self.current_stage.into_startup();
        println!(
            "{:?}",
            self.schedules[self.current_schedule]
                .get_stage::<Schedule>(&CoreStage::Startup)
                .unwrap()
                .get_stage::<SystemStage>(&StageLabelContainer::new(StartupStage::Startup))
                .is_some()
        );
        println!(
            "{:?}",
            self.schedules[self.current_schedule]
                .get_stage::<Schedule>(&CoreStage::Startup)
                .unwrap()
                .get_stage::<SystemStage>(&CoreStage::Startup)
                .is_some()
        );
        println!(
            "{:?}",
            self.schedules[self.current_schedule]
                .get_stage::<Schedule>(&CoreStage::Startup)
                .unwrap()
                .get_stage::<SystemStage>(&StartupStage::Startup)
                .is_some()
        );
        self.schedules[self.current_schedule].stage(
            CoreStage::Startup,
            |schedule: &mut Schedule| {
                println!("begin");
                for (label, stage) in schedule.iter_stages() {
                    println!("stage");
                }
                println!("{:?}", stage);
                schedule.add_system_to_stage(stage, desc)
            },
        );
        println!("finish");
        self
    }

    /// Runs startup stages, does nothing if already called
    pub fn startup(&mut self) {
        let mut world = &mut self.world;
        for schedule in &mut self.schedules {
            schedule.stage(CoreStage::Startup, |schedule: &mut Schedule| {
                schedule.run(world);
                schedule
            });
        }
    }

    pub fn run(&mut self) {
        let mut system_state: SystemState<(ResTopicReader<AddSystem>,)> =
            SystemState::new(&mut self.world);
        let topic = RawTopicReader::from(system_state.get_mut(&mut self.world).0);
        self.startup();
        loop {
            for schedule in &mut self.schedules {
                schedule.run(&mut self.world);
            }

            if self.world.remove_resource::<Reschedule>().is_some() {
                self.schedules[Schedules::Main as usize] = Schedule::default();
                let mut query = self.world.query::<(&SystemComponent, &SystemData)>();
                for (component, data) in query.iter(&self.world) {
                    let defs = self.world.get_resource::<SystemDefs>().unwrap();
                    let stage = defs.0.get(&data.name).unwrap().layout.stage;
                    self.schedules[Schedules::Main as usize]
                        .add_system_to_stage(stage.into_update(), component.factory.system());
                }
            } else {
                let mut system_state: SystemState<(
                    Res<SystemDefs>,
                    Query<(&SystemComponent, &SystemData)>,
                )> = SystemState::new(&mut self.world);
                let (defs, query) = system_state.get_mut(&mut self.world);
                if let Some(commands) = topic.try_read() {
                    for system in commands.read_all() {
                        let (component, data) = query.get(system.entity).unwrap();
                        let stage = defs.0.get(&data.name).unwrap().layout.stage;

                        self.schedules[Schedules::Main as usize]
                            .add_system_to_stage(stage.into_update(), component.factory.system());
                        println!("added");
                    }
                }; // first drop temp variables from if statement then outer block
            }
            if self.world.remove_resource::<Exit>().is_some() {
                break;
            }
        }
    }
}
pub struct Reschedule;
pub struct Exit;

pub struct PluginLoader<'a> {
    zion: &'a mut Zion,
}
impl<'a> PluginLoader<'a> {
    pub fn load<T: ZionPlug + 'static>(&mut self, mut plugin: T) -> &mut Self {
        if self.zion.loaded_plugins.contains(&T::id()) {
            return self;
        }
        plugin.deps(self);
        info!("Loading: {}", std::any::type_name::<T>());
        self.zion.current_stage = Stages::Main;
        self.zion.current_schedule = Schedules::Post as usize;
        plugin.load(self.zion);
        self.zion.loaded_plugins.insert(T::id());
        self
    }
}
pub trait ZionPlug: 'static {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b>;
    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion;
}

struct StageLabelContainer(Box<dyn StageLabel>);

impl StageLabelContainer {
    pub fn new(label: impl StageLabel) -> Self {
        Self(Box::new(label))
    }
}

impl std::fmt::Debug for StageLabelContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("container")
        //        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl DynEq for StageLabelContainer {
    fn as_any(&self) -> &dyn Any {
        DynEq::as_any(&self.0)
    }

    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        DynEq::dyn_eq(&self.0, other)
    }
}

impl DynHash for StageLabelContainer {
    fn as_dyn_eq(&self) -> &dyn DynEq {
        DynHash::as_dyn_eq(&self.0)
    }

    fn dyn_hash(&self, state: &mut dyn Hasher) {
        DynHash::dyn_hash(&self.0, state)
    }
}

impl StageLabel for StageLabelContainer {
    fn dyn_clone(&self) -> Box<dyn StageLabel> {
        Box::new(self.clone())
    }
}

impl Clone for StageLabelContainer {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

fn add_default_stages(schedule: &mut Schedule) {
    schedule
        .add_stage(
            StageLabelContainer::new(StageLabelContainer::new(CoreStage::First)),
            SystemStage::parallel(),
        )
        .add_stage(
            CoreStage::Startup,
            Schedule::default()
                .with_run_criteria(RunOnce::default())
                .with_stage(
                    StageLabelContainer::new(StartupStage::PreStartup),
                    SystemStage::parallel(),
                )
                .with_stage(
                    StageLabelContainer::new(StartupStage::Startup),
                    SystemStage::parallel(),
                )
                .with_stage(
                    StageLabelContainer::new(StartupStage::PostStartup),
                    SystemStage::parallel(),
                ),
        )
        .add_stage(
            StageLabelContainer::new(CoreStage::PreUpdate),
            SystemStage::parallel(),
        )
        .add_stage(
            StageLabelContainer::new(CoreStage::Update),
            SystemStage::parallel(),
        )
        .add_stage(
            StageLabelContainer::new(CoreStage::PostUpdate),
            SystemStage::parallel(),
        )
        .add_stage(
            StageLabelContainer::new(CoreStage::Last),
            SystemStage::parallel(),
        );
}

#[test]
fn test() {
    tracing_subscriber::fmt()
        // enable everything
        .with_max_level(tracing::Level::TRACE)
        // display source code file paths
        .with_file(true)
        // display source code line numbers
        .with_line_number(true)
        // disable targets
        .with_target(false)
        .with_span_events(FmtSpan::ACTIVE)
        // sets this to be the default, global collector for this application.
        .init();
    //    use tracing_subscriber::layer::SubscriberExt;
    //    let layer = tracing_subscriber::fmt::layer()
    //        .with_span_events(FmtSpan::CLOSE)
    //        .finish();
    //    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
    //        .expect("set up the subscriber");
    //    use tracing::trace;
    trace!("executed");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(set_tokio_handle());
    let mut zion = Zion::new();
    zion.world.insert_resource(LogSettings {
        filter: "wgpu=trace".to_string(),
        level: Level::TRACE,
    });
    //    zion.add_legacy_plugin(LogPlugin);
    zion.add_plugin(HelloPlugin);
    let id = zion.world.get_resource::<GlobalEntity>().unwrap().0;
    let topic = zion
        .world
        .entity(id)
        .get::<MemTopic<SpawnSystem>>()
        .unwrap();
    topic.write(SpawnSystem(Arc::new(SpawnSystemInner {
        id: SystemId(0),
        system_name: "Hello".into(),
        consts: vec![],
        reader_topics: Default::default(),
        writer_topics: Default::default(),
    })));
    zion.run();
}
