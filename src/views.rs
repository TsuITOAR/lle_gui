use lle::num_complex::Complex64;

use crate::drawer::ViewField;
use std::array::from_fn;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Views<V> {
    pub(crate) views: V,
    last_plot: Option<Instant>,
}

impl Default for Views<ViewField> {
    fn default() -> Self {
        Self {
            views: ViewField::new(0),
            last_plot: None,
        }
    }
}

impl<const L: usize> Default for Views<[ViewField; L]> {
    fn default() -> Self {
        Self {
            views: from_fn(ViewField::new),
            last_plot: None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Views<ViewField> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            views: ViewField::deserialize(deserializer)?,
            last_plot: None,
        })
    }
}

impl serde::Serialize for Views<ViewField> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.views.serialize(serializer)
    }
}

impl<'de, const L: usize> serde::Deserialize<'de> for Views<[ViewField; L]> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut views = <Vec<ViewField>>::deserialize(deserializer)?.into_iter();
        Ok(Self {
            views: from_fn(move |i| views.next().unwrap_or_else(|| ViewField::new(i))),
            last_plot: None,
        })
    }
}

impl<const L: usize> serde::Serialize for Views<[ViewField; L]> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.views.iter().collect::<Vec<_>>().serialize(serializer)
    }
}

impl<V> Views<V> {
    pub(crate) fn show_fps(&mut self, ui: &mut egui::Ui) {
        let now = Instant::now();
        let last = self.last_plot.replace(now);
        if let Some(last) = last {
            let past = (now - last).as_secs_f32();
            ui.label(format!("{:.0}Hz ({:.1}ms)", 1. / past, past * 1000.));
        } else {
            ui.label("Start to update fps");
        }
    }
}

pub trait Visualize<State> {
    fn adjust_to_state(&mut self, data: State);
    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: State);
    fn clear_his(&mut self);
    fn config(&mut self, ui: &mut egui::Ui);
    fn record(&mut self, data: State);
    fn add_dispersion(&mut self, points: Vec<[f64; 2]>);
    fn plot(
        &mut self,
        data: State,
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    );
}

impl<const L: usize, S, V: Visualize<S>> Visualize<[S; L]> for Views<[V; L]> {
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

    fn add_dispersion(&mut self, points: Vec<[f64; 2]>) {
        for view in self.views.iter_mut() {
            view.add_dispersion(points.clone());
        }
    }

    fn plot(
        &mut self,
        data: [S; L],
        ctx: &egui::Context,
        running: bool,
        #[cfg(feature = "gpu")] render_state: &eframe::egui_wgpu::RenderState,
    ) {
        for (view, data) in self.views.iter_mut().zip(data.into_iter()) {
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

    fn toggle_record_his(&mut self, ui: &mut egui::Ui, data: &'a [Complex64]) {
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

    fn add_dispersion(&mut self, points: Vec<[f64; 2]>) {
        if let Some(ref mut f) = self.f_chart {
            f.additional = Some(egui_plot::PlotPoints::new(points))
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

    fn add_dispersion(&mut self, points: Vec<[f64; 2]>) {
        self.views.add_dispersion(points);
    }

    fn plot(
        &mut self,
        data: &'a [Complex64],
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
