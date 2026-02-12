use std::num::NonZeroUsize;

use ui_traits::ControllerUI;

use crate::drawer::PlotItem;

use super::{FftSource, History, Process, SmartPlot, chart::LleChart};
use lle::{
    FftSource as LleFftSource,
    num_complex::{Complex64, ComplexFloat},
};

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "plotters")]
pub mod plotters;

#[cfg(not(feature = "gpu"))]
pub(crate) type ColorMapDrawer = ColorMapVisualizer;

#[cfg(feature = "gpu")]
pub(crate) type ColorMapDrawer = gpu::Drawer;

pub(crate) trait DrawMat {
    fn draw_mat_on_ui(&mut self, len: usize, ui: &mut egui::Ui) -> Result<(), eframe::Error>;
    fn fetch<S: FftSource>(&mut self, data: &[S], proc: &mut Process<S>, len: usize)
    where
        S::FftProcessor: Sync;
    //fn update(&mut self, data: &[Complex64], proc: &mut Process, len: usize);
    fn max_log(&self) -> Option<NonZeroUsize>;
    fn set_max_log(&mut self, len: NonZeroUsize);
    fn set_align_x_axis(&mut self, _align: impl Into<Option<(f32, f32)>>) {}
    fn set_y_label(&mut self, _label: Option<String>) {}
    fn set_y_tick_shift(&mut self, _shift: i32) {}
    fn fetch_rf_fft_gpu<S: FftSource>(
        &mut self,
        _history_data: &[S],
        _proc: &mut Process<S>,
        _chunk_size: usize,
        _global_norm: bool,
    ) -> bool
    where
        S::FftProcessor: Sync,
    {
        false
    }
    fn sync_labels(&mut self, view: &crate::drawer::HistoryView) {
        let (label, shift) = match view {
            crate::drawer::HistoryView::Raw => ("Record index", 0),
            crate::drawer::HistoryView::RfFft { .. } => {
                let half = self.max_log().map(|x| x.get()).unwrap_or(0) as i32 / 2;
                ("RF index", -half)
            }
        };
        self.set_y_label(Some(label.to_string()));
        self.set_y_tick_shift(shift);
    }
    fn set_matrix(&mut self, width: usize, height: usize, data: &[f32], z_range: Option<[f32; 2]>);
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[derive(Default)]
pub(crate) enum HistoryView {
    #[default]
    Raw,
    RfFft {
        #[serde(skip)]
        #[serde(default)]
        fft_cache: Vec<crate::drawer::processor::FftProcess<Vec<Complex64>>>,
        buffer: Vec<f32>,
    },
}


impl PartialEq for HistoryView {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (HistoryView::Raw, HistoryView::Raw)
                | (HistoryView::RfFft { .. }, HistoryView::RfFft { .. })
        )
    }
}

impl HistoryView {
    fn ensure_rf(&mut self) {
        if !matches!(self, HistoryView::RfFft { .. }) {
            *self = HistoryView::RfFft {
                fft_cache: Vec::new(),
                buffer: Vec::new(),
            };
        }
    }

    fn rf_cache_mut(
        &mut self,
    ) -> Option<(
        &mut Vec<crate::drawer::processor::FftProcess<Vec<Complex64>>>,
        &mut Vec<f32>,
    )> {
        match self {
            HistoryView::RfFft {
                fft_cache: cache,
                buffer: buf,
            } => Some((cache, buf)),
            _ => None,
        }
    }
}

impl<S: FftSource> LleChart<S> {
    pub(crate) fn adjust_to_state(&mut self, data: &S) {
        if self
            .proc
            .core
            .fft
            .as_ref()
            .is_some_and(|x| x.target_len() != Some(data.fft_len()))
        {
            self.proc.core.fft = Some(crate::drawer::processor::FftProcess::default());
        }
    }

