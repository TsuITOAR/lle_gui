#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(hasher_prefixfree_extras)]
#![feature(type_alias_impl_trait)]
mod config;
mod controller;
mod core;
mod drawer;
mod easy_mark;
mod file;
mod notify;
mod property;
mod random;
mod util;
mod views;

pub use notify::*;
pub use util::*;
/*
mod test_app;
pub use test_app::TestApp;
*/

use controller::{Controller, SharedState, Simulator};
use core::{Core, CoreStorage};
use egui::DragValue;

use views::{Views, Visualize};

pub const FONT: &str = "Arial";

pub type App = controller::App;

pub struct GenApp<P, S, V> {
    core: Core<P, S>,
    is_init: bool,
    slider_len: Option<f32>,
    views: Views<V>,
    running: bool,
    profiler: bool,
    add_rand: bool,
    file: file::File,
    #[cfg(feature = "gpu")]
    render_state: eframe::egui_wgpu::RenderState,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(bound(
    serialize = "CoreStorage<P, S>: serde::Serialize, Views<V>: serde::Serialize",
    deserialize = "CoreStorage<P, S>: for<'a> serde::Deserialize<'a>, Views<V>: for<'a> serde::Deserialize<'a> + Default"
))]
pub struct GenAppStorage<P, S, V>
where
    S: Simulator,
{
    core: CoreStorage<P, S>,
    is_init: bool,
    slider_len: Option<f32>,
    #[serde(default)]
    views: Views<V>,
    #[serde(skip)]
    running: bool,
    #[serde(skip)]
    profiler: bool,
    add_rand: bool,
    file: file::File,
}

const APP_NAME: &str = "LLE Simulator";

impl<P, S, V> GenApp<P, S, V>
where
    P: Default + Controller<S> + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    Views<V>: Default
        + for<'a> serde::Deserialize<'a>
        + for<'a> Visualize<<S as SharedState<'a>>::SharedState>,
{
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let c: GenAppStorage<P, S, V> = cc.storage.map_or_else(Default::default, |e| {
            eframe::get_value(e, APP_NAME).unwrap_or_default()
        });

        GenApp {
            core: c.core.into(),
            is_init: c.is_init,
            slider_len: c.slider_len,
            views: c.views,
            running: c.running,
            profiler: c.profiler,
            add_rand: c.add_rand,
            file: c.file,
            #[cfg(feature = "gpu")]
            render_state: cc.wgpu_render_state.clone().unwrap(),
        }
    }
}

impl<P, S, V> eframe::App for GenApp<P, S, V>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    Views<V>: Default
        + for<'a> Visualize<<S as SharedState<'a>>::SharedState>
        + serde::Serialize
        + for<'a> serde::Deserialize<'a>
        + Clone,
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TOASTS.lock().show(ctx);
        puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
        puffin::profile_function!();
        let Self {
            core,
            is_init,
            slider_len,
            views,
            running,
            profiler,
            add_rand,
            file,
            #[cfg(feature = "gpu")]
            render_state,
        } = self;

        if !*is_init {
            let Core {
                dim,
                controller,
                simulator,
                random,
            } = core;
            *running = false;
            *is_init = egui::Window::new("Set simulation parameters")
                .show(ctx, |ui| {
                    controller.show_in_start_window(dim, ui);
                    ui.centered_and_justified(|ui| ui.button("✅").clicked())
                        .inner
                })
                .unwrap()
                .inner
                .unwrap_or(false);
            if !*is_init {
                return;
            } else {
                *simulator = controller.construct_engine(*dim);
                simulator.add_rand(random);
            }
        }

        core.controller.sync_paras(&mut core.simulator);

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

            core.controller.show_in_control_panel(ui);

            core.random.show(ui, add_rand);

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

            views.toggle_record_his(ui, core.simulator.states());

            ui.separator();
            views.config(ui);
            ui.separator();
            views.show_fps(ui);

            ui.separator();
            egui::warn_if_debug_build(ui);
            match file.show(ui, core) {
                Ok(true) => {
                    //*views = Default::default();
                    views.clear_his();
                    views.record(core.simulator.states());
                }
                Err(e) => {
                    TOASTS.lock().error(e.to_string());
                }
                _ => {}
            }

            ui.separator();
            crate::show_profiler(profiler, ui);
        });

        let Core {
            dim: _,
            controller,
            simulator,
            random,
        } = core;

        if reset {
            core.reset();
            return;
        }
        if destruct {
            *is_init = false;
            *views = Default::default();
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
            views.record(simulator.states());
            ctx.request_repaint()
        }
        views.plot(
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
            core: (&self.core).into(),
            is_init: self.is_init,
            slider_len: self.slider_len,
            views: self.views.clone(),
            running: self.running,
            profiler: self.profiler,
            add_rand: self.add_rand,
            file: self.file.clone_for_save(),
        };
        eframe::set_value(storage, APP_NAME, &state);
    }
}

impl<P, S, V> GenApp<P, S, V>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    Views<V>: Default
        + for<'a> Visualize<<S as SharedState<'a>>::SharedState>
        + serde::Serialize
        + for<'a> serde::Deserialize<'a>
        + Clone,
{
}

impl<P, S, V> Default for GenAppStorage<P, S, V>
where
    P: Default + Controller<S>,
    S: Simulator,
    Views<V>: Default,
{
    fn default() -> Self {
        Self {
            core: Default::default(),
            is_init: false,
            slider_len: None,
            views: Default::default(),
            running: false,
            profiler: false,
            add_rand: false,
            file: Default::default(),
        }
    }
}
