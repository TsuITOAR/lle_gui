use lle::{num_complex::ComplexFloat, num_traits::Zero, rustfft::FftPlanner};

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
            Some(c) => {
                if c.contains(new.start())
                    && c.contains(new.end())
                    && self.lazy_count.0 < self.lazy_count.1
                {
                    self.lazy_count.0 += 1;
                    return c;
                }
            }
            None => (),
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LleChart {
    pub(crate) kind: PlotKind,
    #[serde(default)]
    pub(crate) proc: Process,
    #[serde(default)]
    pub(crate) smart_plot: Option<SmartPlot<f64>>,
    #[serde(skip)]
    pub(crate) last_plot: Option<std::time::Instant>,
}

impl LleChart {
    pub(crate) fn plot_in(
        &mut self,
        data: impl IntoIterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
    ) -> egui::Response {
        let now = Instant::now();
        let last = self.last_plot.replace(now);
        if let Some(last) = last {
            let past = (now - last).as_secs_f32();
            ui.label(format!("{:.0}Hz ({:.1}ms)", 1. / past, past * 1000.));
        };
        match self.kind {
            PlotKind::Line => self.plot_line(data, ui, running),
        }
    }

    pub(crate) fn plot_line(
        &mut self,
        evol: impl IntoIterator<Item = f64>,
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
        let set_bound = if let (true, Some(smart)) = (running, self.smart_plot.as_mut()) {
            if let (Some(min), Some(max)) = (min, max) {
                let (y1, y2) = smart.update_range(min..=max).into_inner();
                Some(egui::plot::PlotBounds::from_min_max([0., y1], [n as _, y2]))
            } else {
                None
            }
        } else {
            None
        };
        ui.horizontal(|ui| {
            crate::checkbox_some(ui, &mut self.smart_plot, "Smarter plot");
            #[cfg(debug_assertions)]
            if let (true, Some(smart)) = (running, self.smart_plot.as_mut()) {
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

type Fft = std::sync::Arc<dyn lle::rustfft::Fft<f64>>;

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Process {
    fft: Option<FftProcess>,
    component: Component,
    db_scale: bool,
}

impl Process {
    pub(crate) fn proc(&mut self, data: &[Complex64]) -> impl Iterator<Item = f64> {
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        if let Some((f, b)) = fft.as_mut().map(|x| x.get_fft(data.len())) {
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b)
        }
        let db_scale = *db_scale;
        component
            .extract(data.into_iter())
            .map(move |x| if db_scale { x.log10() * 20. } else { x })
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct FftProcess {
    #[serde(skip)]
    s: Option<(Fft, Vec<Complex64>)>,
}

impl FftProcess {
    pub(crate) fn get_fft(&mut self, len: usize) -> &mut (Fft, Vec<Complex64>) {
        self.s.get_or_insert_with(|| {
            let f = FftPlanner::new().plan_fft_forward(len);
            let buf = vec![Complex64::zero(); f.get_inplace_scratch_len()];
            (f, buf)
        })
    }
}

impl Clone for FftProcess {
    fn clone(&self) -> Self {
        Self { s: None }
    }
}

impl Debug for FftProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FftProcess")
            .field("s", &"dyn type")
            .finish()
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Component {
    Real,
    Imag,
    #[default]
    Abs,
}

impl Component {
    pub fn extract(&self, i: impl Iterator<Item = Complex64>) -> impl Iterator<Item = f64> {
        match self {
            Component::Real => i.map({ |x| x.re } as fn(Complex64) -> f64),
            Component::Imag => i.map({ |x| x.im } as fn(Complex64) -> f64),
            Component::Abs => i.map({ |x| x.abs() } as fn(Complex64) -> f64),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_label("Component")
            .selected_text(format!("{:?}", self))
            .show_ui(ui, |ui| {
                ui.selectable_value(self, Component::Real, "Real");
                ui.selectable_value(self, Component::Imag, "Imag");
                ui.selectable_value(self, Component::Abs, "Abs");
            });
    }
}
