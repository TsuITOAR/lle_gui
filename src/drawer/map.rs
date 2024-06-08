use super::backend::EguiBackend;
use lle::num_traits::Pow;
use plotters::{
    coord::Shift,
    prelude::*,
    style::{RelativeSize, SizeDesc},
};

use super::{chart::Process, *};
use std::{marker::PhantomData, num::NonZeroUsize, ops::Range};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum DrawRange<A> {
    Auto(Option<A>),
    Static(A),
}

impl<A> Default for DrawRange<A> {
    fn default() -> Self {
        Self::Auto(None)
    }
}

impl DrawRange<Range<f64>> {
    pub(crate) fn make_range_legal(&mut self) {
        const MIN_SPAN: f64 = 0.0001;
        if let DrawRange::Auto(a) = self {
            match a {
                Some(ref mut r) if r.end - r.start < MIN_SPAN => {
                    let center = (r.start + r.end) / 2.;
                    *r = (center - MIN_SPAN / 2.)..(center + MIN_SPAN / 2.);
                }
                None => *a = Some((-MIN_SPAN / 2.)..MIN_SPAN / 2.),
                _ => (),
            }
        }
    }
}

#[derive(Clone)]
pub struct Style {
    text: (String, f32, plotters::prelude::RGBAColor),
    bg: plotters::prelude::RGBAColor,
}

fn convert_egui_color(e: egui::Color32) -> plotters::prelude::RGBAColor {
    plotters::prelude::RGBAColor(e.r(), e.g(), e.b(), e.a() as f64 / 255.)
}

