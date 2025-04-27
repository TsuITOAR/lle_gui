use crate::{
    controller::{Controller, SharedState, Simulator},
    views::{State, Views, Visualizer},
};


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct ShowDispersion {
    pub(crate) show: bool,
    pub(crate) scale: f64,
}

impl From<(bool, f64)> for ShowDispersion {
    fn from((show, scale): (bool, f64)) -> Self {
        Self { show, scale }
    }
}

impl Default for ShowDispersion {
    fn default() -> Self {
        Self {
            show: false,
            scale: 1.0,
        }
    }
}

impl ui_traits::ControllerUI for ShowDispersion {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.show, "Show Dispersion").on_hover_text("Display the dispersion curve on frequency domain display windows, with a scale factor multiplied");  
            if self.show {
                ui.label("Scale");
                ui.add(egui::DragValue::new(&mut self.scale)).on_hover_text("Scale factor of the dispersion curve");
            }
        });
    }
}

pub(crate) fn add_dispersion_curve<C, S, V>(
    show_dispersion: &ShowDispersion,
    core: &super::Core<C, S>,
    views: &mut Views<V>,
) where
    C: Controller<S>,
    S: Simulator,
    Views<V>: for<'a> Visualizer<<S as SharedState<'a>>::SharedState>,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
{
    if show_dispersion.show {
        let dispersion = core.controller.dispersion();
        let points =
            crate::controller::dispersion_line(dispersion, core.dim, show_dispersion.scale);
        views.push_elements(points, crate::views::ShowOn::Freq);
    }
}
