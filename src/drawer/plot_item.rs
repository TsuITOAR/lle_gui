use crate::views::PlotElement;

use super::plot_kind::PlotKind;

pub(crate) struct PlotItem {
    pub data: egui_plot::PlotPoints<'static>,
    pub desc: Option<String>,
    pub style: Style,
}

impl From<PlotElement> for PlotItem {
    fn from(plot: PlotElement) -> Self {
        match plot.x {
            Some(x) => PlotItem {
                data: egui_plot::PlotPoints::from_iter(
                    x.into_iter().zip(plot.y).map(|(x, y)| [x, y]),
                ),
                desc: plot.legend,
                style: plot.style.unwrap_or_default(),
            },
            None => PlotItem {
                data: egui_plot::PlotPoints::from_ys_f64(&plot.y),
                desc: plot.legend,
                style: plot.style.unwrap_or_default(),
            },
        }
    }
}

impl PlotItem {
    pub(crate) fn plot(self, plot_ui: &mut egui_plot::PlotUi<'_>, kind: PlotKind) {
        match kind {
            PlotKind::Line => plot_line(self, plot_ui),
            PlotKind::Points => plot_points(self, plot_ui),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub(crate) struct Style {
    pub(crate) width: StyleWidth,
    pub(crate) interleave: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: StyleWidth::Sub,
            interleave: false,
        }
    }
}

impl Style {
    pub(crate) fn set_width(mut self, width: f32) -> Self {
        self.width = StyleWidth::Custom(width);
        self
    }

    pub(crate) fn interleave(mut self) -> Self {
        self.interleave = true;
        self
    }

    pub(crate) fn main(mut self) -> Self {
        self.width = StyleWidth::Main;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum StyleWidth {
    Main,
    #[default]
    Sub,
    Custom(f32),
}

impl Style {
    pub(crate) fn width(&self) -> f32 {
        match &self.width {
            StyleWidth::Main => 2.0,
            StyleWidth::Sub => 1.0,
            StyleWidth::Custom(w) => *w,
        }
    }
}

fn plot_line(item: PlotItem, plot_ui: &mut egui_plot::PlotUi<'_>) {
    let PlotItem {
        data: e,
        desc: d,
        style,
    } = item;
    if let Some(d) = d {
        plot_ui.line(egui_plot::Line::new(d, e).width(style.width()));
    } else {
        plot_ui.line(egui_plot::Line::new("", e).width(style.width()));
    }
}

fn plot_points(item: PlotItem, plot_ui: &mut egui_plot::PlotUi<'_>) {
    let PlotItem {
        data: e,
        desc: d,
        style,
    } = item;

    let width = style.width();

    let points = if style.interleave {
        let (a, b): (Vec<_>, Vec<_>) = e
            .points()
            .chunks_exact(2)
            .map(|x| ([x[0].x, x[0].y], [x[1].x, x[1].y]))
            .unzip();
        Box::<[_]>::from([
            egui_plot::Points::new("", a).radius(width),
            egui_plot::Points::new("", b).radius(width),
        ])
    } else {
        Box::from([egui_plot::Points::new("", e).radius(width)])
    };
    for p in points {
        if let Some(ref d) = d {
            plot_ui.points(p.name(d));
        } else {
            plot_ui.points(p);
        }
    }
}
