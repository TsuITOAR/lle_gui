pub mod clle;

use lle::{num_complex::Complex64, SPhaMod};
use num_traits::zero;

use crate::{default_add_random, default_add_random_with_seed, property::Property};

pub trait Controller<E> {
    fn construct_engine(&self, dim: usize) -> E;
    fn construct_with_seed(&self, dim: usize, seed: u64) -> E;
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui);
    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui);
    fn sync_paras(&mut self, engine: &mut E);
    fn steps(&self) -> u32;
}

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
        let mut init = vec![zero(); dim];
        default_add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear((0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    fn construct_with_seed(&self, dim: usize, seed: u64) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.step_dist.get_value();
        let pump = self.pump.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();

        let mut init = vec![zero(); dim];
        default_add_random_with_seed(init.as_mut_slice(), seed);
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear((0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)))
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

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
            linear: Property::new(-0.0444, "linear").symbol('β'),
            step_dist: Property::new_no_slider(8e-4, "step dist")
                .symbol("Δt")
                .unit(1E-4),
            steps: Property::new(100, "steps").symbol("steps"),
        }
    }
}

pub trait Simulator {
    type State: ?Sized + Record;
    fn states(&self) -> &Self::State;
    fn run(&mut self, steps: u32);
}

pub trait Record {
    fn record_first(&self) -> &[Complex64];
}

impl Record for [Complex64] {
    fn record_first(&self) -> &[Complex64] {
        self
    }
}

impl Record for (&[Complex64], &[Complex64]) {
    fn record_first(&self) -> &[Complex64] {
        self.0
    }
}

impl<NL: lle::NonLinearOp<f64>> Simulator for LleSolver<NL> {
    type State = [Complex64];
    fn states(&self) -> &Self::State {
        use lle::Evolver;
        self.state()
    }
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps as _);
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Core<P, S> {
    pub(crate) dim: usize,
    pub(crate) controller: P,
    #[serde(skip, default = "Default::default")]
    pub(crate) simulator: Option<S>,
}

impl<P: Default, S> Default for Core<P, S> {
    fn default() -> Self {
        Self {
            dim: 128,
            controller: P::default(),
            simulator: None,
        }
    }
}

impl<P, S> Core<P, S>
where
    P: Controller<S>,
    S: Simulator,
{
    pub fn new(controller: P, dim: usize) -> Self {
        Self {
            controller,
            dim,
            simulator: None,
        }
    }
}
