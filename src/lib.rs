#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(hasher_prefixfree_extras)]
#![feature(type_alias_impl_trait)]
mod config;
mod controller;
mod drawer;
mod easy_mark;
mod property;
mod random;
mod util;
mod views;

pub use util::*;
/*
mod test_app;
pub use test_app::TestApp;
*/

use controller::{Controller, Core, Simulator};
use egui::DragValue;

use views::{Views, Visualize};

pub const FONT: &str = "Arial";

pub type App = controller::App;

pub struct GenApp<P, S, V> {
    core: Core<P, S>,
    slider_len: Option<f32>,
    view: Views<V>,
    running: bool,
    profiler: bool,
    random: random::RandomNoise,
    add_rand: bool,
    #[cfg(feature = "gpu")]
    render_state: eframe::egui_wgpu::RenderState,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "P: serde::Serialize, Views<V>: serde::Serialize",
    deserialize = "P: for<'a> serde::Deserialize<'a>, Views<V>: for<'a> serde::Deserialize<'a> + Default"
))]
struct GenAppStorage<P, S, V> {
    core: Core<P, S>,
    slider_len: Option<f32>,
    #[serde(default)]
    views: Views<V>,
    #[serde(skip)]
    running: bool,
    #[serde(skip)]
    profiler: bool,
    random: random::RandomNoise,
    add_rand: bool,
}

impl<'a, P: Default + Controller<S>, S: Simulator<'a>, V> Default for GenAppStorage<P, S, V>
where
    Views<V>: Default + Visualize<S::State>,
{
    fn default() -> Self {
        Self {
            core: Core::new(P::default(), 128),
            slider_len: None,
            views: <Views<V> as Default>::default(),
            running: false,
            profiler: false,
            random: random::RandomNoise::default(),
            add_rand: false,
        }
    }
}

const APP_NAME: &str = "LLE Simulator";

impl<P, S, V> GenApp<P, S, V>
where
    for<'a> S: Simulator<'a>,
    for<'a> P: serde::Deserialize<'a> + Default + Controller<S>,
    for<'a> Views<V>: serde::Deserialize<'a> + Default + Visualize<<S as Simulator<'a>>::State>,
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
        let s: GenAppStorage<P, S, V> = cc.storage.map_or_else(Default::default, |e| {
            eframe::get_value(e, APP_NAME).unwrap_or_default()
        });

        Self {
            core: s.core,
            slider_len: s.slider_len,
            view: s.views,
            running: s.running,
            profiler: s.profiler,
            random: s.random,
            add_rand: s.add_rand,
            #[cfg(feature = "gpu")]
            render_state: cc.wgpu_render_state.as_ref().unwrap().clone(),
        }

        /* if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default() */
    }
}

impl<P, S, V> eframe::App for GenApp<P, S, V>
where
    P: Default + Controller<S> + serde::Serialize + Clone,
    for<'a> S: Simulator<'a>,
    for<'a> Views<V>:
        Default + Visualize<<S as controller::Simulator<'a>>::State> + serde::Serialize + Clone,
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
        puffin::profile_function!();
        let Self {
            core,
            slider_len,
            view,
            running,
            profiler,
            random,
            add_rand,
            #[cfg(feature = "gpu")]
            render_state,
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

        let simulator = simulator.get_or_insert_with(|| controller.construct_engine(*dim, random));
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

            random.show(ui, add_rand);

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
                step = ui.add(step_button).is_pointer_button_down_on();
                reset = ui.button("⏹").clicked();
                destruct = ui.button("⏏").clicked();
            });

            view.toggle_record_his(ui, simulator.states());

            ui.separator();
            view.config(ui);
            ui.separator();
            view.show_fps(ui);

            ui.separator();
            egui::warn_if_debug_build(ui);
            crate::show_profiler(profiler, ui);
        });

        if reset {
            let en = core.simulator.take();
            *core = Core {
                dim: *dim,
                controller: Default::default(),
                simulator: en,
            };
            return;
        }
        if destruct {
            core.simulator = None;
            *view = Default::default();
            return;
        }
        if *running || step {
            if *add_rand {
                puffin::profile_scope!("add random");
                simulator.add_rand(random);
            }
            {
                puffin::profile_scope!("calculate");
                simulator.run(controller.steps());
            }
            view.record(simulator.states());
            ctx.request_repaint()
        }
        view.plot(
            simulator.states(),
            ctx,
            *running || step,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let state = GenAppStorage {
            core: self.core.save_copy(),
            slider_len: self.slider_len,
            views: self.view.clone(),
            running: self.running,
            profiler: self.profiler,
            random: self.random.clone(),
            add_rand: self.add_rand,
        };
        eframe::set_value(storage, APP_NAME, &state);
    }
}
