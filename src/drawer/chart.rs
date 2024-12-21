use std::{borrow::Borrow, num::NonZeroUsize};

use egui::{DragValue, SelectableLabel};
use egui_plot::{PlotPoints, PlotResponse};

use crate::views::{PlotElement, RawPlotElement};

#[cfg(not(feature = "gpu"))]
use super::plotters::ColorMapVisualizer;

use super::*;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SmartPlot<T> {
    pub(crate) dis: Option<T>,
    pub(crate) center: Option<T>,
    pub(crate) lazy_count: (u32, u32),
    pub(crate) adapt: (u32, u32, u32, u32),
    pub(crate) scale_radix: u32,
}

impl<T> Default for SmartPlot<T> {
    fn default() -> Self {
        Self::new(10, 200, 2, 100, 30)
    }
}

impl<T> SmartPlot<T> {
    pub fn new(
        scale_radix: u32,
        max_lazy: u32,
        min_refresh_adapt: u32,
        max_refresh_adapt: u32,
        max_adapt_factor: u32,
    ) -> Self {
        Self {
            dis: None,
            center: None,
            scale_radix,
            lazy_count: (0, max_lazy),
            adapt: (1, min_refresh_adapt, max_refresh_adapt, max_adapt_factor),
        }
    }
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> SmartPlot<T> {
    fn current_bound(&self) -> Option<RangeInclusive<T>> {
        if let (Some(center), Some(dis)) = (self.center, self.dis) {
            let div = T::from_f64(2f64).unwrap();
            Some((center - dis / div)..=(center + dis / div))
        } else {
            None
        }
    }
    pub fn update_range(&mut self, new: RangeInclusive<T>) -> RangeInclusive<T> {
        assert!(!new.is_empty());
        let cbound = self.current_bound();
        match cbound {
            Some(c)
                if c.contains(new.start())
                    && c.contains(new.end())
                    && self.lazy_count.0 < self.lazy_count.1 =>
            {
                self.lazy_count.0 += 1;
                return c;
            }
            _ => (),
        }

        if self.lazy_count.0 < self.adapt.1 {
            self.adapt.0 = (self.adapt.0 + 1).min(self.adapt.3)
        } else if self.lazy_count.0 > self.adapt.2 {
            self.adapt.0 = self.adapt.0.max(6) - 5;
        }
        self.lazy_count.0 = 0;
        let dis = *new.end() - *new.start();
        let mag = {
            let radix: T = T::from_u32(self.scale_radix).unwrap();
            let order = dis.log(radix).ceil() - T::one();
            radix.powf(order)
        } * T::from_u32(self.adapt.0).unwrap();
        let div = T::from_f64(2f64).unwrap();
        let center = (*new.end() + *new.start()) / div;
        let dis = (((*new.end() - *new.start()) / mag).floor() + T::one()) * mag;
        self.center = center.into();
        self.dis = dis.into();
        (center - dis / div)..=(center + dis / div)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LleChart {
    pub(crate) name: String,
    pub(crate) kind: PlotKind,
    #[serde(default)]
    pub(crate) proc: Process,
    #[serde(default)]
    pub(crate) smart_bound: Option<SmartPlot<f64>>,
    #[serde(skip)]
    pub(crate) show_history: Option<ColorMapDrawer>,
    #[serde(skip)]
    pub(crate) additional: Option<Vec<PlotElement>>,
}

impl LleChart {
    pub fn push_additional_raw<S>(&mut self, plot: &RawPlotElement<S>)
    where
        S: Borrow<[Complex64]>,
    {
        let s = self.proc.proc(plot.data.borrow());
        self.additional.get_or_insert_default().push(PlotElement {
            y: s,
            x: plot.x.clone(),
            style: plot.style,
        })
    }
    pub fn push_additional(&mut self, plot: PlotElement) {
        self.additional.get_or_insert_default().push(plot)
    }
}

impl Clone for LleChart {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            kind: self.kind.clone(),
            proc: self.proc.clone(),
            smart_bound: self.smart_bound.clone(),
            show_history: None,
            additional: None,
        }
    }
}

impl Debug for LleChart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LleChart")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("proc", &self.proc)
            .field("smart_plot", &self.smart_bound)
            .field("show_history", &self.show_history)
            .field("additional", &self.additional.is_some())
            .finish()
    }
}

#[cfg(not(feature = "gpu"))]
type ColorMapDrawer = ColorMapVisualizer;

#[cfg(feature = "gpu")]
type ColorMapDrawer = super::gpu::Drawer;

pub(crate) trait DrawMat {
    fn draw_mat_on_ui(&mut self, len: usize, ui: &mut egui::Ui) -> Result<(), eframe::Error>;
    fn fetch(&mut self, data: &[Complex64], proc: &mut Process, len: usize);
    fn update(&mut self, data: &[Complex64], proc: &mut Process, len: usize);
    fn max_log(&self) -> Option<NonZeroUsize>;
    fn set_max_log(&mut self, len: NonZeroUsize);
    fn set_align_x_axis(&mut self, _align: impl Into<Option<(f32, f32)>>) {}
}

impl LleChart {
    pub(crate) fn adjust_to_state(&mut self, data: &[Complex64]) {
        if self
            .proc
            .fft
            .as_ref()
            .is_some_and(|x| x.target_len() != Some(data.len()))
        {
            self.proc.fft = Some(crate::drawer::process::FftProcess::default());
        }
    }

