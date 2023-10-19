use egui_plotter::EguiBackend;
use plotters::{
    coord::Shift,
    prelude::*,
    style::{RelativeSize, SizeDesc},
};

use super::{chart::Process, *};
use std::ops::Range;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
enum DrawRange<A> {
    Auto(Option<A>),
    Static(A),
}

impl<A> Default for DrawRange<A> {
    fn default() -> Self {
        Self::Auto(None)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, getset::Setters)]
pub struct RawMapVisualizer<B = f64> {
    color_range: DrawRange<Range<B>>,
    #[getset(set = "pub(crate)")]
    caption: Option<String>,
    #[getset(set = "pub(crate)")]
    x_desc: Option<String>,
    #[getset(set = "pub(crate)")]
    y_desc: Option<String>,
    #[getset(set = "pub(crate)")]
    auto_range: Option<Range<B>>,
}

impl<B> Default for RawMapVisualizer<B> {
    fn default() -> Self {
        Self {
            color_range: DrawRange::default(),
            caption: None,
            x_desc: None,
            y_desc: None,
            auto_range: None,
        }
    }
}

impl<B> RawMapVisualizer<B> {
    /* pub fn binding(self, matrix: Vec<B>) -> ColorMapVisualizer<B> {
        ColorMapVisualizer { matrix, raw: self }
    } */
    /* pub fn set_color_range(&mut self, x: Range<B>) -> &mut Self {
        self.color_range = DrawRange::Static(x);
        self
    }
    pub fn set_caption<S: ToString>(&mut self, s: S) -> &mut Self {
        self.caption = Some(s.to_string());
        self
    }
    pub fn set_x_desc<S: ToString>(&mut self, s: S) -> &mut Self {
        self.x_desc = Some(s.to_string());
        self
    }
    pub fn set_y_desc<S: ToString>(&mut self, s: S) -> &mut Self {
        self.y_desc = Some(s.to_string());
        self
    } */
    /* pub fn set_x_label_formatter(&mut self, x_formatter: fn(&usize) -> String) -> &mut Self {
        self.x_label_formatter = Some(x_formatter);
        self
    }
    pub fn set_y_label_formatter(&mut self, y_formatter: fn(&usize) -> String) -> &mut Self {
        self.y_label_formatter = Some(y_formatter);
        self
    } */
}
impl RawMapVisualizer<f64> {
    pub fn update_range(&mut self, row: &[f64]) -> &mut Self {
        row.iter().copied().for_each(|x| {
            if let Some(ref mut o) = self.auto_range {
                if o.start > x {
                    o.start = x;
                } else if o.end < x {
                    o.end = x;
                }
            } else {
                self.auto_range = Some((x)..(x));
            }
        });
        self
    }
    pub fn draw_on<DB: DrawingBackend>(
        &self,
        matrix: &Vec<f64>,
        draw_area: &DrawingArea<DB, Shift>,
        chunk_size: usize,
        text_style: TextStyle<'_>,
    ) -> Result<impl Fn((i32, i32)) -> Option<(usize, usize)>, DrawingAreaErrorKind<DB::ErrorType>>
    {
        let row_len = chunk_size;
        let column_len = matrix.len() / chunk_size;
        assert_ne!(row_len * column_len, 0);
        assert_eq!(row_len * column_len, matrix.len()); //make sure it has exact n * chunk_size element
        let (range_max, range_min) = match self.color_range {
            DrawRange::Auto(ref a) => a.as_ref().map(|x| (x.end, x.start)).unwrap_or((1., 0.)),
            DrawRange::Static(ref s) => (s.end, s.start),
        };
        let range = if range_max != range_min {
            range_max - range_min
        } else {
            1.
        };
        let color_map = |v: f64| ((v - range_min) / range);
        draw_area.fill(&WHITE)?;
        let (area, bar) = draw_area.split_horizontally(RelativeSize::Width(0.85));
        let mut builder_map = ChartBuilder::on(&area);
        builder_map
            .margin_right(2.percent_width().in_pixels(draw_area))
            .margin_top(2.percent_height().in_pixels(draw_area))
            .y_label_area_size(10.percent_width().in_pixels(draw_area))
            .x_label_area_size(10.percent_height().in_pixels(draw_area));
        if let Some(ref s) = self.caption {
            builder_map.caption(s, ("sans-serif", 2.5.percent().in_pixels(draw_area)));
        }

        let mut chart_map = builder_map.build_cartesian_2d(0..row_len, 0..column_len)?;
        let mut mesh_map = chart_map.configure_mesh();
        mesh_map
            .x_label_style(("sans-serif", 5.percent().in_pixels(draw_area)))
            .y_label_style(("sans-serif", 5.percent().in_pixels(draw_area)))
            .disable_x_mesh()
            .disable_y_mesh();
        if let Some(ref s) = self.x_desc {
            mesh_map.x_desc(s);
        }
        if let Some(ref s) = self.y_desc {
            mesh_map.y_desc(s);
        }
        /* if let Some(ref f) = self.x_label_formatter {
            mesh_map.x_label_formatter(f);
        }
        if let Some(ref f) = self.y_label_formatter {
            mesh_map.y_label_formatter(f);
        } */
        mesh_map.draw()?;
        draw_map(&mut chart_map, matrix.chunks(chunk_size), color_map);

        let mut builder_bar = ChartBuilder::on(&bar);
        builder_bar
            .margin_right(2.percent_width().in_pixels(draw_area))
            .margin_top(2.percent_height().in_pixels(draw_area))
            .margin_bottom(10.percent_height().in_pixels(draw_area)) //take the space for hidden x axis
            .y_label_area_size(10.percent_width().in_pixels(draw_area));
        let mut chart_bar =
            builder_bar.build_cartesian_2d(0f64..1., range_min..(range + range_min))?;
        let mut mesh_bar = chart_bar.configure_mesh();
        let step = range / (column_len - 1).max(1) as f64;
        mesh_bar
            .disable_x_mesh()
            .disable_y_mesh()
            .x_label_style(text_style.clone())
            .y_label_style(text_style);
        mesh_bar.draw()?;
        chart_bar.draw_series(
            std::iter::successors(Some(range_min), |x| Some(step + x))
                .take_while(|x| *x <= range_max)
                .map(|v| {
                    Rectangle::new(
                        [(0., v - step / 2.), (1., v + step / 2.)],
                        HSLColor(
                            240.0 / 360.0 - 240.0 / 360.0 * color_map(v),
                            0.7,
                            0.1 + 0.4 * color_map(v),
                        )
                        .filled(),
                    )
                }),
        )?;
        draw_area.present()?;
        return Ok(chart_map.into_coord_trans());
    }
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ColorMapVisualizer<B = f64> {
    matrix: Vec<B>,
    raw: RawMapVisualizer<B>,
}

impl Default for ColorMapVisualizer<f64> {
    fn default() -> Self {
        Self {
            matrix: Default::default(),
            raw: Default::default(),
        }
    }
}

impl ColorMapVisualizer<f64> {
    pub fn fetch(
        &mut self,
        data: &Vec<Complex64>,
        proc: &mut Process,
        chunk_size: usize,
    ) -> &mut Self {
        self.matrix.clear();
        self.matrix.reserve(data.len());
        for d in data.chunks(chunk_size) {
            self.push(proc.proc(d));
        }
        self
    }
    pub fn push(&mut self, mut row: Vec<f64>) -> &mut Self {
        self.raw.update_range(&row);
        self.matrix.append(&mut row);
        self
    }
    pub fn draw<DB: DrawingBackend>(
        &self,
        draw_area: DrawingArea<DB, Shift>,
        chunk_size: usize,
        text_style: TextStyle<'_>,
    ) -> Result<impl Fn((i32, i32)) -> Option<(usize, usize)>, DrawingAreaErrorKind<DB::ErrorType>>
    {
        self.raw
            .draw_on(&self.matrix, &draw_area, chunk_size, text_style)
    }
    pub fn draw_on_ui<'a>(
        &self,
        chunk_size: usize,
        ui: &'a egui::Ui,
    ) -> Result<
        impl Fn((i32, i32)) -> Option<(usize, usize)> + 'a,
        DrawingAreaErrorKind<<EguiBackend<'_> as DrawingBackend>::ErrorType>,
    > {
        let f = ui.style().text_styles.get(&egui::TextStyle::Monospace);
        let pixel_per_point = ui.ctx().pixels_per_point();
        match f {
            Some(x) => self.draw(
                EguiBackend::new(ui).into_drawing_area(),
                chunk_size,
                (x.family.to_string().as_str(), x.size * pixel_per_point).into(),
            ),
            None => self.draw(
                EguiBackend::new(ui).into_drawing_area(),
                chunk_size,
                ("monospace", 12. * pixel_per_point).into(),
            ),
        }
    }
}
#[allow(unused)]
impl<B> ColorMapVisualizer<B> {
    pub fn set_color_range(&mut self, x: Range<B>) -> &mut Self {
        self.raw.set_auto_range(x.into());
        self
    }
    pub fn set_caption<S: ToString>(&mut self, s: S) -> &mut Self {
        self.raw.set_caption(s.to_string().into());
        self
    }
    pub fn set_x_desc<S: ToString>(&mut self, s: S) -> &mut Self {
        self.raw.set_x_desc(s.to_string().into());
        self
    }
    pub fn set_y_desc<S: ToString>(&mut self, s: S) -> &mut Self {
        self.raw.set_y_desc(s.to_string().into());
        self
    }
    /* pub fn set_x_label_formatter(&mut self, x_formatter: fn(&usize) -> String) -> &mut Self {
        self.raw.set_x_label_formatter(x_formatter);
        self
    }
    pub fn set_y_label_formatter(&mut self, y_formatter: fn(&usize) -> String) -> &mut Self {
        self.raw.set_y_label_formatter(y_formatter);
        self
    } */
}

