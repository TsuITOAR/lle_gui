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
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
        let src = &src.basic;
        let dst = &mut dst.basic;
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
        let src = &src.basic;
        let dst = &mut dst.basic;
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
        let src = &src.basic;
        let dst = &mut dst.basic;
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
        let src = &src.basic;
        let dst = &mut dst.basic;
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
        let src = &src.basic;
        let dst = &mut dst.basic;
        match self {
            BasicScoutingTarget::Alpha => *dst.alpha.value_mut() = src.alpha.get_value() + value,
            BasicScoutingTarget::Pump => *dst.pump.value_mut() = src.pump.get_value() + value,
            BasicScoutingTarget::Linear => *dst.linear.value_mut() = src.linear.get_value() + value,
            BasicScoutingTarget::StepDist => {
                *dst.step_dist.value_mut() = src.step_dist.get_value() + value
            }
        }
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
