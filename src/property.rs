use egui::{DragValue, Key, Slider};
use num_traits::ToPrimitive;

#[derive(Debug, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) struct Property {
    pub(crate) value: PropertyValue,
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
        .custom_formatter(move |x, _r| format!("{}×{:E}", x / unit, unit))
        .custom_parser(move |s| {
            s.split('×')
                .next()
                .map(|x| x.parse().ok().map(|x: T| x.to_f64() * unit))
                .unwrap_or(None)
        })
        .speed(unit)
}

fn custom_slider<T: egui::emath::Numeric + std::str::FromStr>(
    slider: Slider<'_>,
    unit: T,
) -> Slider<'_> {
    let unit = unit.to_f64();
    slider
        .custom_parser(move |s| {
            s.split('×')
                .next()
                .map(|x| x.parse().ok().map(|x: T| x.to_f64() * unit))
                .unwrap_or(None)
        })
        .custom_formatter(move |x, _r| format!("{}×{:E}", x / unit, unit))
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) struct ValueRange<T> {
    pub(crate) value: T,
    pub(crate) range: Option<(T, T)>,
    pub(crate) unit: Option<T>,
}

impl<T: egui::emath::Numeric + std::str::FromStr> ValueRange<T> {
    fn unit(&mut self, unit: T) {
        self.unit = unit.into();
    }
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, label: &str, suffix: Option<&str>) {
        ui.label(label);
        let drag_value = DragValue::new(&mut self.value);
        let drag_value = if let Some(u) = self.unit {
            custom_drag(drag_value, u)
        } else {
            drag_value
        };
        if let Some(s) = suffix {
            ui.add(drag_value.suffix(s));
        } else {
            ui.add(drag_value);
        }
    }

    pub(crate) fn show_with_slider(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        show_editor: &mut bool,
        suffix: Option<&str>,
    ) {
        let Self { value, range, unit } = self;
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
                let slider = Slider::new(value, range.0..=range.1)
                    .text(label)
                    .smart_aim(false)
                    .max_decimals(10)
                    .min_decimals(5);
                let slider = if let Some(u) = unit {
                    custom_slider(slider, *u)
                } else {
                    slider
                };
                if let Some(s) = suffix {
                    slider.suffix(s)
                } else {
                    slider
                }
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
                    ui.add(DragValue::new(&mut range.0).speed(1.));
                    ui.label("Lower bound");
                });
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut range.1).speed(1.));
                    ui.label("Upper bound");
                });
            });
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            *show_editor = false;
        }
    }
}

impl ValueRange<f64> {
    pub(crate) fn new_float(v: f64) -> Self {
        Self {
            value: v,
            range: Some((v - 10., v + 20.)),
            unit: None,
        }
    }
}

#[allow(unused)]
impl ValueRange<i32> {
    pub(crate) fn new_int(v: i32) -> Self {
        Self {
            value: v,
            range: None,
            unit: None,
        }
    }
}

impl ValueRange<u32> {
    pub(crate) fn new_uint(v: u32) -> Self {
        Self {
            value: v,
            range: None,
            unit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub(crate) enum PropertyValue {
    F64(ValueRange<f64>),
    I32(ValueRange<i32>),
    U32(ValueRange<u32>),
}

impl PropertyValue {
    pub(crate) fn f64(&self) -> f64 {
        match self {
            Self::F64(v) => v.value,
            _ => panic!("Not a f64"),
        }
    }
    #[allow(unused)]
    pub(crate) fn i32(&self) -> i32 {
        match self {
            Self::I32(v) => v.value,
            _ => panic!("Not a i32"),
        }
    }
    pub(crate) fn u32(&self) -> u32 {
        match self {
            Self::U32(v) => v.value,
            _ => panic!("Not a u32"),
        }
    }
    fn unit<T: ToPrimitive>(&mut self, u: T) {
        match self {
            Self::F64(v) => v.unit(u.to_f64().unwrap()),
            Self::I32(v) => v.unit(u.to_i32().unwrap()),
            Self::U32(v) => v.unit(u.to_u32().unwrap()),
        }
    }
    fn show(&mut self, ui: &mut egui::Ui, label: &str, suffix: Option<&str>) {
        match self {
            Self::F64(v) => v.show(ui, label, suffix),
            Self::I32(v) => v.show(ui, label, suffix),
            Self::U32(v) => v.show(ui, label, suffix),
        }
    }
    fn show_with_slider(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        show_editor: &mut bool,
        suffix: Option<&str>,
    ) {
        match self {
            Self::F64(v) => v.show_with_slider(ui, label, show_editor, suffix),
            Self::I32(v) => v.show_with_slider(ui, label, show_editor, suffix),
            Self::U32(v) => v.show_with_slider(ui, label, show_editor, suffix),
        }
    }
}

macro_rules! from_range {
    ($($t:ty=>$i:ident)*) => {
        $(
            impl From<ValueRange<$t>> for PropertyValue {
                fn from(v: ValueRange<$t>) -> Self {
                    Self::$i(v)
                }
            }
        )*
    };
}

from_range!(
    f64=>F64
    i32=>I32
    u32=>U32
);

impl Property {
    pub fn new_float(v: f64, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new_float(v).into(),
            label: label.to_string(),
            symbol: None,
            show_editor: Some(false),
            value_suffix: None,
        }
    }
    pub fn new_float_no_slider(v: f64, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new_float(v).into(),
            label: label.to_string(),
            symbol: None,
            show_editor: None,
            value_suffix: None,
        }
    }
    #[allow(unused)]
    pub fn new_int(v: i32, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new_int(v).into(),
            label: label.to_string(),
            symbol: None,
            show_editor: None,
            value_suffix: None,
        }
    }
    pub fn new_uint(v: u32, label: impl ToString) -> Self {
        Self {
            value: ValueRange::new_uint(v).into(),
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
    pub fn unit<T: ToPrimitive>(mut self, unit: T) -> Self {
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

    pub fn get_value_f64(&self) -> f64 {
        self.value.f64()
    }
}

impl Property {
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
