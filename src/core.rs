use std::fmt::Debug;

use crate::{
    controller::{Controller, Simulator},
    random::RandomNoise,
};

#[derive(Debug)]
pub struct Core<P, S> {
    pub(crate) dim: usize,
    pub(crate) controller: P,
    pub(crate) simulator: S,
    pub(crate) random: RandomNoise,
}

impl<P, S> Default for Core<P, S>
where
    P: Default + Controller<S>,
{
    fn default() -> Self {
        let dim: usize = 128;
        let controller = P::default();
        let simulator = controller.construct_engine(dim);
        Self {
            dim,
            controller,
            simulator,
            random: RandomNoise::default(),
        }
    }
}

impl<'a, P, S> Core<P, S>
where
    P: Controller<S>,
    S: Simulator<'a>,
{
    pub fn new(controller: P, dim: usize) -> Self {
        let simulator = controller.construct_engine(dim);
        Self {
            controller,
            dim,
            simulator,
            random: RandomNoise::default(),
        }
    }
}

impl<P, Q> serde::Serialize for Core<P, Q>
where
    P: serde::Serialize + Clone,
    Q: StoreState,
    <Q as StoreState>::State: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let storage: CoreStorage<_, _> = self.into();
        storage.serialize(serializer)
    }
}

impl<'a, P, S> serde::Deserialize<'a> for Core<P, S>
where
    for<'de> P: serde::Deserialize<'de> + Controller<S>,
    S: StoreState,
    for<'de> <S as StoreState>::State: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(CoreStorage::<P, S>::deserialize(deserializer)?.into())
    }
}

pub trait StoreState
where
    for<'a> Self: Simulator<'a>,
{
    type State: 'static + Debug;
    fn get_owned_state(&self) -> <Self as StoreState>::State;
    fn set_owned_state(&mut self, state: <Self as StoreState>::State);
    fn default_state(dim: usize) -> <Self as StoreState>::State;
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(bound(
    serialize = "S: StoreState, <S as StoreState>::State: serde::Serialize, P: serde::Serialize",
    deserialize = "S: StoreState, <S as StoreState>::State: for<'a> serde::Deserialize<'a>, P: for<'a> serde::Deserialize<'a>"
))]
pub struct CoreStorage<P, S>
where
    S: StoreState,
{
    pub(crate) dim: usize,
    pub(crate) controller: P,
    pub(crate) simulator_state: <S as StoreState>::State,
    pub(crate) random: RandomNoise,
}

impl<'a, P, S> From<&'a Core<P, S>> for CoreStorage<P, S>
where
    P: Clone,
    S: StoreState,
{
    fn from(core: &'a Core<P, S>) -> Self {
        Self {
            dim: core.dim,
            controller: core.controller.clone(),
            simulator_state: core.simulator.get_owned_state(),
            random: core.random.clone(),
        }
    }
}

impl<P, S> From<CoreStorage<P, S>> for Core<P, S>
where
    P: Controller<S>,
    S: StoreState,
{
    fn from(storage: CoreStorage<P, S>) -> Self {
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

impl<P: Default + Controller<S>, S: StoreState> Default for CoreStorage<P, S> {
    fn default() -> Self {
        let dim: usize = 128;
        Self {
            dim,
            controller: P::default(),
            simulator_state: S::default_state(dim),
            random: RandomNoise::default(),
        }
    }
}
