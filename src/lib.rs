#![warn(clippy::all, rust_2018_idioms)]

mod drawer;
mod property;

use std::{collections::BTreeMap, f64::consts::PI};

use drawer::DrawData;
use egui::{ColorImage, TextureHandle};
use lle::{num_complex::Complex64, num_traits::zero, LinearOp};
use property::Property;
type LleSolver<const LEN: usize> = lle::LleSolver<
    f64,
    [Complex64; LEN],
    lle::LinearOpAdd<(lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    Box<dyn Fn(Complex64) -> Complex64>,
>;
pub(crate) fn add_random<'a>(
    intensity: f64,
    sigma: f64,
    state: impl Iterator<Item = &'a mut Complex64>,
) {
    use rand::Rng;
    let mut rand = rand::thread_rng();
    state.for_each(|x| {
        *x += (Complex64::i() * rand.gen::<f64>() * 2. * PI).exp()
            * (-(rand.gen::<f64>() / sigma).powi(2) / 2.).exp()
            / ((2. * PI).sqrt() * sigma)
            * intensity
    })
}

fn default_add_random<'a>(state: impl Iterator<Item = &'a mut Complex64>) {
    add_random((2. * PI).sqrt() * 1e5, 1e5, state)
}

fn synchronize_properties<const L: usize>(
    props: &BTreeMap<String, Property<f64>>,
    engine: &mut LleSolver<L>,
) {
    engine.linear = (0, -(Complex64::i() * props["alpha"].value + 1.))
        .add((2, -Complex64::i() * props["linear"].value / 2.))
        .into();
    engine.constant = Complex64::from(props["pump"].value).into();
}

const LEN: usize = 128;
const DEFAULT_DRAW_RES: (usize, usize) = (640, 640);
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    // Example stuff:
    label: String,

    properties: BTreeMap<String, Property<f64>>,
    #[serde(skip)]
    engine: Option<LleSolver<LEN>>,
    #[serde(skip)]
    drawer: Option<DrawData>,
    #[serde(skip)]
    texture_cache_up: Option<TextureHandle>,
    #[serde(skip)]
    texture_cache_down: Option<TextureHandle>,
    #[serde(skip)]
    seed: Option<u32>,
    #[serde(skip)]
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),

            properties: vec![
                Property::new(-5., "alpha"),
                Property::new(3.94, "pump"),
                Property::new(-0.0444, "linear"),
            ]
            .into_iter()
            .map(|x| (x.label.clone(), x))
            .collect(),
            engine: None,
            drawer: None,
            texture_cache_up: None,
            texture_cache_down: None,
            seed: None,
            running: false,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            label,
            properties,
            engine,
            drawer,
            seed,
            running,
            texture_cache_up,
            texture_cache_down,
        } = self;
        let engine = engine.get_or_insert_with(|| {
            let mut init = [zero(); LEN];
            default_add_random(init.iter_mut());
            const STEP_DIST: f64 = 8e-4;
            const PUMP: f64 = 3.94;
            const LINEAR: f64 = -0.0444;
            const ALPHA: f64 = -5.;
            LleSolver::new(
                init,
                STEP_DIST,
                (0, -(Complex64::i() * ALPHA + 1.)).add((2, -Complex64::i() * LINEAR / 2.)),
                Box::new(|x: Complex64| Complex64::i() * x.norm_sqr())
                    as Box<dyn Fn(Complex64) -> Complex64>,
                Complex64::from(PUMP),
            )
        });
        synchronize_properties(properties, engine);
        let drawer = drawer.get_or_insert_with(|| DrawData::new(LEN, DEFAULT_DRAW_RES));

        if *running {
            use lle::Evolver;
            engine.evolve_n(100);
            drawer.push(engine.state().to_owned());
            drawer.update().unwrap();
        }

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });
        let mut reset = false;
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(label);
            });
            for p in properties.values_mut() {
                use egui::Slider;
                ui.add(
                    Slider::new(&mut p.value, p.range.0..=p.range.1)
                        .text(&p.label)
                        .smart_aim(false),
                );
            }
            let button_text = if *running { "running" } else { "waiting" };
            if ui.button(button_text).clicked() {
                *running = !*running;
            };
            reset = ui.button("reset").clicked();
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to(
                        "eframe",
                        "https://github.com/emilk/egui/tree/master/crates/eframe",
                    );
                    ui.label(".");
                });
            });
        });
        if reset {
            *self = Default::default();
            return;
        }
        const TEXTURE_FILTER: egui::TextureFilter = egui::TextureFilter::Linear;
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            let (size, buff_upper, buff_lower) = drawer.fetch().unwrap();
            ui.heading("diagram area");
            egui::warn_if_debug_build(ui);
            let max_size = ui.available_size();
            let half_max_size = egui::Vec2::new(max_size[0], max_size[1] * 0.5);
            let texture_cache_up = texture_cache_up.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "freq space",
                    ColorImage::from_rgba_unmultiplied([size.0, size.1], buff_upper),
                    TEXTURE_FILTER,
                )
            });
            texture_cache_up.set(
                ColorImage::from_rgba_unmultiplied([size.0, size.1], buff_upper),
                TEXTURE_FILTER,
            );
            ui.image(texture_cache_up, half_max_size);
            let texture_cache_down = texture_cache_down.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "freq space",
                    ColorImage::from_rgba_unmultiplied([size.0, size.1], buff_lower),
                    TEXTURE_FILTER,
                )
            });
            texture_cache_down.set(
                ColorImage::from_rgba_unmultiplied([size.0, size.1], buff_lower),
                TEXTURE_FILTER,
            );
            ui.image(texture_cache_down, half_max_size)
        });
        if *running {
            ctx.request_repaint();
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
