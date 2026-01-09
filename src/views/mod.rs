use crate::{FftSource, drawer::ViewField};
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

impl<S: FftSource> Default for Views<ViewField<S>> {
    fn default() -> Self {
        Self {
            views: ViewField::<S>::new(0),
            last_plot: None,
        }
    }
}

impl<const L: usize, S: FftSource> Default for Views<[ViewField<S>; L]> {
    fn default() -> Self {
        Self {
            views: from_fn(ViewField::<S>::new),
            last_plot: None,
        }
    }
}

impl<'de, S: FftSource + for<'a> serde::Deserialize<'a>> serde::Deserialize<'de>
    for Views<ViewField<S>>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            views: ViewField::<S>::deserialize(deserializer)?,
            last_plot: None,
        })
    }
}

impl<F: FftSource + serde::Serialize> serde::Serialize for Views<ViewField<F>> {
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

pub type Width = f64;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RawPlotData<S> {
    pub(crate) data: S,
    pub(crate) x: Option<Vec<f64>>,
    pub(crate) width: Width,
    #[serde(default)]
    pub(crate) style: Option<crate::drawer::plot_item::Style>,
}

impl<S, const L: usize> RawPlotData<[S; L]> {
    fn split(self) -> [RawPlotData<S>; L] {
        let RawPlotData {
            data,
            x,
            width,
            style,
        } = self;
        data.map(|d| RawPlotData {
            data: d,
            x: x.clone(),
            width,
            style,
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlotElement {
    pub(crate) x: Option<Vec<f64>>,
    pub(crate) y: Vec<f64>,
    pub(crate) legend: Option<String>,
    #[serde(default)]
    pub(crate) style: Option<crate::drawer::plot_item::Style>,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub enum ShowOn {
    #[default]
    Both,
    Time,
    Freq,
}
