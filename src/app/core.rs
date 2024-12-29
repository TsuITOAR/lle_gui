use std::fmt::Debug;

use crate::{
    controller::{Controller, Simulator, StoreState},
    random::RandomNoise,
};

use super::storage::CoreStorage;

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
        let mut random = RandomNoise::default();
        let simulator = controller.construct_engine_random_init(dim, &mut random);
        Self {
            controller,
            dim,
            simulator,
            random,
        }
    }
    pub fn sync_paras(&mut self) {
        self.controller.sync_paras(&mut self.simulator);
    }
    pub fn add_random(&mut self) {
        self.simulator.add_rand(&mut self.random);
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
