use crate::random::RandomNoise;

pub trait Controller<E> {
    const EXTENSION: &'static str;
    type Dispersion: lle::LinearOp<f64>;
    fn dispersion(&self) -> Self::Dispersion;
    fn construct_engine(&self, dim: usize) -> E;
    fn construct_engine_random_init(&self, dim: usize, rand: &mut RandomNoise) -> E
    where
        E: Simulator,
    {
        let mut e = self.construct_engine(dim);
        e.add_rand(rand);
        e
    }
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui);
    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui);
    fn sync_paras(&mut self, engine: &mut E);
    fn steps(&self) -> u32;
}

/// For monitor and visualize state
pub trait SharedState<'a> {
    /// this should be a reference to the state of the simulator
    type SharedState: 'a;
    fn states(&'a self) -> Self::SharedState;
    fn set_state(&mut self, state: Self::SharedState);
}

/// For save and recover state
pub trait StoreState {
    type OwnedState: 'static + std::fmt::Debug + serde::Serialize + for<'a> serde::Deserialize<'a>;
    fn get_owned_state(&self) -> <Self as StoreState>::OwnedState;
    fn set_owned_state(&mut self, state: <Self as StoreState>::OwnedState);
    fn default_state(dim: usize) -> <Self as StoreState>::OwnedState;
}

pub trait Simulator: for<'a> SharedState<'a> + StoreState {
    fn add_rand(&mut self, random: &mut RandomNoise);
    fn run(&mut self, steps: u32);
    fn cur_step(&self) -> u32;
}
