use std::sync::Arc;

use ::bevy::prelude::*;
use ::bevy::utils::{HashMap, HashSet};
use bevy::utils::Instant;
use db::Db;
use mouse::futures_util::StreamExt;
use mouse::sync::{Mutex, RwLock};
use mouse::time::Utc;

use crate::db::{update_topic_layouts_modified_after_ts, DbConnectedLabel};
use crate::definitions::{TopicId, TopicKind, TopicLayout, TopicLayoutId};
use crate::error::ZionResult;
// use crate::topic::bevy_topic::{BevyTopic, BevyTopics, DynBevyTopic};
use crate::topic::mem::{MemTopic, ResTopicReader, ResTopicWriter, TopicReader, TopicWriter};
use crate::{DbPlugin, PluginLoader, Schedules, Stages, Zion, ZionPlug};

// pub mod bevy_topic;
pub mod mem;
pub mod tmp;

// Each system should have only one topic type regardless if it is bevy, async, kafka
// each topic has bevy update systems

pub struct TopicPlugin;

impl ZionPlug for TopicPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(DbPlugin)
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion.init_resource::<TopicLayouts>()
            .init_resource::<TopicIdToEntity>()
            .init_resource::<TopicSystems>()
            .add_local_topic::<SpawnTopic>()
            .add_local_topic::<DespawnTopic>()
            .add_local_topic::<TopicSpawned>()
            //            .add_startup_system(spawn_update_topic_layouts)
            .add_system(spawn_topics)
            .add_system(despawn_topics)
    }
}

#[derive(Debug)]
pub struct SpawnTopic {
    pub layout_id: TopicLayoutId,
    pub id: TopicId,
}

pub struct DespawnTopic {
    pub id: TopicId,
}

pub struct TopicSpawned {
    pub id: TopicId,
}

#[derive(Default)]
pub struct TopicSystems(pub HashSet<usize>);

fn despawn_topics(
    topic_commands: ResTopicReader<DespawnTopic>,
    mut map: ResMut<TopicIdToEntity>,
    mut commands: Commands,
) {
    for command in read_all!(topic_commands) {
        let entity = map.0.remove(&command.id).unwrap();
        commands.entity(entity).despawn();
    }
}

fn spawn_topics(
    topic_commands: ResTopicReader<SpawnTopic>,
    mut map: ResMut<TopicIdToEntity>,
    writer: ResTopicWriter<TopicSpawned>,
    mut commands: Commands,
) {
    for command in read_all!(topic_commands) {
        //        let layout = some_loop!(layouts
        //            .get(&command.layout_id)
        //            .ok()
        //            .log_with_context(|| format!("failed to spawn {:?}", command)));
        let mut spawner = commands.spawn();
        let entity = spawner.id();
        // actual container is built when system spawns it because here we don't have access to
        // generics
        spawner.insert(TopicState {
            id: command.id,
            layout_id: command.layout_id,
            n_local_systems: 0,
        });
        drop(spawner);
        map.0.insert(command.id, entity);
        writer.write(TopicSpawned { id: command.id });
    }
}

async fn update_topic_layouts(layouts: TopicLayouts, db: Db) {
    let mut ts = 0;
    loop {
        let result: Result<()> = try {
            let mut stream = update_topic_layouts_modified_after_ts(&db, ts);
            while let Some(result) = stream.next().await {
                let (id, layout) = result?;
                layouts.0.lock().insert(id, layout);
            }
        };
        result.log();
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        ts = Utc::now().timestamp();
    }
}

#[derive(Default)]
pub struct TopicLayouts(pub Arc<Mutex<HashMap<TopicLayoutId, TopicLayout>>>);

#[derive(Component)]
pub struct TopicState {
    pub id: TopicId,
    pub layout_id: TopicLayoutId,
    pub n_local_systems: usize,
}

#[derive(Component)]
pub(crate) struct Writers(Vec<Entity>);
#[derive(Component)]
pub(crate) struct Readers(Vec<Entity>);

#[derive(Default)]
pub struct TopicIdToEntity(pub HashMap<TopicId, Entity>);

