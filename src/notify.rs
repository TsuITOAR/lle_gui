use std::sync::LazyLock;

use egui::mutex::Mutex;
use egui_notify::Toasts;

pub static TOASTS: LazyLock<Mutex<Toasts>> = LazyLock::new(|| Mutex::new(Toasts::new()));

pub trait ResultExt: Sized {
    type Output;
    fn notify(self, toasts: &mut Toasts) -> Option<Self::Output>;
    fn notify_global(self) -> Option<Self::Output> {
        self.notify(&mut TOASTS.lock())
    }
}

impl<T, E: std::error::Error> ResultExt for std::result::Result<T, E> {
    type Output = T;
    fn notify(self, toasts: &mut Toasts) -> Option<T> {
        match self {
            Ok(o) => Some(o),
            Err(e) => {
                toasts
                    .error(e.to_string())
                    .duration(std::time::Duration::from_secs(30).into());
                None
            }
        }
    }
}
