use egui::DragValue;

use crate::easy_mark::easy_mark;

pub trait ControllerAsGrid {
    fn show(&mut self, ui: &mut egui::Ui);
}

impl<T: crate::property::Num + std::str::FromStr> ControllerAsGrid
    for crate::property::Property<T>
{
    fn show(&mut self, ui: &mut egui::Ui) {
        self.show_as_drag_value(ui);
    }
}

pub(crate) fn config<C: ControllerAsGrid>(dim: &mut usize, properties: &mut C, ui: &mut egui::Ui) {
    easy_mark(ui, LLE_EQUATION);
    egui::Grid::new("Controller grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Dimension");
            let mut d_log = (*dim as f64).log(2.) as u32;
            ui.add(
                DragValue::new(&mut d_log)
                    .speed(0.1)
                    .range(7..=15)
                    .custom_parser(|s| {
                        Some(
                            (s.parse::<u32>()
                                .map(|x| (x as f64).log(2.) as u32)
                                .unwrap_or(7)) as _,
                        )
                    })
                    .custom_formatter(|v, _| format!("{}", 2u32.pow(v as u32)))
                    .clamp_existing_to_range(true), //.suffix(format!("(2^{})", (*dim as f64).log(2.) as u32)),
            );
            *dim = 2u32.pow(d_log) as usize;
            ui.end_row();
            properties.show(ui);
        });
}

const LLE_EQUATION: &str = r#"∂ψ\/∂t = - ( 1 + i α ) ψ + i |ψ|^2^ ψ - i β\/2 ∂^2^ψ\/∂θ^2^ + F"#;