// pub trait Topic {
//    fn id(&self) -> u64;
//}
// struct TopicSpawnCommand {
//    id: TopicId,
//    layout_id: TopicLayoutId,
//}
// impl TopicSpawnCommand {
//    pub fn new(layout_id: TopicLayoutId, id: u64) -> Self {
//        Self {
//            id: TopicId(id),
//            layout_id,
//        }
//    }
//}
// pub(crate) struct TopicManager {
//    bevy_topics_to_spawn: Vec<TopicSpawnCommand>,
//    topic_layouts: HashMap<TopicLayoutId, TopicLayout>,
//}
// impl TopicManager {
//    pub fn new() -> Self {
//        Self {
//            bevy_topics_to_spawn: vec![],
//            topic_layouts: Default::default(),
//        }
//    }
//
//    pub async fn update(&mut self, loader: &mut impl ZionLoader) -> ZionResult<()> {
//        loader.extend_topic_layouts(Utc::now().timestamp(), &mut self.topic_layouts);
//        //        self.topic_layouts.clear();
//        //                self.topic_layouts
//        //                    .extend(layouts.into_iter().map(|x| (x.id, x.layout)));
//        Ok(())
//    }
//
//    fn begin(&mut self) {
//        self.bevy_topics_to_spawn.clear();
//    }
//
//    pub fn rollback(&mut self, app: &mut App) -> ZionResult<()> {
//        let mut topics = app.world.get_resource_mut::<BevyTopics>().unwrap();
//        for command in self.bevy_topics_to_spawn.drain(..) {
//            let mut state = self.topic_states.get_mut(&command.layout_id).unwrap();
//            if !command.is_temp {
//                state.is_active = false;
//            }
//            topics.despawn(state.bevy_topic_id);
//        }
//        Ok(())
//    }
//
//    pub fn spawn_topics<'a>(
//        &mut self,
//        app: &mut App,
//        topics: impl IntoIterator<Item = &'a TopicLayoutWithId>,
//        topic_id_start: &mut u64,
//        workflow_ids: &Ids,
//    ) -> ZionResult<()> {
//        self.begin();
//        self.prepare_topics(topics, topic_id_start)?;
//
//        match self.spawn_topics_inner(app, workflow_ids) {
//            Ok(_) => Ok(()),
//            Err(e) => {
//                let e = ZionError::FailedToSpawnTopics(Box::new(e));
//                match self.rollback(app) {
//                    Ok(_) => Err(ZionError::TransactionRolledBackSuccessfully(Box::new(e))),
//                    Err(e) => Err(ZionError::Critical(Box::new(e))),
//                }
//            }
//        }
//    }
//
//    fn spawn_topics_inner(&mut self, app: &mut App, workflow_ids: &Ids) -> ZionResult<()> {
//        for command in self.bevy_topics_to_spawn.drain(..) {
//            let layout = self.topic_layouts.get(&command.layout_id).ok()?;
//            let mut spawner = app.world.spawn();
//            let entity = spawner.id();
//            // actual container is built when system spawns it because here we don't have access
// to            // generics
//            spawner.insert(TopicState {
//                //                is_active: true,
//                id: command.id,
//                layout_id: command.layout_id,
//            });
//            drop(spawner);
//            app.world
//                .get_resource_mut::<TopicIdToEntity>()
//                .unwrap()
//                .0
//                .insert(command.id, entity);
//            //            if let WorkflowTopicId::Temp(_) = command.id {
//            //                app.world
//            //                    .entity_mut(workflow_ids.entity)
//            //                    .push_children(&[entity]);
//            //                lookup.bevy.push(entity);
//            //            }
//            //            app.world.entity_mut(workflow_ids.entity).get_mut::<TempTopicsLookup>().
//        }
//        Ok(())
//    }
//
//    fn prepare_topics<'a, I: IntoIterator<Item = &'a TopicLayoutWithId>>(
//        &mut self,
//        topics: I,
//        topic_id_start: &mut u64,
//    ) -> ZionResult<()> {
//        for layout in topics.into_iter() {
//            let layout: &TopicLayoutWithId = layout;
//            //            let is_tmp = layout.layout.access == TopicAccess::Private
//            //                && layout.layout.lifetime == TopicLifetime::Workflow;
//            self.topic_layouts
//                .get_mut(&layout.id)
//                .ok_or(ZionError::UnknownTopicLayout(layout.id))?;
//
//            match layout.layout.config.kind() {
//                TopicKind::Bevy => {
//                    self.bevy_topics_to_spawn
//                        .push(TopicSpawnCommand::new(layout.id, *topic_id_start));
//                }
//                TopicKind::Async => {
//                    unimplemented!()
//                }
//            }
//
//            *topic_id_start += 1;
//        }
//        Ok(())
//    }
//}
//
pub trait Consumer: Sized {
    type StorageComponent: Component + Default;
    const TOPIC_KIND: TopicKind;
    fn new(world: &World, id: Entity) -> ZionResult<Self>;
}
pub trait Producer: Sized {
    type StorageComponent: Component + Default;
    const TOPIC_KIND: TopicKind;
    fn new(world: &World, id: Entity) -> ZionResult<Self>;
}
//
//#[derive(Default, Component)]
// pub(crate) struct TempTopicsLookup {
//    pub bevy: Vec<Entity>,
//}
// impl TempTopicsLookup {
//    fn clear(&self) {
//        const_assert_eq!(TempTopicsLookup::size(), std::mem::size_of::<Vec<()>>() * 1);
//        self.bevy.clear();
//    }
//}
//
//#[derive(Default)]
// pub struct ActiveGlobalTopics(HashSet<TopicId>);
//
// fn spawn_bevy_topics<'a>(
//    app: &mut App,
//    states: &mut HashMap<TopicLayoutId, TopicState>,
//    commands: impl IntoIterator<Item = &'a TopicSpawnCommand>,
//    f: impl FnMut(&mut TopicState, bool, usize),
//) -> ZionResult<()> {
//    let mut bevy_topics = app.world.get_resource_mut::<BevyTopics>().unwrap();
//    for command in commands.into_iter() {
//        let state = states.get_mut(&command.layout_id).ok()?;
//        let topic = (state.bevy_builder.unwrap())();
//        f(state, command.is_temp, bevy_topics.spawn(topic));
//    }
//    Ok(())
//}
//

fn spawn_update_topic_layouts(db: Res<Db>, layouts: Res<TopicLayouts>) {
    update_topic_layouts(TopicLayouts(layouts.0.clone()), db.clone()).spawn();
}
