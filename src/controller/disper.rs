use lle::{num_complex::Complex64, Freq, LinearOp, Step};
use num_traits::{zero, Zero};

use crate::{default_add_random, default_add_random_with_seed};

use super::{Controller, Property};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CosDispersionProperty {
    center_pos: Property<isize>,
    period: Property<usize>,
    strength: Property<f64>,
}

impl Default for CosDispersionProperty {
    fn default() -> Self {
        Self {
            center_pos: Property::new_no_slider(0, "Center Position").range((-20, 20)),
            period: Property::new_no_slider(1, "Period").range((10, 100)),
            strength: Property::new(0.0, "Strength").range((0.0, 10.)),
        }
    }
}

impl CosDispersionProperty {
    pub(crate) fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        self.center_pos.show_in_control_panel(ui);
        self.period.show_in_control_panel(ui);
        self.strength.show_in_control_panel(ui);
    }

    pub fn generate_op(&self) -> CosDispersion {
        CosDispersion {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            strength: self.strength.get_value(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CosDispersion {
    center_pos: isize,
    period: usize,
    strength: f64,
}

impl LinearOp<f64> for CosDispersion {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        Complex64::i()
            * (1.
                - ((freq as f64 - self.center_pos as f64) / self.period as f64
                    * std::f64::consts::PI
                    * 2.)
                    .cos())
            * -self.strength
    }
    fn skip(&self) -> bool {
        self.strength.is_zero()
    }
}

pub type LleSolver<NL> = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<
        f64,
        lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
        CosDispersion,
    >,
    NL,
>;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct DisperLleController {
    basic: super::LleController,
    disper: CosDispersionProperty,
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL>> for DisperLleController {
    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let linear = self.basic.linear.get_value();
        let alpha = self.basic.alpha.get_value();
        let mut init = vec![zero(); dim];
        default_add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add((2, Complex64::i() * linear / 2.))
                    .add(self.disper.generate_op()),
            )
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }
    fn construct_with_seed(&self, dim: usize, seed: u64) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let linear = self.basic.linear.get_value();
        let alpha = self.basic.alpha.get_value();

        let mut init = vec![zero(); dim];
        default_add_random_with_seed(init.as_mut_slice(), seed);
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add((2, Complex64::i() * linear / 2.))
                    .add(self.disper.generate_op()),
            )
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
        engine.linear = (0, -(Complex64::i() * self.basic.alpha.get_value() + 1.))
            .add((2, Complex64::i() * self.basic.linear.get_value() / 2.))
            .add(self.disper.generate_op())
            .into();
    }
}
