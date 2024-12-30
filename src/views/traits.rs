use lle::num_complex::Complex64;

use super::{PlotElement, RawPlotElement, ShowOn};

pub trait State: Clone + Copy {
    type OwnedState: Clone;
    fn to_owned(&self) -> Self::OwnedState;
}

impl<T: State, const L: usize> State for [T; L] {
    type OwnedState = [T::OwnedState; L];
    fn to_owned(&self) -> Self::OwnedState {
        std::array::from_fn(|i| self[i].to_owned())
    }
}

impl State for &'_ [Complex64] {
    type OwnedState = Vec<Complex64>;
    fn to_owned(&self) -> Self::OwnedState {
        self.to_vec()
    }
}

impl<T: ToOwned> State for &'_ T
where
    T::Owned: Clone,
{
    type OwnedState = T::Owned;
    fn to_owned(&self) -> Self::OwnedState {
        (*self).to_owned()
    }
}

pub trait Visualizer<S: State>: ui_traits::ControllerUI {
    fn adjust_to_state(&mut self, data: S);
    fn record(&mut self, data: S);
    fn push_elements_raw(&mut self, points: RawPlotElement<S::OwnedState>, on: ShowOn);
    fn push_elements(&mut self, points: PlotElement, on: ShowOn);
    fn plot(
        &mut self,
        data: S,
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    );
}
