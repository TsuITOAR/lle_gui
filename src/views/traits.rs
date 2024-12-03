use lle::num_complex::Complex64;

use crate::drawer::ViewField;

use super::{PlotElement, RawPlotElement, ShowOn, Views};

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

pub trait Visualize<S: State> {
    fn adjust_to_state(&mut self, data: S);
    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: S);
    fn clear_his(&mut self);
    fn config(&mut self, ui: &mut egui::Ui);
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

impl<const L: usize, S: State, V: Visualize<S>> Visualize<[S; L]> for Views<[V; L]> {
    fn adjust_to_state(&mut self, data: [S; L]) {
        for (view, data) in self.views.iter_mut().zip(data.into_iter()) {
            view.adjust_to_state(data);
        }
    }
    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: [S; L]) {
        for (view, data) in self.views.iter_mut().zip(data.into_iter()) {
            view.toggle_record_his(ui, data);
        }
    }

    fn clear_his(&mut self) {
        for view in self.views.iter_mut() {
            view.clear_his();
        }
    }

    fn config(&mut self, ui: &mut egui::Ui) {
        for (i, view) in self.views.iter_mut().enumerate() {
            ui.collapsing(format!("View {}", i), |ui| {
                view.config(ui);
            });
        }
    }

    fn record(&mut self, data: [S; L]) {
        for (view, data) in self.views.iter_mut().zip(data.into_iter()) {
            view.record(data);
        }
    }

    fn push_elements_raw(
        &mut self,
        points: RawPlotElement<<[S; L] as State>::OwnedState>,
        on: ShowOn,
    ) {
        for (view, p) in self.views.iter_mut().zip(points.split()) {
            view.push_elements_raw(p, on);
        }
    }

    fn push_elements(&mut self, points: PlotElement, on: ShowOn) {
        for view in self.views.iter_mut() {
            view.push_elements(points.clone(), on);
        }
    }

    fn plot(
        &mut self,
        data: [S; L],
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        /* let mut new_scouting: [Option<Vec<_>>; L] = [const { None }; L];
        for ((i, s), offset) in scouting {
            new_scouting[i].get_or_insert_default().push((s, offset));
        } */
        for (view, data) in self.views.iter_mut().zip(data) {
            view.plot(
                data,
                ctx,
                running,
                #[cfg(feature = "gpu")]
                render_state,
            );
        }
    }
}

impl<'a> Visualize<&'a [Complex64]> for ViewField {
    fn adjust_to_state(&mut self, data: &'a [Complex64]) {
        if let Some(ref mut f) = self.f_chart {
            f.adjust_to_state(data);
        }
        if let Some(ref mut r) = self.r_chart {
            r.adjust_to_state(data);
        }
        self.clear_his();
        self.record(data);
    }

    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: &[Complex64]) {
        self.toggle_record_his(ui, data);
    }

    fn clear_his(&mut self) {
        if let Some(ref mut his) = self.history {
            his.clear();
        }
    }

    fn config(&mut self, ui: &mut egui::Ui) {
        self.show_which(ui);
    }

    fn record(&mut self, data: &'a [Complex64]) {
        self.log_his(data);
    }

    fn push_elements_raw(&mut self, points: RawPlotElement<Vec<Complex64>>, on: ShowOn) {
        match on {
            ShowOn::Both => {
                if let Some(ref mut f) = self.f_chart {
                    f.push_additional_raw(&points)
                }
                if let Some(ref mut r) = self.r_chart {
                    r.push_additional_raw(&points)
                }
            }
            ShowOn::Time => {
                if let Some(ref mut r) = self.r_chart {
                    r.push_additional_raw(&points)
                }
            }
            ShowOn::Freq => {
                if let Some(ref mut f) = self.f_chart {
                    f.push_additional_raw(&points)
                }
            }
        }
    }

    fn push_elements(&mut self, points: PlotElement, on: ShowOn) {
        match on {
            ShowOn::Both => {
                if let Some(ref mut f) = self.f_chart {
                    f.push_additional(points.clone())
                }
                if let Some(ref mut r) = self.r_chart {
                    r.push_additional(points)
                }
            }
            ShowOn::Time => {
                if let Some(ref mut r) = self.r_chart {
                    r.push_additional(points)
                }
            }
            ShowOn::Freq => {
                if let Some(ref mut f) = self.f_chart {
                    f.push_additional(points)
                }
            }
        }
    }

    fn plot(
        &mut self,
        data: &'a [Complex64],
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        self.visualize_state(
            data,
            ctx,
            running,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }
}

impl<'a> Visualize<&'a [Complex64]> for Views<ViewField> {
    fn adjust_to_state(&mut self, data: &'a [Complex64]) {
        self.views.adjust_to_state(data);
    }
    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: &'a [Complex64]) {
        self.views.toggle_record_his(ui, data);
    }

    fn clear_his(&mut self) {
        self.views.clear_his();
    }

    fn config(&mut self, ui: &mut egui::Ui) {
        self.views.show_which(ui);
    }

    fn record(&mut self, data: &'a [Complex64]) {
        self.views.log_his(data);
    }

    fn push_elements_raw(&mut self, points: RawPlotElement<Vec<Complex64>>, on: ShowOn) {
        self.views.push_elements_raw(points, on);
    }

    fn push_elements(&mut self, points: PlotElement, on: ShowOn) {
        self.views.push_elements(points, on);
    }

    fn plot(
        &mut self,
        data: &[Complex64],
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        self.views.visualize_state(
            data,
            ctx,
            running,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }
}
