// use bevy::app::ScheduleRunnerPlugin;
// use bevy::ecs::schedule::SystemDescriptor;
// use bevy::prelude::*;
// use bevy::{CorePlugin, Scheduler};
// use mouse::ext::DynClone;
// use zion_db::definitions::SystemId;
//
// pub struct SystemPlugin;
//
// impl Plug for SystemPlugin {
//    fn deps<'a>(&mut self, app: &'a mut App) -> &'a mut App {
//        app.load(CorePlugin)
//    }
//
//    fn load<'a>(&mut self, app: &'a mut App) -> &'a mut App {
//        app
//    }
//
//    fn schedule<'a>(&mut self, scheduler: &'a mut Scheduler<'a>) -> &'a mut Scheduler<'a> {
//        let (schedule, world) = scheduler.schedule_and_world_mut();
//        let query = world.query::<&SystemComponent>() ;
//        for system in query.iter(world){
//            schedule.add_system_to_stage(system.stage.clone_box(), system.factory.system());
//        }
//        scheduler
//    }
//}
// fn add_system(app: &mut App) {
//    let query = app
//        .world
//        .query_filtered::<SystemComponent, Changed<SystemComponent>>();
//    for system in query.iter(&app.world) {
//        app.schedule
//            .add_system_to_stage(system.stage, system.factory.system());
//    }
//}
// fn remove_system() {}
//
// fn reschedule_systems(app: &mut App) {
//    let (removals, systems): SystemState<(
//        RemovedComponents<SystemComponent>,
//        Query<&SystemComponent>,
//    )> = SystemState::new(&mut app.world);
//    if removals.iter().next().is_some() {
//        let mut schedule = Schedule::default();
//        for system in systems.iter() {
//            let system: &SystemComponent = system;
//            schedule.add_system_to_stage(system.stage.clone_box(), system.factory.system());
//        }
//        stage.downcast_ref::<SystemStage>()
//        app.schedule = schedule;
//    }
//    let (a, query) = system_state.get(&world);
//    let query = app
//        .world
//        .query_filtered::<SystemComponent, Added<SystemComponent>>();
//    let query = app
//        .world
//        .query_filtered::<SystemComponent, Added<SystemComponent>>();
//    for system in query.iter(&app.world) {
//        app.schedule
//            .add_system_to_stage(system.stage, system.factory.system());
//    }
//}
//
//// pub enum ScheduleCommand {
////    AddSystem(AddSystem),
////    RemoveSystem(RemoveSystem),
//// }
////
// pub struct RemoveSystem {
//    pub id: Entity,
//    pub stage: Box<dyn StageLabel>,
//}
//
