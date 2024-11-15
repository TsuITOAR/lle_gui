use lle::{num_complex::Complex64, Evolver, Freq, LinearOp, LinearOpCached, Step};
use num_traits::{zero, Zero};

use super::{Controller, Property};

#[allow(unused)]
pub type App =
    crate::GenApp<DisperLleController2, LleSolver<lle::SPhaMod>, crate::drawer::ViewField>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CosDispersionProperty2 {
    center_pos: Property<f64>,
    period: Property<f64>,
    strength: Property<f64>,
}

impl Default for CosDispersionProperty2 {
    fn default() -> Self {
        Self {
            center_pos: Property::new(0., "Center Position").range((-20., 20.)),
            period: Property::new(10., "Period").range((10., 100.)),
            strength: Property::new(0.0, "Strength").range((-50., 50.)),
        }
    }
}

impl CosDispersionProperty2 {
    pub(crate) fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        self.center_pos.show_in_control_panel(ui);
        self.period.show_in_control_panel(ui);
        self.strength.show_in_control_panel(ui);
    }

    pub fn generate_op(&self) -> CosDispersion2 {
        CosDispersion2 {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            strength: self.strength.get_value(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CosDispersion2 {
    center_pos: f64,
    period: f64,
    strength: f64,
}

impl LinearOp<f64> for CosDispersion2 {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let branch = freq % 2;
        let f = |f: Freq| {
            -(((f / 2) as f64 - self.center_pos) / self.period * std::f64::consts::PI * 2.).cos()
        };
        if branch == 0 {
            -Complex64::i() * (f(freq) - f(0)) * self.strength
        } else {
            Complex64::i() * (f(freq) - f(0)) * self.strength - self.strength * 2.
        }
    }
    fn skip(&self) -> bool {
        self.strength.is_zero()
    }
}

pub type LleSolver<NL> = lle::LleSolver<f64, Vec<Complex64>, LinearOpCached<f64>, NL>;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct DisperLleController2 {
    basic: super::LleController,
    disper: CosDispersionProperty2,
}

impl DisperLleController2 {
    pub fn linear_op(&self) -> impl LinearOp<f64> {
        let basic_linear = self.basic.linear.get_value();
        (0, -(Complex64::i() * self.basic.alpha.get_value() + 1.))
            .add(move |_: Step, f: Freq| -> Complex64 {
                Complex64::i() * basic_linear / 2. * ((f / 2) as f64).powi(2)
            })
            .add(self.disper.generate_op())
    }
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL>> for DisperLleController2 {
    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(self.linear_op().cached(dim))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        Controller::<super::LleSolver<NL>>::show_in_control_panel(&mut self.basic, ui);
        self.disper.show_in_control_panel(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(dim, &mut self.basic, ui)
    }

    fn steps(&self) -> u32 {
        self.basic.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL>) {
        engine.constant = Complex64::from(self.basic.pump.get_value()).into();
        engine.step_dist = self.basic.step_dist.get_value();
        engine.linear = Some(self.linear_op().cached(engine.state().len()));
    }
}
