#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(hasher_prefixfree_extras)]

mod checkpoint;
mod config;
mod controller;
mod drawer;
mod easy_mark;
mod file;
mod lle_util;
mod notify;
mod preview;
mod property;
mod random;
mod util;
mod views;

pub const FONT: &str = "Arial";

mod construct;

mod app;

pub use crate::construct::App;
pub use drawer::FftSource;
