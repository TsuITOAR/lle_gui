mod traits;
pub use traits::*;

pub mod clle;
pub mod disper;
pub mod disper_2modes;
pub mod cprt;

use lle::{num_complex::Complex64, Evolver, SPhaMod};
use num_traits::{zero, Zero};

use crate::{property::Property, random::RandomNoise};

#[allow(unused)]
pub type App = crate::GenApp<LleController, LleSolver<lle::SPhaMod>, crate::drawer::ViewField>;

pub type LleSolver<NL> = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    NL,
>;
impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL>> for LleController {
    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.step_dist.get_value();
        let pump = self.pump.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear((0, -(Complex64::i() * alpha + 1.)).add((2, Complex64::i() * linear / 2.)))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        self.alpha.show_in_control_panel(ui);
        self.linear.show_in_control_panel(ui);
        self.pump.show_in_control_panel(ui);
        self.step_dist.show_in_control_panel(ui);
        self.steps.show_in_control_panel(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(dim, self, ui)
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL>) {
        crate::synchronize_properties(self, engine);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LleController {
    pub(crate) alpha: Property<f64>,
    pub(crate) pump: Property<f64>,
    pub(crate) linear: Property<f64>,
    pub(crate) step_dist: Property<f64>,
    pub(crate) steps: Property<u32>,
}

impl Default for LleController {
    fn default() -> Self {
        Self {
            alpha: Property::new(-5., "alpha").symbol('α'),
            pump: Property::new(3.94, "pump").symbol('F'),
            linear: Property::new(-0.0444, "linear")
                .symbol('β')
                .range((-0.1, 0.1)),
            step_dist: Property::new_no_slider(8e-4, "step dist")
                .range((1E-10, 1E-3))
                .symbol("Δt")
                .unit(1E-4),
            steps: Property::new_no_slider(100, "steps")
                .symbol("steps")
                .range((1, u32::MAX)),
        }
    }
}

impl<
        'a,
        S: AsMut<[Complex64]> + AsRef<[Complex64]>,
        L: lle::LinearOp<f64>,
        NL: lle::NonLinearOp<f64>,
    > SharedState<'a> for lle::LleSolver<f64, S, L, NL>
{
    type SharedState = &'a [Complex64];
    fn states(&'a self) -> Self::SharedState {
        use lle::Evolver;
        self.state()
    }
    fn set_state(&mut self, state: &[Complex64]) {
        self.state_mut().copy_from_slice(state);
    }
}

impl<
        S: AsMut<[Complex64]> + AsRef<[Complex64]>,
        L: lle::LinearOp<f64>,
        NL: lle::NonLinearOp<f64>,
    > StoreState for lle::LleSolver<f64, S, L, NL>
{
    type OwnedState = Vec<Complex64>;
    fn get_owned_state(&self) -> Self::OwnedState {
        self.state().to_vec()
    }
    fn set_owned_state(&mut self, state: Self::OwnedState) {
        if self.state().len() != state.len() {
            crate::TOASTS.lock().warning(format!(
                "Skipping restore state for mismatched length between simulator({}) and storage({})",
                self.state().len(),
                state.len()
            ));
            return;
        }
        self.state_mut().copy_from_slice(&state);
    }
    fn default_state(dim: usize) -> Self::OwnedState {
        vec![Complex64::zero(); dim]
    }
}

impl<
        S: AsMut<[Complex64]> + AsRef<[Complex64]>,
        L: lle::LinearOp<f64>,
        NL: lle::NonLinearOp<f64>,
    > Simulator for lle::LleSolver<f64, S, L, NL>
{
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps as _);
    }
    fn add_rand(&mut self, r: &mut RandomNoise) {
        r.add_random(self.state_mut());
    }
}
