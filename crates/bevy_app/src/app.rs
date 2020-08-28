use crate::app_builder::AppBuilder;
use bevy_ecs::{Resources, Schedule, ScheduleContext, ParallelExecutor, World};
use std::collections::HashMap;
use rayon::prelude::*;
use parking_lot::{RwLock, MutexGuard};

#[allow(clippy::needless_doctest_main)]
/// Containers of app logic and data
///
/// App store the ECS World, Resources, Schedule, and Executor. They also store the "run" function of the App, which
/// by default executes the App schedule once. Apps are constructed using the builder pattern.
///
/// ## Example
/// Here is a simple "Hello World" Bevy app:
/// ```
///use bevy_app::prelude::*;
///use bevy_ecs::prelude::*;
///
///fn main() {
///    App::build()
///        .add_system(hello_world_system.system())
///        .run();
///}
///
///fn hello_world_system() {
///    println!("hello world");
///}
/// ```
pub struct App {
    pub world: RwLock<World>,
    pub resources: RwLock<Resources>,
    pub schedules: HashMap<&'static str, ScheduleContext>,
    pub startup_schedule: Schedule,
    pub startup_executor: ParallelExecutor,
}

impl Default for App {
    fn default() -> Self {
        Self {
            world: Default::default(),
            resources: Default::default(),
            schedules: vec![("default", Default::default())].into_iter().collect(),
            startup_schedule: Default::default(),
            startup_executor: ParallelExecutor::without_tracker_clears(),
        }
    }
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }


    pub fn run(mut self) {
        self.startup_schedule.initialize(&mut self.resources.write());
        self.startup_executor.run(
            &mut self.startup_schedule,
            &mut self.world.write(),
            &mut self.resources.write(),
        );

        let resources = &mut self.resources;
        let world = &mut self.world;

        self.schedules.par_iter_mut().for_each(|(_,schedule_context)| {
            schedule_context.run(&mut world.write(), &mut resources.write());
        })
    }

    pub fn run_schedule(&mut self, schedule_name: &'static str) {
        self.schedules
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
            .run(&mut self.world.write(), &mut self.resources.write())
    }

    pub fn schedule_mut(&mut self, schedule_name: &'static str) -> MutexGuard<Schedule> {
        self.schedules
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
            .schedule.lock()
    }

    pub fn schedule_context_mut(&mut self, schedule_name: &'static str) -> &mut ScheduleContext {
        self.schedules
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
    }

    pub fn default_schedule_mut(&mut self) -> MutexGuard<Schedule> {
        self.schedules.get_mut("default").expect("A default schedule should exist").schedule.lock()
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
