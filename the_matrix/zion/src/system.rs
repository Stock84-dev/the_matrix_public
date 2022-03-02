use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bevy::ecs::schedule::{IntoSystemDescriptor, SystemDescriptor};
use bevy::ecs::system::{FunctionSystem, IsFunctionSystem, SystemParamFunction, SystemParamState};
use bevy::prelude::*;
use bevy::task::Task;
use bevy::utils::HashMap;
use db::Db;
use futures_util::task::Spawn;
use mouse::dyn_clone::DynClone;
use mouse::futures_util::StreamExt;
use mouse::mem::DenseVec;
use mouse::smallbox::space::S1;
use mouse::sync::{priority, Mutex, RwLock};
use mouse::time::Utc;

use crate::db::DbConnectedLabel;
use crate::definitions::{
    SystemId, SystemLayout, SystemLayoutId, SystemTopicConfig, TopicKind, TopicLayout,
};
use crate::error::{ZionError, ZionResult};
use crate::topic::mem::{
    LocalTopicReadGuard, MemTopic, RawTopicReader, ResTopicReader, ResTopicWriter, TopicReader,
    TopicWriter,
};
use crate::topic::{Consumer, DespawnTopic, Producer, TopicIdToEntity, TopicLayouts, TopicState};
use crate::{DbPlugin, PluginLoader, Reschedule, Stages, Zion, ZionPlug};

pub struct SystemPlugin;

impl ZionPlug for SystemPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(DbPlugin)
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion.add_local_topic::<SpawnSystem>();
        let mut system_state: SystemState<(ResTopicReader<SpawnSystem>,)> =
            SystemState::new(&mut zion.world);
        let topic = RawTopicReader::from(system_state.get_mut(&mut zion.world).0);
        zion.init_resource::<SystemDefs>()
            .init_resource::<SystemIdToEntity>()
            .insert_resource(PrepareSystemSpawnReader(topic))
            .add_local_topic::<AddSystem>()
            .add_local_topic::<DespawnSystem>()
            .add_system(prepare_system_spawn.exclusive_system())
            .add_system(spawn_system)
            .add_system(despawn_system)
    }
}

pub struct SystemFactoryContainer {
    pub new: for<'a> fn(builder: &mut SystemBuilder<'a>) -> ZionResult<Box<dyn SystemFactory>>,
    //    pub factory: for<'a> fn(
    //        Box<dyn SystemFactory>,
    //    ) -> Pin<
    //        Box<dyn Future<Output = ZionResult<Box<dyn SystemFactory>>> + Send>,
    //    >,
    pub layout: SystemLayout,
}

#[async_trait]
pub trait SystemFactory: Send + Sync + 'static {
    fn layout() -> SystemLayout
    where
        Self: Sized;

    fn new<'a>(builder: &mut SystemBuilder<'a>) -> ZionResult<Box<dyn SystemFactory>>
    where
        Self: Sized;

    async fn spawn(&mut self) -> ZionResult<()>;

    fn system(&self) -> SystemDescriptor;
}

#[async_trait]
impl<T: SystemFactory> SystemFactory for Box<T> {
    fn layout() -> SystemLayout
    where
        Self: Sized,
    {
        T::layout()
    }

    fn new<'a>(builder: &mut SystemBuilder<'a>) -> ZionResult<Box<dyn SystemFactory>>
    where
        Self: Sized,
    {
        T::new(builder)
    }

    async fn spawn(&mut self) -> ZionResult<()> {
        T::spawn(self).await
    }

    fn system(&self) -> SystemDescriptor {
        T::system(self)
    }
}

