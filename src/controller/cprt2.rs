use std::f64::consts::{FRAC_PI_2, PI};

use lle::{
    num_complex::Complex64, DiffOrder, Evolver, Freq, LinearOp, LinearOpCached, NoneOp,
    StaticLinearOp, Step,
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
    #[serde(default = "default_decay")]
    couple_decay: Property<f64>,
    frac_d1_2pi: Property<f64>,
}

pub(crate) fn default_decay() -> Property<f64> {
    Property::new(250., "Couple decay")
        .range((100., 1000.))
        .on_hover_text("Wavelength dependent coupling strength")
}

impl Default for Cprt2 {
    fn default() -> Self {
        Self {
            center_pos: Property::new(0., "Center Position").range((-20., 20.)),
            period: Property::new(100., "Period").range((50., 100.)),
            couple_decay: default_decay(),
            couple_strength: Property::new(1., "Couple strength").range((0., PI)),
            frac_d1_2pi: Property::new(100., "d1/2pi").range((50., 200.)),
        }
    }
}

impl Cprt2 {
    pub fn generate_op(&self) -> CprtDispersion2 {
        CprtDispersion2 {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            couple_strength: CoupleStrength {
                couple_strength: self.couple_strength.get_value(),
                decay: self.couple_decay.get_value(),
            },

            frac_d1_2pi: self.frac_d1_2pi.get_value(),
        }
    }
}

impl StaticLinearOp<f64> for CprtDispersion2 {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CprtDispersion2 {
    pub(crate) center_pos: f64,
    pub(crate) period: f64,
    #[serde(default)]
    pub(crate) couple_strength: CoupleStrength,
    pub(crate) frac_d1_2pi: f64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct CoupleStrength {
    pub(crate) couple_strength: f64,
    pub(crate) decay: f64,
}

impl CoupleStrength {
    pub(crate) fn get_coupling(&self, f: f64) -> f64 {
        self.couple_strength * (-f / self.decay).exp()
    }
}

impl Default for CoupleStrength {
    fn default() -> Self {
        Self {
            couple_strength: 1.,
            decay: default_decay().get_value(),
        }
    }
}

impl LinearOp<f64> for CprtDispersion2 {
    fn get_value(&self, _step: Step, freq: Freq) -> Complex64 {
        let branch = freq.rem_euclid(2);
        debug_assert!(branch == 0 || branch == 1);
        let f = |f: Freq| {
            let f = f.div_euclid(2) as f64;
            let cos1 = ((f - self.center_pos) / self.period * std::f64::consts::PI * 2.).cos();
            let couple_strength = self.couple_strength.get_coupling(f);
            //dbg!(f, couple_strength);
            let cos2 = couple_strength.cos();

            ((cos1 * cos2).acos()) * self.frac_d1_2pi - self.frac_d1_2pi * FRAC_PI_2
        };

        if branch == 0 {
            -Complex64::i() * (f(freq) - f(0))
        } else {
            -Complex64::i() * (-f(freq) - f(0))
        }
    }
    fn skip(&self) -> bool {
        self.couple_strength.couple_strength.is_zero()
    }
}

#[cfg(test)]
mod test {
    use lle::LinearOp;

    use super::*;
    #[test]
    fn test_cprt2_dispersion_symmetric() {
        let mut c = Cprt2::default().generate_op();
        c.couple_strength.decay = f64::INFINITY;
        for i in 0..100 {
            assert_eq!(c.get_value(0, i * 2), c.get_value(0, -i * 2), "{i}");
            assert_eq!(c.get_value(0, i * 2 + 1), c.get_value(0, -i * 2 + 1));
        }
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
