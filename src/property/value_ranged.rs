use egui::{DragValue, Slider};
use num_traits::FromPrimitive;

use crate::property::custom_slider;

use super::{custom_drag, Num};

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) struct ValueRange<T> {
    pub(crate) value: T,
    pub(crate) clamp: bool,
    pub(crate) range: Option<(T, T)>,
    pub(crate) unit: Option<T>,
}

impl<T: egui::emath::Numeric + std::str::FromStr> ValueRange<T> {
    pub(crate) fn unit(&mut self, unit: T) {
        self.unit = unit.into();
    }
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        suffix: Option<&str>,
        on_hover_text: Option<&str>,
    ) {
        let r = ui.label(label);
        if let Some(text) = on_hover_text {
            r.on_hover_text(text);
        }
        let mut drag_value = DragValue::new(&mut self.value)
            .update_while_editing(false)
            .clamp_existing_to_range(self.clamp);
        if let Some(u) = self.unit {
            drag_value = custom_drag(drag_value, u);
        }
        if let (Some(range), true) = (self.range, self.clamp) {
            drag_value = drag_value.range(range.0..=range.1);
        }
        if let Some(s) = suffix {
            drag_value = drag_value.suffix(s);
        }

        let r = ui.add(drag_value);
        if let Some(text) = on_hover_text {
            r.on_hover_text(text);
        }
    }

    pub(crate) fn show_with_slider(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        show_editor: &mut bool,
        suffix: Option<&str>,
        on_hover_text: Option<&str>,
    ) {
        let Self {
            value,
            range,
            unit,
            clamp,
        } = self;
        debug_assert!(range.is_some());
        if range.is_none() {
            self.show(ui, label, suffix, on_hover_text);
            return;
        }
        let range = range.as_mut().unwrap();
        ui.horizontal(|ui| {
            if ui.button("🔧").clicked() {
                *show_editor = !*show_editor;
            }
            let r = ui.add({
                let mut slider = Slider::new(value, range.0..=range.1)
                    .text(label)
                    .smart_aim(false)
                    .max_decimals(10)
                    .min_decimals(5)
                    .clamping(if *clamp {
                        egui::SliderClamping::Edits
                    } else {
                        egui::SliderClamping::Never
                    });
                if let Some(u) = unit {
                    slider = custom_slider(slider, *u);
                }
                if let Some(s) = suffix {
                    slider = slider.suffix(s);
                }
                slider
            });
            if let Some(text) = on_hover_text {
                r.on_hover_text(text);
            }
        });
        let ctx = ui.ctx();
        egui::Window::new(format!("{} range", label))
            .title_bar(true)
            //.collapsible(false)
            .resizable(false)
            .open(show_editor)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut range.0)
                            .speed(1.)
                            .update_while_editing(false),
                    );
                    ui.label("Lower bound");
                });
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut range.1)
                            .speed(1.)
                            .update_while_editing(false),
                    );
                    ui.label("Upper bound");
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *show_editor = false;
        }
    }
}

impl<T: Copy + Num + FromPrimitive> ValueRange<T> {
    pub(crate) fn new(v: T) -> Self {
        Self {
            value: v,
            range: Some((
                v - <T as egui::emath::Numeric>::from_f64(10.),
                v + <T as egui::emath::Numeric>::from_f64(20.),
            )),
            clamp: false,
            unit: None,
        }
    }

    pub(crate) fn new_no_range(v: T) -> Self {
        Self {
            value: v,
            range: None,
            clamp: false,
            unit: None,
        }
    }

    pub(crate) fn clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
}