#[derive(Default)]
pub struct SystemDefs(pub HashMap<&'static str, SystemFactoryContainer>);

#[derive(Default)]
pub struct SystemIdToEntity(pub HashMap<SystemId, Entity>);

/// Systems must not use added/changed/removed because they are respawned when schedule changes
#[derive(Component)]
pub struct SystemComponent {
    pub factory: Box<dyn SystemFactory>,
}

// pub struct SystemDef {
//    pub layout: SystemLayout,
//    pub factory: SystemFactory,
//}

#[derive(Debug)]
pub struct SpawnSystemInner {
    pub id: SystemId,
    // 12345678901234561234567890123456
    pub system_name: SmallString<[u8; 32]>,
    pub consts: Vec<u8>,
    pub reader_topics: SmallVec<[SystemTopicConfig; 4]>,
    pub writer_topics: SmallVec<[SystemTopicConfig; 4]>,
}

#[derive(Debug, Clone)]
pub struct SpawnSystem(pub Arc<SpawnSystemInner>);

/// Adds a bevy system to schedule, entity must have `BevySystemComponent`
pub struct AddSystem {
    pub entity: Entity,
}

pub struct DespawnSystem {
    pub entity: Entity,
}

#[derive(Component)]
pub struct SystemData {
    pub id: SystemId,
    pub name: &'static str,
    pub reader_topics: SmallVec<[Entity; 4]>,
    pub writer_topics: SmallVec<[Entity; 4]>,
}

struct PrepareSystemSpawnReader(RawTopicReader<SpawnSystem>);

fn prepare_system_spawn(world: &mut World) {
    fn spawn(world: &mut World, id: Entity) -> ZionResult<Option<()>> {
        let mut system_state: SystemState<(
            Res<PrepareSystemSpawnReader>,
            Res<SystemDefs>,
            Res<TopicIdToEntity>,
        )> = SystemState::new(world);
        let mut reader_topics: SmallVec<[Entity; 4]> = Default::default();
        let mut writer_topics: SmallVec<[Entity; 4]> = Default::default();
        let command;
        let new;
        {
            let (system_commands, defs, map) = system_state.get_mut(world);
            command = match system_commands.0.try_read() {
                None => return Ok(None),
                Some(x) => x,
            }
            .read()
            .clone();
            new = defs.0.get(command.0.system_name.as_str()).ok()?.new;
            let mapper = |x: &SystemTopicConfig| *map.0.get(&x.topic_id).unwrap();
            reader_topics.extend(command.0.reader_topics.iter().map(mapper));
            writer_topics.extend(command.0.writer_topics.iter().map(mapper));
        }
        let mut builder =
            SystemBuilder::new(world, id, command.clone(), &reader_topics, &writer_topics);
        let mut factory = (new)(&mut builder)?;
        let task = async move { factory.spawn().await.map(|x| factory) }.spawn_update();
        let mut system_state: SystemState<(Res<SystemDefs>, ResMut<SystemIdToEntity>)> =
            SystemState::new(world);
        let (defs, mut map) = system_state.get_mut(world);
        map.0.insert(command.0.id, id);
        let key = *defs.0.get_key_value(command.0.system_name.as_str()).ok()?.0;
        world
            .entity_mut(id)
            .insert(SystemData {
                id: command.0.id,
                name: key,
                reader_topics,
                writer_topics,
            })
            .insert(SystemTask(task));

        Ok(Some(()))
    }
    loop {
        let id = world.spawn().id();
        match spawn(world, id) {
            Ok(Some(())) => {}
            Ok(None) => break,
            Err(e) => {
                world.despawn(id);
                e.log_context("failed to spawn a system factory");
                todo!("handle error")
            }
        }
    }
}

fn spawn_system(
    mut query: Query<(Entity, &mut SystemTask)>,
    mut commands: Commands,
    add: ResTopicWriter<AddSystem>,
    despawn: ResTopicWriter<DespawnSystem>,
) {
    for (id, mut task) in query.iter_mut() {
        let result = ready_loop!(task.0);
        let factory = match result {
            Ok(x) => x,
            Err(e) => {
                todo!("handle");
                despawn.write(DespawnSystem { entity: id });
            }
        };
        commands
            .entity(id)
            .insert(SystemComponent { factory })
            .remove::<SystemTask>();
        add.write(AddSystem { entity: id });
    }
}

fn despawn_system(
    despawn_system: ResTopicReader<DespawnSystem>,
    mut map: ResMut<SystemIdToEntity>,
    query: Query<&SystemData>,
    mut states: Query<&mut TopicState>,
    writer: ResTopicWriter<DespawnTopic>,
    mut commands: Commands,
) {
    for e in read_all!(despawn_system) {
        let system = query.get(e.entity).unwrap();
        commands.entity(e.entity).despawn();
        map.0.remove(&system.id);
        let mut despawn = |topics: &SmallVec<[Entity; 4]>| {
            for reader in topics {
                let mut state = states.get_mut(*reader).unwrap();
                state.n_local_systems -= 1;
                if state.n_local_systems == 0 {
                    writer.write(DespawnTopic { id: state.id });
                }
            }
        };
        despawn(&system.reader_topics);
        despawn(&system.writer_topics);
        commands.insert_resource(Reschedule);
    }
}

#[macro_use]
pub(crate) mod registry;

pub trait BevySystem: Send + Sync {
    /// This function should not have any side effects when calling multiple times.
    /// Bevy schedule doesn't allow removal of systems. So we create a new schedule and add existing
    /// systems.
    fn system(&self) -> SystemDescriptor;
}

pub struct HelloWorld {}

impl HelloWorld {
    fn hello(res: Res<()>) {
        println!("hello world");
    }
}

pub fn hello_world_factory<'a>(builder: &'a mut SystemBuilder<'a>) -> Box<dyn BevySystem> {
    BevySystemContainer::new(HelloWorld::hello, (), |a, config| {})
}

