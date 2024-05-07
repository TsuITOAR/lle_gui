use egui::DragValue;

use crate::{easy_mark::easy_mark, property::Property};

pub(crate) fn config<'a>(
    dim: &mut usize,
    properties: impl Iterator<Item = &'a mut Property>,
    ui: &mut egui::Ui,
) -> bool {
    easy_mark(ui, LLE_EQUATION);
    egui::Grid::new("Controller grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Dimension");
            ui.add(DragValue::new(dim).speed(1));
            ui.end_row();
            properties.for_each(|x| {
                x.show_as_drag_value(ui);
                ui.end_row();
            })
        });

    ui.centered_and_justified(|ui| ui.button("✅").clicked())
        .inner
}

const LLE_EQUATION: &str = r#"∂ψ\/∂t = - ( 1 + i α ) ψ + i |ψ|^2^ ψ - i β\/2 ∂^2^ψ\/∂θ^2^ + F"#;
