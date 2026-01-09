use crate::controller::dual_pulse_pump::DualPulsePumpLleController;
use crate::controller::gencprt::GenCprtController;
use crate::controller::interleave_self_pump::InterleaveSelfPumpLleController;
use crate::controller::pulse_pump::PulsePumpLleController;
use crate::controller::self_pump::SelfPumpLleController;
use crate::controller::{self, LleController, clle::CoupleLleController};

use super::*;

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
    enum_iterator::Sequence,
)]
pub enum BasicPreviewTarget {
    #[default]
    Alpha,
    Pump,
    Linear,
    StepDist,
}

impl crate::util::DisplayStr for BasicPreviewTarget {
    fn desc(&self) -> &str {
        match self {
            BasicPreviewTarget::Alpha => "α",
            BasicPreviewTarget::Pump => "F",
            BasicPreviewTarget::Linear => "β",
            BasicPreviewTarget::StepDist => "Δt",
        }
    }
}

impl<S> PreviewTarget<LleController, S> for BasicPreviewTarget
where
    S: Simulator,
    LleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &LleController, dst: &mut LleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut LleController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<CoupleLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    CoupleLleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &CoupleLleController, dst: &mut CoupleLleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut CoupleLleController) {
        let controller = &mut controller.basic;
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

use controller::cprt::CprtLleController;
use controller::cprt2::CprtLleController2;

use controller::disper::DisperLleController;
use controller::disper2::DisperLleController2;

impl<S> PreviewTarget<CprtLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    CprtLleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &CprtLleController, dst: &mut CprtLleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut CprtLleController) {
        let controller = &mut controller.basic;
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<CprtLleController2, S> for BasicPreviewTarget
where
    S: Simulator,
    CprtLleController2: Controller<S>,
{
    fn sync(&self, value: f64, src: &CprtLleController2, dst: &mut CprtLleController2) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut CprtLleController2) {
        let controller = &mut controller.basic;
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<DisperLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    DisperLleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &DisperLleController, dst: &mut DisperLleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut DisperLleController) {
        let controller = &mut controller.basic;
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<DisperLleController2, S> for BasicPreviewTarget
where
    S: Simulator,
    DisperLleController2: Controller<S>,
{
    fn sync(&self, value: f64, src: &DisperLleController2, dst: &mut DisperLleController2) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut DisperLleController2) {
        let controller = &mut controller.basic;
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<PulsePumpLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    PulsePumpLleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &PulsePumpLleController, dst: &mut PulsePumpLleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut PulsePumpLleController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.peak.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<SelfPumpLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    SelfPumpLleController: Controller<S>,
{
    fn sync(&self, value: f64, src: &SelfPumpLleController, dst: &mut SelfPumpLleController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut SelfPumpLleController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.loop_loss.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<DualPulsePumpLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    DualPulsePumpLleController: Controller<S>,
{
    fn sync(
        &self,
        value: f64,
        src: &DualPulsePumpLleController,
        dst: &mut DualPulsePumpLleController,
    ) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut DualPulsePumpLleController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => {
                *controller.pump.pulse1.peak.value_mut() += value;
                *controller.pump.pulse2.peak.value_mut() += value
            }
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<InterleaveSelfPumpLleController, S> for BasicPreviewTarget
where
    S: Simulator,
    InterleaveSelfPumpLleController: Controller<S>,
{
    fn sync(
        &self,
        value: f64,
        src: &InterleaveSelfPumpLleController,
        dst: &mut InterleaveSelfPumpLleController,
    ) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut InterleaveSelfPumpLleController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.loop_loss.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> PreviewTarget<GenCprtController, S> for BasicPreviewTarget
where
    S: Simulator,
    GenCprtController: Controller<S>,
{
    fn sync(&self, value: f64, src: &GenCprtController, dst: &mut GenCprtController) {
        *dst = src.clone();
        self.apply(value, dst);
    }
    fn apply(&self, value: f64, controller: &mut GenCprtController) {
        match self {
            BasicPreviewTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicPreviewTarget::Pump => *controller.pump.amplitude.value_mut() += value,
            BasicPreviewTarget::Linear => *controller.disper.linear.value_mut() += value,
            BasicPreviewTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}