pub struct BevySystemContainer<
    A: Clone + 'static,
    C: Fn(A, &mut <Param::Fetch as SystemParamState>::Config) + 'static,
    S: SystemParamFunction<(), (), Param, Marker>
        + IntoSystem<
            (),
            (),
            (IsFunctionSystem, Param, Marker),
            System = FunctionSystem<(), (), Param, Marker, S>,
        > + ConfigurableSystem<(), (), Param, Marker>
        + Clone
        + Send
        + Sync
        + 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
> {
    system: S,
    args: A,
    config: C,
    _param: PhantomData<Param>,
    _marker: PhantomData<Marker>,
}

impl<A, C, S, Param, Marker> BevySystemContainer<A, C, S, Param, Marker>
where
    A: Clone + Send + Sync + 'static,
    C: Fn(A, &mut <Param::Fetch as SystemParamState>::Config) + Send + Sync + 'static,
    S: SystemParamFunction<(), (), Param, Marker>
        + IntoSystem<
            (),
            (),
            (IsFunctionSystem, Param, Marker),
            System = FunctionSystem<(), (), Param, Marker, S>,
        > + ConfigurableSystem<(), (), Param, Marker>
        + Clone
        + Send
        + Sync
        + 'static,
    Param: SystemParam + Send + Sync + 'static,
    Marker: Send + Sync + 'static,
{
    pub fn new(system: S, args: A, config: C) -> Box<dyn BevySystem> {
        Box::new(Self {
            system,
            args,
            config,
            _param: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<A, C, S, Param, Marker> BevySystem for BevySystemContainer<A, C, S, Param, Marker>
where
    A: Clone + Send + Sync + 'static,
    C: Fn(A, &mut <Param::Fetch as SystemParamState>::Config) + Send + Sync + 'static,
    S: SystemParamFunction<(), (), Param, Marker>
        + IntoSystem<
            (),
            (),
            (IsFunctionSystem, Param, Marker),
            System = FunctionSystem<(), (), Param, Marker, S>,
        > + ConfigurableSystem<(), (), Param, Marker>
        + Clone
        + Send
        + Sync
        + 'static,
    Param: SystemParam + Send + Sync + 'static,
    Marker: Send + Sync + 'static,
{
    fn system(&self) -> SystemDescriptor {
        self.system
            .clone()
            .system()
            .config(|x| (self.config)(self.args.clone(), x))
            .into_descriptor()
    }
}

// pub enum SystemFactory {
//    Bevy(Box<dyn BevySystemFactory>),
//    Async(),
//}

//#[derive(Component)]
// struct SystemFactoryComponent(Option<SystemFactory>);

/// There can be multiple factories for the same system
//#[async_trait]
// pub trait BevySystemFactory: DynClone + Send + Sync + 'static {
//    //    fn new() -> Self
//    //    where
//    //        Self: Sized;
//    /// Validate config and save arguments for spawn call.
//    async fn validate_and_update(&mut self, config: SpawnSystem) -> ZionResult<()>;
//    /// will only be called once after `validate_and_update`
//    fn spawn<'a>(&mut self, builder: &mut SystemBuilder<'a>) -> Box<dyn BevySystem>;
//}

// struct SystemState {
//    reader_cursors: Vec<u64>,
//    writer_cursors: Vec<u64>,
//    state: Vec<u8>,
//}

// struct SystemPrefab {
//    layout: SystemLayout,
//    factory: SystemFactory,
//}
// struct Fabrication<T> {
//    factory: T,
//    config: *const SystemSpawnConfig,
//}

// pub struct SystemManager {
//    system_factories: HashMap<SystemLayoutId, SystemPrefab>,
//    systems: HashMap<SystemId, Arc<Mutex<SystemState>>>,
//    system_layouts: HashMap<SystemLayoutId, SystemLayout>,
//    bevy_fabrications: Vec<Fabrication<Box<dyn BevySystemFactory>>>,
//    bavy_factories: Vec<Box<dyn BevySystemFactory>>,
//    //    bevy: SystemWorkingSet,
//}

// struct SpawnSystem {
//    config: *const SystemSpawnConfig,
//    factory: usize,
//}

#[derive(Default)]
pub struct WorkflowSystems {
    bevy: Vec<Box<dyn BevySystem>>,
}

//#[derive(Default)]
// struct SystemSpawnConfigs {
//    configs: Vec<*const SystemSpawnConfig>,
//    //    indices: Vec<usize>,
//}
// impl SystemSpawnConfigs {
//    fn get_config<'a>(&'a self, i: usize) -> &'a SystemSpawnConfig {
//        unsafe {
//            // SAFETY: system working set gets cleared every time we add systems and it is only
//            // being used when Self::spawn is called
//            let configs: Vec<&'a SystemSpawnConfig> = std::mem::transmute(&self.configs);
//            configs[i]
//        }
//    }
//}
// impl SystemManager {
//    pub fn new() -> Self {
//        Self {
//            system_factories: Default::default(),
//            systems: Default::default(),
//            system_layouts: Default::default(),
//            bevy_fabrications: Default::default(),
//            bavy_factories: vec![],
//        }
//    }
//
//    pub async fn update(&mut self, loader: &mut impl ZionLoader) -> ZionResult<()> {
//        loader
//            .extend_system_layouts(Utc::now().timestamp(), &mut self.system_layouts)
//            .await?;
//        Ok(())
//    }
//
//    pub fn spawn<'a>(
//        &mut self,
//        app: &mut App,
//        configs: impl IntoIterator<Item = &'a SystemSpawnConfig>,
//        temp_topics: &TempTopicsLookup,
//        workflow_id: WorkflowId,
//    ) -> ZionResult<()> {
//        self.begin();
//        for config in configs.into_iter() {
//            let factory = self
//                .system_factories
//                .get(&config.id)
//                .ok_or(ZionError::UnknownSystem(config.id))?;
//            match factory.factory {
//                SystemFactory::Bevy(bevy_factory) => {
//                    let mut bevy_factory = bevy_factory.clone_box();
//                    bevy_factory.validate_and_update(config)?;
//                    self.bevy_fabrications.push(Fabrication {
//                        factory: bevy_factory,
//                        config: config,
//                    });
//                }
//                SystemFactory::Async() => {}
//            }
//        }
//        self.spawn_inner(app, temp_topics, workflow_id)
//    }
//
//    fn begin(&mut self) {
//        self.bevy_fabrications.clear();
//    }
//
//    fn rollback(&mut self) {}
//
//    fn spawn_inner(
//        &mut self,
//        app: &mut App,
//        temp_topics: &TempTopicsLookup,
//        workflow_id: WorkflowId,
//    ) -> ZionResult<()> {
//        for fab in self.bevy_fabrications {
//            let config = unsafe { &*fab.config };
//            let mut builder = SystemBuilder::new(temp_topics, &mut app.world, config);
//            let system = fab.factory.spawn();
//            app.schedule
//                .add_system_to_stage(CoreStage::Update, system.system());
//            app.world
//                .spawn()
//                .insert(BevySystemComponent(system))
//                .insert(workflow_id);
//        }
//
//        Ok(())
//    }
//}

pub struct SystemBuilder<'a> {
    world: &'a mut World,
    id: Entity,
    command: SpawnSystem,
    reader_topics: &'a SmallVec<[Entity; 4]>,
    writer_topics: &'a SmallVec<[Entity; 4]>,
    reader_i: usize,
    writer_i: usize,
}

impl<'a> SystemBuilder<'a> {
    pub fn new(
        world: &'a mut World,
        id: Entity,
        command: SpawnSystem,
        reader_topics: &'a SmallVec<[Entity; 4]>,
        writer_topics: &'a SmallVec<[Entity; 4]>,
    ) -> SystemBuilder<'a> {
        SystemBuilder {
            world,
            id,
            command,
            reader_topics,
            writer_topics,
            reader_i: 0,
            writer_i: 0,
        }
    }

    pub fn entity(&self) -> Entity {
        self.id
    }

    pub fn get_reader<T: Consumer>(&mut self) -> ZionResult<T> {
        let config = self.command.0.reader_topics.get(self.reader_i).ok_or(
            ZionError::NotEnoughTopicReaders(self.command.0.reader_topics.len()),
        )?;
        let topic_kind = {
            let layouts = self.world.get_resource::<TopicLayouts>().unwrap();
            let layouts_guard = layouts.0.lock();
            let layout: &TopicLayout = layouts_guard
                .get(&config.topic_layout_id)
                .ok_or(ZionError::UnknownTopicLayout(config.topic_layout_id))?;
            layout.config.kind()
        };
        let reader = match T::TOPIC_KIND {
            TopicKind::Bevy => {
                if topic_kind != TopicKind::Bevy {
                    return Err(ZionError::TopicReaderLayoutError {
                        system: T::TOPIC_KIND,
                        config: topic_kind,
                    });
                }
                let id = self.reader_topics[self.reader_i];
                if self.world.entity(id).get::<T::StorageComponent>().is_none() {
                    self.world
                        .entity_mut(id)
                        .insert(<T::StorageComponent as Default>::default());
                }
                T::new(self.world, id)
            }
            TopicKind::Async => {
                unimplemented!()
            }
        };
        self.reader_i += 1;
        reader
    }

    pub fn get_writer<T: Producer>(&mut self) -> ZionResult<T> {
        let config = self.command.0.writer_topics.get(self.writer_i).ok_or(
            ZionError::NotEnoughTopicWriters(self.command.0.writer_topics.len()),
        )?;
        let topic_kind = {
            let layouts = self.world.get_resource::<TopicLayouts>().unwrap();
            let layouts = layouts.0.lock();
            let layout: &TopicLayout = layouts
                .get(&config.topic_layout_id)
                .ok_or(ZionError::UnknownTopicLayout(config.topic_layout_id))?;
            layout.config.kind()
        };
        let writer = match T::TOPIC_KIND {
            TopicKind::Bevy => {
                if topic_kind != TopicKind::Bevy {
                    return Err(ZionError::TopicWriterLayoutError {
                        system: T::TOPIC_KIND,
                        config: topic_kind,
                    });
                }
                let id = self.writer_topics[self.reader_i];
                if self.world.entity(id).get::<T::StorageComponent>().is_none() {
                    self.world
                        .entity_mut(id)
                        .insert(<T::StorageComponent as Default>::default());
                }
                T::new(self.world, id)
            }
            TopicKind::Async => {
                unimplemented!()
            }
        };
        self.writer_i += 1;
        writer
    }
}

#[derive(Component)]
struct SystemTask(Task<ZionResult<Box<dyn SystemFactory>>>);
