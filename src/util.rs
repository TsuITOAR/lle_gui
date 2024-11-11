use lle::{num_complex::Complex64, LinearOp, NonLinearOp};

use crate::controller;

pub fn synchronize_properties<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add((2, Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.constant = Complex64::from(props.pump.get_value()).into();
    engine.step_dist = props.step_dist.get_value();
}

pub fn synchronize_properties_no_pump<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add((2, -Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.step_dist = props.step_dist.get_value();
}

pub(crate) fn toggle_option<T: Default>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut ch = v.is_some();
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = T::default().into();
    } else if !ch {
        *v = None;
    }

    r
}

pub(crate) fn toggle_option_with<T, F>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
    f: F,
) -> egui::Response
where
    F: FnOnce() -> Option<T>,
{
    let mut ch = v.is_some();
    //let r = ui.checkbox(&mut ch, text);
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = f();
    } else if !ch {
        *v = None;
    }

    r
}

pub fn show_profiler(show: &mut bool, ui: &mut egui::Ui) {
    if ui.toggle_value(show, "profile performance").clicked() {
        puffin::set_scopes_on(*show); // Remember to call this, or puffin will be disabled!
    }
    if *show {
        puffin_egui::profiler_ui(ui)
    }
}
