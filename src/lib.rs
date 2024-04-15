#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
mod configer;
mod drawer;
mod easy_mark;
mod property;

use std::{collections::BTreeMap, f64::consts::PI};

use drawer::ViewField;
use egui::DragValue;
use lle::{num_complex::Complex64, num_traits::zero, Evolver, LinearOp, NonLinearOp};
use property::Property;
type LleSolver<NL> = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    NL,
>;

pub const FONT: &str = "Arial";

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

fn synchronize_properties<NL: NonLinearOp<f64>>(
    props: &BTreeMap<String, Property<f64>>,
    engine: &mut LleSolver<NL>,
) {
    puffin::profile_function!();
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
#[serde(default)]
// if we add new fields, give them default values when deserializing old state
pub struct App<NL> {
    slider_len: Option<f32>,
    properties: BTreeMap<String, Property<f64>>,
    dim: usize,
    #[serde(skip)]
    engine: Option<LleSolver<NL>>,
    #[serde(default)]
    view: ViewField,
    #[serde(skip)]
    seed: Option<u32>,
    #[serde(skip)]
    running: bool,
    #[serde(skip)]
    profiler: bool,
}

impl<NL: NonLinearOp<f64>> Default for App<NL> {
    fn default() -> Self {
        Self {
            slider_len: None,
            dim: 128,
            properties: vec![
                Property::new(-5., "alpha").symbol('α'),
                Property::new(3.94, "pump").symbol('F'),
                Property::new(-0.0444, "linear").symbol('β'),
                Property::new_no_slider(8., "step dist")
                    .symbol("Δt")
                    .unit(1E-4)
                    .suffix("E-4"),
            ]
            .into_iter()
            .map(|x| (x.label.clone(), x))
            .collect(),
            engine: None,
            view: Default::default(),
            seed: None,
            running: false,
            profiler: false,
        }
    }
}

impl<NL: Fn(Complex64) -> Complex64> App<NL> {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Disable feathering as it causes artifacts
        cc.egui_ctx.tessellation_options_mut(|tess_options| {
            tess_options.feathering = false;
        });

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        cc.storage.map_or_else(Default::default, |e| {
            eframe::get_value(e, eframe::APP_KEY).unwrap_or_default()
        })

        /* if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default() */
    }
}

#[derive(Clone, Copy, Default)]
pub struct LleNonLin;

impl FnOnce<(Complex64,)> for LleNonLin {
    type Output = Complex64;

    extern "rust-call" fn call_once(self, args: (Complex64,)) -> Self::Output {
        Complex64::i() * args.0.norm_sqr()
    }
}
impl FnMut<(Complex64,)> for LleNonLin {
    extern "rust-call" fn call_mut(&mut self, args: (Complex64,)) -> Self::Output {
        Complex64::i() * args.0.norm_sqr()
    }
}
impl Fn<(Complex64,)> for LleNonLin {
    extern "rust-call" fn call(&self, args: (Complex64,)) -> Self::Output {
        Complex64::i() * args.0.norm_sqr()
    }
}

impl<NL: NonLinearOp<f64> + Default> eframe::App for App<NL> {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
        puffin::profile_function!();
        let Self {
            dim,
            slider_len,
            properties,
            engine,
            view,
            seed: _,
            running,
            profiler,
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
            LleSolver::builder()
                .state(init.to_vec())
                .step_dist(step_dist)
                .linear((0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)))
                .nonlin(NL::default())
                .constant(Complex64::from(pump))
                .build()
        });
        synchronize_properties(properties, engine);

        let mut reset = false;
        let mut destruct = false;
        let mut step = false;
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            puffin::profile_scope!("control panel");
            ui.heading("Control Panel");

            let slider_len = slider_len.get_or_insert_with(|| ui.spacing().slider_width);
            if slider_len.is_sign_positive() {
                ui.spacing_mut().slider_width = *slider_len;
            }
            for p in properties.values_mut() {
                p.show_in_control_panel(ui, ctx)
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
                let step_button = egui::Button::new("⏩").sense(egui::Sense::click_and_drag());
                step = ui.add(step_button).dragged();
                reset = ui.button("⏹").clicked();
                destruct = ui.button("⏏").clicked();
            });

            view.toggle_record_his(ui, engine.state());

            ui.separator();
            view.show_which(ui);
            ui.separator();
            view.show_fps(ui);

            ui.separator();
            show_profiler(profiler, ui);
        });

        if reset {
            let en = self.engine.take();
            *self = Default::default();
            self.engine = en;
            return;
        }
        if destruct {
            self.engine = None;
            self.view = Default::default();
            return;
        }
        if *running || step {
            puffin::profile_scope!("lle");
            engine.evolve_n(100);
            view.log_his(engine.state());
            ctx.request_repaint()
        }
        view.visualize_state(engine.state(), ctx, *running || step);
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

pub(crate) fn toggle_option<T: Default>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut ch = v.is_some();
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = T::default().into();
    } else if !ch {
        *v = None;
    }

    r
}

pub(crate) fn toggle_option_with<T, F>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
    f: F,
) -> egui::Response
where
    F: FnOnce() -> Option<T>,
{
    let mut ch = v.is_some();
    //let r = ui.checkbox(&mut ch, text);
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = f();
    } else if !ch {
        *v = None;
    }

    r
}

fn show_profiler(show: &mut bool, ui: &mut egui::Ui) {
    if ui.toggle_value(show, "profile performance").clicked() {
        puffin::set_scopes_on(*show); // Remember to call this, or puffin will be disabled!
    }
    if *show {
        puffin_egui::profiler_ui(ui)
    }
}
