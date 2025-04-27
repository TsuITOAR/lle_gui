use crate::drawer::plot_item::Style;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub struct Debugger;

impl crate::app::Debugger<&'_ super::state::State> for Debugger {
    fn visualize(&mut self, source: &'_ super::state::State) -> Vec<crate::views::PlotElement> {
        let dim = source.data.len() as i32;
        let cp = source.cp.clone();
        /* let vis1: Vec<_> = (-(dim / 2 + 1)..(dim / 2))
        .map(|x| {
            let y = cp.singularity_point(x);
            -(y as i8) as f64 + 0.5
        })
        .enumerate()
        .collect(); */

        let vis1 = source
            .coupling_iter_positive()
            .map(|x| {
                use crate::controller::gencprt::state::Mode;

                match x {
                    Mode::Single { meta, .. } => {
                        Box::<[_]>::from([(((dim / 2 + 1) + meta.freq) as f64, -2.)])
                    }
                    Mode::Pair { meta, .. } => Box::<[_]>::from(
                        [
                            (((dim / 2 + 1) + meta.freq) as f64, -1.),
                            (((dim / 2 + 1) + meta.freq) as f64 + 1., 1.),
                        ]
                        .as_slice(),
                    ),
                }
            })
            .chain(source.coupling_iter_negative().map(|x| {
                use crate::controller::gencprt::state::Mode;

                match x {
                    Mode::Single { meta, .. } => {
                        Box::<[_]>::from([(((dim / 2 + 1) + meta.freq) as f64, -2.)])
                    }
                    Mode::Pair { meta, .. } => Box::<[_]>::from(
                        [
                            (((dim / 2 + 1) + meta.freq) as f64, -1.),
                            (((dim / 2 + 1) + meta.freq) as f64 - 1., 1.),
                        ]
                        .as_slice(),
                    ),
                }
            }))
            .flatten()
            .collect::<Vec<_>>();

        let vis2: Vec<_> = (-(dim / 2 + 1)..(dim / 2))
            .map(|x| {
                use std::f64::consts::PI;
                let m = cp.m_original(x);
                
                cp.cp_angle((x + m) / 2, m) / PI * 10.
            })
            .enumerate()
            .map(|(x, y)| (x as f64, y))
            .collect();
        let legend = ["singularity", "cp_angle"];
        [vis1, vis2]
            .into_iter()
            .zip(legend)
            .map(|(v, legend)| {
                let (x, y) = v.into_iter().unzip();
                crate::views::PlotElement {
                    x: Some(x),
                    y,
                    style: Some(Style::default().interleave().set_width(2.)),
                    legend: Some(legend.to_string()),
                }
            })
            .collect::<Vec<_>>()
    }

    fn legend(&self) -> Vec<String> {
        vec!["m".to_string()]
    }

    fn show(&mut self, _ui: &mut egui::Ui) {}
}
