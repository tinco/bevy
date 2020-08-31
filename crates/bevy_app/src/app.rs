use crate::app_builder::AppBuilder;
use bevy_ecs::{Resources, Schedule, ParallelExecutor, World};
use std::{
    collections::HashMap,
    thread,
};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock, MutexGuard};

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
    pub world: Arc<RwLock<World>>,
    pub resources: Arc<RwLock<Resources>>,
    pub schedule_contexts: HashMap<&'static str, ScheduleContext>,
    pub startup_schedule: Schedule,
    pub startup_executor: ParallelExecutor,
}

impl Default for App {
    fn default() -> Self {
        Self {
            world: Default::default(),
            resources: Default::default(),
            schedule_contexts: vec![("default", Default::default())].into_iter().collect(),
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
        self.startup_schedule.initialize(self.resources.clone());
        self.startup_executor.run(
            &mut self.startup_schedule, self.world.clone(), self.resources.clone()
        );

        let world = self.world;
        let resources = self.resources;
        let schedule_contexts = self.schedule_contexts;
        
        schedule_contexts.into_iter().for_each(|(_, schedule_context)| {
            let world = world.clone();
            let resources = resources.clone();
            thread::spawn(move || {
                // TODO I'm fairly certain from this point on we can just deref the Arc
                schedule_context.run(world, resources);
            });
        });
    }

    pub fn schedule_mut(&mut self, schedule_name: &'static str) -> MutexGuard<Schedule> {
        self.schedule_contexts
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
            .schedule.lock()
    }

    pub fn schedule_context_mut(&mut self, schedule_name: &'static str) -> &mut ScheduleContext {
        self.schedule_contexts
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
    }

    pub fn default_schedule_mut(&mut self) -> MutexGuard<Schedule> {
        self.schedule_contexts
            .get_mut("default").expect("A default schedule should exist").schedule.lock()
    }
}

type Runner = dyn Fn(ScheduleContext, Arc<RwLock<World>>,Arc<RwLock<Resources>>) + Send;

pub struct ScheduleContext {
    pub schedule: Mutex<Schedule>,
    pub executor: ParallelExecutor,
    pub runner: Box<Runner>,
}

impl ScheduleContext {
    pub fn run(mut self, world: Arc<RwLock<World>>, resources: Arc<RwLock<Resources>>) {
        let runner = std::mem::replace(&mut self.runner, Box::new(ScheduleContext::run_once));
        (runner)(self, world, resources);
    }

    pub fn update(&mut self, world: Arc<RwLock<World>>, resources: Arc<RwLock<Resources>>) {
        let mut schedule = self.schedule.lock();
        schedule.initialize(resources.clone());
        self.executor.run(&mut schedule, world, resources);
    }

    pub fn run_once(mut self, world: Arc<RwLock<World>>, resources: Arc<RwLock<Resources>>) {
        self.update(world, resources);
    }

    pub fn set_runner(&mut self, run_fn: impl Fn(ScheduleContext, Arc<RwLock<World>>,Arc<RwLock<Resources>>) + 'static + Send) -> &mut Self {
        self.runner = Box::new(run_fn);
        self
    }
}

impl Default for ScheduleContext {
     fn default() -> Self {
        Self {
            schedule: Default::default(),
            runner: Box::new(ScheduleContext::run_once),
            executor: Default::default(),
        }
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
