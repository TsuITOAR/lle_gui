#[cfg(target_arch = "wasm32")]
pub type App = crate::controller::App;

#[cfg(not(target_arch = "wasm32"))]
pub type App = crate::controller::cprt2::App;
