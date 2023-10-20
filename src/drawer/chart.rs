use std::iter::Map;

use lle::{num_complex::ComplexFloat, num_traits::Zero, rustfft::FftPlanner};

use super::{map::ColorMapVisualizer, *};

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LleChart {
    pub(crate) name: String,
    pub(crate) kind: PlotKind,
    #[serde(default)]
    pub(crate) proc: Process,
    #[serde(default)]
    pub(crate) smart_plot: Option<SmartPlot<f64>>,
    #[serde(default, skip)]
    pub(crate) show_history: Option<ColorMapVisualizer>,
}

impl LleChart {
    pub(crate) fn plot_on_new_window(
        s: &mut Option<Self>,
        data: &[Complex64],
        ctx: &Context,
        running: bool,
        his: &Option<(Vec<Complex64>, usize)>,
    ) {
        if let Some(ss) = s {
            let mut open = true;
            egui::Window::new(&ss.name).open(&mut open).show(ctx, |ui| {
                ss.proc.controller(ui);

                crate::toggle_option_with(ui, &mut ss.show_history, "History log", || match his {
                    Some(h) => {
                        let mut s = ColorMapVisualizer::default();
                        s.fetch(&h.0, &mut ss.proc, h.1);
                        Some(s)
                    }
                    None => None,
                });

                let d = ss.proc.proc(data);
                if ss.show_history.is_some() {
                    #[allow(unused_must_use)]
                    ui.columns(2, |columns| {
                        ss.plot_in(d.iter().copied(), &mut columns[0], running);
                        let ss = ss.show_history.as_mut().expect("checked some");
                        if running {
                            ss.push(d);
                        }
                        ss.draw_on_ui(data.len(), &columns[1])
                            .expect("can't plot colormap");
                    });
                    /* ui.vertical(|ui| {
                        ss.plot_in(d.iter().copied(), ui, running);
                        ss.show_history
                            .as_mut()
                            .expect("checked some")
                            .push(d)
                            .draw_on_ui(data.len(), &ui)
                            .expect("can't plot colormap");
                    }); */
                } else {
                    ss.plot_in(d, ui, running);
                }
            });
            if !open {
                *s = None;
            }
        }
    }

    pub(crate) fn plot_in(
        &mut self,
        data: impl IntoIterator<Item = f64>,
        ui: &mut egui::Ui,
        running: bool,
    ) -> egui::Response {
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
        let mut plot = Plot::new(&self.name);
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
                    if x.is_normal() && *min.get_or_insert(x) > x {
                        min = Some(x);
                    }
                    if x.is_normal() && *max.get_or_insert(x) < x {
                        max = Some(x);
                    }
                    n += 1;
                })
                .enumerate()
                .map(|(x, y)| [x as _, y])
                .collect::<egui::plot::PlotPoints>(),
        )
        .name(self.proc.component.desc());
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
            crate::toggle_option(ui, &mut self.smart_plot, "Smarter plot");
            #[cfg(debug_assertions)]
            if let Some(smart) = self.smart_plot.as_mut() {
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
    pub(crate) fft: Option<FftProcess>,
    pub(crate) component: Component,
    pub(crate) db_scale: bool,
}

impl Process {
    pub(crate) fn new_freq_domain() -> Self {
        Self {
            fft: Some(Default::default()),
            db_scale: true,
            ..Default::default()
        }
    }
    pub(crate) fn proc(&mut self, data: &[Complex64]) -> Vec<f64> {
        let Process {
            fft,
            component,
            db_scale,
        } = self;
        let mut data = data.to_owned();
        if let Some((f, b)) = fft.as_mut().map(|x| x.get_fft(data.len())) {
            debug_assert_eq!(b.len(), f.get_inplace_scratch_len());
            f.process_with_scratch(&mut data, b);
            let split_pos = (data.len() + 1) / 2; //for odd situations, need to shift (len+1)/2..len, for evens, len/2..len
            let (pos_freq, neg_freq) = data.split_at_mut(split_pos);
            data = neg_freq.iter().chain(pos_freq.iter()).copied().collect();
        }

        if *db_scale {
            component
                .extract(data.into_iter())
                .map({ |x: f64| x.log10() * 20. } as fn(_) -> _)
                .collect()
        } else {
            component.extract(data.into_iter()).collect()
        }
    }

    pub(crate) fn controller(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            crate::toggle_option(ui, &mut self.fft, "FFT");
            ui.separator();
            self.component.show(ui);
            ui.separator();
            ui.toggle_value(&mut self.db_scale, "dB scale")
        });
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

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    enum_iterator::Sequence,
)]
pub enum Component {
    Real,
    Imag,
    #[default]
    Abs,
    Arg,
}

impl Component {
    pub fn desc(&self) -> &str {
        match self {
            Component::Real => "Real",
            Component::Imag => "Imag",
            Component::Abs => "Abs",
            Component::Arg => "Arg",
        }
    }
    pub fn extract<B: Iterator<Item = Complex64>>(&self, i: B) -> Map<B, fn(Complex64) -> f64> {
        match self {
            Component::Real => i.map({ |x| x.re } as fn(Complex64) -> f64),
            Component::Imag => i.map({ |x| x.im } as fn(Complex64) -> f64),
            Component::Abs => i.map({ |x| x.abs() } as fn(Complex64) -> f64),
            Component::Arg => i.map({ |x| x.arg() } as fn(Complex64) -> f64),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        enum_iterator::all::<Component>().for_each(|s| {
            if ui.selectable_label(self == &s, s.desc()).clicked() {
                *self = s;
            }
        })
    }
}
