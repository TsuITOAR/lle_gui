use egui::{Context, Response};
use lle::num_traits::{Float, FromPrimitive};
use std::{fmt::Debug, ops::RangeInclusive, time::Instant};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Plot<T> {
    pub(crate) name: String,
    pub(crate) kind: PlotKind,
    pub(crate) smart_plot: bool,
    pub(crate) dis: T,
    pub(crate) center: T,
    pub(crate) lazy_count: (u32, u32),
    pub(crate) adapt: (u32, u32, u32, u32),
    pub(crate) scale_radix: u32,
    #[serde(skip)]
    last_plot: Option<Instant>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum PlotKind {
    Line,
}

impl<T: Debug + Float + PartialOrd + FromPrimitive + Copy> Plot<T> {
    pub fn new(
        name: impl ToString,
        kind: PlotKind,
        scale_radix: u32,
        max_lazy: u32,
        min_refresh_adapt: u32,
        max_refresh_adapt: u32,
        max_adapt_factor: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            kind,
            smart_plot: true,
            dis: T::one(),
            center: T::zero(),
            scale_radix,
            lazy_count: (0, max_lazy),
            adapt: (1, min_refresh_adapt, max_refresh_adapt, max_adapt_factor),
            last_plot: None,
        }
    }
    fn current_bound(&self) -> RangeInclusive<T> {
        let div = T::from_f64(2f64).unwrap();
        (self.center - self.dis / div)..=(self.center + self.dis / div)
    }
    pub fn update_range(&mut self, new: RangeInclusive<T>) -> RangeInclusive<T> {
        assert!(!new.is_empty());
        let cbound = self.current_bound();
        if cbound.contains(new.start())
            && cbound.contains(new.end())
            && self.lazy_count.0 < self.lazy_count.1
        {
            self.lazy_count.0 += 1;
            return cbound;
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
        self.center = center;
        self.dis = dis;
        (center - dis / div)..=(center + dis / div)
    }
}

impl Plot<f64> {
    pub(crate) fn plot_on_new_window(
        &mut self,
        data: impl Iterator<Item = f64>,
        ctx: &Context,
        running: bool,
    ) {
        egui::Window::new(&self.name).show(ctx, |ui| self.plot_in(data, ui, running));
    }
    pub(crate) fn plot_in(
        &mut self,
        data: impl Iterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
    ) -> egui::Response {
        let now = Instant::now();
        let last = self.last_plot.replace(now);
        if let Some(last) = last {
            let past = (now - last).as_secs_f32();
            ui.label(format!("{}Hz ({:.1}ms)", 1. / past, past * 1000.));
        };
        match self.kind {
            PlotKind::Line => self.plot_line(data, ui, running),
        }
    }

    pub(crate) fn plot_line(
        &mut self,
        evol: impl Iterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
    ) -> Response {
        use egui::plot::Plot;
        let mut plot = Plot::new("line");
        plot = plot.coordinates_formatter(
            egui::plot::Corner::LeftBottom,
            egui::plot::CoordinatesFormatter::default(),
        );
        let mut min = None;
        let mut max = None;
        let mut n: i32 = 0;
        let line = egui::plot::Line::new(
            evol.into_iter()
                .inspect(|&x| {
                    if *min.get_or_insert(x) > x {
                        min = Some(x);
                    }
                    if *max.get_or_insert(x) < x {
                        max = Some(x);
                    }
                    n += 1;
                })
                .enumerate()
                .map(|(x, y)| [x as _, y])
                .collect::<egui::plot::PlotPoints>(),
        )
        .name("Real");
        let set_bound = if running && self.smart_plot {
            if let (Some(min), Some(max)) = (min, max) {
                let (y1, y2) = self.update_range(min..=max).into_inner();
                Some(egui::plot::PlotBounds::from_min_max([0., y1], [n as _, y2]))
            } else {
                None
            }
        } else {
            None
        };
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.smart_plot, "Smarter plot");
            #[cfg(debug_assertions)]
            if self.smart_plot {
                ui.collapsing("Status", |ui| {
                    ui.label(format!(
                        "Plot center,distance: {},{}",
                        self.center, self.dis
                    ));
                    ui.label(format!("Plot scale_radix: {}", self.scale_radix));
                    ui.label(format!(
                        "Auto shrink range: {}/{}",
                        self.lazy_count.0, self.lazy_count.1
                    ));
                    ui.label(format!(
                        "Scale magnify factor: {}/{}",
                        self.adapt.0, self.adapt.3
                    ));
                    ui.label(format!(
                        "Refresh threshold of scale factor: ({},{})",
                        self.adapt.1, self.adapt.2
                    ));
                });
            }
        });
        plot.show(ui, |plot_ui| {
            if let Some(bound) = set_bound {
                plot_ui.set_plot_bounds(bound);
            }
            plot_ui.line(line)
        })
        .response
    }
}
