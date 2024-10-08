#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
mod config;
mod controller;
mod drawer;
mod easy_mark;
mod property;

use std::f64::consts::PI;

use controller::{Controller, Core, Simulator};
use drawer::ViewField;
use egui::DragValue;
use lle::{num_complex::Complex64, LinearOp, NonLinearOp};

use crate::controller::Record;

pub const FONT: &str = "Arial";

pub(crate) fn add_random(intensity: f64, sigma: f64, state: &mut [Complex64], seed: Option<u64>) {
    use rand::Rng;
    if let Some(seed) = seed {
        use rand::SeedableRng;
        let mut rand = rand::rngs::StdRng::seed_from_u64(seed);
        state.iter_mut().for_each(|x| {
            *x += (Complex64::i() * rand.gen::<f64>() * 2. * PI).exp()
                * (-(rand.gen::<f64>() / sigma).powi(2) / 2.).exp()
                / ((2. * PI).sqrt() * sigma)
                * intensity
        })
    } else {
        let mut rand = rand::thread_rng();
        state.iter_mut().for_each(|x| {
            *x += (Complex64::i() * rand.gen::<f64>() * 2. * PI).exp()
                * (-(rand.gen::<f64>() / sigma).powi(2) / 2.).exp()
                / ((2. * PI).sqrt() * sigma)
                * intensity
        })
    }
}

fn default_add_random(state: &mut [Complex64]) {
    add_random((2. * PI).sqrt() * 1e5, 1e5, state, None)
}

fn default_add_random_with_seed(state: &mut [Complex64], seed: u64) {
    add_random((2. * PI).sqrt() * 1e5, 1e5, state, Some(seed))
}

fn synchronize_properties<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add((2, -Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.constant = Complex64::from(props.pump.get_value()).into();
    engine.step_dist = props.step_dist.get_value();
}

fn synchronize_properties_no_pump<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add((2, -Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.step_dist = props.step_dist.get_value();
}

//pub type App = GenApp<crate::controller::LleController, crate::controller::LleSolver<lle::SPhaMod>>;

pub type App =
    GenApp<crate::controller::clle::CoupleLleController, crate::controller::clle::CLleSolver>;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "P: serde::Serialize",
    deserialize = "P: for<'a> serde::Deserialize<'a>"
))]
pub struct GenApp<P, S> {
    core: Core<P, S>,
    slider_len: Option<f32>,
    #[serde(default)]
    view: ViewField,
    #[serde(skip)]
    seed: Option<u64>,
    #[serde(skip)]
    running: bool,
    #[serde(skip)]
    profiler: bool,
}

impl<P: Default + Controller<S>, S: Simulator> Default for GenApp<P, S> {
    fn default() -> Self {
        Self {
            core: Core::new(P::default(), 128),
            slider_len: None,
            view: ViewField::default(),
            seed: None,
            running: false,
            profiler: false,
        }
    }
}

impl<P: Controller<S> + Default, S: Simulator> GenApp<P, S>
where
    for<'a> P: serde::Deserialize<'a>,
{
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

impl<P, S> eframe::App for GenApp<P, S>
where
    P: Default + Controller<S> + serde::Serialize,
    S: Simulator<State = [Complex64]>,
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
        puffin::profile_function!();
        let Self {
            core,
            slider_len,
            view,
            seed,
            running,
            profiler,
        } = self;

        let Core {
            dim,
            controller,
            simulator,
        } = core;

        if simulator.is_none() {
            *running = false;
            let build: bool = egui::Window::new("Set simulation parameters")
                .show(ctx, |ui| {
                    controller.show_in_start_window(dim, ui);
                    ui.centered_and_justified(|ui| ui.button("✅").clicked())
                        .inner
                })
                .map(|x| x.inner.unwrap_or(false))
                .unwrap_or(true);
            if !build || *dim == 0 {
                return;
            }
        }

        let simulator = simulator.get_or_insert_with(|| {
            if let Some(s) = seed {
                controller.construct_with_seed(*dim, *s)
            } else {
                controller.construct_engine(*dim)
            }
        });
        controller.sync_paras(simulator);

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

            controller.show_in_control_panel(ui);

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

            view.toggle_record_his(ui, simulator.states().record_first());

            ui.separator();
            view.show_which(ui);
            ui.separator();
            view.show_fps(ui);

            ui.separator();
            egui::warn_if_debug_build(ui);
            crate::show_profiler(profiler, ui);
        });

        if reset {
            let en = core.simulator.take();
            *core = Default::default();
            core.simulator = en;
            return;
        }
        if destruct {
            core.simulator = None;
            self.view = Default::default();
            return;
        }
        if *running || step {
            puffin::profile_scope!("calculate");
            simulator.run(controller.steps());
            view.log_his(simulator.states().record_first());
            ctx.request_repaint()
        }
        view.visualize_state(simulator.states().record_first(), ctx, *running || step);
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