    pub(crate) fn control_ui_history(&mut self, ui: &mut egui::Ui, history: &History<S>) {
        let r=ui.add_enabled_ui(history.is_active(), |ui| {
            ui.toggle_value(&mut self.show_history, "History").on_disabled_hover_text(
                "Active the \"Record\" button (on the right side panel) to enable the history display",
            )
        });
        if r.inner.changed() && !self.show_history {
            self.drawer = None;
        }

        if self.show_history {
            let mut v = self
                .drawer
                .as_ref()
                .and_then(|d| d.max_log().map(|x| x.get()))
                .unwrap_or_default();
            if ui
                .horizontal(|ui| {
                    ui.label("Record length: ");
                    ui.add(
                        egui::DragValue::new(&mut v)
                            .range(2..=usize::MAX)
                            .update_while_editing(false),
                    )
                })
                .inner
                .changed()
            {
                let new = NonZeroUsize::new(v);
                if let Some(len) = new
                    && let Some(ref mut drawer) = self.drawer
                {
                    drawer.set_max_log(len);
                }
            }
            let mut changed = false;
            if self.proc.core.fft.is_some() {
                ui.horizontal(|ui| {
                    ui.label("History view:");
                    if ui
                        .selectable_value(&mut self.history_view, HistoryView::Raw, "Time")
                        .changed()
                    {
                        self.history_view = HistoryView::Raw;
                        changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut self.history_view,
                            HistoryView::RfFft {
                                fft_cache: Vec::new(),
                                buffer: Vec::new(),
                            },
                            "RF FFT",
                        )
                        .changed()
                    {
                        self.history_view.ensure_rf();
                        changed = true;
                    }
                });
                if let Some(ref mut drawer) = self.drawer
                    && changed
                {
                    drawer.sync_labels(&self.history_view);
                }
                if matches!(self.history_view, HistoryView::RfFft { .. }) {
                    ui.toggle_value(&mut self.rf_fft_global_norm, "Global normalize");
                }
            }
        }
    }

    pub(crate) fn plot_on_new_window(
        chart: &mut Option<Self>,
        data: &S,
        ctx: &egui::Context,
        running: bool,
        history: &History<S>,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) -> Option<()>
    where
        S::FftProcessor: Sync,
    {
        let chart0 = chart.as_mut()?;
        let name = chart0.name.as_str();
        puffin_egui::puffin::profile_scope!("plot", name);
        let mut open = true;
        egui::Window::new(name)
            .open(&mut open)
            .show(ctx, |ui| -> Option<()> {
                ui.horizontal(|ui| {
                    chart0.proc.show_controller(ui);
                    ui.separator();
                    chart0.kind.show_controller(ui);
                    ui.separator();
                    smarter_bound_controller(&mut chart0.smart_bound, ui);
                });
                ui.horizontal(|ui| chart0.control_ui_history(ui, history));

                match (chart0.show_history, history.get_data_size()) {
                    (true, Some((history_data, chunk_size))) => {
                        let created = chart0.drawer.is_none();
                        let fetch = running || created;
                        let r = {
                            #[cfg(not(feature = "gpu"))]
                            {
                                chart0.drawer.get_or_insert_with(ColorMapDrawer::default)
                            }
                            #[cfg(feature = "gpu")]
                            {
                                chart0.drawer.get_or_insert_with(|| {
                                    ColorMapDrawer::new(
                                        &chart0.name,
                                        chunk_size as _,
                                        128,
                                        render_state,
                                    )
                                })
                            }
                        };
                        if created {
                            let shift = match chart0.history_view {
                                HistoryView::Raw => 0,
                                HistoryView::RfFft { .. } => {
                                    let half = r.max_log().map(|x| x.get()).unwrap_or(0) as i32 / 2;
                                    -half
                                }
                            };
                            r.set_y_tick_shift(shift);
                        }
                        if fetch {
                            if chart0.proc.core.fft.is_some()
                                && let Some((cache, buffer)) = chart0.history_view.rf_cache_mut()
                            {
                                if !r.fetch_rf_fft_gpu(
                                    history_data,
                                    &mut chart0.proc,
                                    chunk_size,
                                    chart0.rf_fft_global_norm,
                                ) {
                                    fetch_rf_fft(
                                        r,
                                        history_data,
                                        &mut chart0.proc,
                                        chunk_size,
                                        chart0.rf_fft_global_norm,
                                        cache,
                                        buffer,
                                    );
                                }
                            } else {
                                r.fetch(history_data, &mut chart0.proc, chunk_size);
                            }
                        }
                    }
                    #[cfg(debug_assertions)]
                    (true, None) => unreachable!("history is active but no data"),
                    _ => (),
                };

                let data = chart0.proc.proc(data, running);
                let mut ui = crate::util::allocate_remained_space(ui);
                if chart0.drawer.is_some() {
                    let h = (ui.available_height() - ui.spacing().item_spacing.y) / 2.;
                    let len = data.len();
                    let r = chart0.plot_in(data.into_iter(), &mut ui, running, Some(h));

                    let min = r.transform.position_from_point_x(0.);
                    let max = r.transform.position_from_point_x((len - 1) as f64);

                    ui.separator();
                    let drawer = chart0.drawer.as_mut().unwrap();
                    drawer.set_align_x_axis((min, max));
                    ui.add_space(ui.spacing().item_spacing.y);
                    let (_id, rect) = ui.allocate_space(ui.available_size());
                    let mut cui = ui.new_child(
                        egui::UiBuilder::default()
                            .max_rect(rect)
                            .layout(*ui.layout()),
                    );
                    drawer
                        .draw_mat_on_ui(len, &mut cui)
                        .expect("can't plot colormap");
                } else {
                    chart0.plot_in(data.into_iter(), &mut ui, running, None);
                }

                Some(())
            });
        if !open {
            *chart = None;
        }
        Some(())
    }

