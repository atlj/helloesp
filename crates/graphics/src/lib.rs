#![no_std]
#![feature(const_option_ops)]
#![feature(const_trait_impl)]

pub mod geometry;
mod screen;
pub mod shape;

pub use screen::{DrawCommand, Screen};
