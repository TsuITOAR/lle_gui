mod core;
mod impls;
mod dispersion;
mod storage;

pub use core::Core;
use dispersion::ShowDispersion;
pub use storage::CoreStorage;

pub mod debugger;
pub use debugger::Debugger;

use egui::{DragValue, Widget};
use storage::GenAppStorage;

use crate::{
    checkpoint,
    controller::{Controller, SharedState, Simulator},
    file::{self, FileManager},
    notify::{ResultExt, TOASTS},
    scouting::{BasicScoutingTarget, Scouter, ScoutingTarget},
    util::{attractive_button, attractive_head},
    views::{ShowOn, State, Views, Visualizer},
};
pub struct GenApp<P, S, V, T = BasicScoutingTarget, D = ()>
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
    show_dispersion: ShowDispersion, //show, scale
    check_points: checkpoint::CheckPoints<CoreStorage<P, S>>,
    file_state: file::FileManager,
    file_checkpoints: file::FileManager,
    #[cfg(feature = "gpu")]
    render_state: eframe::egui_wgpu::RenderState,
    debugger: Option<D>,
}

const APP_NAME: &str = "LLE Simulator";

impl<P, S, V, T, D> GenApp<P, S, V, T, D>
where
    P: Default + Controller<S> + for<'a> serde::Deserialize<'a> + Clone,
    S: Simulator,
    S::OwnedState: Clone,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    Views<V>: Default
        + for<'a> serde::Deserialize<'a>
        + for<'a> Visualizer<<S as SharedState<'a>>::SharedState>,
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
            show_dispersion: c.show_dispersion,
            check_points: c.check_points.clone(),
            #[cfg(feature = "gpu")]
            render_state: cc.wgpu_render_state.clone().unwrap(),
            debugger: None,
        }
    }
}

impl<P, S, V, T, D> eframe::App for GenApp<P, S, V, T, D>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    T: ScoutingTarget<P, S> + serde::Serialize + for<'a> serde::Deserialize<'a> + Default + Clone,
    Views<V>: Default
        + for<'a> Visualizer<<S as SharedState<'a>>::SharedState>
        + serde::Serialize
        + for<'a> serde::Deserialize<'a>
        + Clone,
    D: for<'a> Debugger<<S as SharedState<'a>>::SharedState> + Default,
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_toasts(ctx);
        self.start_profiler();
        self.check_initialization(ctx);
        if !self.is_init {
            return;
        }
        let play_control = self.control_panel(ctx);

        let refresh = self.run_simulation(play_control);

        if refresh {
            ctx.request_repaint();
        }

        self.update_views(ctx, refresh, self.running);
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
            show_dispersion: self.show_dispersion.clone(),
            file_state: self.file_state.clone_for_save(),
            file_checkpoints: self.file_checkpoints.clone_for_save(),
        };
        eframe::set_value(storage, APP_NAME, &state);
    }
}
