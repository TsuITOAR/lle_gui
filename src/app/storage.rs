use crate::{
    checkpoint,
    controller::{Controller, Simulator},
    file::{self, FileManager},
    random::RandomNoise,
    scouting,
    views::Views,
};

use super::Core;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(bound(
    serialize = "S: Simulator, S::OwnedState: serde::Serialize, P: serde::Serialize",
    deserialize = "S: Simulator, S::OwnedState: for<'a> serde::Deserialize<'a>, P: for<'a> serde::Deserialize<'a>"
))]
pub struct CoreStorage<P, S>
where
    S: Simulator,
{
    pub(crate) dim: usize,
    pub(crate) controller: P,
    pub(crate) simulator_state: S::OwnedState,
    pub(crate) random: RandomNoise,
}

impl<P, S: Simulator> Clone for CoreStorage<P, S>
where
    P: Clone,
    S::OwnedState: Clone,
{
    fn clone(&self) -> Self {
        Self {
            dim: self.dim,
            controller: self.controller.clone(),
            simulator_state: self.simulator_state.clone(),
            random: self.random.clone(),
        }
    }
}

impl<'a, C, S> From<&'a Core<C, S>> for CoreStorage<C, S>
where
    C: Clone,
    S: Simulator,
{
    fn from(core: &'a Core<C, S>) -> Self {
        Self {
            dim: core.dim,
            controller: core.controller.clone(),
            simulator_state: core.simulator.get_owned_state(),
            random: core.random.clone(),
        }
    }
}

impl<C, S> From<CoreStorage<C, S>> for Core<C, S>
where
    C: Controller<S>,
    S: Simulator,
{
    fn from(storage: CoreStorage<C, S>) -> Self {
        let mut e = storage.controller.construct_engine(storage.dim);
        e.set_owned_state(storage.simulator_state);
        Self {
            dim: storage.dim,
            controller: storage.controller,
            simulator: e,
            random: storage.random,
        }
    }
}

impl<C, S> Default for CoreStorage<C, S>
where
    C: Default + Controller<S>,
    S: Simulator,
{
    fn default() -> Self {
        let dim: usize = 128;
        Self {
            dim,
            controller: C::default(),
            simulator_state: S::default_state(dim),
            random: RandomNoise::default(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "CoreStorage<P, S>: serde::Serialize, Views<V>: serde::Serialize, T: serde::Serialize",
    deserialize = "CoreStorage<P, S>: for<'a> serde::Deserialize<'a>, Views<V>: for<'a> serde::Deserialize<'a> + Default, T: for<'a> serde::Deserialize<'a>"
))]
pub struct GenAppStorage<P, S, V, T>
where
    P: Controller<S>,
    S: Simulator,
    T: scouting::ScoutingTarget<P, S> + Default,
{
    pub(crate) core: CoreStorage<P, S>,
    #[serde(default)]
    pub(crate) scout: scouting::Scouter<P, S, T>,
    pub(crate) is_init: bool,
    pub(crate) slider_len: Option<f32>,
    #[serde(default)]
    pub(crate) views: Views<V>,
    #[serde(skip)]
    pub(crate) running: bool,
    #[serde(skip)]
    pub(crate) profiler: bool,
    pub(crate) add_rand: bool,
    pub(crate) show_disper: (bool, f64),
    pub(crate) check_points: checkpoint::CheckPoints<CoreStorage<P, S>>,
    pub(crate) file_state: file::FileManager,
    pub(crate) file_checkpoints: file::FileManager,
}

impl<P, S, V, T> Default for GenAppStorage<P, S, V, T>
where
    P: Default + Controller<S>,
    S: Simulator,
    Views<V>: Default,
    T: scouting::ScoutingTarget<P, S> + Default,
{
    fn default() -> Self {
        Self {
            core: Default::default(),
            scout: Default::default(),
            is_init: false,
            slider_len: None,
            views: Default::default(),
            running: false,
            profiler: false,
            add_rand: false,
            show_disper: (false, 1.),
            check_points: Default::default(),
            file_state: FileManager::default_state(),
            file_checkpoints: FileManager::default_check_points(),
        }
    }
}
