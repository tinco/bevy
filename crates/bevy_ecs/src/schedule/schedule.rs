use crate::{
    resource::Resources,
    schedule::{ParallelExecutorOptions},
    system::{System, SystemId, ThreadLocalExecution},
};
use bevy_hecs::World;
use parking_lot::{Mutex,RwLock};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// An ordered collection of stages, which each contain an ordered list of [System]s.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    pub(crate) stages: HashMap<Cow<'static, str>, Vec<Arc<Mutex<Box<dyn System>>>>>,
    pub(crate) stage_order: Vec<Cow<'static, str>>,
    pub(crate) system_ids: HashSet<SystemId>,
    generation: usize,
    last_initialize_generation: usize,
}

impl Schedule {
    pub fn add_stage(&mut self, stage: impl Into<Cow<'static, str>>) {
        let stage: Cow<str> = stage.into();
        if self.stages.get(&stage).is_some() {
            panic!("Stage already exists: {}", stage);
        } else {
            self.stages.insert(stage.clone(), Vec::new());
            self.stage_order.push(stage);
        }
    }

    pub fn add_stage_after(
        &mut self,
        target: impl Into<Cow<'static, str>>,
        stage: impl Into<Cow<'static, str>>,
    ) {
        let target: Cow<str> = target.into();
        let stage: Cow<str> = stage.into();
        if self.stages.get(&stage).is_some() {
            panic!("Stage already exists: {}", stage);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage)| **stage == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}", target));

        self.stages.insert(stage.clone(), Vec::new());
        self.stage_order.insert(target_index + 1, stage);
    }

    pub fn add_stage_before(
        &mut self,
        target: impl Into<Cow<'static, str>>,
        stage: impl Into<Cow<'static, str>>,
    ) {
        let target: Cow<str> = target.into();
        let stage: Cow<str> = stage.into();
        if self.stages.get(&stage).is_some() {
            panic!("Stage already exists: {}", stage);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage)| **stage == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}", target));

        self.stages.insert(stage.clone(), Vec::new());
        self.stage_order.insert(target_index, stage);
    }

    pub fn add_system_to_stage(
        &mut self,
        stage_name: impl Into<Cow<'static, str>>,
        system: Box<dyn System>,
    ) -> &mut Self {
        let stage_name = stage_name.into();
        let systems = self
            .stages
            .get_mut(&stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        if self.system_ids.contains(&system.id()) {
            panic!(
                "System with id {:?} ({}) already exists",
                system.id(),
                system.name()
            );
        }
        self.system_ids.insert(system.id());
        systems.push(Arc::new(Mutex::new(system)));

        self.generation += 1;
        self
    }

    pub fn add_system_to_stage_front(
        &mut self,
        stage_name: impl Into<Cow<'static, str>>,
        system: Box<dyn System>,
    ) -> &mut Self {
        let stage_name = stage_name.into();
        let systems = self
            .stages
            .get_mut(&stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        if self.system_ids.contains(&system.id()) {
            panic!(
                "System with id {:?} ({}) already exists",
                system.id(),
                system.name()
            );
        }
        self.system_ids.insert(system.id());
        systems.insert(0, Arc::new(Mutex::new(system)));

        self.generation += 1;
        self
    }

    pub fn run_once(&mut self, world: Arc<RwLock<World>>, resources: Arc<RwLock<Resources>>) {
        for stage_name in self.stage_order.iter() {
            if let Some(stage_systems) = self.stages.get_mut(stage_name) {
                for system in stage_systems.iter_mut() {
                    let mut system = system.lock();
                    #[cfg(feature = "profiler")]
                    {
                        let resources = resources.read();
                        crate::profiler_start(&resources, system.name().clone());
                    }
                    {
                        let world = world.read();
                        system.update_archetype_access(&world);
                    }
                    match system.thread_local_execution() {
                        ThreadLocalExecution::NextFlush => {
                            {
                                let world = world.read();
                                let resources = resources.read();
                                system.run(&world, &resources);
                            }                        },
                        ThreadLocalExecution::Immediate => {
                            {
                                let world = world.read();
                                let resources = resources.read();
                                system.run(&world, &resources);
                            }
                            // NOTE: when this is made parallel a full sync is required here
                            // TODO: is this a full sync now?
                            {
                                let mut world = world.write();
                                let mut resources = resources.write();
                                system.run_thread_local(&mut world, &mut resources);
                            }
                        }
                    }
                    #[cfg(feature = "profiler")]
                    {
                        let resources = resources.read();
                        crate::profiler_stop(resources, system.name().clone());
                    }
                }

                // "flush"
                // NOTE: when this is made parallel a full sync is required here
                // TODO: is this a full sync now?
                for system in stage_systems.iter_mut() {
                    let mut system = system.lock();
                    match system.thread_local_execution() {
                        ThreadLocalExecution::NextFlush => {
                            let mut world = world.write();
                            let mut resources = resources.write();
                            system.run_thread_local(&mut world, &mut resources)
                        }
                        ThreadLocalExecution::Immediate => { /* already ran immediate */ }
                    }
                }
            }
        }

        world.write().clear_trackers();
    }

    // TODO: move this code to ParallelExecutor
    pub fn initialize(&mut self, resources: Arc<RwLock<Resources>>) {
        if self.last_initialize_generation == self.generation {
            return;
        }

        let thread_pool_builder = resources.read()
            .get::<ParallelExecutorOptions>()
            .map(|options| (*options).clone())
            .unwrap_or_else(ParallelExecutorOptions::default)
            .create_builder();
        // For now, bevy_ecs only uses the global thread pool so it is sufficient to configure it once here.
        // Dont call .unwrap() as the function is called twice..
        let _ = thread_pool_builder.build_global();

        for stage in self.stages.values_mut() {
            for system in stage.iter_mut() {
                let mut system = system.lock();
                let mut resources = resources.write();
                system.initialize(&mut resources);
            }
        }

        self.last_initialize_generation = self.generation;
    }

    pub fn generation(&self) -> usize {
        self.generation
    }
}
