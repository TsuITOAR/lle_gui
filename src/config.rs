use egui::DragValue;

use crate::easy_mark::easy_mark;

pub trait ControllerAsGrid {
    fn show(&mut self, ui: &mut egui::Ui);
}

impl ControllerAsGrid for crate::controller::LleController {
    fn show(&mut self, ui: &mut egui::Ui) {
        self.alpha.show_as_drag_value(ui);
        ui.end_row();
        self.linear.show_as_drag_value(ui);
        ui.end_row();
        self.pump.show_as_drag_value(ui);
        ui.end_row();
        self.step_dist.show_as_drag_value(ui);
        ui.end_row();
        self.steps.show_as_drag_value(ui);
        ui.end_row();
    }
}

pub(crate) fn config<C: ControllerAsGrid>(dim: &mut usize, properties: &mut C, ui: &mut egui::Ui) {
    easy_mark(ui, LLE_EQUATION);
    egui::Grid::new("Controller grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Dimension");
            ui.add(DragValue::new(dim).speed(1));
            ui.end_row();
            properties.show(ui);
        });
}

const LLE_EQUATION: &str = r#"∂ψ\/∂t = - ( 1 + i α ) ψ + i |ψ|^2^ ψ - i β\/2 ∂^2^ψ\/∂θ^2^ + F"#;
