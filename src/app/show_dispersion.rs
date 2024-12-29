use crate::{
    controller::{Controller, SharedState, Simulator},
    views::{State, Views, Visualize},
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

impl ShowDispersion {
    pub(crate) fn controller(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show, "Show Dispersion").on_hover_text("Display the dispersion curve on frequency domain display windows, with a scale factor multiplied");  
            if self.show {
                ui.add(egui::Slider::new(&mut self.scale, 0.1..=10.0).text("Scale")).on_hover_text("Scale factor of the dispersion curve");
            }
        });
    }
}

pub(crate) fn add_dispersion_curve<P, S, V>(
    show_dispersion: &ShowDispersion,
    core: &super::Core<P, S>,
    views: &mut Views<V>,
) where
    P: Controller<S>,
    S: Simulator,
    Views<V>: for<'a> Visualize<<S as SharedState<'a>>::SharedState>,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
{
    if show_dispersion.show {
        let dispersion = core.controller.dispersion();
        let points =
            crate::controller::dispersion_line(dispersion, core.dim, show_dispersion.scale);
        views.push_elements(points, crate::views::ShowOn::Freq);
    }
}
