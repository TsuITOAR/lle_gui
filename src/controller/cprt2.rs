use std::f64::consts::{FRAC_PI_2, PI};

use lle::{
    num_complex::Complex64, DiffOrder, Evolver, Freq, LinearOp, LinearOpCached, StaticLinearOp,
    Step,
};
use num_traits::{zero, Zero};

use super::{Controller, Property};

#[allow(unused)]
pub type App = crate::app::GenApp<
    CprtLleController2,
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
pub struct Cprt2 {
    center_pos: Property<f64>,
    period: Property<f64>,
    couple_strength: Property<f64>,
    frac_d1_2pi: Property<f64>,
}

impl Default for Cprt2 {
    fn default() -> Self {
        Self {
            center_pos: Property::new(0., "Center Position").range((-20., 20.)),
            period: Property::new(10., "Period").range((10., 100.)),
            couple_strength: Property::new(FRAC_PI_2, "Couple strength").range((0., PI)),
            frac_d1_2pi: Property::new(1E3, "d1/2pi").range((1., 1E9)),
        }
    }
}

impl Cprt2 {
    pub fn generate_op(&self) -> CprtDispersion2 {
        CprtDispersion2 {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            couple_strength: self.couple_strength.get_value(),
            frac_d1_2pi: self.frac_d1_2pi.get_value(),
        }
    }
}

impl StaticLinearOp<f64> for CprtDispersion2 {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CprtDispersion2 {
    center_pos: f64,
    period: f64,
    couple_strength: f64,
    frac_d1_2pi: f64,
}

impl LinearOp<f64> for CprtDispersion2 {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let branch = freq % 2;
        let f = |f: f64| {
            let cos1 =
                ((f.div_euclid(2.) - self.center_pos) / self.period * std::f64::consts::PI * 2.)
                    .cos();

            let cos2 = self.couple_strength.cos();

            self.frac_d1_2pi * (((cos1 * cos2).acos()).rem_euclid(PI))
        };
        let gap = f(self.center_pos) * 2. * self.frac_d1_2pi;

        if branch == 0 {
            Complex64::i() * (-f(freq as _) - (-f(0.))) * self.frac_d1_2pi
        } else {
            Complex64::i() * (f(freq as _) - (-f(0.))) * self.frac_d1_2pi + self.frac_d1_2pi
                - gap * 2.
        }
    }
    fn skip(&self) -> bool {
        self.couple_strength.is_zero()
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
pub struct CprtLleController2 {
    pub(crate) basic: super::LleController,
    pub(crate) disper: Cprt2,
}

impl CprtLleController2 {
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
    for CprtLleController2
{
    const EXTENSION: &'static str = "cprt2";
    type Dispersion = lle::LinearOpAdd<f64, (DiffOrder, Complex64), CprtDispersion2>;
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
