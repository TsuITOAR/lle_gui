use crate::{
    controller::{Controller, SharedState, Simulator},
    views::{PlotElement, State, Views, Visualizer},
};

pub trait Debugger<S> {
    fn visualize(&mut self, source: S) -> Vec<PlotElement>;
    fn on(&self) -> crate::views::ShowOn {
        crate::views::ShowOn::Freq
    }
    fn legend(&self) -> Vec<String>;
    fn show(&mut self, ui: &mut egui::Ui);
    fn active_debugger(ui: &mut egui::Ui, debugger: &mut Option<Self>)
    where
        Self: Default,
    {
        crate::util::show_option(ui, debugger, "Debugger");
        if let Some(debugger) = debugger {
            debugger.show(ui);
        }
    }
}

impl<S> Debugger<S> for () {
    fn visualize(&mut self, _source: S) -> Vec<PlotElement> {
        vec![]
    }
    fn legend(&self) -> Vec<String> {
        vec![]
    }
    fn show(&mut self, _ui: &mut egui::Ui) {
        // No-op
    }
    fn active_debugger(_ui: &mut egui::Ui, _debugger: &mut Option<Self>)
    where
        Self: Default,
    {
        // No-op
    }
}

pub(crate) fn show_debugger<S, D: Debugger<S> + Default>(
    ui: &mut egui::Ui,
    debugger: &mut Option<D>,
) {
    D::active_debugger(ui, debugger);
}

pub(crate) fn add_debugger<C, S, V, D>(
    core: &super::Core<C, S>,
    views: &mut Views<V>,
    debugger: &mut Option<D>,
) where
    C: Controller<S>,
    S: Simulator,
    Views<V>: for<'a> Visualizer<<S as SharedState<'a>>::SharedState>,
    for<'a> <S as SharedState<'a>>::SharedState: State<OwnedState = S::OwnedState>,
    D: for<'a> Debugger<<S as SharedState<'a>>::SharedState> + Default,
{
    if let Some(debugger) = debugger {
        let state = core.simulator.states();
        let visuals = debugger.visualize(state);
        let on = debugger.on();
        for v in visuals {
            views.push_elements(v, on);
        }
    }
}
