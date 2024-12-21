use super::*;

#[allow(unused)]
pub type App =
    crate::app::GenApp<PulsePumpLleController, LleSolver<lle::SPhaMod>, crate::drawer::ViewField>;

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct PulsePumpLleController {
    pub(crate) alpha: Property<f64>,
    pub(crate) linear: Property<f64>,
    pub(crate) pump: Pump,
    pub(crate) step_dist: Property<f64>,
    pub(crate) steps: Property<u32>,
}

impl std::default::Default for PulsePumpLleController {
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
pub struct Pump {
    pub(crate) peak: Property<f64>,
    pub(crate) width: Property<f64>,
    pub(crate) d1_mismatch: Property<f64>,
}

impl std::default::Default for Pump {
    fn default() -> Self {
        Self {
            peak: Property::new(10., "Peak").range((0.01, 100.)),
            width: Property::new(1., "Width").range((0.01, 100.)),
            d1_mismatch: Property::new(0., "D1 Mismatch"),
        }
    }
}

impl Pump {
    pub fn get_pump_op(&self) -> crate::lle_util::PulsePumpOp {
        crate::lle_util::PulsePumpOp {
            peak: self.peak.get_value(),
            width: self.width.get_value(),
        }
    }
}

pub type LinearOpAdd<A, B> = lle::LinearOpAdd<f64, A, B>;

pub type LinearOp = LinearOpAdd<
    LinearOpAdd<(lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    (lle::DiffOrder, Complex64),
>;

pub type LleSolver<NL> =
    lle::LleSolver<f64, Vec<Complex64>, LinearOp, NL, crate::lle_util::PulsePumpOp>;

impl<NL: lle::NonLinearOp<f64> + Default> Controller<LleSolver<NL>> for PulsePumpLleController {
    const EXTENSION: &'static str = "plle";

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
            .constant(pump)
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
        engine.constant = pump;
        engine.step_dist = step_dist;
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
}
