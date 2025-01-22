use super::state::State;
use crate::{
    drawer::ViewField,
    views::{ShowOn, Visualizer},
};

impl<'a> Visualizer<&'a State> for ViewField<State> {
    fn adjust_to_state(&mut self, data: &'a State) {
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

    fn record(&mut self, data: &'a State) {
        self.history.push(data);
    }

    fn push_elements_raw(
        &mut self,
        points: crate::views::RawPlotElement<<&'a State as crate::views::State>::OwnedState>,
        on: ShowOn,
    ) {
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

    fn push_elements(&mut self, points: crate::views::PlotElement, on: ShowOn) {
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
        data: &'a State,
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
