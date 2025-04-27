use egui::Context;
use lle::{num_complex::Complex64, num_traits::FromPrimitive};
use plot_item::PlotItem;
use plot_kind::PlotKind;
use static_assertions::assert_impl_all;
use std::fmt::Debug;

mod auto_bound;
pub use auto_bound::SmartPlot;

mod colormap;
pub(crate) use colormap::{ColorMapDrawer, DrawMat};

mod history;
pub use history::History;

mod processor;
pub use processor::{FftSource, Process};

pub mod chart;

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

pub mod plot_item;
pub mod plot_kind;