    pub(crate) fn plot_in(
        &mut self,
        data: impl ExactSizeIterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
        height: Option<f32>,
    ) -> egui_plot::PlotResponse<()> {
        use crate::drawer::plot_item::Style;
        let (bound, line) = self.convert_data(data, running);
        use ui_traits::DisplayStr;
        let desc = self.proc.core.component.desc();
        let additional = self.additional.take();
        let plot = self.create_plot(ui, height);

        let main = PlotItem {
            data: line,
            desc: Some(desc.to_string()),
            style: Style::default().interleave().main(),
        };
        let kind = self.kind;
        plot.show(ui, |plot_ui| {
            if let Some(bound) = bound {
                plot_ui.set_plot_bounds(bound);
            }
            main.plot(plot_ui, kind);
            for element in additional.into_iter().flatten() {
                PlotItem::from(element).plot(plot_ui, kind);
            }
        })
    }

    pub(crate) fn convert_data(
        &mut self,
        data: impl ExactSizeIterator<Item = f64>,
        running: bool,
    ) -> (
        Option<egui_plot::PlotBounds>,
        egui_plot::PlotPoints<'static>,
    ) {
        let mut min = None;
        let mut max = None;
        let n = data.len();
        let points = data
            .into_iter()
            .inspect(|&x| {
                if x.is_normal() && *min.get_or_insert(x) > x {
                    min = Some(x);
                }
                if x.is_normal() && *max.get_or_insert(x) < x {
                    max = Some(x);
                }
            })
            .enumerate()
            .map(|(x, y)| [x as _, y])
            .collect::<egui_plot::PlotPoints<'static>>();
        let bound = if let (true, Some(smart)) = (running, self.smart_bound.as_mut()) {
            if let (Some(min), Some(max)) = (min, max) {
                let (y1, y2) = smart.update_range(min..=max).into_inner();
                Some(egui_plot::PlotBounds::from_min_max([0., y1], [n as _, y2]))
            } else {
                None
            }
        } else {
            None
        };
        (bound, points)
    }

    fn create_plot(&self, ui: &mut egui::Ui, height: Option<f32>) -> egui_plot::Plot<'_> {
        let mut plot = egui_plot::Plot::new(&self.name)
            .y_axis_min_width(Y_AXIS_MIN_WIDTH)
            .x_axis_position(egui_plot::VPlacement::Top);
        plot = plot.coordinates_formatter(
            egui_plot::Corner::LeftBottom,
            egui_plot::CoordinatesFormatter::default(),
        );
        plot.height(height.unwrap_or(ui.available_height()))
    }
}

pub(crate) const Y_AXIS_MIN_WIDTH: f32 = 40.0;

