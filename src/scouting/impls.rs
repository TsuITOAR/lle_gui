use crate::controller::pulse_pump::PulsePumpLleController;
use crate::controller::self_pump::SelfPumpLleController;
use crate::controller::{self, clle::CoupleLleController, LleController};

use super::*;

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    enum_iterator::Sequence,
)]
pub enum BasicScoutingTarget {
    #[default]
    Alpha,
    Pump,
    Linear,
    StepDist,
}

impl BasicScoutingTarget {
    pub fn desc(&self) -> &str {
        match self {
            BasicScoutingTarget::Alpha => "α",
            BasicScoutingTarget::Pump => "F",
            BasicScoutingTarget::Linear => "β",
            BasicScoutingTarget::StepDist => "Δt",
        }
    }
}

impl Config for BasicScoutingTarget {
    fn config(&mut self, ui: &mut egui::Ui) {
        enum_iterator::all::<BasicScoutingTarget>().for_each(|s| {
            if ui.selectable_label(self == &s, s.desc()).clicked() {
                *self = s;
            }
        })
    }
}

impl<S> ScoutingTarget<LleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<CoupleLleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

use controller::cprt::CprtLleController;
use controller::cprt2::CprtLleController2;

use controller::disper::DisperLleController;
use controller::disper2::DisperLleController2;

impl<S> ScoutingTarget<CprtLleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<CprtLleController2, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<DisperLleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<DisperLleController2, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<PulsePumpLleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.peak.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}

impl<S> ScoutingTarget<SelfPumpLleController, S> for BasicScoutingTarget
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
            BasicScoutingTarget::Alpha => *controller.alpha.value_mut() += value,
            BasicScoutingTarget::Pump => *controller.pump.loop_loss.value_mut() += value,
            BasicScoutingTarget::Linear => *controller.linear.value_mut() += value,
            BasicScoutingTarget::StepDist => *controller.step_dist.value_mut() += value,
        }
    }
}
