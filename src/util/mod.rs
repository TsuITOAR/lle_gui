
mod show_stuff;

pub use show_stuff::*;

mod sync_stuff;

pub use sync_stuff::*;

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
