use lle::{num_complex::Complex64, DiffOrder, LinearOpCached, NoneOp, StaticLinearOp};
use ops::PumpFreq;
use state::CoupleInfo;

use super::{cprt2::CoupleStrength, Controller, Property};

pub use walkoff::WalkOff;

pub mod ops;
pub mod state;
pub mod visualizer;
mod walkoff;

#[allow(unused)]
pub type App = crate::app::GenApp<
    GenCprtController,
    WalkOff<LleSolver<lle::SPhaMod, NoneOp<f64>, PumpFreq>>,
    crate::drawer::ViewField<state::State>,
>;

pub type LleSolver<NL, C, CF> = lle::LleSolver<f64, state::State, LinearOpCached<f64>, NL, C, CF>;

#[derive(
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct GenCprtController {
    pub(crate) alpha: Property<f64>,
    pub(crate) disper: GenCprtDisperSubController,
    pub(crate) pump: GenCprtPumpSubController,
    pub(crate) step_dist: Property<f64>,
    pub(crate) steps: Property<u32>,
}

impl GenCprtController {
    pub fn get_dispersion(&self) -> impl StaticLinearOp<f64> {
        use lle::LinearOp;
        let beta = self.disper.linear.get_value();
        let alpha = self.alpha.get_value();
        (0, -(Complex64::i() * alpha + 1.))
            .add_linear_op(move |_: lle::Step, f: lle::Freq| -> Complex64 {
                Complex64::i() * beta / 2. * ((f as f64).div_euclid(2.)).powi(2)
            })
            .add_linear_op(self.disper.get_cprt_dispersion())
    }
}

#[cfg(test)]
mod test {
    use lle::LinearOp;

    use super::*;
    #[test]
    fn test_gencprt_dispersion_symmetric() {
        let mut c = GenCprtController::default();
        *c.disper.couple_decay.value_mut() = f64::INFINITY;
        let c = c.get_dispersion();
        for i in 0..100 {
            assert_eq!(c.get_value(0, i * 2), c.get_value(0, -i * 2));
            assert_eq!(c.get_value(0, i * 2 + 1), c.get_value(0, -i * 2 + 1));
        }
    }
}

impl Default for GenCprtController {
    fn default() -> Self {
        Self {
            alpha: Property::new(-5., "alpha").range((-5., 10.)).symbol('α'),
            disper: GenCprtDisperSubController::default(),
            pump: GenCprtPumpSubController::default(),
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

#[derive(
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct GenCprtDisperSubController {
    pub(crate) linear: Property<f64>,
    pub(crate) center_pos: Property<f64>,
    pub(crate) period: Property<f64>,
    pub(crate) couple_strength: Property<f64>,
    #[serde(default = "super::cprt2::default_decay")]
    pub(crate) couple_decay: Property<f64>,
    pub(crate) frac_d1_2pi: Property<f64>,
}

impl GenCprtDisperSubController {
    fn get_cprt_dispersion(&self) -> super::cprt2::CprtDispersion2 {
        super::cprt2::CprtDispersion2 {
            center_pos: self.center_pos.get_value(),
            period: self.period.get_value(),
            couple_strength: CoupleStrength {
                couple_strength: self.couple_strength.get_value(),
                decay: self.couple_decay.get_value(),
            },

            frac_d1_2pi: self.frac_d1_2pi.get_value(),
        }
    }
    fn get_coup_info(&self) -> CoupleInfo {
        CoupleInfo {
            g: self.couple_strength.get_value(),
            mu: self.center_pos.get_value(),
            center: self.center_pos.get_value(),
            period: self.period.get_value(),
            frac_d1_2pi: self.frac_d1_2pi.get_value(),
        }
    }
}

impl Default for GenCprtDisperSubController {
    fn default() -> Self {
        Self {
            linear: Property::new(0.05, "linear").symbol('β').range((-0.1, 0.1)),
            center_pos: Property::new(0., "Center Position").range((-20., 20.)),
            period: Property::new(200., "Period").range((50., 400.)),
            couple_strength: Property::new(std::f64::consts::FRAC_PI_2 * 0.8, "Couple strength")
                .range((0., std::f64::consts::PI)),
            couple_decay: super::cprt2::default_decay(),
            frac_d1_2pi: Property::new(100., "d1/2pi").range((50., 200.)),
        }
    }
}

#[derive(
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct GenCprtPumpSubController {
    pub(crate) mode_number: Property<i32>,
    pub(crate) amplitude: Property<f64>,
}

impl GenCprtPumpSubController {
    pub fn get_pump(&self) -> PumpFreq {
        PumpFreq {
            mode: self.mode_number.get_value(),
            amp: self.amplitude.get_value(),
        }
    }
}

impl Default for GenCprtPumpSubController {
    fn default() -> Self {
        Self {
            mode_number: Property::new_no_slider(0, "pump mode"),
            amplitude: Property::new(5., "pump amplitude").range((1., 8.)),
        }
    }
}

impl<NL: Default + lle::NonLinearOp<f64>> Controller<LleSolver<NL, NoneOp<f64>, PumpFreq>>
    for GenCprtController
{
    const EXTENSION: &'static str = "gencprt";
    type Dispersion = lle::LinearOpAdd<f64, (DiffOrder, Complex64), super::cprt2::CprtDispersion2>;
    fn dispersion(&self) -> Self::Dispersion {
        use lle::LinearOp;
        (2, Complex64::i() * self.disper.linear.get_value() / 2. / 4.)
            .add_linear_op(self.disper.get_cprt_dispersion())
    }
    fn construct_engine(&self, dim: usize) -> LleSolver<NL, NoneOp<f64>, PumpFreq> {
        let step_dist = self.step_dist.get_value();
        let pump = self.pump.get_pump();
        let state = state::State::new(dim, self.disper.get_coup_info());
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(state)
            .step_dist(step_dist)
            .linear(self.get_dispersion().cached_linear_op(dim))
            .nonlin(NL::default())
            .constant(NoneOp::default())
            .constant_freq(pump)
            .build()
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
    fn sync_paras(&mut self, engine: &mut LleSolver<NL, NoneOp<f64>, PumpFreq>) {
        use lle::Evolver;
        engine.get_raw_state_mut().cp = self.disper.get_coup_info();
        engine.constant_freq = self.pump.get_pump();
        engine.step_dist = self.step_dist.get_value();
        engine.linear = self
            .get_dispersion()
            .cached_linear_op(engine.state().as_ref().len());
    }
}

fn singularity_point(freq0: i32, center: f64, period: f64) -> bool {
    let freq = freq0 as f64 - center;
    let diff = (freq + period / 4.).rem_euclid(period / 2.);
    let ret = (0. ..1.).contains(&diff);
    ret
}
