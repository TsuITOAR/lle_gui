use crate::drawer::ViewField;
use std::array::from_fn;

mod traits;
pub use traits::*;

mod impls;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Views<V> {
    pub(crate) views: V,
    last_plot: Option<Instant>,
}

impl Default for Views<ViewField> {
    fn default() -> Self {
        Self {
            views: ViewField::new(0),
            last_plot: None,
        }
    }
}

impl<const L: usize> Default for Views<[ViewField; L]> {
    fn default() -> Self {
        Self {
            views: from_fn(ViewField::new),
            last_plot: None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Views<ViewField> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            views: ViewField::deserialize(deserializer)?,
            last_plot: None,
        })
    }
}

impl serde::Serialize for Views<ViewField> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.views.serialize(serializer)
    }
}

impl<'de, const L: usize> serde::Deserialize<'de> for Views<[ViewField; L]> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut views = <Vec<ViewField>>::deserialize(deserializer)?.into_iter();
        Ok(Self {
            views: from_fn(move |i| views.next().unwrap_or_else(|| ViewField::new(i))),
            last_plot: None,
        })
    }
}

impl<const L: usize> serde::Serialize for Views<[ViewField; L]> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.views.iter().collect::<Vec<_>>().serialize(serializer)
    }
}

impl<V> Views<V> {
    pub(crate) fn show_fps(&mut self, ui: &mut egui::Ui) {
        let now = Instant::now();
        let last = self.last_plot.replace(now);
        if let Some(last) = last {
            let past = (now - last).as_secs_f32();
            ui.label(format!("{:.0}Hz ({:.1}ms)", 1. / past, past * 1000.));
        } else {
            ui.label("Start to update fps");
        }
    }
}

pub type Style = f64;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RawPlotElement<S> {
    pub(crate) data: S,
    pub(crate) x: Option<Vec<f64>>,
    pub(crate) style: Style,
}

impl<S, const L: usize> RawPlotElement<[S; L]> {
    fn split(self) -> [RawPlotElement<S>; L] {
        let RawPlotElement { data, x, style } = self;
        data.map(|d| RawPlotElement {
            data: d,
            x: x.clone(),
            style,
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlotElement {
    pub(crate) x: Option<Vec<f64>>,
    pub(crate) y: Vec<f64>,
    pub(crate) style: Style,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub enum ShowOn {
    #[default]
    Both,
    Time,
    Freq,
}
