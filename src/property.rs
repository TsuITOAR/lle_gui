use egui::{DragValue, Key};

use crate::{show_as_drag_value, show_as_drag_value_with_suffix};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub(crate) struct Property<T> {
    pub(crate) value: T,
    pub(crate) range: (T, T),
    pub(crate) label: String,
    pub(crate) symbol: Option<String>,
    pub(crate) edit_range_window: Option<bool>,
    pub(crate) unit: Option<T>,
    pub(crate) value_suffix: Option<String>,
}

impl Property<f64> {
    pub fn new(v: f64, label: impl ToString) -> Self {
        Self {
            value: v,
            range: (v - 10., v + 20.),
            label: label.to_string(),
            symbol: None,
            edit_range_window: Some(false),
            unit: None,
            value_suffix: None,
        }
    }
    pub fn new_no_range(v: f64, label: impl ToString) -> Self {
        Self {
            value: v,
            range: (v - 10., v + 20.),
            label: label.to_string(),
            symbol: None,
            edit_range_window: None,
            unit: None,
            value_suffix: None,
        }
    }
    pub fn symbol(mut self, symbol: impl ToString) -> Self {
        self.symbol = symbol.to_string().into();
        self
    }
    pub fn unit(mut self, unit: f64) -> Self {
        self.unit = unit.into();
        self
    }
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.value_suffix = suffix.to_string().into();
        self
    }
    #[allow(unused)]
    pub fn text(&self) -> &str {
        self.symbol
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|| self.label.as_str())
    }
    pub fn get_value(&self) -> f64 {
        if let Some(u) = self.unit {
            self.value * u
        } else {
            self.value
        }
    }
}

impl Property<f64> {
    pub(crate) fn value_suffix(&self) -> Option<String> {
        self.value_suffix
            .clone()
            .or_else(|| self.unit.as_ref().map(|u| format!("*{u:E}")))
    }
    pub(crate) fn show_as_drag_value_in_grid(&mut self, ui: &mut egui::Ui) {
        let label = self
            .symbol
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|| self.label.as_str());
        match self.value_suffix() {
            Some(s) => show_as_drag_value_with_suffix(label, &mut self.value, ui, s),
            None => show_as_drag_value(label, &mut self.value, ui),
        }
    }
    pub(crate) fn show_as_drag_value(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let label = self
                .symbol
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| self.label.as_str());
            match self.value_suffix() {
                Some(s) => show_as_drag_value_with_suffix(label, &mut self.value, ui, s),
                None => show_as_drag_value(label, &mut self.value, ui),
            }
        });
    }
    pub(crate) fn show_in_builder(&mut self, ui: &mut egui::Ui) {
        self.show_as_drag_value(ui);
    }
    pub(crate) fn show_in_control_pannel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use egui::Slider;
        if self.edit_range_window.is_none() {
            self.show_in_builder(ui);
        } else {
            let s = self.value_suffix();
            let Self {
                value,
                range,
                label,
                symbol,
                edit_range_window,
                ..
            } = self;
            let edit_range = edit_range_window.as_mut().unwrap();
            let label = symbol
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| label.as_str());
            ui.horizontal(|ui| {
                if ui.button("🔧").clicked() {
                    *edit_range = !*edit_range;
                }
                ui.add({
                    if let Some(s) = s {
                        Slider::new(value, range.0..=range.1)
                            .text(label)
                            .smart_aim(false)
                            .max_decimals(10)
                            .min_decimals(5)
                            .suffix(s)
                    } else {
                        Slider::new(value, range.0..=range.1)
                            .text(label)
                            .smart_aim(false)
                            .max_decimals(10)
                            .min_decimals(5)
                    }
                });
            });
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
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                *edit_range = false;
            }
        }
    }
}
