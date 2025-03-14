use egui::Context;
use lle::{
    num_complex::Complex64,
    num_traits::{Float, FromPrimitive},
};
use static_assertions::assert_impl_all;
use std::{fmt::Debug, ops::RangeInclusive};

mod history;
pub use history::History;

mod process;
pub use process::{FftSource, Process};

pub mod chart;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "plotters")]
pub mod plotters;

use self::chart::LleChart;

pub(crate) fn default_r_chart<S: FftSource>(index: usize) -> LleChart<S> {
    LleChart {
        name: format! {"real domain {index}"},
        kind: PlotKind::Line,
        proc: Default::default(),
        smart_bound: Some(Default::default()),
        show_history: false,
        drawer: None,
        additional: None,
    }
}

pub(crate) fn default_f_chart<S: FftSource>(index: usize) -> LleChart<S> {
    LleChart {
        name: format! {"freq domain {index}"},
        kind: PlotKind::Line,
        proc: Process::new_freq_domain(),
        smart_bound: Some(Default::default()),
        show_history: false,
        drawer: None,
        additional: None,
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "S:FftSource+serde::Serialize",
    deserialize = "S:FftSource+for<'a> serde::Deserialize<'a>"
))]
pub struct ViewField<S: FftSource = Vec<Complex64>> {
    #[serde(default)]
    pub(crate) r_chart: Option<LleChart<S>>,
    #[serde(default)]
    pub(crate) f_chart: Option<LleChart<S>>,
    #[serde(skip)]
    pub(crate) history: History<S>,
    index: usize,
}

assert_impl_all!(ViewField: crate::views::Visualizer<&'static Vec<Complex64>>);
assert_impl_all!(ViewField<crate::controller::gencprt::state::State>: crate::views::Visualizer<&'static crate::controller::gencprt::state::State>);

impl<S: FftSource> ViewField<S> {
    pub(crate) fn new(index: usize) -> Self {
        Self {
            r_chart: Some(default_r_chart(index)),
            f_chart: None,
            history: History::Inactive,
            index,
        }
    }
}

impl<S: FftSource> ViewField<S> {
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
        data: &S,
        ctx: &Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) where
        S::FftProcessor: Sync,
    {
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
                                let (a, b): (Vec<_>, Vec<_>) = e
                                    .points()
                                    .chunks_exact(2)
                                    .map(|x| ([x[0].x, x[0].y], [x[1].x, x[1].y]))
                                    .unzip();
                                plot_ui.points(
                                    egui_plot::Points::new(a)
                                        .name(format!("{d}1"))
                                        .radius(style.width()),
                                );
                                plot_ui.points(
                                    egui_plot::Points::new(b)
                                        .name(format!("{d}2"))
                                        .radius(style.width()),
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
