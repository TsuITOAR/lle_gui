use lle::num_complex::Complex64;
use ui_traits::ControllerUI;

use crate::FftSource;

use super::*;

impl<const L: usize, S: FftSource> ControllerUI for Views<[ViewField<S>; L]> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| self.views.iter_mut().for_each(|v| v.toggle_record_his(ui)));
        for (i, view) in self.views.iter_mut().enumerate() {
            ui.collapsing(format!("View {}", i), |ui| {
                view.show_which(ui);
            });
        }
    }
}

impl<S: FftSource> ControllerUI for Views<ViewField<S>> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        self.views.show_controller(ui);
    }
}

impl<S: FftSource> ControllerUI for ViewField<S> {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        self.toggle_record_his(ui);
        self.show_which(ui);
    }
}

impl<const L: usize, S: State, FS: FftSource> Visualizer<[S; L]> for Views<[ViewField<FS>; L]>
where
    ViewField<FS>: Visualizer<S>,
{
    fn adjust_to_state(&mut self, data: [S; L]) {
        for (view, data) in self.views.iter_mut().zip(data.into_iter()) {
            view.adjust_to_state(data);
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

impl<'a> Visualizer<&'a Vec<Complex64>> for ViewField<Vec<Complex64>> {
    fn adjust_to_state(&mut self, data: &'a Vec<Complex64>) {
        if let Some(ref mut f) = self.f_chart {
            f.adjust_to_state(data);
        }
        if let Some(ref mut r) = self.r_chart {
            r.adjust_to_state(data);
        }
        if self.history.is_active() {
            self.history.reset();
            self.history.active();
            self.history.push(data);
        }
    }

    fn record(&mut self, data: &'a Vec<Complex64>) {
        self.history.push(data);
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
        data: &'a Vec<Complex64>,
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

impl<P: State, T: Visualizer<P>> Visualizer<P> for Views<T>
where
    Views<T>: ControllerUI,
{
    fn adjust_to_state(&mut self, data: P) {
        self.views.adjust_to_state(data);
    }

    fn record(&mut self, data: P) {
        self.views.record(data);
    }

    fn push_elements_raw(&mut self, points: RawPlotElement<P::OwnedState>, on: ShowOn) {
        self.views.push_elements_raw(points, on);
    }

    fn push_elements(&mut self, points: PlotElement, on: ShowOn) {
        self.views.push_elements(points, on);
    }

    fn plot(
        &mut self,
        data: P,
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        self.views.plot(
            data,
            ctx,
            running,
            #[cfg(feature = "gpu")]
            render_state,
        );
    }
}
