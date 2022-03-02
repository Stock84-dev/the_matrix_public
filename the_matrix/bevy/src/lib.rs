#![feature(fn_traits)]
#![feature(unboxed_closures)]

//! This crate must be called bevy so that derive macros could work

use std::any::TypeId;
use std::hash::Hash;
use std::ops::Deref;

use bevy_asset::AssetServer;
use bevy_ecs::prelude::{Mut, NonSendMut, ResMut};
use bevy_ecs::schedule::IntoSystemDescriptor;
use bevy_utils::HashSet;
use mouse::traits::AsyncWriteSeek;
use prelude::*;
pub use {
    bevy_app as app, bevy_ecs as ecs, bevy_log as log, bevy_math as math, bevy_reflect as reflect,
    bevy_scene as scene, bevy_transform as transform, bevy_utils as utils,
};

pub mod event;
pub mod task;

pub mod prelude {
    pub type Mutable<'a, T> = Mut<'a, T>;
    pub use mouse::prelude::*;

    pub use super::app::prelude::*;
    pub use super::app::Events;
    pub use super::ecs::prelude::*;
    pub use super::ecs::system::{Resource, SystemParam, SystemState};
    pub use super::event::*;
    pub use super::math::prelude::*;
    pub use super::reflect::prelude::*;
    pub use super::reflect::TypeRegistry;
    pub use super::scene::prelude::*;
    pub use super::task::{Task, *};
    pub use super::transform::prelude::*;
    pub use super::{AppExt, InitStages, MutExt, Plug, PluginLoader, Stages};

    type Mut<'a, T> = super::ecs::change_detection::Mut<'a, T>;
}

pub struct PluginLoader<'a> {
    app: &'a mut App,
    loaded_plugins: HashSet<TypeId>,
}

impl<'a> PluginLoader<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self {
            app,
            loaded_plugins: Default::default(),
        }
    }

    pub fn load<T: Plug + 'static>(&mut self, _plugin: T) -> &mut Self {
        if self.loaded_plugins.contains(&T::id()) {
            return self;
        }
        T::deps(self);
        info!("Loading: {}", std::any::type_name::<T>());
        T::load(self.app);
        self.loaded_plugins.insert(T::id());
        self
    }
}

pub trait AppExt {
    fn render_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn startup_wgpu_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self;
    fn startup_wgpu_pipeline_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self;
    fn pre_pre_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn input_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn pre_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn added_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn post_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self;
    fn add_pipeline<T: Resource>(&mut self) -> &mut Self;
    fn add_unique_event<T: Resource + Hash + Eq>(&mut self) -> &mut Self;
    fn add_pipelined_portal<T: Resource>(&mut self) -> &mut Self;
    fn add_portal<T: Resource>(&mut self, bound: usize) -> &mut Self;
    fn add_unbounded_portal<T: Resource>(&mut self) -> &mut Self;
}

impl AppExt for App {
    fn render_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(Stages::Render, system)
    }

    fn startup_wgpu_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.add_startup_system_to_stage(StartupStage::Startup, system)
    }

    fn startup_wgpu_pipeline_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.add_startup_system_to_stage(StartupStage::PostStartup, system)
    }

    fn added_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(Stages::Added, system)
    }

    fn post_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(CoreStage::PostUpdate, system)
    }

    fn pre_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(CoreStage::PreUpdate, system)
    }

    fn pre_pre_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(Stages::PrePreUpdate, system)
    }

    //    fn add_pipeline<T: Resource>(&mut self) -> &mut Self {
    //        self.init_resource::<PipelinedEvents<T>>()
    //            .add_system_to_stage(CoreStage::First, PipelinedEvents::<T>::update_system)
    //    }
    //
    //    fn add_unique_event<T: Resource + Hash + Eq>(&mut self) -> &mut Self {
    //        self.insert_resource(UniqueEvents::<T>::default());
    //        self.add_system_to_stage(CoreStage::First, UniqueEvents::<T>::update_system)
    //    }

    fn input_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(Stages::Input, system)
    }

    //    fn add_portal<T: Resource>(&mut self, bound: usize) -> &mut Self {
    //        self.insert_resource(Portal::<T>::new(bound))
    //            .add_system_to_stage(CoreStage::First, Portal::<T>::update_system)
    //    }

    fn add_unbounded_portal<T: Resource>(&mut self) -> &mut Self {
        self.add_portal::<T>(usize::MAX)
    }

    fn add_pipeline<T: Resource>(&mut self) -> &mut Self {
        todo!()
    }

    fn add_unique_event<T: Resource + Hash + Eq>(&mut self) -> &mut Self {
        todo!()
    }

    fn add_pipelined_portal<T: Resource>(&mut self) -> &mut Self {
        todo!()
    }

    fn add_portal<T: Resource>(&mut self, bound: usize) -> &mut Self {
        todo!()
    }

    //    fn add_pipelined_portal<T: Resource>(&mut self) -> &mut Self {
    //        self.insert_resource(PipelinedPortal(Portal::<Pipelined<T>>::new(1)))
    //            .add_system_to_stage(CoreStage::First, PipelinedPortal::<T>::update_system)
    //    }
}

#[derive(StageLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub enum InitStages {
    /// Runs before `PreStartup`
    Window,
    Pipeline,
}

#[derive(StageLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Stages {
    /// Runs before `PrePreUpdate`
    Input,
    /// Runs before `PreUpdate`
    PrePreUpdate,
    /// Runs after `Update`
    Added,
    /// Runs after `PostUpdate`
    Render,
    /// Runs after `Render`
    Draw,
}

pub trait Plug {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b>;
    fn load<'a>(app: &'a mut App) -> &mut App;
}

pub trait MutExt {
    type Target;
    /// Derefs into mutable reference without setting component to changed.
    fn deref_mut_sneak(&mut self) -> &mut Self::Target;
}

macro_rules! impl_mut_ext {
    ($item:ident) => {
        impl<'a, T: Resource> MutExt for $item<'a, T> {
            type Target = T;

            fn deref_mut_sneak(&mut self) -> &'a mut Self::Target {
                unsafe { &mut *(self.deref().deref() as *const T as *mut T) }
            }
        }
    };
}
impl_mut_ext!(ResMut);
impl_mut_ext!(Mut);
impl_mut_ext!(NonSendMut);

#[test]
fn test_s() {
    fn startup() {
        println!("startup");
    }
    fn update() {
        println!("update");
    }
    let mut app = App::empty();
    app.add_default_stages()
        .add_startup_system(|| println!("hi"))
        .add_system(update);
    app.set_runner(|mut app| {
        app.schedule
            .stage(CoreStage::Startup, |sch: &mut Schedule| {
                sch.run(&mut app.world);
                sch
            });
        println!("s");
        app.update();
        println!("s");
        app.update();
        println!("s");
    });
    app.run();
}
