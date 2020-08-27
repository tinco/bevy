use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;

// This example shows multiple schedules running at the same time. So you can have
// the default schedule at your target screen framerate, and for example your
// physics game loop at a fixed 30 fps.
fn main() {
    // this app has the default schedule running at 1 fps and a second schedule
    // running at 2 fps.
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0,
        )))
        .add_schedule("faster", ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            2.0,
        )))
        .add_system(hello_world_system.system())
        .add_system_to_schedule("faster", zippy_system.system())
        .run();
}

fn hello_world_system() {
    println!("hello world");
}

fn zippy_system() {
    println!("zip");
}
