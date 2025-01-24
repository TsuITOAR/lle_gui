use ui_traits::ControllerUI;

use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlayControl {
    pub(crate) reset: bool,
    pub(crate) destruct: bool,
    pub(crate) step: bool,
    pub(crate) refresh: bool,
}

impl<P, S, V, T> GenApp<P, S, V, T>
where
    P: Default + Clone + Controller<S> + serde::Serialize + for<'a> serde::Deserialize<'a>,
    S: Simulator,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    T: ScoutingTarget<P, S> + Default + Clone,
    Views<V>: Default + for<'a> Visualizer<<S as SharedState<'a>>::SharedState> + Clone,
{
    pub(crate) fn show_toasts(&self, ctx: &egui::Context) {
        TOASTS.lock().show(ctx);
    }

    pub(crate) fn start_profiler(&self) {
        puffin_egui::puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
        puffin_egui::puffin::profile_function!();
    }

    pub(crate) fn check_initialization(&mut self, ctx: &egui::Context) {
        let Self {
            is_init,
            core,
            running,
            ..
        } = self;
        if !*is_init {
            let Core {
                dim,
                controller,
                simulator,
                random,
            } = core;
            *running = false;
            *is_init = egui::Window::new("Welcome to LLE Simulator")
                .show(ctx, |ui| {
                    controller.show_in_start_window(dim, ui);

                    ui.centered_and_justified(|ui| {
                        ui.button(egui::RichText::new("Click to start simulator").heading())
                            .clicked()
                    })
                    .inner
                })
                .unwrap()
                .inner
                .unwrap_or(false);
            if *is_init {
                *simulator = controller.construct_engine(*dim);
                simulator.add_rand(random);
            }
        }
    }

    pub(crate) fn control_panel(&mut self, ctx: &egui::Context) -> PlayControl {
        let Self {
            core,
            running,
            views,
            show_dispersion,
            file_state,
            file_checkpoints,
            check_points,
            profiler,
            slider_len,
            scout,
            add_rand,
            ..
        } = self;
        let PlayControl {
            mut reset,
            mut destruct,
            mut step,
            mut refresh,
        } = PlayControl::default();

        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            puffin_egui::puffin::profile_scope!("control panel");

            // cause display error of super and subscript in easy_mark
            //ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

            attractive_head(
                "Simulation parameters control",
                ui.visuals().strong_text_color(),
            )
            .ui(ui);

            let slider_len = slider_len.get_or_insert_with(|| ui.spacing().slider_width);
            if slider_len.is_sign_positive() {
                ui.spacing_mut().slider_width = *slider_len;
            }

            core.controller.show_in_control_panel(ui);

            ui.separator();

            let button_text = if *running { "⏸" } else { "⏵" };
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(attractive_button(
                        button_text,
                        Some(ui.visuals().error_fg_color),
                    ))
                    .highlight()
                    .on_hover_text("Start/Pause")
                    .clicked()
                {
                    *running = !*running;
                };
                let step_button =
                    attractive_button("⏩", None).sense(egui::Sense::click_and_drag());
                step = step_button
                    .ui(ui)
                    .on_hover_text("Step")
                    .is_pointer_button_down_on();
                refresh = attractive_button("🔄", None)
                    .ui(ui)
                    .on_hover_text("Refresh the state to 0")
                    .clicked();
                reset = attractive_button("⏹", None)
                    .ui(ui)
                    .on_hover_text("Reset model")
                    .clicked();
                destruct = attractive_button("⏏", None)
                    .ui(ui)
                    .on_hover_text("Return to start window\nYou can set model dimension there")
                    .clicked();
            });

            // end of basic control
            ui.separator();

            attractive_head(
                "Advanced simulation control",
                ui.visuals().strong_text_color(),
            )
            .ui(ui);

            core.random.show(ui, add_rand);

            scout.show(core, ui);

            // advanced simulation control
            ui.separator();

            attractive_head("Visualization control", ui.visuals().strong_text_color()).ui(ui);

            views.show_controller(ui);

            show_dispersion.show_controller(ui);

            // visualize strategy
            ui.separator();

            attractive_head("Save/Load model", ui.visuals().strong_text_color()).ui(ui);

            if let Some(true) = file_state.show_save_load(ui, core).notify_global() {
                views.adjust_to_state(core.simulator.states());
            }

            ui.separator();

            attractive_head("Checkpoints", ui.visuals().strong_text_color()).ui(ui);

            if check_points.show(ui, core) {
                views.adjust_to_state(core.simulator.states());
            }
            file_checkpoints
                .show_save_load(ui, check_points)
                .notify_global();

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Slider length");
                ui.add(DragValue::new(slider_len));
            });

            // information display
            ui.separator();

            egui::warn_if_debug_build(ui);

            ui.hyperlink_to("GitHub repository", "https://github.com/TsuITOAR/lle_gui");
            ui.separator();

            views.show_fps(ui);

            crate::util::show_profiler(profiler, ui);
        });
        PlayControl {
            reset,
            destruct,
            step,
            refresh,
        }
    }

    // if return true, require repaint and visualize refresh
    pub(crate) fn run_simulation(&mut self, play_control: PlayControl) -> bool {
        let PlayControl {
            reset,
            destruct,
            step,
            refresh,
        } = play_control;

        let Self {
            core,
            is_init,
            running,
            views,
            scout,
            add_rand,
            file_state,
            file_checkpoints,
            ..
        } = self;

        if reset {
            core.reset();
            *running = false;
            *views = Default::default();
            *file_state = FileManager::default_state();
            *file_checkpoints = FileManager::default_check_points();
            return false;
        }
        if destruct {
            *is_init = false;
            *views = Default::default();
            return false;
        }
        if refresh {
            core.simulator = core
                .controller
                .construct_engine_random_init(core.dim, &mut core.random);
            return true;
        }
        if *running || step {
            core.sync_paras();
            scout.sync_paras(core);
            scout.tick(core);
            if *add_rand {
                puffin_egui::puffin::profile_scope!("add random");
                core.add_random();
            }

            let Core {
                controller,
                simulator,
                ..
            } = core;

            {
                puffin_egui::puffin::profile_scope!("calculate");
                simulator.run(controller.steps());
                scout.poll_scouters(controller.steps(), *add_rand);
            }
            return true;
        }
        false
    }

    pub(crate) fn update_views(
        &mut self,
        ctx: &egui::Context,
        visual_refresh: bool,
        running: bool,
    ) {
        let Self {
            core,
            views,
            show_dispersion,
            scout,
            #[cfg(feature = "gpu")]
            render_state,
            ..
        } = self;

        scout.push_to_views(views, ShowOn::Both, running);

        show_dispersion::add_dispersion_curve(show_dispersion, core, views);

        views.plot(
            core.simulator.states(),
            ctx,
            visual_refresh,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }
}
