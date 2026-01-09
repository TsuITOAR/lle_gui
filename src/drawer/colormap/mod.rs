use std::num::NonZeroUsize;

use ui_traits::ControllerUI;

use crate::drawer::PlotItem;

use super::{chart::LleChart, FftSource, History, Process, SmartPlot};

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
        if let Some(ref mut drawer) = self.drawer {
            let mut v = drawer.max_log().map(|x| x.get()).unwrap_or_default();
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
                drawer.set_max_log(new.unwrap());
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
                        let fetch = running || chart0.drawer.is_none();
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
                                        100,
                                        render_state,
                                    )
                                })
                            }
                        };
                        if fetch {
                            r.fetch(history_data, &mut chart0.proc, chunk_size);
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
    ) -> (Option<egui_plot::PlotBounds>, egui_plot::PlotPoints<'static>) {
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