fn smarter_bound_controller(smart_bound: &mut Option<SmartPlot<f64>>, ui: &mut egui::Ui) {
    crate::util::show_option(ui, smart_bound, "Smart bound");
    #[cfg(debug_assertions)]
    if let Some(smart) = smart_bound.as_mut() {
        ui.collapsing("Status", |ui| {
            ui.label(format!(
                "Plot center,distance: {},{}",
                smart.center.unwrap_or(f64::NAN),
                smart.dis.unwrap_or(f64::NAN),
            ));
            ui.label(format!("Plot scale_radix: {}", smart.scale_radix));
            ui.label(format!(
                "Auto shrink range: {}/{}",
                smart.lazy_count.0, smart.lazy_count.1
            ));
            ui.label(format!(
                "Scale magnify factor: {}/{}",
                smart.adapt.0, smart.adapt.3
            ));
            ui.label(format!(
                "Refresh threshold of scale factor: ({},{})",
                smart.adapt.1, smart.adapt.2
            ));
        });
    }
}

fn fetch_rf_fft<S: FftSource, D: DrawMat>(
    drawer: &mut D,
    history_data: &[S],
    proc: &mut Process<S>,
    chunk_size: usize,
    global_norm: bool,
    cache: &mut Vec<crate::drawer::processor::FftProcess<Vec<Complex64>>>,
    buffer: &mut Vec<f32>,
) where
    S::FftProcessor: Sync,
{
    let max_log = drawer
        .max_log()
        .map(|x| x.get())
        .unwrap_or(history_data.len());
    let mut time_len = history_data.len().min(max_log);
    if time_len < 2 {
        return;
    }
    if time_len % 2 == 1 {
        time_len -= 1;
    }
    if time_len < 2 {
        return;
    }

    let start = history_data.len().saturating_sub(time_len);
    let history_slice = &history_data[start..];

    let mut time_matrix = vec![Complex64::new(0.0, 0.0); time_len * chunk_size];
    for (t, d) in history_slice.iter().enumerate() {
        let row = proc.proc_complex(d, true);
        if row.len() != chunk_size {
            return;
        }
        for (bin, v) in row.into_iter().enumerate() {
            time_matrix[bin * time_len + t] = v;
        }
    }
    buffer.resize(time_len * chunk_size, 0.0);
    let out: &mut [f32] = buffer.as_mut();
    let split_pos = time_len.div_ceil(2);
    let db_scale = proc.core.db_scale;

    use rayon::prelude::*;
    if cache.len() != chunk_size {
        cache.resize_with(chunk_size, Default::default);
    }
    let results: Vec<(usize, Vec<f32>, f32, f32)> = time_matrix
        .par_chunks_exact_mut(time_len)
        .zip(cache.par_iter_mut())
        .enumerate()
        .map(|(bin, (chunk, fft))| {
            let (f, _) = fft.get_fft(time_len);
            chunk.fft_process_forward(f);

            let (pos, neg) = chunk.split_at(split_pos);
            let mut col_min = f32::INFINITY;
            let mut col_max = f32::NEG_INFINITY;
            let mut col = Vec::with_capacity(time_len);
            for c in neg.iter().chain(pos) {
                let mut v = c.abs() as f32;
                if db_scale {
                    v = if v > 0.0 {
                        20.0 * v.log10()
                    } else {
                        f32::NEG_INFINITY
                    };
                }
                if v.is_finite() {
                    col_min = col_min.min(v);
                    col_max = col_max.max(v);
                }
                col.push(v);
            }
            (bin, col, col_min, col_max)
        })
        .collect();

    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    if global_norm {
        for (_, _, col_min, col_max) in &results {
            if col_min.is_finite() && col_max.is_finite() {
                min = min.min(*col_min);
                max = max.max(*col_max);
            }
        }
    }

    for (bin, mut col, col_min, col_max) in results {
        if !global_norm {
            let span = col_max - col_min;
            if span.is_finite() && span > 0.0 {
                for v in &mut col {
                    if v.is_finite() {
                        *v = (*v - col_min) / span;
                    }
                }
            } else {
                col.fill(0.0);
            }
        }
        for (rf_idx, v) in col.into_iter().enumerate() {
            out[rf_idx * chunk_size + bin] = v;
        }
    }

    if global_norm {
        if !min.is_finite() || !max.is_finite() || min >= max {
            min = 0.0;
            max = 1.0;
        }
        drawer.set_matrix(chunk_size, time_len, out, Some([min, max]));
    } else {
        drawer.set_matrix(chunk_size, time_len, out, Some([0.0, 1.0]));
    }
}
