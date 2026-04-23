#![no_std]
#![feature(const_option_ops)]
#![feature(const_trait_impl)]

pub use color::Color;

pub mod geometry;
mod screen;

pub use screen::{DrawCommand, Screen};
