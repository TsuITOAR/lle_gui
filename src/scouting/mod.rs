use std::{fmt::Debug, sync::Arc};

use egui::mutex::Mutex;
use rayon::prelude::*;
use refresh::Refresh;

pub use impls::BasicScoutingTarget;
use ui_traits::ControllerUI;

use crate::{
    app::Core,
    controller::{Controller, SharedState, Simulator, StoreState},
    util::{try_poll, Promise},
    views::{RawPlotData, ShowOn, State, Visualizer},
};

mod impls;
mod refresh;

type Style = crate::drawer::plot_item::Style;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: for<'a> serde::Deserialize<'a>"
))]
pub struct Scouter<C, S, T>
where
    T: ScoutingTarget<C, S>,
    C: Controller<S>,
    S: Simulator,
{
    pub(crate) config: ScouterConfig<T, C, S>,
    #[serde(skip)]
    pub(crate) sub_cores: Option<SubCores<Core<C, S>>>,
    pub(crate) refresh: Refresh,
    #[serde(skip)]
    promise: Option<Promise<Vec<<S as StoreState>::OwnedState>>>,
    #[serde(skip)]
    cache: Option<Vec<<S as StoreState>::OwnedState>>,
}

impl<C, S, T> std::fmt::Debug for Scouter<C, S, T>
where
    T: ScoutingTarget<C, S> + Debug,
    C: Controller<S>,
    S: Simulator,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scouter")
            .field("config", &self.config)
            .field("sub_cores", &self.sub_cores.is_some())
            .field("refresh", &self.refresh)
            .finish()
    }
}

impl<C, S, T> Scouter<C, S, T>
where
    T: ScoutingTarget<C, S>,
    C: Controller<S> + Clone,
    S: Simulator,
{
    pub fn clone_for_save(&self) -> Self
    where
        T: Clone,
    {
        Self {
            config: self.config.clone(),
            sub_cores: None,
            refresh: self.refresh.clone(),
            promise: None,
            cache: None,
        }
    }

    pub fn show(&mut self, e: &Core<C, S>, ui: &mut egui::Ui) {
        ui.collapsing("Parameter scouting", |ui| {
            self.refresh.show(ui);
            self.config.show(ui);
            crate::util::show_option_with(ui, &mut self.sub_cores, "Scouters", || {
                self.config.refresh(e)
            });

            if self.sub_cores.is_none() {
                self.cache = None;
            }

            #[cfg(target_arch = "wasm32")]
            crate::util::warn_single_thread(ui);
        });
    }

    pub fn plot_elements(&self) -> Option<Vec<RawPlotData<<S as StoreState>::OwnedState>>> {
        Some(
            self.states()?
                .iter()
                .zip(self.config.offsets.iter().map(|x| x.1))
                .map(|state| RawPlotData {
                    data: state.0.clone(),
                    x: None,
                    width: state.1,
                    style: Some(Style::default()),
                })
                .collect(),
        )
    }

    pub fn push_to_views<'a, V>(&'a mut self, views: &mut V, on: ShowOn, running: bool)
    where
        V: Visualizer<<S as SharedState<'a>>::SharedState>,
        <S as SharedState<'a>>::SharedState: State<OwnedState = <S as StoreState>::OwnedState>,
    {
        if let Some(elements) = self.plot_elements() {
            for e in elements {
                views.push_elements_raw(e, on, running);
            }
        }
    }

    pub fn sync_paras(&mut self, e: &Core<C, S>) {
        if let Some(dst) = self.sub_cores.as_mut() {
            self.config.sync(e, dst);
        }
    }

    pub fn tick(&mut self, e: &Core<C, S>) {
        if let Some(cores) = self.sub_cores.as_mut() {
            if self.refresh.tick() {
                *cores = self.config.refresh(e);
            }
        }
    }

    pub fn poll_scouters(&mut self, steps: u32, add_random: bool) -> Option<()> {
        puffin_egui::puffin::profile_function!();
        let sub_cores = self.sub_cores.as_mut()?;
        self.promise.get_or_insert_with(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Promise::new_thread("run_sub_cores", sub_cores.run_sub_cores(steps, add_random))
            }
            #[cfg(target_arch = "wasm32")]
            {
                Promise::new_web("run_sub_cores", sub_cores.run_sub_cores(steps, add_random))
            }
        });
        if let Some(p) = try_poll(&mut self.promise) {
            self.cache = Some(p);
        }
        Some(())
    }
    pub fn states(&self) -> Option<&Vec<<S as StoreState>::OwnedState>> {
        self.cache.as_ref()
    }
}

