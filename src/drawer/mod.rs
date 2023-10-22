use egui::{Context, Response};
use lle::{
    num_complex::Complex64,
    num_traits::{Float, FromPrimitive},
};
use std::{fmt::Debug, ops::RangeInclusive};

mod backend;
pub mod chart;
pub mod map;

pub use rand::*;

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
        show_history: None,
    })
}

pub(crate) fn default_f_chart() -> Option<LleChart> {
    Some(LleChart {
        name: "freq domain".to_string(),
        kind: PlotKind::Line,
        proc: chart::Process::new_freq_domain(),
        smart_plot: Some(Default::default()),
        show_history: None,
    })
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ViewField {
    #[serde(default = "default_r_chart")]
    pub(crate) r_chart: Option<LleChart>,
    #[serde(default)]
    pub(crate) f_chart: Option<LleChart>,
    #[serde(skip)]
    pub(crate) history: Option<(Vec<Complex64>, usize)>,
    #[serde(skip)]
    last_plot: Option<Instant>,
}

impl Default for ViewField {
    fn default() -> Self {
        Self {
            r_chart: default_r_chart(),
            f_chart: None,
            last_plot: None,
            history: None,
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
    pub(crate) fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: &[Complex64]) {
        if crate::toggle_option_with(ui, &mut self.history, "Record history", || {
            Some((Vec::from(data), data.len()))
        })
        .clicked()
            && self.history.is_none()
        {
            for c in [self.r_chart.as_mut(), self.f_chart.as_mut()]
                .into_iter()
                .flatten()
            {
                c.show_history = None;
            }
        }
    }

    pub(crate) fn log_his(&mut self, data: &[Complex64]) {
        if let Some((ref mut s, _)) = self.history {
            s.extend_from_slice(data)
        }
    }

    pub(crate) fn show_which(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            crate::toggle_option_with(ui, &mut self.r_chart, "real domain", default_r_chart);
            crate::toggle_option_with(ui, &mut self.f_chart, "freq domain", default_f_chart);
        });
    }
    pub(crate) fn plot_on_new_windows(&mut self, data: &[Complex64], ctx: &Context, running: bool) {
        LleChart::plot_on_new_window(&mut self.r_chart, data, ctx, running, &self.history);
        LleChart::plot_on_new_window(&mut self.f_chart, data, ctx, running, &self.history);
    }
}
