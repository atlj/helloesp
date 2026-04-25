#![no_std]

/// RGB565 color packed into a `u16`.
/// Bits 15–11: Red (5), 10–5: Green (6), 4–0: Blue (5).
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8) -> Color {
        Color {
            r: red,
            g: green,
            b: blue,
        }
    }
}
