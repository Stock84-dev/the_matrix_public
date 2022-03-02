#![feature(derive_default_enum)]
#![feature(try_blocks)]

use std::collections::VecDeque;
use std::io::SeekFrom;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use bevy::ecs::entity::EntityMap;
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use bevy::scene::serde::SceneSerializer;
use mouse::macros::tokio::io::{AsyncSeekExt, AsyncWriteExt};
use mouse::some_loop;
use mouse::sync::AsyncMutex;
use mouse::traits::AsyncWriteSeek;

pub struct GraphPlugin;

impl Plug for GraphPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.add_event::<GraphSpawned>()
            .add_event::<RunPipeline>()
            .add_event::<SavePipeline>()
            .add_event::<PipelineStalled>()
            .register_type::<Pipeline>()
            .register_type::<PipelineState>()
            .register_type::<PipelineStallers>()
            .register_type::<PipelineScene>()
            .register_type::<PipelinePath>()
            .register_type::<Node>()
            .add_system_to_stage(CoreStage::Last, pipeline_checkpoint)
            .add_system_to_stage(CoreStage::First, pipeline_system)
            .pre_system(node_spawned)
            .add_system(pipeline_spawned_saver)
    }
}

fn pipeline_spawned_saver(
    mut reader: EventReader<GraphSpawned>,
    //    mut commands: Commands,
) {
    for e in reader.iter() {
        todo!("Saver unimplemented");
        //        commands.entity(e.id).insert()
    }
}

#[derive(Bundle)]
struct PipelineBundle {
    path: PipelinePath,
    stallers: PipelineStallers,
    scene: PipelineScene,
    pipeline: Pipeline,
}

#[derive(Component)]
struct PipelineSaver(Arc<AsyncMutex<dyn AsyncWriteSeek + Unpin>>);
#[derive(Component)]
pub struct SerializedNodeId(pub Entity);

fn pipeline_checkpoint(
    type_registry: Res<TypeRegistryArc>,
    mut reader: EventReader<SavePipeline>,
    mut pipelines: Query<(
        &mut PipelineScene,
        &mut PipelineStallers,
        &PipelineSaver,
        &PipelinePath,
        &mut Pipeline,
        Entity,
    )>,
) {
    for e in reader.iter() {
        let (mut scene, mut stallers, saver, path, mut pipeline, pipeline_id) =
            ok_loop!(pipelines.get_mut(e.id));
        let mut world = World::new();
        std::mem::swap(&mut world, &mut scene.world);
        world
            .spawn()
            .insert_bundle(PipelineBundle {
                path: path.clone(),
                stallers: PipelineStallers { stallers: vec![] },
                scene: PipelineScene {
                    world: World::new(),
                },
                pipeline: Pipeline::default(),
            })
            .insert(path.clone());

        let dynamic_scene = DynamicScene::from_world(&scene.world, &type_registry);
        let saver: Arc<AsyncMutex<dyn AsyncWriteSeek + Unpin>> = saver.0.clone();
        let scene_serializer = SceneSerializer::new(&dynamic_scene, &type_registry);
        let s = serde_yaml::to_string(&scene_serializer).unwrap();

        stallers.push(
            async move {
                let result: Result<()> = try {
                    let mut saver = saver.lock().await;
                    saver.seek(SeekFrom::Start(0)).await?;
                    saver.write_all(&s.as_bytes()).await?;
                };
                result.context("Failed to save pipeline state")
            }
            .spawn_update(),
        );
    }
}

pub struct GraphSpawned {
    pub scene_pipeline_id: Entity,
    pub world_pipeline_id: Entity,
    pub map: EntityMap,
}

pub struct SavePipeline {
    pub id: Entity,
}

pub struct RunPipeline {
    pub id: Entity,
}

pub struct PipelineStalled {
    pub id: Entity,
}

fn node_spawned(mut reader: EventReader<GraphSpawned>, mut nodes: Query<&mut Node>) {
    for e in reader.iter() {
        for mut node in nodes
            .iter_mut()
            .filter(|x| x.pipeline_id == e.scene_pipeline_id)
        {
            node.pipeline_id = e.world_pipeline_id;
            for id in &mut node.inputs {
                *id = e.map.get(*id).unwrap();
            }
            for id in &mut node.outputs {
                *id = e.map.get(*id).unwrap();
            }
        }
    }
}

fn pipeline_system(
    mut pipelines: Query<(
        &mut Pipeline,
        &mut PipelineStallers,
        &mut PipelineScene,
        Entity,
    )>,
    mut nodes: Query<(&Node, &mut SerializedNodeId)>,
    mut checkpoints: EventWriter<SavePipeline>,
    mut runs: EventWriter<RunPipeline>,
    mut stalls: EventWriter<PipelineStalled>,
) {
    for (mut pipeline, mut stallers, mut scene, pipeline_id) in pipelines.iter_mut() {
        stallers.stallers.keep(|x| match x.poll() {
            Some(result) => {
                if result.is_err() {
                    pipeline.queue.clear();
                    result.log_context("Found error in pipeline, halting...");
                }
                false
            }
            None => true,
        });
        if !stallers.stallers.is_empty() {
            continue;
        }
        let state = some_loop!(pipeline.queue.pop_front());
        match state {
            PipelineState::Stalled => {
                stalls.send(PipelineStalled { id: pipeline_id });
            }
            PipelineState::Running => {
                if pipeline.queue.is_empty() {
                    pipeline.queue.push_back(PipelineState::Running);
                }
                runs.send(RunPipeline { id: pipeline_id });
            }
            PipelineState::Checkpoint => {
                checkpoints.send(SavePipeline { id: pipeline_id });
                let mut world = World::new();
                for (node, mut serialized_id) in
                    nodes.iter_mut().filter(|x| x.0.pipeline_id == pipeline_id)
                {
                    serialized_id.0 = world.spawn().insert(node.clone()).id();
                }
                scene.world = world;
            }
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, PartialEq)]
#[reflect(Component)]
pub enum PipelineState {
    Running,
    Checkpoint,
    Stalled,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::Running
    }
}

