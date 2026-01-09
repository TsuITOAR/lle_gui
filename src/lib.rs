#![warn(clippy::all, rust_2018_idioms)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(hasher_prefixfree_extras)]
#![feature(type_alias_impl_trait)]
#![feature(iter_array_chunks)]

mod checkpoint;
mod config;
mod controller;
mod drawer;
mod easy_mark;
mod file;
mod lle_util;
mod notify;
mod property;
mod random;
mod scouting;
mod util;
mod views;

pub const FONT: &str = "Arial";

mod construct;

mod app;

pub use crate::construct::App;
pub use drawer::FftSource;
