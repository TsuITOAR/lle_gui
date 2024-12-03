use lle::{num_complex::Complex64, LinearOp, NonLinearOp, NoneOp};

use crate::controller;

pub fn synchronize_properties<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL, Complex64>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add_linear_op((2, Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.constant = Complex64::from(props.pump.get_value()).into();
    engine.step_dist = props.step_dist.get_value();
}

pub fn synchronize_properties_no_pump<NL: NonLinearOp<f64>>(
    props: &controller::LleController,
    engine: &mut crate::controller::LleSolver<NL, NoneOp<f64>>,
) {
    puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add_linear_op((2, -Complex64::i() * props.linear.get_value() / 2.))
        .into();
    engine.step_dist = props.step_dist.get_value();
}

pub(crate) fn toggle_option<T: Default>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut ch = v.is_some();
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = T::default().into();
    } else if !ch {
        *v = None;
    }

    r
}

pub(crate) fn toggle_option_with<T, F>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    text: impl Into<egui::WidgetText>,
    f: F,
) -> egui::Response
where
    F: FnOnce() -> Option<T>,
{
    let mut ch = v.is_some();
    //let r = ui.checkbox(&mut ch, text);
    let r = ui.toggle_value(&mut ch, text);
    if v.is_none() && ch {
        *v = f();
    } else if !ch {
        *v = None;
    }

    r
}

pub fn show_profiler(show: &mut bool, ui: &mut egui::Ui) {
    if ui.toggle_value(show, "profile performance").clicked() {
        puffin::set_scopes_on(*show); // Remember to call this, or puffin will be disabled!
    }
    if *show {
        puffin_egui::profiler_ui(ui)
    }
}

pub(crate) fn allocate_remained_space(ui: &mut egui::Ui) -> egui::Ui {
    const MIN_WIDTH: f32 = 256.;
    const MIN_HEIGHT: f32 = 256.;
    let (_id, rect) = ui.allocate_space(
        (
            MIN_WIDTH
                .max(256. / ui.ctx().pixels_per_point())
                .max(ui.available_width()),
            MIN_HEIGHT
                .max(256. / ui.ctx().pixels_per_point())
                .max(ui.available_height()),
        )
            .into(),
    );
    ui.new_child(
        egui::UiBuilder::default()
            .max_rect(rect)
            .layout(*ui.layout()),
    )
}

pub(crate) fn show_vector<V: Config + Default>(ui: &mut egui::Ui, v: &mut Vec<V>) {
    ui.vertical(|ui| {
        let mut to_remove = None;
        for (i, value) in v.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                value.config(ui);
                ui.add_space(4.0);
                if ui.button("🗑").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        // 删除选中的元素
        if let Some(index) = to_remove {
            v.remove(index);
        }

        ui.add_space(8.0);

        // 添加新元素按钮
        if ui.button("➕").clicked() {
            v.push(V::default());
        }
    });
}

pub trait Config {
    fn config(&mut self, ui: &mut egui::Ui);
}

pub type FutureHandler<T> = Promise<T>;

#[cfg(not(target_arch = "wasm32"))]
pub fn try_poll<T: Send>(handle: &mut Option<FutureHandler<T>>) -> Option<T> {
    let h = handle.take()?;
    match h.try_take() {
        Ok(x) => Some(x),
        Err(e) => {
            *handle = Some(e);
            None
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn try_poll<T>(handle: &mut Option<FutureHandler<T>>) -> Option<T> {
    let h = handle.take()?;
    match h.try_take() {
        Ok(x) => Some(x),
        Err(e) => {
            *handle = Some(e);
            None
        }
    }
}

pub struct Promise<T: 'static>(poll_promise::Promise<T>);

#[cfg(not(target_arch = "wasm32"))]
mod runtime {
    use std::sync::LazyLock;
    use tokio::runtime::Runtime;
    pub static RUNTIME: LazyLock<Runtime> = LazyLock::new(default_runtime);
    fn default_runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("Can't initialize runtime")
    }
}

#[cfg(not(target_arch = "wasm32"))]
use runtime::RUNTIME;

#[cfg(not(target_arch = "wasm32"))]
impl<T: 'static + Send> Promise<T> {
    pub fn new(f: impl std::future::Future<Output = T> + Send + 'static) -> Self {
        let _guard = RUNTIME.enter();
        Self(poll_promise::Promise::spawn_async(f))
    }

    pub fn new_thread<F>(thread_name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let _guard = RUNTIME.enter();
        Self(poll_promise::Promise::spawn_thread(thread_name, f))
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: 'static> Promise<T> {
    pub fn new(f: impl std::future::Future<Output = T> + 'static) -> Self {
        Self(poll_promise::Promise::spawn_local(f))
    }

    pub fn new_web(_: impl Into<String>, f: impl FnOnce() -> T + 'static) -> Self {
        Self(poll_promise::Promise::spawn_local(async { f() }))
    }
}
impl<T: 'static> Promise<T> {
    pub fn try_take(self) -> Result<T, Self> {
        match self.0.try_take() {
            Ok(x) => Ok(x),
            Err(e) => Err(Self(e)),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn warn_message(t: impl ToString, hover_text: impl ToString, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(t.to_string())
            .small()
            .color(ui.visuals().warn_fg_color),
    )
    .on_hover_text(hover_text.to_string());
}