#[cfg(not(feature = "rayon"))]
fn draw_map<'a, DB: DrawingBackend>(
    ctx: &mut ChartContext<
        '_,
        DB,
        Cartesian2d<
            plotters::coord::types::RangedCoordusize,
            plotters::coord::types::RangedCoordusize,
        >,
    >,
    data: impl Iterator<Item = &'a [f64]>,
    color_map: impl Fn(f64) -> f64,
) {
    ctx.draw_series(
        data.enumerate()
            .flat_map(|(y, l)| l.iter().enumerate().map(move |(x, v)| (x, y, *v)))
            .map(|(x, y, v)| {
                Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    HSLColor(
                        240.0 / 360.0 - 240.0 / 360.0 * color_map(v),
                        0.7,
                        0.1 + 0.4 * color_map(v),
                    )
                    .filled(),
                )
            }),
    )
    .expect("plotting reactangles");
}

/*
#[cfg(feature = "rayon")]
fn draw_map<DB: DrawingBackend>(
    ctx: &mut ChartContext<
        DB,
        Cartesian2d<
            plotters::coord::types::RangedCoordusize,
            plotters::coord::types::RangedCoordusize,
        >,
    >,
    data: &Vec<Vec<f64>>,
    color_map: impl Fn(f64) -> f64 + Sync,
) {
    const MAX_PARTS_NUM: usize = 8;
    const MIN_PARTS_LEN: usize = 1024;
    let column_num = data.first().map_or(0, |x| x.len());
    let row_num = data.len();
    let parts_num = ((column_num * row_num + MIN_PARTS_LEN - 1) / MIN_PARTS_LEN).min(MAX_PARTS_NUM);
    let parts_row_len = (row_num + parts_num - 1) / parts_num;
    let mut areas = Vec::with_capacity(parts_num);
    let mut area_left = ctx.plotting_area().strip_coord_spec();
    let split_pixels = (100. * parts_row_len as f64 / row_num as f64)
        .percent()
        .in_pixels(&area_left);
    let total_pixels = area_left.dim_in_pixel().1;
    (0..(parts_num - 1)).for_each(|i| {
        let (upper, lower) =
            area_left.split_vertically(total_pixels as i32 - (i as i32 + 1) * split_pixels);
        area_left = upper;
        areas.push(lower);
    });
    areas.push(area_left);
    use rayon::prelude::*;
    let mut sub_plots: Vec<_> = areas
        .iter()
        .map(|x| BitMapElement::new((0, 0), x.dim_in_pixel()))
        .collect::<Vec<_>>();
    let color_map = &color_map;
    sub_plots
        .par_iter_mut()
        .zip(data.par_chunks(parts_row_len))
        .for_each(|(s, d)| {
            ChartBuilder::on(&s.as_bitmap_backend().into_drawing_area())
                .build_cartesian_2d(0..d.first().map_or(0, |x| x.len()), 0..d.len())
                .expect("chart builder on subplot")
                .draw_series(
                    d.iter()
                        .enumerate()
                        .map(move |(y, v)| {
                            v.iter().enumerate().map(move |(x, v)| {
                                Rectangle::new(
                                    [(x, y), (x + 1, y + 1)],
                                    HSLColor(
                                        240.0 / 360.0 - 240.0 / 360.0 * color_map(*v),
                                        0.7,
                                        0.1 + 0.4 * color_map(*v),
                                    )
                                    .filled(),
                                )
                            })
                        })
                        .flatten(),
                )
                .expect("drawing rectangles");
        });
    areas
        .into_iter()
        .zip(sub_plots.into_iter())
        .for_each(|(a, s)| a.draw(&s).expect("placing subplots on area"))
}
 */
