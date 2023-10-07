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

fn default_r_chart() -> Option<LleChart> {
    Some(LleChart {
        kind: PlotKind::Line,
        proc: Default::default(),
        smart_plot: Some(Default::default()),
        last_plot: None,
    })
}

fn default_f_chart() -> Option<LleChart> {
    todo!()
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ViewField {
    pub(crate) name: String,
    #[serde(default = "default_r_chart")]
    pub(crate) r_chart: Option<LleChart>,
    #[serde(default)]
    pub(crate) f_chart: Option<LleChart>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum PlotKind {
    Line,
}

impl ViewField {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            r_chart: default_r_chart(),
            f_chart: None,
        }
    }

    pub(crate) fn plot_on_new_window(&mut self, data: &[Complex64], ctx: &Context, running: bool) {
        egui::Window::new(&self.name).show(ctx, |ui| {
            if let Some(ref mut r) = self.r_chart {
                let d = r.proc.proc(data);
                r.plot_in(d, ui, running);
            }
            if let Some(ref mut f) = self.f_chart {
                let d = f.proc.proc(data);
                f.plot_in(d, ui, running);
            }
        });
    }
}
