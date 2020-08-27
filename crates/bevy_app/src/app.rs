use crate::app_builder::AppBuilder;
use bevy_ecs::{ParallelExecutor, Resources, Schedule, World};
use std::collections::HashMap;

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
    pub world: Box<World>,
    pub resources: Box<Resources>,
    pub runner: Box<dyn Fn(App)>,
    pub schedules: HashMap<&'static str, Schedule>,
    pub executor: ParallelExecutor,
    pub startup_schedule: Schedule,
    pub startup_executor: ParallelExecutor,
}

impl Default for App {
    fn default() -> Self {
        Self {
            world: Default::default(),
            resources: Default::default(),
            schedules: vec![("default", Default::default())].into_iter().collect(),
            executor: Default::default(),
            startup_schedule: Default::default(),
            startup_executor: ParallelExecutor::without_tracker_clears(),
            runner: Box::new(run_once),
        }
    }
}

fn run_once(mut app: App) {
    app.update();
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn update(&mut self) {
        let mut schedule = self.schedules.get_mut("default").expect("A default schedule should exist");
        schedule.initialize(&mut self.resources);
        self.executor
            .run(&mut schedule, &mut self.world, &mut self.resources);
    }

    pub fn run(mut self) {
        self.startup_schedule.initialize(&mut self.resources);
        self.startup_executor.run(
            &mut self.startup_schedule,
            &mut self.world,
            &mut self.resources,
        );

        let runner = std::mem::replace(&mut self.runner, Box::new(run_once));
        (runner)(self);
    }

    pub fn run_schedule(&mut self, schedule_name: &'static str) {
        self.schedules
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
            .run(&mut self.world, &mut self.resources)
    }

    // TODO if we remove this, also unbox world and resources
    pub fn run_schedules(&mut self) {
        for schedule in self.schedules.values_mut() {
            schedule.run(&mut self.world, &mut self.resources);
        }
    }

    pub fn schedule_mut(&mut self, schedule_name: &'static str) -> &mut Schedule {
        self.schedules
            .get_mut(schedule_name)
            .unwrap_or_else(|| panic!("Schedule {} should exist.", schedule_name))
    }

    pub fn default_schedule_mut(&mut self) -> &mut Schedule {
        self.schedules.get_mut("default").expect("A default schedule should exist")
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
