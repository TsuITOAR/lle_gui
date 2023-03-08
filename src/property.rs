#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub(crate) struct Property<T> {
    pub(crate) value: T,
    pub(crate) range: (T, T),
    pub(crate) label: String,
    pub(crate) edit_range: bool,
}

impl Property<f64> {
    pub fn new(v: f64, label: impl ToString) -> Self {
        Self {
            value: v,
            range: (v - 10., v + 20.),
            label: label.to_string(),
            edit_range: false,
        }
    }
}

impl Property<f64> {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use egui::Slider;
        let Self {
            value,
            range,
            label,
            edit_range,
        } = self;
        let label = label.as_str();
        ui.horizontal(|ui| {
            if ui.button("🔧").clicked() {
                *edit_range = !*edit_range
            }
            ui.add(
                Slider::new(value, range.0..=range.1)
                    .text(&label)
                    .smart_aim(false),
            );
        });

        use egui::DragValue;
        egui::Window::new(format!("{} range", label))
            .title_bar(true)
            //.collapsible(false)
            .resizable(false)
            .open(edit_range)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut range.0).speed(1.));
                    ui.label("Lower bound");
                });
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut range.1).speed(1.));
                    ui.label("Upper bound");
                });
            });
    }
}
