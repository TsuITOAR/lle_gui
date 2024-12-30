use egui::Context;
use lle::{
    num_complex::Complex64,
    num_traits::{Float, FromPrimitive},
};
use std::{fmt::Debug, ops::RangeInclusive};

mod history;
pub use history::History;

mod process;
pub use process::Process;

pub mod chart;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "plotters")]
pub mod plotters;

use self::chart::LleChart;

pub(crate) fn default_r_chart(index: usize) -> Option<LleChart> {
    Some(LleChart {
        name: format! {"real domain {index}"},
        kind: PlotKind::Line,
        proc: Default::default(),
        smart_bound: Some(Default::default()),
        show_history: false,
        drawer: None,
        additional: None,
    })
}

pub(crate) fn default_f_chart(index: usize) -> Option<LleChart> {
    Some(LleChart {
        name: format! {"freq domain {index}"},
        kind: PlotKind::Line,
        proc: Process::new_freq_domain(),
        smart_bound: Some(Default::default()),
        show_history: false,
        drawer: None,
        additional: None,
    })
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ViewField {
    #[serde(default)]
    pub(crate) r_chart: Option<LleChart>,
    #[serde(default)]
    pub(crate) f_chart: Option<LleChart>,
    #[serde(skip)]
    pub(crate) history: History,
    index: usize,
}

impl ViewField {
    pub(crate) fn new(index: usize) -> Self {
        Self {
            r_chart: default_r_chart(index),
            f_chart: None,
            history: History::Inactive,
            index,
        }
    }
}

impl ViewField {
    pub(crate) fn toggle_record_his(&mut self, ui: &mut egui::Ui) {
        let index = self.index;
        ui.horizontal(|ui| {
            if self.history.show_controller(index, ui).changed() && !self.history.is_active() {
                for c in [self.r_chart.as_mut(), self.f_chart.as_mut()]
                    .into_iter()
                    .flatten()
                {
                    c.unset_display_history();
                }
            }

            #[cfg(target_arch = "wasm32")]
            if self.history.is_active() {
                crate::util::warn_single_thread(ui);
            }
        });
    }

    pub(crate) fn show_which(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            crate::util::show_option_with(ui, &mut self.r_chart, "real domain", || {
                default_r_chart(self.index)
            });
            crate::util::show_option_with(ui, &mut self.f_chart, "freq domain", || {
                default_f_chart(self.index)
            });
        });
    }
    pub(crate) fn visualize_state(
        &mut self,
        data: &[Complex64],
        ctx: &Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        puffin_egui::puffin::profile_function!();
        if running || matches!(self.history, History::ReadyToRecord) {
            self.history.push(data); // judge whether to record history internally
        }

        LleChart::plot_on_new_window(
            &mut self.r_chart,
            data,
            ctx,
            running,
            &self.history,
            #[cfg(feature = "gpu")]
            render_state,
        );
        LleChart::plot_on_new_window(
            &mut self.f_chart,
            data,
            ctx,
            running,
            &self.history,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }
}

#[derive(
    Debug,
    Clone,
    enum_iterator::Sequence,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum PlotKind {
    Line,
    Points,
}

impl crate::util::DisplayStr for PlotKind {
    fn desc(&self) -> &str {
        match self {
            PlotKind::Line => "Line",
            PlotKind::Points => "Points",
        }
    }
}

impl PlotKind {
    pub(crate) fn plot(
        &self,
        plot: egui_plot::Plot<'_>,
        ui: &mut egui::Ui,
        bound: Option<egui_plot::PlotBounds>,
        elements: impl Iterator<Item = PlotItem>,
    ) -> egui_plot::PlotResponse<()> {
        plot.show(ui, |plot_ui| {
            if let Some(bound) = bound {
                plot_ui.set_plot_bounds(bound);
            }
            match self {
                PlotKind::Line => {
                    elements.for_each(
                        |PlotItem {
                             data: e,
                             desc: d,
                             style,
                         }| {
                            if let Some(d) = d {
                                plot_ui.line(egui_plot::Line::new(e).name(d).width(style.width()));
                            } else {
                                plot_ui.line(egui_plot::Line::new(e).width(style.width()));
                            }
                        },
                    );
                }
                PlotKind::Points => {
                    elements.for_each(
                        |PlotItem {
                             data: e,
                             desc: d,
                             style,
                         }| {
                            if let Some(d) = d {
                                plot_ui.points(
                                    egui_plot::Points::new(e).name(d).radius(style.width()),
                                );
                            } else {
                                plot_ui.points(egui_plot::Points::new(e).radius(style.width()));
                            }
                        },
                    );
                }
            }
        })
    }
}

pub(crate) struct PlotItem {
    data: egui_plot::PlotPoints,
    desc: Option<String>,
    style: Style,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) enum Style {
    Main,
    Sub,
}

impl Style {
    pub(crate) fn width(&self) -> f32 {
        match self {
            Style::Main => 2.0,
            Style::Sub => 1.0,
        }
    }
}