impl<C, S, T> Default for Scouter<C, S, T>
where
    T: ScoutingTarget<C, S> + Default,
    C: Controller<S>,
    S: Simulator,
{
    fn default() -> Self {
        Self {
            config: Default::default(),
            sub_cores: None,
            refresh: Default::default(),
            promise: None,
            cache: None,
        }
    }
}

#[must_use]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SubCores<E> {
    #[serde(skip)]
    cores: Arc<Vec<Mutex<E>>>,
}

impl<C, S> SubCores<Core<C, S>>
where
    C: Controller<S>,
    S: Simulator,
    Core<C, S>: Send + Sync,
{
    pub fn run_sub_cores(
        &self,
        steps: u32,
        add_random: bool,
    ) -> impl FnOnce() -> Vec<<S as StoreState>::OwnedState>
    where
        <S as StoreState>::OwnedState: Send,
    {
        let cores = self.cores.clone();

        move || {
            let mut vec = Vec::new();
            cores
                .par_iter()
                .map(|c| {
                    let mut c = c.lock();
                    if add_random {
                        c.add_random();
                    }
                    c.sync_paras();
                    c.simulator.run(steps);
                    c.simulator.get_owned_state()
                })
                .collect_into_vec(&mut vec);
            vec
        }
    }
}

pub trait ScoutingTarget<C: Controller<E>, E: Simulator>:
    Send + Sync + ControllerUI + Default
{
    fn apply(&self, value: f64, controller: &mut C);
    fn sync(&self, value: f64, src: &C, dst: &mut C);
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub struct Offset<T>(T, f64);

impl<T: ControllerUI> ControllerUI for Offset<T> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.0.show_controller(ui);
            ui.add(egui::DragValue::new(&mut self.1).prefix("Δ = "));
        });
    }
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: for<'a> serde::Deserialize<'a>"
))]
pub struct ScouterConfig<T, C, S>
where
    T: ScoutingTarget<C, S>,
    C: Controller<S>,
    S: Simulator,
{
    offsets: Vec<Offset<T>>,
    phantom: std::marker::PhantomData<Core<C, S>>,
}

impl<T, C, S> std::fmt::Debug for ScouterConfig<T, C, S>
where
    T: ScoutingTarget<C, S> + Debug,
    C: Controller<S>,
    S: Simulator,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("offsets", &self.offsets)
            .finish()
    }
}

impl<T, C, S> Clone for ScouterConfig<T, C, S>
where
    T: ScoutingTarget<C, S> + Clone,
    C: Controller<S>,
    S: Simulator,
{
    fn clone(&self) -> Self {
        Self {
            offsets: self.offsets.clone(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T, C, S> Default for ScouterConfig<T, C, S>
where
    T: ScoutingTarget<C, S> + Default,
    C: Controller<S>,
    S: Simulator,
{
    fn default() -> Self {
        Self {
            offsets: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T, C, S> ScouterConfig<T, C, S>
where
    T: ScoutingTarget<C, S> + Send + Sync,
    S: Simulator + Send + Sync,
    C: Controller<S> + Clone + Send + Sync,
{
    pub fn show(&mut self, ui: &mut egui::Ui) {
        crate::util::show_vector(ui, &mut self.offsets);
    }

    pub fn refresh(&mut self, e: &Core<C, S>) -> SubCores<Core<C, S>> {
        puffin_egui::puffin::profile_function!();
        let mut ret = Vec::new();
        for Offset(target, value) in &self.offsets {
            let mut c = e.controller.clone();
            let state = e.simulator.get_owned_state();
            let dim = e.dim;
            let r = e.random.clone();
            target.apply(*value, &mut c);
            let mut s: S = c.construct_engine(dim);
            s.set_owned_state(state);
            let core = Core {
                dim,
                controller: c,
                simulator: s,
                random: r,
            };
            ret.push(Mutex::new(core));
        }
        SubCores {
            cores: Arc::new(ret),
        }
    }

    pub fn sync(&mut self, src: &Core<C, S>, dst: &mut SubCores<Core<C, S>>) {
        puffin_egui::puffin::profile_function!();
        self.offsets.par_iter().zip(dst.cores.par_iter()).for_each(
            |(Offset(target, value), core)| {
                let mut core = core.lock();
                let c = &mut core.controller;
                target.sync(*value, &src.controller, c);
                core.random = src.random.clone();
            },
        );
    }
}
