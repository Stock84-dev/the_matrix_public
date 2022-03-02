//use std::marker::PhantomData;
//
//use bevy::ecs::schedule::{IntoSystemDescriptor, RunCriteriaDescriptor};
//use bevy::prelude::*;
//use mouse::ext::IdExt;
//
//use crate::Stages;
//
//pub struct SystemBuilder<'a, S, P> {
//    app: &'a mut App,
//    system: S,
//    params: PhantomData<P>,
//    stage: Stages,
//}
//
//pub struct LabelSystemBuilder<'a, P> {
//    app: &'a mut App,
//    system: RunCriteriaDescriptor,
//    params: PhantomData<P>,
//    stage: Stages,
//}
//
//impl<'a, P> LabelSystemBuilder<'a, P> {
//    pub fn new(app: &'a mut App, system: RunCriteriaDescriptor, stage: Stages) -> Self {
//        Self {
//            app,
//            system,
//            params: Default::default(),
//            stage: Stages::Update,
//        }
//    }
//
//    pub fn label<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        Self::new(self.app, self.system.label(T::id()), self.stage)
//    }
//
//    pub fn before<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        Self::new(self.app, self.system.before(T::id()), self.stage)
//    }
//
//    pub fn after<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        Self::new(self.app, self.system.after(T::id()), self.stage)
//    }
//
//    pub fn add(self) -> &'a mut App {
//        self.app.add_system_to_stage(self.stage, self.system);
//        self.app
//    }
//}
//
//impl<'a, P, S: IntoSystemDescriptor<P>> SystemBuilder<'a, S, P> {
//    pub fn new(system: S, app: &'a mut App) -> Self {
//        Self {
//            app,
//            system,
//            params: Default::default(),
//            stage: Stages::Update,
//        }
//    }
//
//    pub fn label<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        LabelSystemBuilder::new(self.app, self.system.label(T::id()), self.stage)
//    }
//
//    pub fn before<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        LabelSystemBuilder::new(self.app, self.system.before(T::id()), self.stage)
//    }
//
//    pub fn after<T: 'static>(self, label: T) -> LabelSystemBuilder<'a, P> {
//        LabelSystemBuilder::new(self.app, self.system.after(T::id()), self.stage)
//    }
//
//    pub fn add(self) -> &'a mut App {
//        self.app.add_system_to_stage(self.stage, self.system);
//        self.app
//    }
//}
//
//pub trait AppExt {
//    fn system<'a, S: IntoSystemDescriptor<P>, P>(
//        &'a mut self,
//        system: S,
//    ) -> SystemBuilder<'a, S, P>;
//}
//
//impl AppExt for App {
//    fn system<'a, S: IntoSystemDescriptor<P>, P>(
//        &'a mut self,
//        system: S,
//    ) -> SystemBuilder<'a, S, P> {
//        SystemBuilder::new(system, self)
//    }
//}
