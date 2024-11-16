use std::fmt::Debug;

use crate::{
    controller::{Controller, Simulator, StoreState},
    random::RandomNoise,
};

#[derive(Debug)]
pub struct Core<C, S> {
    pub(crate) dim: usize,
    pub(crate) controller: C,
    pub(crate) simulator: S,
    pub(crate) random: RandomNoise,
}

impl<C, S> Default for Core<C, S>
where
    C: Default + Controller<S>,
{
    fn default() -> Self {
        let dim: usize = 128;
        let controller = C::default();
        let simulator = controller.construct_engine(dim);
        Self {
            dim,
            controller,
            simulator,
            random: RandomNoise::default(),
        }
    }
}

impl<C, S> Core<C, S>
where
    C: Controller<S>,
    S: Simulator,
{
    pub fn new(controller: C, dim: usize) -> Self {
        let simulator = controller.construct_engine(dim);
        Self {
            controller,
            dim,
            simulator,
            random: RandomNoise::default(),
        }
    }
}

impl<C, Q> serde::Serialize for Core<C, Q>
where
    C: serde::Serialize + Clone,
    Q: Simulator,
    <Q as StoreState>::OwnedState: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let storage: CoreStorage<_, _> = self.into();
        storage.serialize(serializer)
    }
}

impl<'a, C, S> serde::Deserialize<'a> for Core<C, S>
where
    for<'de> C: serde::Deserialize<'de> + Controller<S>,
    S: Simulator,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(CoreStorage::<C, S>::deserialize(deserializer)?.into())
    }
}

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

impl<P, S> Core<P, S>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    S::OwnedState: serde::Serialize + for<'a> serde::Deserialize<'a>,
{
    pub(crate) fn reset(&mut self) {
        *self = Self::new(P::default(), self.dim);
    }
}
