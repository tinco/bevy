use crate::{
    app::{App, AppExit},
    event::Events,
    plugin::{dynamically_load_plugin, Plugin},
    schedule_runner::{ScheduleRunnerPlugin},
    stage, startup_stage,
};
use bevy_ecs::{FromResources, IntoQuerySystem, Resources, System, World};

/// Configure [App]s using the builder pattern
pub struct AppBuilder {
    pub app: App,
}

impl Default for AppBuilder {
    fn default() -> Self {
        let mut app_builder = AppBuilder {
            app: App::default(),
        };

        app_builder.add_default_stages();
        app_builder.add_event::<AppExit>();
        app_builder
    }
}

impl AppBuilder {
    pub fn empty() -> AppBuilder {
        AppBuilder {
            app: App::default(),
        }
    }

    pub fn resources(&self) -> &Resources {
        &self.app.resources
    }

    pub fn resources_mut(&mut self) -> &mut Resources {
        &mut self.app.resources
    }

    pub fn run(&mut self) {
        let app = std::mem::take(&mut self.app);
        app.run();
    }

    pub fn set_world(&mut self, world: World) -> &mut Self {
        self.app.world = Box::new(world);
        self
    }

    pub fn add_schedule(&mut self, schedule_name: &'static str, mut schedule_runner: ScheduleRunnerPlugin) -> &mut Self {
        self.app.schedules.insert(schedule_name, Default::default());
        self.add_default_stages_to_schedule(schedule_name);
        schedule_runner.schedule_name = schedule_name;
        self.add_plugin(schedule_runner);
        self
    }

    pub fn add_stage(&mut self, stage_name: &'static str) -> &mut Self {
        self.app.default_schedule_mut().add_stage(stage_name);
        self
    }

    pub fn add_stage_to_schedule(&mut self, schedule_name: &'static str, stage_name: &'static str) -> &mut Self {
        self.app.schedule_mut(schedule_name).add_stage(stage_name);
        self
    }

    pub fn add_stage_after(&mut self, target: &'static str, stage_name: &'static str) -> &mut Self {
        self.app.default_schedule_mut().add_stage_after(target, stage_name);
        self
    }

    pub fn add_stage_before(
        &mut self,
        target: &'static str,
        stage_name: &'static str,
    ) -> &mut Self {
        self.app.default_schedule_mut().add_stage_before(target, stage_name);
        self
    }

    pub fn add_startup_stage(&mut self, stage_name: &'static str) -> &mut Self {
        self.app.startup_schedule.add_stage(stage_name);
        self
    }

    pub fn add_system(&mut self, system: Box<dyn System>) -> &mut Self {
        self.add_system_to_stage(stage::UPDATE, system)
    }

    pub fn add_systems(&mut self, systems: Vec<Box<dyn System>>) -> &mut Self {
        self.add_systems_to_stage(stage::UPDATE, systems)
    }

    pub fn add_system_to_schedule(&mut self, schedule_name: &'static str, system: Box<dyn System>) -> &mut Self {
        self.add_system_to_stage_on_schedule(schedule_name, stage::UPDATE, system)
    }

    pub fn add_systems_to_schedule(&mut self, schedule_name: &'static str, systems: Vec<Box<dyn System>>) -> &mut Self {
        self.add_systems_to_stage_on_schedule(schedule_name, stage::UPDATE, systems)
    }

    pub fn init_system(
        &mut self,
        build: impl FnMut(&mut Resources) -> Box<dyn System>,
    ) -> &mut Self {
        self.init_system_to_stage(stage::UPDATE, build)
    }

    pub fn init_system_to_stage(
        &mut self,
        stage: &'static str,
        mut build: impl FnMut(&mut Resources) -> Box<dyn System>,
    ) -> &mut Self {
        let system = build(&mut self.app.resources);
        self.add_system_to_stage(stage, system)
    }

    pub fn add_startup_system_to_stage(
        &mut self,
        stage_name: &'static str,
        system: Box<dyn System>,
    ) -> &mut Self {
        self.app
            .startup_schedule
            .add_system_to_stage(stage_name, system);
        self
    }

    pub fn add_startup_systems_to_stage(
        &mut self,
        stage_name: &'static str,
        systems: Vec<Box<dyn System>>,
    ) -> &mut Self {
        for system in systems {
            self.app
                .startup_schedule
                .add_system_to_stage(stage_name, system);
        }
        self
    }

