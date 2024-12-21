use crate::easy_mark::easy_mark;

pub use ui_traits::ControllerStartWindow;
pub use ui_traits::ControllerUI;

impl<T: crate::property::Num + std::str::FromStr> ControllerStartWindow
    for crate::property::Property<T>
{
    fn show_start_window(&mut self, ui: &mut egui::Ui) {
        self.show_as_drag_value(ui);
        ui.end_row();
    }
}

impl<T: crate::property::Num + std::str::FromStr> ControllerUI for crate::property::Property<T> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        self.show_in_control_panel(ui);
        ui.end_row();
    }
}

pub(crate) fn config<C: ControllerStartWindow + ?Sized>(
    dim: &mut usize,
    properties: &mut C,
    ui: &mut egui::Ui,
) {
    easy_mark(ui, LLE_EQUATION);
    egui::Grid::new("Controller grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            crate::util::show_dim(ui, dim);
            ui.end_row();
            properties.show_start_window(ui);
        });
}

const LLE_EQUATION: &str = r#"∂ψ\/∂t = - ( 1 + i α ) ψ + i |ψ|^2^ ψ - i β\/2 ∂^2^ψ\/∂θ^2^ + F"#;
