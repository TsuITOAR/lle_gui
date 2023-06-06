#![warn(clippy::all, rust_2018_idioms)]

mod configer;
mod drawer;
mod easy_mark;
mod property;
use std::{collections::BTreeMap, f64::consts::PI};

use drawer::PlotRange;
use egui::{plot::PlotBounds, DragValue, Response, Ui};
use lle::{num_complex::Complex64, num_traits::zero, Evolver, LinearOp};
use property::Property;
type LleSolver = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<(lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    Box<dyn Fn(Complex64) -> Complex64>,
>;
pub(crate) fn add_random<'a>(
    intensity: f64,
    sigma: f64,
    state: impl Iterator<Item = &'a mut Complex64>,
) {
    use rand::Rng;
    let mut rand = rand::thread_rng();
    state.for_each(|x| {
        *x += (Complex64::i() * rand.gen::<f64>() * 2. * PI).exp()
            * (-(rand.gen::<f64>() / sigma).powi(2) / 2.).exp()
            / ((2. * PI).sqrt() * sigma)
            * intensity
    })
}

fn default_add_random<'a>(state: impl Iterator<Item = &'a mut Complex64>) {
    add_random((2. * PI).sqrt() * 1e5, 1e5, state)
}

fn synchronize_properties(props: &BTreeMap<String, Property<f64>>, engine: &mut LleSolver) {
    engine.linear = (0, -(Complex64::i() * props["alpha"].get_value() + 1.))
        .add((2, -Complex64::i() * props["linear"].get_value() / 2.))
        .into();
    engine.constant = Complex64::from(props["pump"].get_value()).into();
    engine.step_dist = props["step dist"].get_value();
}

fn show_as_drag_value<T: egui::emath::Numeric>(label: &str, value: &mut T, ui: &mut egui::Ui) {
    ui.label(label);
    ui.add(DragValue::new(value));
}

fn show_as_drag_value_with_suffix<T: egui::emath::Numeric>(
    label: &str,
    value: &mut T,
    ui: &mut egui::Ui,
    suffix: String,
) {
    ui.label(label);
    ui.add(DragValue::new(value).suffix(suffix));
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    slider_len: Option<f32>,
    properties: BTreeMap<String, Property<f64>>,
    #[serde(default)]
    dim: usize,
    #[serde(skip)]
    engine: Option<LleSolver>,
    #[serde(skip)]
    plot_range: Option<PlotRange<f64>>,
    #[serde(skip)]
    seed: Option<u32>,
    #[serde(skip)]
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            slider_len: None,
            dim: 0,
            properties: vec![
                Property::new(-5., "alpha").symbol('α'),
                Property::new(3.94, "pump").symbol('F'),
                Property::new(-0.0444, "linear").symbol('β'),
                Property::new_no_range(8., "step dist")
                    .symbol("Δt")
                    .unit(1E-4)
                    .suffix("E-4"),
            ]
            .into_iter()
            .map(|x| (x.label.clone(), x))
            .collect(),
            engine: None,
            plot_range: None,
            seed: None,
            running: false,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn plot_line(
        evol: impl Iterator<Item = f64>,
        ui: &mut Ui,
        plot_range: &mut PlotRange<f64>,
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
        let mut n = 0;
        plot.show(ui, |plot_ui| {
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
            if running {
                if let (Some(min), Some(max)) = (min, max) {
                    let (y1, y2) = plot_range.update(min..=max).into_inner();
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0., y1], [n as _, y2]));
                }
            }

            plot_ui.line(line)
        })
        .response
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            dim,
            slider_len,
            properties,
            engine,
            plot_range,
            seed: _,
            running,
        } = self;
        if engine.is_none() {
            *running = false;
            let build: bool = egui::Window::new("Set simulation parameters")
                .show(ctx, |ui| configer::config(dim, properties.values_mut(), ui))
                .map(|x| x.inner.unwrap_or(false))
                .unwrap_or(true);
            if !build || *dim == 0 {
                return;
            }
        }

        let engine = engine.get_or_insert_with(|| {
            let step_dist = properties["step dist"].value;
            let pump = properties["pump"].value;
            let linear = properties["linear"].value;
            let alpha = properties["alpha"].value;
            let mut init = vec![zero(); *dim];
            default_add_random(init.iter_mut());
            LleSolver::new(
                init.to_vec(),
                step_dist,
                (0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)),
                Box::new(|x: Complex64| Complex64::i() * x.norm_sqr())
                    as Box<dyn Fn(Complex64) -> Complex64>,
                Complex64::from(pump),
            )
        });
        synchronize_properties(properties, engine);
        let plot_range = plot_range.get_or_insert_with(|| PlotRange::new(10, 200, 2, 100));
        /*
        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
             egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });
        */
        let mut reset = false;
        let mut destruct = false;
        let mut step = false;
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Control Panel");

            let slider_len = slider_len.get_or_insert_with(|| ui.spacing().slider_width);
            if slider_len.is_sign_positive() {
                ui.spacing_mut().slider_width = *slider_len;
            }
            for p in properties.values_mut() {
                p.show_in_control_pannel(ui, ctx)
            }

            ui.horizontal(|ui| {
                ui.label("Slider length");
                ui.add(DragValue::new(slider_len));
            });
            let button_text = if *running { "⏸" } else { "⏵" };
            ui.horizontal_wrapped(|ui| {
                if ui.button(button_text).clicked() {
                    *running = !*running;
                };
                step = ui.button("⏩").clicked();
                reset = ui.button("⏹").clicked();
                destruct = ui.button("⏏").clicked();
            });
        });
        if reset {
            let en = self.engine.take();
            *self = Default::default();
            self.engine = en;
            return;
        }
        if destruct {
            self.engine = None;
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("diagram area");
            egui::warn_if_debug_build(ui);
            if *running || step {
                engine.evolve_n(100);
                ctx.request_repaint()
            }
            Self::plot_line(
                engine.state().iter().map(|x| x.re),
                ui,
                plot_range,
                *running || step,
            );
        });
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