    pub fn add_startup_system(&mut self, system: Box<dyn System>) -> &mut Self {
        self.app
            .startup_schedule
            .add_system_to_stage(startup_stage::STARTUP, system);
        self
    }

    pub fn add_startup_systems(&mut self, systems: Vec<Box<dyn System>>) -> &mut Self {
        self.add_startup_systems_to_stage(startup_stage::STARTUP, systems)
    }

    pub fn init_startup_system(
        &mut self,
        build: impl FnMut(&mut Resources) -> Box<dyn System>,
    ) -> &mut Self {
        self.init_startup_system_to_stage(startup_stage::STARTUP, build)
    }

    pub fn init_startup_system_to_stage(
        &mut self,
        stage: &'static str,
        mut build: impl FnMut(&mut Resources) -> Box<dyn System>,
    ) -> &mut Self {
        let system = build(&mut self.app.resources);
        self.add_startup_system_to_stage(stage, system)
    }

    pub fn add_default_stages(&mut self) -> &mut Self {
        self.add_startup_stage(startup_stage::STARTUP)
            .add_startup_stage(startup_stage::POST_STARTUP)
            .add_stage(stage::FIRST)
            .add_stage(stage::EVENT_UPDATE)
            .add_stage(stage::PRE_UPDATE)
            .add_stage(stage::UPDATE)
            .add_stage(stage::POST_UPDATE)
            .add_stage(stage::LAST)
    }

    pub fn add_default_stages_to_schedule(&mut self, schedule_name: &'static str) -> &mut Self {
        self.add_stage_to_schedule(schedule_name, stage::FIRST)
            .add_stage_to_schedule(schedule_name, stage::EVENT_UPDATE)
            .add_stage_to_schedule(schedule_name, stage::PRE_UPDATE)
            .add_stage_to_schedule(schedule_name, stage::UPDATE)
            .add_stage_to_schedule(schedule_name, stage::POST_UPDATE)
            .add_stage_to_schedule(schedule_name, stage::LAST)
    }

    pub fn add_system_to_stage(
        &mut self,
        stage_name: &'static str,
        system: Box<dyn System>,
    ) -> &mut Self {
        self.app.default_schedule_mut().add_system_to_stage(stage_name, system);
        self
    }

    pub fn add_system_to_stage_front(
        &mut self,
        stage_name: &'static str,
        system: Box<dyn System>,
    ) -> &mut Self {
        self.app
            .default_schedule_mut()
            .add_system_to_stage_front(stage_name, system);
        self
    }

    pub fn add_systems_to_stage(
        &mut self,
        stage_name: &'static str,
        systems: Vec<Box<dyn System>>,
    ) -> &mut Self {
        for system in systems {
            self.app.default_schedule_mut().add_system_to_stage(stage_name, system);
        }
        self
    }

    pub fn add_system_to_stage_on_schedule(
        &mut self,
        schedule_name: &'static str,
        stage_name: &'static str,
        system: Box<dyn System>,
    ) -> &mut Self {
        self.app.schedule_mut(schedule_name).add_system_to_stage(stage_name, system);
        self
    }

    pub fn add_systems_to_stage_on_schedule(
        &mut self,
        schedule_name: &'static str,
        stage_name: &'static str,
        systems: Vec<Box<dyn System>>,
    ) -> &mut Self {
        for system in systems {
            self.app.schedule_mut(schedule_name).add_system_to_stage(stage_name, system);
        }
        self
    }

    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.add_resource(Events::<T>::default())
            .add_system_to_stage(stage::EVENT_UPDATE, Events::<T>::update_system.system())
    }

    pub fn add_event_to_schedule<T>(&mut self, schedule_name: &'static str) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.add_resource(Events::<T>::default())
            .app.schedule_mut(schedule_name)
                .add_system_to_stage(stage::EVENT_UPDATE, Events::<T>::update_system.system());
        self
    }

    pub fn add_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.app.resources.insert(resource);
        self
    }

    pub fn init_resource<R>(&mut self) -> &mut Self
    where
        R: FromResources + Send + Sync + 'static,
    {
        let resource = R::from_resources(&self.app.resources);
        self.app.resources.insert(resource);

        self
    }

    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
        self.app.runner = Box::new(run_fn);
        self
    }

    pub fn load_plugin(&mut self, path: &str) -> &mut Self {
        let (_lib, plugin) = dynamically_load_plugin(path);
        log::debug!("loaded plugin: {}", plugin.name());
        plugin.build(self);
        self
    }

    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        log::debug!("added plugin: {}", plugin.name());
        plugin.build(self);
        self
    }
}
