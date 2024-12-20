mod core;
mod storage;

pub use core::Core;
pub use storage::CoreStorage;

use egui::DragValue;
use storage::GenAppStorage;

use crate::{
    checkpoint,
    controller::{dispersion_line, Controller, SharedState, Simulator},
    file::{self, FileManager},
    notify::{ResultExt, TOASTS},
    scouting::{BasicScoutingTarget, Scouter, ScoutingTarget},
    views::{ShowOn, State, Views, Visualize},
};
pub struct GenApp<P, S, V, T = BasicScoutingTarget>
where
    P: Controller<S>,
    S: Simulator,
    T: ScoutingTarget<P, S>,
{
    core: Core<P, S>,
    scout: Scouter<P, S, T>,
    is_init: bool,
    slider_len: Option<f32>,
    views: Views<V>,
    running: bool,
    profiler: bool,
    add_rand: bool,
    show_disper: (bool, f64), //show, scale
    check_points: checkpoint::CheckPoints<CoreStorage<P, S>>,
    file_state: file::FileManager,
    file_checkpoints: file::FileManager,
    #[cfg(feature = "gpu")]
    render_state: eframe::egui_wgpu::RenderState,
}

const APP_NAME: &str = "LLE Simulator";

impl<P, S, V, T> GenApp<P, S, V, T>
where
    P: Default + Controller<S> + for<'a> serde::Deserialize<'a> + Clone,
    S: Simulator,
    S::OwnedState: Clone,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    Views<V>: Default
        + for<'a> serde::Deserialize<'a>
        + for<'a> Visualize<<S as SharedState<'a>>::SharedState>,
    T: ScoutingTarget<P, S> + for<'a> serde::Deserialize<'a> + Default,
{
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let c: GenAppStorage<P, S, V, T> = cc.storage.map_or_else(Default::default, |e| {
            eframe::get_value(e, APP_NAME).unwrap_or_default()
        });

        GenApp {
            core: c.core.into(),
            scout: c.scout,
            is_init: c.is_init,
            slider_len: c.slider_len,
            views: c.views,
            running: c.running,
            profiler: c.profiler,
            add_rand: c.add_rand,
            file_state: c.file_state,
            file_checkpoints: c.file_checkpoints,
            show_disper: c.show_disper,
            check_points: c.check_points.clone(),
            #[cfg(feature = "gpu")]
            render_state: cc.wgpu_render_state.clone().unwrap(),
        }
    }
}

impl<P, S, V, T> eframe::App for GenApp<P, S, V, T>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    T: ScoutingTarget<P, S> + serde::Serialize + for<'a> serde::Deserialize<'a> + Default + Clone,
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
            scout,
            is_init,
            slider_len,
            views,
            running,
            profiler,
            add_rand,
            file_state,
            file_checkpoints,
            show_disper,
            check_points,
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
            ui.horizontal(|ui| {
                ui.checkbox(&mut show_disper.0, "Show dispersion")
                    .on_hover_text("Show dispersion");
                if show_disper.0 {
                    ui.add(DragValue::new(&mut show_disper.1));
                }
            });

            scout.show(core, ui);

            if show_disper.0 {
                let disper = core.controller.dispersion();
                let points = dispersion_line(disper, core.dim, show_disper.1);
                views.push_elements(points, ShowOn::Freq);
            }

            views.toggle_record_his(ui, core.simulator.states());

            ui.separator();
            views.config(ui);

            ui.separator();
            egui::warn_if_debug_build(ui);
            if let Some(true) = file_state.show_save_load(ui, core).notify_global() {
                views.adjust_to_state(core.simulator.states());
            }

            ui.separator();

            if check_points.show(ui, core) {
                views.adjust_to_state(core.simulator.states());
            }
            file_checkpoints
                .show_save_load(ui, check_points)
                .notify_global();

            ui.separator();
            views.show_fps(ui);

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Slider length");
                ui.add(DragValue::new(slider_len));
            });

            crate::util::show_profiler(profiler, ui);
        });

        if reset {
            core.reset();
            *views = Default::default();
            *file_state = FileManager::default_state();
            *file_checkpoints = FileManager::default_check_points();
            return;
        }
        if destruct {
            *is_init = false;
            *views = Default::default();
            return;
        }
        if *running || step {
            core.sync_paras();
            scout.sync_paras(core);
            scout.tick(core);
            if *add_rand {
                puffin::profile_scope!("add random");
                core.add_random();
            }

            let Core {
                dim: _,
                controller,
                simulator,
                ..
            } = core;

            {
                puffin::profile_scope!("calculate");
                simulator.run(controller.steps());
                scout.poll_scouters(controller.steps(), *add_rand);
            }

            views.record(simulator.states());
            ctx.request_repaint()
        }
        scout.push_to_views(views, ShowOn::Both);
        views.plot(
            core.simulator.states(),
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
            scout: self.scout.clone_for_save(),
            is_init: self.is_init,
            slider_len: self.slider_len,
            views: self.views.clone(),
            running: self.running,
            profiler: self.profiler,
            add_rand: self.add_rand,
            check_points: self.check_points.clone(),
            show_disper: self.show_disper,
            file_state: self.file_state.clone_for_save(),
            file_checkpoints: self.file_checkpoints.clone_for_save(),
        };
        eframe::set_value(storage, APP_NAME, &state);
    }
}
