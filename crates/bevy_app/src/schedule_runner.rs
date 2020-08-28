use super::{AppBuilder};
use crate::{
    app::AppExit,
    event::{EventReader, Events},
    plugin::Plugin,
};
use std::{
    thread,
    time::{Duration, Instant},
};

use bevy_ecs::{Resources, ScheduleContext, World};

/// Determines the method used to run an [App]'s `Schedule`
#[derive(Copy, Clone, Debug)]
pub enum RunMode {
    Loop { wait: Option<Duration> },
    Once,
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Loop { wait: None }
    }
}

/// Configures an App to run its [Schedule](bevy_ecs::Schedule) according to a given [RunMode]
pub struct ScheduleRunnerPlugin {
    pub run_mode: RunMode,
    pub schedule_name: &'static str,
}

impl Default for ScheduleRunnerPlugin {
    fn default() -> Self {
        ScheduleRunnerPlugin {
            run_mode: Default::default(),
            schedule_name: "default",
        }
    }
}

impl ScheduleRunnerPlugin {
    pub fn run_once() -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Once,
            schedule_name: "default",
        }
    }

    pub fn run_loop(wait_duration: Duration) -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Loop {
                wait: Some(wait_duration),
            },
            schedule_name: "default",
        }
    }
}

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let run_mode = self.run_mode;
        let schedule_context = app.app.schedule_context_mut(self.schedule_name);
        // TODO make a system where multiple runners can co exist
        // I think by having app have `runners` instead of just one
        // and having each runner be an iterator returning the amount
        // of time to wait until the next iteration or Nothing if it's
        // done.

        schedule_context.set_runner(move |schedule_context: &mut ScheduleContext, world: &mut World, resources: &mut Resources| {
            let mut app_exit_event_reader = EventReader::<AppExit>::default();
            match run_mode {
                RunMode::Once => {
                    schedule_context.update(world, resources);
                }
                RunMode::Loop { wait } => loop {
                    let start_time = Instant::now();

                    if let Some(app_exit_events) = resources.get_mut::<Events<AppExit>>() {
                        if app_exit_event_reader.latest(&app_exit_events).is_some() {
                            break;
                        }
                    }

                    schedule_context.update(world, resources);

                    if let Some(app_exit_events) = resources.get_mut::<Events<AppExit>>() {
                        if app_exit_event_reader.latest(&app_exit_events).is_some() {
                            break;
                        }
                    }

                    let end_time = Instant::now();

                    if let Some(wait) = wait {
                        let exe_time = end_time - start_time;
                        if exe_time < wait {
                            thread::sleep(wait - exe_time);
                        }
                    }
                },
            }
        });
    }
}
