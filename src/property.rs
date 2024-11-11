use std::str::FromStr;

use egui::{DragValue, Key, Slider};
use num_traits::FromPrimitive;

pub trait Num:
    num_traits::Num
    + num_traits::ToPrimitive
    + num_traits::FromPrimitive
    + Copy
    + eframe::emath::Numeric
{
}

impl<T> Num for T where
    T: num_traits::Num
        + num_traits::ToPrimitive
        + num_traits::FromPrimitive
        + Copy
        + eframe::emath::Numeric
{
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) struct Property<T: Num + Copy> {
    pub(crate) value: ValueRange<T>,
    pub(crate) label: String,
    pub(crate) symbol: Option<String>,
    pub(crate) show_editor: Option<bool>,
    pub(crate) value_suffix: Option<String>,
}

fn custom_drag<T: egui::emath::Numeric + std::str::FromStr>(
    drag_value: DragValue<'_>,
    unit: T,
) -> DragValue<'_> {
    let unit = unit.to_f64();
    drag_value
        .custom_formatter(move |x, _r| format!("{:E}", x))
        .speed(unit)
}

fn custom_slider<T: egui::emath::Numeric + std::str::FromStr>(
    slider: Slider<'_>,
    unit: T,
) -> Slider<'_> {
    let unit = unit.to_f64();
    slider
        .step_by(unit)
        .custom_formatter(move |x, _r| format!("{:E}", x))
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) struct ValueRange<T> {
    pub(crate) value: T,
    pub(crate) clamp: bool,
    pub(crate) range: Option<(T, T)>,
    pub(crate) unit: Option<T>,
}

impl<T: egui::emath::Numeric + std::str::FromStr> ValueRange<T> {
    fn unit(&mut self, unit: T) {
        self.unit = unit.into();
    }
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, label: &str, suffix: Option<&str>) {
        ui.label(label);
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
        ui.add(drag_value);
    }

    pub(crate) fn show_with_slider(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        show_editor: &mut bool,
        suffix: Option<&str>,
    ) {
        let Self {
            value,
            range,
            unit,
            clamp,
        } = self;
        debug_assert!(range.is_some());
        if range.is_none() {
            self.show(ui, label, suffix);
            return;
        }
        let range = range.as_mut().unwrap();
        ui.horizontal(|ui| {
            if ui.button("🔧").clicked() {
                *show_editor = !*show_editor;
            }
            ui.add({
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
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
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

    pub(crate) fn clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
}

impl<T: Num + Copy> Property<T> {
    pub fn new(v: T, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new(v).clamp(false),
            label: label.to_string(),
            symbol: None,
            show_editor: Some(false),
            value_suffix: None,
        }
    }
    pub fn new_no_slider(v: T, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new(v).clamp(true),
            label: label.to_string(),
            symbol: None,
            show_editor: None,
            value_suffix: None,
        }
    }
    pub fn symbol(mut self, symbol: impl ToString) -> Self {
        self.symbol = symbol.to_string().into();
        self
    }
    pub fn unit(mut self, unit: T) -> Self
    where
        T: FromStr,
    {
        self.value.unit(unit);
        self
    }
    #[allow(unused)]
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.value_suffix = suffix.to_string().into();
        self
    }

    /* #[allow(unused)]
    pub fn text(&self) -> &str {
        self.symbol.as_deref().unwrap_or(self.label.as_str())
    } */

    /* pub fn get_value(&self) -> &PropertyValue {
        &self.value
    } */

    pub fn range(mut self, range: (T, T)) -> Self {
        self.value.range = Some(range);
        self
    }

    pub fn get_value(&self) -> T {
        self.value.value
    }
}

impl<T: Num + FromStr> Property<T> {
    /* pub(crate) fn value_suffix(&self) -> Option<String> {
        self.value_suffix
            .clone()
            .or_else(|| self.unit.as_ref().map(|u| format!("*{u:E}")))
    } */
    /* pub(crate) fn show_as_drag_value_in_grid(&mut self, ui: &mut egui::Ui) {
        let label = self.symbol.as_deref().unwrap_or(self.label.as_str());
        let suffix = self
            .value_suffix
            .clone()
            .or_else(|| self.unit.as_ref().map(|u| format!("*{u:E}")));
        self.value.show(ui, label, suffix.as_deref());
    } */
    pub(crate) fn show_as_drag_value(&mut self, ui: &mut egui::Ui) {
        //ui.horizontal_wrapped(|ui| {
        let label = self.symbol.as_deref().unwrap_or(self.label.as_str());
        // let suffix = self.value_suffix.clone();
        self.value.show(ui, label, self.value_suffix.as_deref());
        //});
    }
    pub(crate) fn show_in_builder(&mut self, ui: &mut egui::Ui) {
        self.show_as_drag_value(ui);
    }
    pub(crate) fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        if self.show_editor.is_none() {
            ui.horizontal_wrapped(|ui| {
                self.show_in_builder(ui);
            });
        } else {
            let suffix = self.value_suffix.as_deref();
            //let suffix = self.value_suffix();
            let Self {
                value,
                label,
                symbol,
                show_editor,
                ..
            } = self;
            let show_editor = show_editor.as_mut().unwrap();
            let label = symbol
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| label.as_str());
            ui.horizontal_wrapped(|ui| {
                value.show_with_slider(ui, label, show_editor, suffix);
            });
        }
    }
}
