use egui::{Context, Response};
use lle::{
    num_complex::Complex64,
    num_traits::{Float, FromPrimitive},
};
use std::{fmt::Debug, ops::RangeInclusive};

pub mod chart;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use self::chart::LleChart;

pub(crate) fn default_r_chart() -> Option<LleChart> {
    Some(LleChart {
        name: "real domain".to_string(),
        kind: PlotKind::Line,
        proc: Default::default(),
        smart_plot: Some(Default::default()),
    })
}

pub(crate) fn default_f_chart() -> Option<LleChart> {
    Some(LleChart {
        name: "freq domain".to_string(),
        kind: PlotKind::Line,
        proc: chart::Process::new_freq_domain(),
        smart_plot: Some(Default::default()),
    })
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ViewField {
    #[serde(default = "default_r_chart")]
    pub(crate) r_chart: Option<LleChart>,
    #[serde(default)]
    pub(crate) f_chart: Option<LleChart>,
    #[serde(skip)]
    last_plot: Option<std::time::Instant>,
}

impl Default for ViewField {
    fn default() -> Self {
        Self {
            r_chart: default_r_chart(),
            f_chart: None,
            last_plot: None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum PlotKind {
    Line,
}

impl ViewField {
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
    pub(crate) fn show_which(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            crate::checkbox_with(ui, &mut self.r_chart, "show real domain", default_r_chart);
            crate::checkbox_with(ui, &mut self.f_chart, "show freq domain", default_f_chart);
        });
    }
    pub(crate) fn plot_on_new_windows(&mut self, data: &[Complex64], ctx: &Context, running: bool) {
        if let Some(ref mut r) = self.r_chart {
            r.plot_on_new_window(data, ctx, running)
        }
        if let Some(ref mut f) = self.f_chart {
            f.plot_on_new_window(data, ctx, running)
        }
    }
}