impl Style {
    pub fn from_ui(ui: &'_ egui::Ui) -> Self {
        let f = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Monospace)
            .unwrap();
        let text = (
            f.family.to_string(),
            f.size,
            convert_egui_color(ui.visuals().text_color()),
        );
        Self {
            text,
            bg: convert_egui_color(ui.visuals().extreme_bg_color),
        }
    }
    pub fn text_color(&self) -> plotters::prelude::RGBAColor {
        self.text.2
    }

    pub fn text_style(&'_ self) -> TextStyle<'_> {
        let text_style: TextStyle<'_> = (self.text.0.as_str(), self.text.1).into();
        text_style.color(&self.text.2)
    }

    pub fn caption_style(&'_ self) -> impl IntoTextStyle<'_> {
        (self.text.0.as_str(), self.text.1 * 2., &self.text.2)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, getset::Setters)]
pub struct RawMapVisualizer<B = f64, Backend = ()> {
    backend: PhantomData<Backend>,
    #[getset(set = "pub(crate)")]
    color_range: DrawRange<Range<B>>,
    #[getset(set = "pub(crate)")]
    caption: Option<String>,
    #[getset(set = "pub(crate)")]
    x_desc: Option<String>,
    #[getset(set = "pub(crate)")]
    y_desc: Option<String>,
}

impl<B> Default for RawMapVisualizer<B> {
    fn default() -> Self {
        Self {
            color_range: DrawRange::default(),
            caption: None,
            x_desc: None,
            y_desc: None,
            backend: PhantomData,
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
impl<Backend> RawMapVisualizer<f64, Backend> {
    pub fn update_range(&mut self, row: &[f64]) -> &mut Self {
        match self.color_range {
            DrawRange::Auto(ref mut r) => {
                row.iter().copied().filter(|x| x.is_normal()).for_each(|x| {
                    if let Some(ref mut o) = r {
                        if o.start > x {
                            o.start = x;
                        } else if o.end < x {
                            o.end = x;
                        }
                    } else {
                        *r = Some((x)..(x));
                    }
                });
            }
            DrawRange::Static(_) => (),
        }

        self
    }
}

const COLOR_MAP: plotters::prelude::ViridisRGB = plotters::prelude::ViridisRGB {};

impl RawMapVisualizer<f64> {
    pub fn draw_on<DB: DrawingBackend>(
        &self,
        matrix: &[f64],
        draw_area: &DrawingArea<DB, Shift>,
        chunk_size: usize,
        style: Style,
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
        let map_range = range_min..(range_min + range);
        //draw_area.fill(&style.bg)?;
        let (area, bar) = draw_area.split_horizontally(RelativeSize::Width(0.9));
        let mut builder_map = ChartBuilder::on(&area);

        let text_style = style.text_style();
        builder_map
            //.margin_right(2.percent_width().in_pixels(draw_area))
            .margin_top(2.percent_height().in_pixels(draw_area))
            .margin_bottom(2.percent_height().in_pixels(draw_area))
            .y_label_area_size(5.percent_width().in_pixels(draw_area))
            .x_label_area_size(5.percent_height().in_pixels(draw_area));
        if let Some(ref s) = self.caption {
            builder_map.caption(s, style.caption_style());
        }

        let mut chart_map = builder_map.build_cartesian_2d(0..row_len, 0..column_len)?;
        chart_map.plotting_area().fill(&style.bg)?;
        let mut mesh_map = chart_map.configure_mesh();
        mesh_map
            .axis_style(style.text_color())
            .x_label_style(text_style.clone())
            .y_label_style(text_style)
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
        draw_map(&mut chart_map, matrix.chunks(chunk_size), map_range.clone());

        let mut builder_bar = ChartBuilder::on(&bar);
        builder_bar
            .margin_right(10.percent_width().in_pixels(&bar))
            .margin_top(2.percent_height().in_pixels(&bar))
            .margin_bottom(7.percent_height().in_pixels(&bar)) //take the space for hidden x axis
            .y_label_area_size(60.percent_width().in_pixels(&bar));
        let mut chart_bar =
            builder_bar.build_cartesian_2d(0f64..1., range_min..(range + range_min))?;
        let mut mesh_bar = chart_bar.configure_mesh();
        let step = range / 2.pow(8u8) as f64;
        mesh_bar
            .disable_x_mesh()
            .disable_y_mesh()
            .x_label_style(style.text_style())
            .y_label_style(style.text_style());
        mesh_bar.draw()?;
        chart_bar.draw_series(
            std::iter::successors(Some(range_min), |x| Some(step + x))
                .take_while(|x| *x <= range_max)
                .map(|v| {
                    Rectangle::new(
                        [(0., v - step / 2.), (1., v + step / 2.)],
                        COLOR_MAP
                            .get_color_normalized(v, map_range.start, map_range.end)
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
    pub(crate) max_log: Option<NonZeroUsize>,
    matrix: Vec<B>,
    raw: RawMapVisualizer<B>,
}

impl Default for ColorMapVisualizer<f64> {
    fn default() -> Self {
        Self {
            max_log: NonZeroUsize::new(100),
            matrix: Default::default(),
            raw: Default::default(),
        }
    }
}

impl ColorMapVisualizer<f64> {
    fn clear(&mut self) {
        self.matrix.clear();
        self.raw.color_range = Default::default();
    }

    pub fn fetch(
        &mut self,
        data: &[Complex64],
        proc: &mut Process,
        chunk_size: usize,
    ) -> &mut Self {
        puffin::profile_function!();
        self.clear();
        match self.max_log {
            Some(max) => {
                self.matrix.reserve(chunk_size * max.get());
                for d in data.rchunks(chunk_size).take(max.get()).rev() {
                    self.push(proc.proc(d));
                }
            }
            None => {
                for d in data.chunks(chunk_size) {
                    self.push(proc.proc(d));
                }
            }
        }
        self.raw.color_range.make_range_legal();
        self
    }

    pub fn update(
        &mut self,
        data: &[Complex64],
        proc: &mut Process,
        chunk_size: usize,
    ) -> &mut Self {
        match self.max_log {
            Some(_) => {
                self.fetch(data, proc, chunk_size);
            }
            None => {
                data.rchunks(chunk_size)
                    .next()
                    .map(|d| self.push(proc.proc(d)));
            }
        }
        self
    }

    fn push(&mut self, mut row: Vec<f64>) -> &mut Self {
        self.raw.update_range(&row);
        self.matrix.append(&mut row);
        self
    }

    pub fn draw_mat<DB: DrawingBackend>(
        &self,
        draw_area: DrawingArea<DB, Shift>,
        chunk_size: usize,
        style: Style,
    ) -> Result<impl Fn((i32, i32)) -> Option<(usize, usize)>, DrawingAreaErrorKind<DB::ErrorType>>
    {
        self.raw
            .draw_on(&self.matrix, &draw_area, chunk_size, style)
    }
    pub fn draw_mat_on_ui<'a>(
        &self,
        chunk_size: usize,
        ui: &'a egui::Ui,
    ) -> Result<
        impl Fn((i32, i32)) -> Option<(usize, usize)> + 'a,
        DrawingAreaErrorKind<<EguiBackend<'_> as DrawingBackend>::ErrorType>,
    > {
        puffin::profile_function!();
        self.draw_mat(
            EguiBackend::new(ui).into_drawing_area(),
            chunk_size,
            Style::from_ui(ui),
        )
    }
}
#[allow(unused)]
impl<B> ColorMapVisualizer<B> {
    pub fn set_color_range(&mut self, x: DrawRange<Range<B>>) -> &mut Self {
        self.raw.set_color_range(x);
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

//#[cfg(not(feature = "rayon"))]
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
    range: Range<f64>,
) {
    puffin::profile_scope!("iterator matrix elements");
    ctx.draw_series(
        data.enumerate()
            .flat_map(|(y, l)| l.iter().enumerate().map(move |(x, v)| (x, y, *v)))
            .map(|(x, y, v)| {
                Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    COLOR_MAP
                        .get_color_normalized(v, range.start, range.end)
                        .filled(),
                )
            }),
    )
    .expect("plotting rectangles");
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
