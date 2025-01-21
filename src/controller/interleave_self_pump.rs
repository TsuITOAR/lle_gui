use egui::mutex::RwLock;

use super::*;

#[allow(unused)]
pub type App = crate::app::GenApp<
    InterleaveSelfPumpLleController,
    LleSolver<lle::SPhaMod>,
    crate::drawer::ViewField,
>;

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct InterleaveSelfPumpLleController {
    pub(crate) alpha: Property<f64>,
    pub(crate) linear: Property<f64>,
    pub(crate) pump: InterleaveSelfPump,
    pub(crate) step_dist: Property<f64>,
    pub(crate) steps: Property<u32>,
}

impl std::default::Default for InterleaveSelfPumpLleController {
    fn default() -> Self {
        Self {
            alpha: Property::new(-5., "alpha").symbol('α'),
            linear: Property::new(-0.0444, "linear")
                .symbol('β')
                .range((-0.1, 0.1)),
            pump: InterleaveSelfPump::default(),
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
pub struct InterleaveSelfPump {
    pub(crate) const_pump: Property<f64>,
    pub(crate) main_delay: Property<usize>,
    pub(crate) interleave_delay: Property<usize>,
    pub(crate) interleave_ratio: Property<f64>,
    pub(crate) d1_mismatch: Property<f64>,
    pub(crate) loop_dispersion: Property<f64>,
    pub(crate) loop_loss: Property<f64>,
    pub(crate) loop_window: Property<usize>,
}

impl std::default::Default for InterleaveSelfPump {
    fn default() -> Self {
        Self {
            const_pump: Property::new(1e-2, "Cw Pump").symbol('F'),
            main_delay: Property::new_no_slider(10, "Main Delay"),
            interleave_delay: Property::new_no_slider(5, "Interleave Delay"),
            interleave_ratio: Property::new(0.9, "Interleave Ratio").range((0., 1.)),
            loop_dispersion: Property::new(0., "Loop Dispersion").range((-1., 1.)),
            d1_mismatch: Property::new(0., "D1 Mismatch").range((-1., 1.)),
            loop_loss: Property::new(1., "Loop Loss").range((0., 1.5)),
            loop_window: Property::new_no_slider(128, "Loop Window"),
        }
    }
}

impl InterleaveSelfPump {
    pub fn new_pump_op(
        &self,
    ) -> lle::ConstOpAdd<f64, crate::lle_util::InterleaveSelfPumpOp, Complex64> {
        let self_pump1 = crate::lle_util::SelfPumpOp {
            now: RwLock::new(0),
            delay: self.main_delay.get_value(),
            d1_mismatch: self.d1_mismatch.get_value(),
            loop_dispersion: self.loop_dispersion.get_value(),
            loop_loss: self.loop_loss.get_value(),
            window: self.loop_window.get_value(),
            cache: RwLock::new(Vec::new()),
            fft: RwLock::new(None),
        };
        let self_pump2 = crate::lle_util::SelfPumpOp {
            now: RwLock::new(0),
            delay: self.main_delay.get_value() + self.interleave_delay.get_value(),
            d1_mismatch: self.d1_mismatch.get_value(),
            loop_dispersion: self.loop_dispersion.get_value(),
            loop_loss: self.loop_loss.get_value(),
            window: self.loop_window.get_value(),
            cache: RwLock::new(Vec::new()),
            fft: RwLock::new(None),
        };

        crate::lle_util::InterleaveSelfPumpOp {
            channel1: self_pump1,
            channel2: self_pump2,
            mix: self.interleave_ratio.get_value(),
        }
        .add_const_op(Complex64::from(self.const_pump.get_value()))
    }

    pub fn update_pump_op(&self, pump: &mut Pump) {
        pump.op1.channel1.delay = self.main_delay.get_value();
        pump.op1.channel1.d1_mismatch = self.d1_mismatch.get_value();
        pump.op1.channel1.loop_dispersion = self.loop_dispersion.get_value();
        pump.op1.channel1.loop_loss = self.loop_loss.get_value();
        pump.op1.channel1.window = self.loop_window.get_value();

        pump.op1.channel2.delay = self.main_delay.get_value() + self.interleave_delay.get_value();
        pump.op1.channel2.d1_mismatch = self.d1_mismatch.get_value();
        pump.op1.channel2.loop_dispersion = self.loop_dispersion.get_value();
        pump.op1.channel2.loop_loss = self.loop_loss.get_value();
        pump.op1.channel2.window = self.loop_window.get_value();

        pump.op1.mix = self.interleave_ratio.get_value();

        pump.op2 = Complex64::from(self.const_pump.get_value());
    }
}

pub type LinearOpAdd<A, B> = lle::LinearOpAdd<f64, A, B>;

pub type LinearOp = LinearOpAdd<(lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>;

pub type Pump = lle::ConstOpAdd<f64, crate::lle_util::InterleaveSelfPumpOp, Complex64>;

pub type LleSolver<NL> = lle::LleSolver<f64, Vec<Complex64>, LinearOp, NL, Pump>;

impl<NL: lle::NonLinearOp<f64> + Default> Controller<LleSolver<NL>>
    for InterleaveSelfPumpLleController
{
    const EXTENSION: &'static str = "islle";

    type Dispersion = (lle::DiffOrder, Complex64);

    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.linear.get_value() / 2.)
    }

    fn construct_engine(&self, dim: usize) -> LleSolver<NL> {
        use lle::LinearOp;
        let step_dist = self.step_dist.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();
        let pump = self.pump.new_pump_op();
        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        LleSolver::builder()
            .state(init.to_vec())
            .step_dist(step_dist)
            .linear(
                (0, -(Complex64::i() * alpha + 1.))
                    .add_linear_op((2, Complex64::i() * linear / 2.)),
            )
            .nonlin(NL::default())
            .constant(pump)
            .constant_freq(NoneOp::default())
            .build()
    }

    fn sync_paras(&mut self, engine: &mut LleSolver<NL>) {
        puffin_egui::puffin::profile_function!();
        let step_dist = self.step_dist.get_value();
        let linear = self.linear.get_value();
        let alpha = self.alpha.get_value();
        use lle::LinearOp;
        engine.linear =
            (0, -(Complex64::i() * alpha + 1.)).add_linear_op((2, Complex64::i() * linear / 2.));
        self.pump.update_pump_op(&mut engine.constant);
        engine.step_dist = step_dist;
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
}
