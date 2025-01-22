use lle::{
    num_complex::Complex64, DiffOrder, Evolver, Freq, LinearOp, LinearOpCached, NoneOp,
    StaticLinearOp, Step,
};
use num_traits::{zero, Zero};

use super::{Controller, Property};

#[allow(unused)]
pub type App = crate::app::GenApp<
    DisperLleController2,
    LleSolver<lle::SPhaMod, Complex64>,
    crate::drawer::ViewField,
>;

#[derive(
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
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
    pub fn generate_op(&self) -> CosDispersion2 {
        CosDispersion2 {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            strength: self.strength.get_value(),
        }
    }
}

impl StaticLinearOp<f64> for CosDispersion2 {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CosDispersion2 {
    center_pos: f64,
    period: f64,
    strength: f64,
}

impl LinearOp<f64> for CosDispersion2 {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let branch = freq.rem_euclid(2);
        debug_assert!(branch == 0 || branch == 1);
        let f = |f: Freq| {
            -(((f.div_euclid(2)) as f64 - self.center_pos) / self.period
                * std::f64::consts::PI
                * 2.)
                .cos()
        };
        if branch == 0 {
            -Complex64::i() * (f(freq) - f(0)) * self.strength
        } else {
            -Complex64::i() * ((-f(freq) - f(0)) * self.strength - self.strength * 2.)
        }
    }
    fn skip(&self) -> bool {
        self.strength.is_zero()
    }
}

pub type LleSolver<NL, C> = lle::LleSolver<f64, Vec<Complex64>, LinearOpCached<f64>, NL, C>;

#[derive(
    Debug,
    Clone,
    Default,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct DisperLleController2 {
    pub(crate) basic: super::LleController,
    pub(crate) disper: CosDispersionProperty2,
}

#[cfg(test)]
mod test {
    use lle::LinearOp;

    use super::*;
    #[test]
    fn test_disper2_dispersion_symmetric() {
        let c = CosDispersionProperty2::default().generate_op();
        for i in 0..100 {
            assert_eq!(c.get_value(0, i), c.get_value(0, -i));
        }
    }
}

impl DisperLleController2 {
    pub fn linear_op(&self) -> impl StaticLinearOp<f64> {
        let basic_linear = self.basic.linear.get_value();
        (0, -(Complex64::i() * self.basic.alpha.get_value() + 1.))
            .add_linear_op(move |_: Step, f: Freq| -> Complex64 {
                Complex64::i() * basic_linear / 2. * ((f / 2) as f64).powi(2)
            })
            .add_linear_op(self.disper.generate_op())
    }
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL, Complex64>>
    for DisperLleController2
{
    const EXTENSION: &'static str = "dis2";
    type Dispersion = lle::LinearOpAdd<f64, (DiffOrder, Complex64), CosDispersion2>;
    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.basic.linear.get_value() / 2.)
            .add_linear_op(self.disper.generate_op())
    }
    fn construct_engine(&self, dim: usize) -> LleSolver<NL, Complex64> {
        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(self.linear_op().cached_linear_op(dim))
            .nonlin(NL::default())
            .constant(Complex64::from(pump))
            .constant_freq(NoneOp::default())
            .build()
    }

    fn steps(&self) -> u32 {
        self.basic.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL, Complex64>) {
        engine.constant = Complex64::from(self.basic.pump.get_value());
        engine.step_dist = self.basic.step_dist.get_value();
        engine.linear = self.linear_op().cached_linear_op(engine.state().len());
    }
}