    pub(crate) fn control_panel_history(
        &mut self,
        ui: &mut egui::Ui,
        his: &Option<History>,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        let mut show_his = self.show_history.is_some();
        if ui
            .add_enabled(
                his.is_some(),
                SelectableLabel::new(self.show_history.is_some(), "History"),
            )
            .clicked()
        {
            show_his = !show_his;
        }
        match self.show_history.as_mut() {
            Some(_) if !show_his => self.show_history = None,
            Some(ss) if show_his => {
                let his = his.as_ref().unwrap();
                if running {
                    ss.update(&his.data, &mut self.proc, his.dim);
                }
                let mut v = ss.max_log().map(|x| x.get()).unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label("Record length: ");
                    ui.add(
                        DragValue::new(&mut v)
                            .range(2..=usize::MAX)
                            .update_while_editing(false),
                    )
                });
                let new = NonZeroUsize::new(v);
                if new != ss.max_log() {
                    ss.set_max_log(new.unwrap());
                    ss.fetch(&his.data, &mut self.proc, his.dim);
                }
            }
            None if show_his => {
                let his = his.as_ref().unwrap();
                self.show_history = Some({
                    #[cfg(not(feature = "gpu"))]
                    let mut t = ColorMapDrawer::default();
                    #[cfg(feature = "gpu")]
                    let mut t = ColorMapDrawer::new(&self.name, his.dim as _, 100, render_state);
                    t.fetch(&his.data, &mut self.proc, his.dim);
                    t
                })
            }
            _ => (),
        }
    }

    pub(crate) fn plot_on_new_window(
        chart: &mut Option<Self>,
        data: &[Complex64],
        ctx: &Context,
        running: bool,
        his: &Option<History>,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        if chart.is_none() {
            return;
        }
        let name = chart
            .as_ref()
            .map(|x| x.name.as_str())
            .unwrap_or("Empty name");
        puffin::profile_scope!("plot", name);
        let mut open = true;
        egui::Window::new(name)
            .open(&mut open)
            .show(ctx, |ui| -> Option<()> {
                let chart = chart.as_mut().expect("checked branch");
                ui.horizontal(|ui| {
                    chart.proc.controller(ui);
                    ui.separator();
                    chart.kind.controller(ui);
                    ui.separator();
                    smarter_bound_controller(&mut chart.smart_bound, ui);
                });
                ui.horizontal(|ui| {
                    chart.control_panel_history(
                        ui,
                        his,
                        running,
                        #[cfg(feature = "gpu")]
                        render_state,
                    )
                });
                let data = chart.proc.proc(data);

                let mut ui = crate::util::allocate_remained_space(ui);
                if chart.show_history.is_some() {
                    let h = (ui.available_height() - ui.spacing().item_spacing.y) / 2.;
                    let len = data.len();
                    let r = chart.plot_in(data.into_iter(), &mut ui, running, Some(h));

                    let min = r.transform.position_from_point_x(0.);
                    let max = r.transform.position_from_point_x((len - 1) as f64);

                    ui.separator();
                    let history = chart.show_history.as_mut().expect("checked brach");

                    history.set_align_x_axis((min, max));
                    ui.add_space(ui.spacing().item_spacing.y);
                    let (_id, rect) = ui.allocate_space(ui.available_size());
                    let mut cui = ui.new_child(
                        egui::UiBuilder::default()
                            .max_rect(rect)
                            .layout(*ui.layout()),
                    );
                    history
                        .draw_mat_on_ui(len, &mut cui)
                        .expect("can't plot colormap");
                } else {
                    chart.plot_in(data.into_iter(), &mut ui, running, None);
                }

                Some(())
            });
        if !open {
            *chart = None;
        }
    }

    pub(crate) fn plot_in(
        &mut self,
        data: impl ExactSizeIterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
        height: Option<f32>,
    ) -> PlotResponse<()> {
        let (bound, line) = self.convert_data(data, running);
        let plot_kind = &self.kind;
        let desc = self.proc.component.desc();
        let additional = self.additional.take();
        let plot = self.plot(ui, height);
        plot_kind.plot(
            plot,
            ui,
            bound,
            std::iter::once(PlotItem {
                data: line,
                desc: Some(desc.to_string()),
                style: Style::Main,
            })
            .chain(
                additional
                    .into_iter()
                    .flatten()
                    .map(|element| match element.x {
                        Some(x) => PlotItem {
                            data: PlotPoints::from_iter(
                                x.into_iter().zip(element.y).map(|(x, y)| [x, y]),
                            ),
                            desc: Some(element.style.to_string()),
                            style: Style::Sub,
                        },
                        None => PlotItem {
                            data: PlotPoints::from_ys_f64(&element.y),
                            desc: Some(element.style.to_string()),
                            style: Style::Sub,
                        },
                    }),
            ),
        )
    }

    pub(crate) fn convert_data(
        &mut self,
        data: impl ExactSizeIterator<Item = f64>,
        running: bool,
    ) -> (Option<egui_plot::PlotBounds>, egui_plot::PlotPoints) {
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
            .collect();
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

    fn plot(&self, ui: &mut egui::Ui, height: Option<f32>) -> egui_plot::Plot<'_> {
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
