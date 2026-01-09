mod traits;
use std::fmt::Debug;

pub use traits::*;

pub mod clle;
pub mod cprt;
pub mod cprt2;
pub mod disper;
pub mod disper2;
pub mod dual_pulse_pump;
pub mod fp;
pub mod gencprt;
pub mod interleave_self_pump;
pub mod pulse_pump;
pub mod self_pump;

use lle::{ConstOp, Evolver, NoneOp, SPhaMod, num_complex::Complex64};
use num_traits::{Zero, zero};

use crate::{FftSource, property::Property, random::RandomNoise, views::PlotElement};

#[allow(unused)]
pub type App = crate::app::GenApp<
    LleController,
    LleSolver<lle::SPhaMod, Complex64, NoneOp<f64>>,
    crate::drawer::ViewField,
>;

pub type LleSolver<NL, C, CF> = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    NL,
    C,
    CF,
>;

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL, Complex64, NoneOp<f64>>>
    for LleController
{
    const EXTENSION: &'static str = "lle";
    type Dispersion = (lle::DiffOrder, Complex64);
    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.linear.get_value() / 2.)
    }
    fn construct_engine(&self, dim: usize) -> LleSolver<NL, Complex64, NoneOp<f64>> {
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
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add_linear_op((2, Complex64::i() * linear / 2.)),
            )
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .constant_freq(NoneOp::default())
            .build()
    }

    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        use crate::{config::LLE_EQUATION, easy_mark::easy_mark};
        use ui_traits::ControllerUI;
        easy_mark(ui, LLE_EQUATION);
        self.show_controller(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        use crate::{config::LLE_EQUATION, easy_mark::easy_mark};
        ui.label("The model is described by the following equation:");
        easy_mark(ui, LLE_EQUATION);
        crate::config::config(dim, self, ui)
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL, Complex64, NoneOp<f64>>) {
        crate::util::synchronize_properties(self, engine);
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
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
            alpha: Property::new(-5., "alpha")
                .symbol('α')
                .on_hover_text("Detunning of the pump"),
            pump: Property::new(3.94, "pump")
                .symbol('F')
                .on_hover_text("Amplitude of external pump"),
            linear: Property::new(-0.0444, "linear")
                .symbol('β')
                .range((-0.1, 0.1))
                .on_hover_text("Dispersion of the cavity\nPositive for normal dispersion"),
            step_dist: Property::new_no_slider(8e-4, "step dist")
                .range((1E-10, 1E-3))
                .symbol("Δt")
                .unit(1E-4)
                .on_hover_text("Step size for each simulation iteration"),
            steps: Property::new_no_slider(100, "steps")
                .symbol("steps")
                .range((1, u32::MAX))
                .on_hover_text(
                    "Number of steps to between each visualization and parameters update",
                ),
        }
    }
}

impl<
    'a,
    S: FftSource,
    L: lle::LinearOp<f64>,
    NL: lle::NonLinearOp<f64>,
    C: ConstOp<f64>,
    CF: ConstOp<f64>,
> SharedState<'a> for lle::LleSolver<f64, S, L, NL, C, CF>
{
    type SharedState = &'a S;
    fn states(&'a self) -> Self::SharedState {
        self.get_raw_state()
    }
    fn set_state(&mut self, state: &S) {
        *self.get_raw_state_mut() = state.clone();
    }
}

impl<
    S: FftSource + for<'a> serde::Deserialize<'a> + serde::Serialize,
    L: lle::LinearOp<f64>,
    NL: lle::NonLinearOp<f64>,
    C: ConstOp<f64>,
    CF: ConstOp<f64>,
> StoreState for lle::LleSolver<f64, S, L, NL, C, CF>
{
    type OwnedState = S;
    fn get_owned_state(&self) -> Self::OwnedState {
        self.get_raw_state().clone()
    }
    fn set_owned_state(&mut self, state: Self::OwnedState) {
        if self.state().len() != state.as_ref().len() {
            crate::notify::TOASTS.lock().warning(format!(
                "Skipping restore state for mismatched length between simulator({}) and storage({})",
                self.state().len(),
                state.as_ref().len()
            ));
            return;
        }
        *self.get_raw_state_mut() = state;
    }
    fn default_state(dim: usize) -> Self::OwnedState {
        S::default_with_len(dim)
    }
}

impl<
    S: FftSource + for<'a> serde::Deserialize<'a> + serde::Serialize,
    L: lle::LinearOp<f64> + Send + Sync + 'static,
    NL: lle::NonLinearOp<f64> + Send + Sync + 'static,
    C: ConstOp<f64> + Send + Sync + 'static,
    CF: ConstOp<f64> + Send + Sync + 'static,
> Simulator for lle::LleSolver<f64, S, L, NL, C, CF>
where
    S::FftProcessor: Send + Sync,
{
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps as _);
    }
    fn add_rand(&mut self, r: &mut RandomNoise) {
        r.add_random(self.state_mut());
    }
    fn cur_step(&self) -> u32 {
        <Self as lle::Evolver<f64>>::cur_step(self)
    }
}

pub fn dispersion_line<L: lle::LinearOp<f64>>(l: L, dim: usize, scale: f64) -> PlotElement {
    let dim = dim as i32;
    let split_pos = (dim + 1) / 2;
    let (x, y) = (0..dim)
        .map(|i| {
            let d = l.get_value(0, i - (dim - split_pos));
            (i as f64, -d.im / scale)
        })
        .unzip();
    PlotElement {
        x: Some(x),
        y,
        legend: Some("Dispersion".to_string()),
        style: Some(crate::drawer::plot_item::Style::default().interleave()),
    }
}
