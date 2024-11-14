use std::fmt::Debug;

use crate::{
    controller::{Controller, Simulator, StoreState},
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

impl<P, S> Core<P, S>
where
    P: Controller<S>,
    S: Simulator,
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

impl<'a, P, S> serde::Deserialize<'a> for Core<P, S>
where
    for<'de> P: serde::Deserialize<'de> + Controller<S>,
    S: Simulator,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(CoreStorage::<P, S>::deserialize(deserializer)?.into())
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

impl<'a, P, S> From<&'a Core<P, S>> for CoreStorage<P, S>
where
    P: Clone,
    S: Simulator,
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
    S: Simulator,
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

impl<P, S> Default for CoreStorage<P, S>
where
    P: Default + Controller<S>,
    S: Simulator,
{
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
