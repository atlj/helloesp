#![no_std]

/// RGB565 color packed into a `u16`.
/// Bits 15–11: Red (5), 10–5: Green (6), 4–0: Blue (5).
#[derive(Debug, Clone, Copy)]
pub struct Color(pub u16);

impl Color {
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Color {
        let red_5 = ((red as f32 * 15.0) / (u8::MAX as f32)) as u16;
        let green_6 = ((green as f32 * 31.0) / (u8::MAX as f32)) as u16;
        let blue_5 = ((blue as f32 * 15.0) / (u8::MAX as f32)) as u16;

        let color = (red_5 << 11) + (green_6 << 5) + blue_5;
        Color(color)
    }
}
