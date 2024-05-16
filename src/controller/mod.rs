use lle::num_complex::Complex64;
use num_traits::zero;
use std::collections::BTreeMap;

use crate::{default_add_random, property::Property};

pub trait Controller<E> {
    fn construct_engine(&self, dim: usize) -> E;
    fn construct_with_seed(&self, dim: usize, seed: u32) -> E;
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui);
    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui);
    fn sync_paras(&mut self, engine: &mut E);
    fn steps(&self) -> u32;
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LleController {
    properties: BTreeMap<String, Property>,
}

impl Default for LleController {
    fn default() -> Self {
        let properties = vec![
            Property::new_float(-5., "alpha").symbol('α'),
            Property::new_float(3.94, "pump").symbol('F'),
            Property::new_float(-0.0444, "linear").symbol('β'),
            Property::new_float_no_slider(8e-4, "step dist")
                .symbol("Δt")
                .unit(1E-4),
            Property::new_uint(100, "steps").symbol("steps"),
        ]
        .into_iter()
        .map(|x| (x.label.clone(), x))
        .collect();
        Self { properties }
    }
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<crate::LleSolver<NL>> for LleController {
    fn construct_engine(&self, dim: usize) -> crate::LleSolver<NL> {
        use lle::LinearOp;
        let properties = &self.properties;
        let step_dist = properties["step dist"].value.f64().unwrap();
        let pump = properties["pump"].value.f64().unwrap();
        let linear = properties["linear"].value.f64().unwrap();
        let alpha = properties["alpha"].value.f64().unwrap();
        let mut init = vec![zero(); dim];
        default_add_random(init.iter_mut());
        crate::LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear((0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    // todo: use seed
    fn construct_with_seed(&self, dim: usize, _seed: u32) -> crate::LleSolver<NL> {
        use lle::LinearOp;
        let properties = &self.properties;
        let step_dist = properties["step dist"].value.f64().unwrap();
        let pump = properties["pump"].value.f64().unwrap();
        let linear = properties["linear"].value.f64().unwrap();
        let alpha = properties["alpha"].value.f64().unwrap();
        let mut init = vec![zero(); dim];
        default_add_random(init.iter_mut());
        crate::LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear((0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        for p in self.properties.values_mut() {
            p.show_in_control_panel(ui)
        }
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(dim, self.properties.values_mut(), ui)
    }

    fn sync_paras(&mut self, engine: &mut crate::LleSolver<NL>) {
        crate::synchronize_properties(&self.properties, engine);
    }

    fn steps(&self) -> u32 {
        self.properties["steps"].value.u32().unwrap()
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

impl<NL: lle::NonLinearOp<f64>> Simulator for crate::LleSolver<NL> {
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
