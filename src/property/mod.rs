use std::str::FromStr;

use egui::{DragValue, Slider};

mod value_ranged;
pub(crate) use value_ranged::ValueRange;

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

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub(crate) struct Property<T: Num> {
    pub(crate) value: ValueRange<T>,
    pub(crate) label: String,
    pub(crate) symbol: Option<String>,
    pub(crate) show_editor: Option<bool>,
    pub(crate) value_suffix: Option<String>,
    #[serde(default)]
    pub(crate) on_hover_text: Option<String>,
}

fn custom_drag<T: egui::emath::Numeric + std::str::FromStr>(
    drag_value: DragValue<'_>,
    unit: T,
) -> DragValue<'_> {
    let unit = unit.to_f64();
    drag_value
        .custom_formatter(move |x, _r| format!("{x:E}"))
        .speed(unit)
}

fn custom_slider<T: egui::emath::Numeric + std::str::FromStr>(
    slider: Slider<'_>,
    unit: T,
) -> Slider<'_> {
    let unit = unit.to_f64();
    slider
        .step_by(unit)
        .custom_formatter(move |x, _r| format!("{x:E}"))
}

impl<T: Num + Copy> Property<T> {
    pub fn new(v: T, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new(v).clamp(false),
            label: label.to_string(),
            symbol: None,
            show_editor: Some(false),
            value_suffix: None,
            on_hover_text: None,
        }
    }
    pub fn new_no_slider(v: T, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new_no_range(v).clamp(true),
            label: label.to_string(),
            symbol: None,
            show_editor: None,
            value_suffix: None,
            on_hover_text: None,
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

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value.value
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
        self.value.show(
            ui,
            label,
            self.value_suffix.as_deref(),
            self.on_hover_text.as_deref(),
        );
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
                value.show_with_slider(
                    ui,
                    label,
                    show_editor,
                    suffix,
                    self.on_hover_text.as_deref(),
                );
            });
        }
    }
}

impl<T: Num> Property<T> {
    pub fn on_hover_text(mut self, text: impl ToString) -> Self {
        self.on_hover_text = text.to_string().into();
        self
    }
}
