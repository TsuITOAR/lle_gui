use crate::views::{PlotElement, RawPlotData};
use processor::FftSource;

#[cfg(not(feature = "gpu"))]
use super::plotters::ColorMapVisualizer;

use super::{plot_kind::PlotKind, *};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "S: FftSource + serde::Serialize",
    deserialize = "S: FftSource + for <'a> serde::Deserialize<'a>"
))]
pub struct LleChart<S: FftSource = Vec<Complex64>> {
    pub(crate) name: String,
    pub(crate) kind: PlotKind,
    #[serde(default)]
    pub(crate) proc: Process<S>,
    #[serde(default)]
    pub(crate) smart_bound: Option<SmartPlot<f64>>,
    #[serde(skip)]
    pub(crate) show_history: bool,
    #[serde(skip)]
    pub(crate) drawer: Option<ColorMapDrawer>,
    #[serde(skip)]
    pub(crate) additional: Option<Vec<PlotElement>>,
}

impl<S: FftSource> LleChart<S> {
    pub fn push_additional_raw_data(&mut self, plot_data: &RawPlotData<S>, running: bool) {
        let s = self.proc.proc(&plot_data.data, running);
        self.additional.get_or_insert_default().push(PlotElement {
            y: s,
            x: plot_data.x.clone(),
            legend: None,
            style: plot_data.style,
        })
    }
    pub fn push_additional(&mut self, plot: PlotElement) {
        self.additional.get_or_insert_default().push(plot)
    }
    pub fn unset_display_history(&mut self) {
        self.show_history = false;
        self.drawer = None;
    }
}

impl<S: FftSource> Clone for LleChart<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            kind: self.kind,
            proc: self.proc.clone(),
            smart_bound: self.smart_bound.clone(),
            show_history: self.show_history,
            drawer: None,
            additional: None,
        }
    }
}

impl<S: FftSource + Debug> Debug for LleChart<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LleChart")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("proc", &self.proc)
            .field("smart_plot", &self.smart_bound)
            .field("show_history", &self.drawer)
            .field("additional", &self.additional.is_some())
            .finish()
    }
}
