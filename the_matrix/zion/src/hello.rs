use std::sync::Arc;

use bevy::ecs::schedule::{IntoSystemDescriptor, SystemDescriptor};
use bevy::prelude::*;

use crate::definitions::SystemKind;
use crate::error::ZionResult;
use crate::system::{BevySystem, BevySystemContainer, DespawnSystem, SpawnSystem, SystemBuilder};
use crate::topic::mem::ResTopicWriter;
use crate::{Stages, SystemFactory, SystemLayout, Zion, ZionPlug};

pub struct HelloPlugin;

impl ZionPlug for HelloPlugin {
    fn deps<'a, 'b>(
        &mut self,
        loader: &'a mut crate::PluginLoader<'b>,
    ) -> &'a mut crate::PluginLoader<'b> {
        loader
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion.register_system::<Hello>()
    }
}

struct State {
    id: Entity,
    despawned: bool,
}

impl Default for State {
    fn default() -> Self {
        panic!()
    }
}

#[derive(Clone)]
pub struct Hello {
    state: Arc<State>,
}

impl Hello {
    fn hello(local: Local<Arc<State>>, writer: ResTopicWriter<DespawnSystem>) {
        //        let local = unsafe { (**local).as_mut_cast() };
        //        if !local.despawned {
        //            local.despawned = true;
        //            writer.write(DespawnSystem { entity: local.id });
        //            println!("despawn");
        //        }
        trace!("hello world");
    }
}

#[async_trait]
impl SystemFactory for Hello {
    fn layout() -> SystemLayout
    where
        Self: Sized,
    {
        SystemLayout {
            stage: Stages::PreMain,
            input_topics: vec![],
            output_topics: vec![],
            static_estimations: None,
            kind: SystemKind::Bevy,
        }
    }

    fn new<'a>(builder: &mut SystemBuilder<'a>) -> ZionResult<Box<dyn SystemFactory>>
    where
        Self: Sized,
    {
        builder.get_reader()
        Ok(Box::new(Self {
            state: Arc::new(State {
                id: builder.entity(),
                despawned: false,
            }),
        }))
    }

    async fn spawn(&mut self) -> ZionResult<()> {
        Ok(())
    }

    fn system(&self) -> SystemDescriptor {
        Self::hello
            .system()
            .config(|x| x.0 = Some(self.state.clone()))
            .into_descriptor()
    }
}

//#[async_trait]
// impl BevySystemFactory for Hello {
//    async fn validate_and_update(&mut self, config: SpawnSystem) -> ZionResult<()> {
//        Ok(())
//    }
//
//    fn spawn<'a>(&mut self, builder: &mut SystemBuilder<'a>) -> Box<dyn BevySystem> {
//        BevySystemContainer::new(Hello::hello, (), |a, config| {})
//    }
//}