#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct PipelinePath {
    pub path: String,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct PipelineScene {
    #[reflect(ignore)]
    pub world: World,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Pipeline {
    #[reflect(ignore)]
    queue: VecDeque<PipelineState>,
}

impl Pipeline {
    pub fn enq_stall(&mut self) {
        self.queue.push_back(PipelineState::Stalled);
    }

    pub fn enq_save_and_continue(&mut self) {
        self.queue.push_back(PipelineState::Checkpoint);
        self.queue.push_back(PipelineState::Running);
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(PipelineState::Running);
        Self { queue }
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct PipelineStallers {
    #[reflect(ignore)]
    stallers: Vec<Task<Result<()>>>,
}

impl PipelineStallers {
    pub fn push(&mut self, task: Task<Result<()>>) {
        self.stallers.push(task);
    }
}

// pub struct PipelineSpawner {}
//
// impl PipelineSpawner {
//    pub fn spawn(&self) {
//        let deserializer = SceneDeserializer::new();
//        let spawner = SceneSpawner::default();
//        let server = AssetServer::
//    }
//}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Node {
    pub inputs: Vec<Entity>,
    pub outputs: Vec<Entity>,
    pub pipeline_id: Entity,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            inputs: vec![],
            outputs: vec![],
            pipeline_id: Entity::from_raw(u32::MAX),
        }
    }
}

pub struct NodeEvent<T> {
    pub receiver: Entity,
    pub data: T,
}

#[derive(Component)]
struct StageStartNode {}

#[derive(Component)]
struct StageEndNode {}

#[test]
fn test() {
    #[derive(Component)]
    struct NonReflect;
    #[derive(Component, Reflect, Default, Debug)]
    #[reflect(Component)]
    struct Comp(f32);

    #[derive(Component, Reflect, Default, Debug)]
    #[reflect(Component)]
    struct Comp2(String);

    #[derive(Component, Reflect, Debug)]
    #[reflect(Component)]
    struct Reference(#[reflect(ignore)] Entity);

    impl FromWorld for Reference {
        fn from_world(world: &mut World) -> Self {
            Self(Entity::new(u32::MAX))
        }
    }
    let mut app = App::new();
    let registry = TypeRegistryArc::default();
    registry.write().register::<Comp>();
    registry.write().register::<Comp2>();
    registry.write().register::<Reference>();
    registry.write().register::<f32>();
    registry.write().register::<u32>();
    registry.write().register::<String>();
    registry.write().register::<Entity>();
    let mut world = World::new();
    world.spawn().insert(NonReflect);
    let a = world.spawn().insert(Comp(123.)).id();
    let b = world
        .spawn()
        .insert(Comp(321.))
        .insert(Comp2("abc".into()))
        .id();
    world.entity_mut(a).insert(Reference(b));
    world.entity_mut(b).insert(Reference(a));
    //    dbg!(world.archetypes().len());
    let dynamic_scene = DynamicScene::from_world(&world, &registry);
    let ron = dynamic_scene.serialize_ron(&registry).unwrap();
    let scene_serializer = SceneSerializer::new(&dynamic_scene, &registry);
    let s = serde_yaml::to_string(&scene_serializer).unwrap();
    println!("{}", s);
    //    println!("{}", ron);
    let reg = registry.read();
    let des = SceneDeserializer {
        type_registry: &*reg,
    };
    let scene = des
        .deserialize(serde_yaml::Deserializer::from_str(&s))
        .unwrap();
    let mut entity_map = EntityMap::default();
    let mut world2 = World::new();
    world2.spawn();
    world2.spawn();
    world2.spawn();
    world2.spawn();
    drop(reg);
    world2.insert_resource(registry);
    //    for entity in scene.entities {
    //        entity_map.insert(Entity::new(entity.entity), world2.spawn().id());
    //    }
    scene.write_to_world(&mut world2, &mut entity_map).unwrap();
    dbg!(entity_map);
    let mut query = world2.query::<(Entity, &Comp, &Reference)>();
    for q in query.iter(&world2) {
        dbg!(q);
    }
    let mut query = world2.query::<(Entity, &Comp, &Comp2, &Reference)>();
    for q in query.iter(&world2) {
        dbg!(q);
    }
}

#[derive(Component)]
struct BruteForceNode {}

#[derive(Component)]
struct ConstructNode {}

#[derive(Component)]
pub struct ResourceUsage {
    pub opencl_ram: u64,
    pub ram: u64,
    pub storage: u64,
}
