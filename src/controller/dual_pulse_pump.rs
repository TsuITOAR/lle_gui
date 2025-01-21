use lle::StaticConstOp;

use super::*;

#[allow(unused)]
pub type App = crate::app::GenApp<
    DualPulsePumpLleController,
    LleSolver<lle::SPhaMod>,
    crate::drawer::ViewField,
>;

pub type LleSolver<NL> = lle::LleSolver<f64, Vec<Complex64>, LinearOp, NL, lle::ConstOpCached<f64>>;

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct DualPulsePumpLleController {
    pub(crate) alpha: Property<f64>,
    pub(crate) linear: Property<f64>,
    pub(crate) pump: Pump,
    pub(crate) step_dist: Property<f64>,
    pub(crate) steps: Property<u32>,
}

impl std::default::Default for DualPulsePumpLleController {
    fn default() -> Self {
        Self {
            alpha: Property::new(-5., "alpha").symbol('α'),
            linear: Property::new(-0.0444, "linear")
                .symbol('β')
                .range((-0.1, 0.1)),
            pump: Pump::default(),
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
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct SinglePump {
    pub(crate) peak: Property<f64>,
    pub(crate) width: Property<f64>,
}

impl std::default::Default for SinglePump {
    fn default() -> Self {
        Self {
            peak: Property::new(10., "peak").range((0.01, 1.)),
            width: Property::new(10., "width").range((1., 100.)),
        }
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
pub(crate) struct Pump {
    pub(crate) pulse1: SinglePump,
    pub(crate) pulse2: SinglePump,
    pub(crate) distance: Property<f64>,
    pub(crate) d1_mismatch: Property<f64>,
}

impl std::default::Default for Pump {
    fn default() -> Self {
        Self {
            pulse1: SinglePump::default(),
            pulse2: SinglePump::default(),
            distance: Property::new(50., "distance").range((10., 200.)),
            d1_mismatch: Property::new(0., "D1 mismatch").range((-0.1, 0.1)),
        }
    }
}

pub type PumpOp = lle::ConstOpAdd<f64, crate::lle_util::PulsePumpOp, crate::lle_util::PulsePumpOp>;

impl Pump {
    pub fn get_pump_op(&self) -> PumpOp {
        use crate::lle_util::PulsePumpOp;
        PulsePumpOp {
            center: self.distance.get_value() / -2.,
            peak: self.pulse1.peak.get_value(),
            width: self.pulse1.width.get_value(),
        }
        .add_const_op(PulsePumpOp {
            center: self.distance.get_value() / 2.,
            peak: self.pulse2.peak.get_value(),
            width: self.pulse2.width.get_value(),
        })
    }
}

pub type LinearOpAdd<A, B> = lle::LinearOpAdd<f64, A, B>;

pub type LinearOp = LinearOpAdd<
    LinearOpAdd<(lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    (lle::DiffOrder, Complex64),
>;

impl<NL: lle::NonLinearOp<f64> + Default> Controller<LleSolver<NL>> for DualPulsePumpLleController {
    const EXTENSION: &'static str = "dplle";

    type Dispersion = (lle::DiffOrder, Complex64);

    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.linear.get_value() / 2.)
    }

    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.step_dist.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();
        let pump = self.pump.get_pump_op();
        let d1_mismatch = self.pump.d1_mismatch.get_value();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add_linear_op((1, -Complex64::i() * d1_mismatch))
                    .add_linear_op((2, Complex64::i() * linear / 2.)),
            )
            .nonlin(NL::default())
            .constant(pump.cached_const_op(dim))
            .constant_freq(NoneOp::default())
            .build()
    }

    fn sync_paras(&mut self, engine: &mut LleSolver<NL>) {
        puffin_egui::puffin::profile_function!();
        let step_dist = self.step_dist.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();
        let pump = self.pump.get_pump_op();
        let d1_mismatch = self.pump.d1_mismatch.get_value();
        use lle::LinearOp;
        engine.linear = (0, -(Complex64::i() * alpha + 1.))
            .add_linear_op((1, -Complex64::i() * d1_mismatch))
            .add_linear_op((2, Complex64::i() * linear / 2.));
        engine.constant = pump.cached_const_op(engine.states().len());
        engine.step_dist = step_dist;
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
}
