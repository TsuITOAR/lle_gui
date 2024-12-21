use lle::{num_complex::Complex64, DiffOrder, Freq, LinearOp, Step};
use num_traits::{zero, Zero};

use super::{Controller, Property};

#[allow(unused)]
pub type App =
    crate::app::GenApp<DisperLleController, LleSolver<lle::SPhaMod>, crate::drawer::ViewField>;

#[derive(
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct CosDispersionProperty {
    center_pos: Property<f64>,
    period: Property<f64>,
    strength: Property<f64>,
}

impl Default for CosDispersionProperty {
    fn default() -> Self {
        Self {
            center_pos: Property::new(0., "Center Position").range((-20., 20.)),
            period: Property::new(10., "Period").range((10., 100.)),
            strength: Property::new(0.0, "Strength").range((-50., 50.)),
        }
    }
}

impl CosDispersionProperty {
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
    center_pos: f64,
    period: f64,
    strength: f64,
}

impl LinearOp<f64> for CosDispersion {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let f = |f: Freq| {
            -((f as f64 - self.center_pos) / self.period * std::f64::consts::PI * 2.).cos()
        };
        let _ff = |f: Freq| {
            ((f as f64 - self.center_pos) / self.period * std::f64::consts::PI * 2.).sin()
        };
        -Complex64::i() * (f(freq) - f(0)) * self.strength
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
    Complex64,
>;

#[derive(
    Debug,
    Clone,
    Default,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct DisperLleController {
    pub(crate) basic: super::LleController,
    pub(crate) disper: CosDispersionProperty,
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL>> for DisperLleController {
    const EXTENSION: &'static str = "dis";
    type Dispersion = lle::LinearOpAdd<f64, (DiffOrder, Complex64), CosDispersion>;
    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.basic.linear.get_value() / 2.)
            .add_linear_op(self.disper.generate_op())
    }
    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let linear = self.basic.linear.get_value();
        let alpha = self.basic.alpha.get_value();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add_linear_op((2, Complex64::i() * linear / 2.))
                    .add_linear_op(self.disper.generate_op()),
            )
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .build()
    }

    fn steps(&self) -> u32 {
        self.basic.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL>) {
        engine.constant = Complex64::from(self.basic.pump.get_value());
        engine.step_dist = self.basic.step_dist.get_value();
        engine.linear = (0, -(Complex64::i() * self.basic.alpha.get_value() + 1.))
            .add_linear_op((2, Complex64::i() * self.basic.linear.get_value() / 2.))
            .add_linear_op(self.disper.generate_op());
    }
}
