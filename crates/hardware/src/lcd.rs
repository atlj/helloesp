//! Minimal LCD support for the ESP323248S035 board.
//!
//! This module is intentionally board-specific. It assumes the display wiring
//! and backlight setup described in `docs/hardware-slop.md` and exposes a small
//! API for panel init, brightness control, and simple solid-color drawing.

use byte_slice_cast::AsByteSlice;
use embassy_time::{Duration, Timer};
use esp_hal::Blocking;
use esp_hal::gpio::{DriveMode, Level, Output, OutputConfig};
use esp_hal::ledc::channel::{self, ChannelHW, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};
use esp_hal::ledc::{LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::peripherals::{GPIO2, GPIO15, GPIO27, LEDC};
use esp_hal::spi::Error as SpiError;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use graphics::geometry::Size2;
use graphics::geometry::validity::{Unchecked, Valid};
use graphics::{DrawCommand, Screen};
use static_cell::StaticCell;
use thiserror::Error as ThisError;

mod command {
    pub const SWRESET: u8 = 0x01;
    pub const SLPOUT: u8 = 0x11;
    pub const NORON: u8 = 0x13;
    pub const INVOFF: u8 = 0x20;
    pub const DISPON: u8 = 0x29;
    pub const CASET: u8 = 0x2A;
    pub const RASET: u8 = 0x2B;
    pub const RAMWR: u8 = 0x2C;
    pub const MADCTL: u8 = 0x36;
    pub const COLMOD: u8 = 0x3A;
    pub const PGC: u8 = 0xE0;
    pub const NGC: u8 = 0xE1;
    pub const CSCON: u8 = 0xF0;
}

const COLMOD_RGB565: u8 = 0x55;
const MADCTL_WIDE_INVERTED_RGB: u8 = 0xE0;

const POSITIVE_GAMMA: [u8; 14] = [
    0xF0, 0x09, 0x0B, 0x06, 0x04, 0x15, 0x2F, 0x54, 0x42, 0x3C, 0x17, 0x14, 0x18, 0x1B,
];
const NEGATIVE_GAMMA: [u8; 14] = [
    0xE0, 0x09, 0x0B, 0x06, 0x04, 0x03, 0x2B, 0x43, 0x42, 0x3B, 0x16, 0x14, 0x17, 0x1B,
];
const BACKLIGHT_PWM_FREQUENCY_HZ: u32 = 5_000;

/// Blocking SPI type used for panel transfers.
pub type ScreenSpi = Spi<'static, Blocking>;

static BACKLIGHT_TIMER: StaticCell<timer::Timer<'static, LowSpeed>> = StaticCell::new();

#[derive(Debug, Clone, Copy, PartialEq, ThisError)]
pub enum Error {
    /// An SPI transaction with the panel failed.
    #[error("panel SPI transfer failed: {0:?}")]
    Spi(SpiError),
    /// LEDC timer setup for the backlight failed.
    #[error("backlight timer configuration failed: {0:?}")]
    BacklightTimer(timer::Error),
    /// LEDC channel setup for the backlight failed.
    #[error("backlight channel configuration failed: {0:?}")]
    BacklightChannel(channel::Error),
    /// A rectangle with zero width or height was requested.
    #[error("rectangle width and height must be non-zero")]
    EmptyRect,
}

impl From<SpiError> for Error {
    fn from(value: SpiError) -> Self {
        Self::Spi(value)
    }
}

impl From<timer::Error> for Error {
    fn from(value: timer::Error) -> Self {
        Self::BacklightTimer(value)
    }
}

impl From<channel::Error> for Error {
    fn from(value: channel::Error) -> Self {
        Self::BacklightChannel(value)
    }
}

/// Minimal LCD driver for this board's display and PWM backlight.
pub struct Lcd {
    spi: ScreenSpi,
    dc: Output<'static>,
    cs: Output<'static>,
    _ledc: Ledc<'static>,
    backlight: channel::Channel<'static, LowSpeed>,
    brightness: u8,
}

impl Screen for Lcd {
    type Error = Error;

    const SIZE: Size2<Valid> = Size2::<Unchecked>::new(480, 320).unchecked_validate();

    /// Brightness goes from 0 (minimum) to 255 (maximum).
    fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
        self.backlight.set_duty_hw(brightness.into());
    }

    fn get_brightness(&self) -> u8 {
        self.brightness
    }

    fn draw(&mut self, command: DrawCommand<Valid>) -> Result<(), Self::Error> {
        let DrawCommand {
            at,
            size,
            color_data,
            ..
        } = command;

        self.set_window(at.x, at.y, at.x + size.width - 1, at.y + size.height - 1)?;

        self.cs.set_low();

        self.dc.set_low();
        self.spi.write(&[command::RAMWR])?;

        self.dc.set_high();

        let mut remaining = usize::from(size.width) * usize::from(size.height);
        let mut sink = [0u16; 8192];
        let mut did_log_underflow = false;

        while remaining > 0 {
            let chunk_len = sink.len().min(remaining);

            for item in sink.iter_mut().take(chunk_len) {
                *item = match color_data.next() {
                    Some(color) => color.0,
                    None => {
                        if !did_log_underflow {
                            log::error!(
                                "draw color_data ended early; padding remaining pixels with Color(0)"
                            );
                            did_log_underflow = true;
                        }

                        0
                    }
                };
            }

            self.spi.write(sink[..chunk_len].as_byte_slice())?;
            remaining -= chunk_len;
        }

        self.cs.set_high();

        Ok(())
    }
}

impl Lcd {
    /// Creates the LCD driver using this board's fixed DC, CS, and backlight
    /// pins.
    ///
    /// The backlight is configured as 5 kHz, 8-bit PWM to match the hardware
    /// notes.
    pub fn new(
        spi: ScreenSpi,
        ledc: LEDC<'static>,
        dc: GPIO2<'static>,
        cs: GPIO15<'static>,
        backlight: GPIO27<'static>,
    ) -> Result<Self, Error> {
        let mut ledc = Ledc::new(ledc);
        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

        let mut backlight_timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
        backlight_timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty8Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(BACKLIGHT_PWM_FREQUENCY_HZ),
        })?;
        let backlight_timer = BACKLIGHT_TIMER.init(backlight_timer);

        // Hardware doc says backlight uses channel 12. On ESP32 LEDC that maps to LS channel 4.
        let mut backlight = ledc.channel(channel::Number::Channel4, backlight);
        backlight.configure(channel::config::Config {
            timer: backlight_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })?;

        Ok(Self {
            spi,
            dc: Output::new(dc, Level::Low, OutputConfig::default()),
            cs: Output::new(cs, Level::High, OutputConfig::default()),
            _ledc: ledc,
            backlight,
            brightness: 128,
        })
    }

    /// Runs the panel initialization sequence for the ST7796-compatible display.
    pub async fn init(&mut self) -> Result<(), Error> {
        self.write_command(command::SWRESET, None)?;
        Timer::after(Duration::from_millis(100)).await;

        self.write_command(command::CSCON, Some(&[0xC3]))?;
        self.write_command(command::CSCON, Some(&[0x96]))?;
        self.write_command(command::COLMOD, Some(&[COLMOD_RGB565]))?;
        self.write_command(command::MADCTL, Some(&[MADCTL_WIDE_INVERTED_RGB]))?;
        self.write_command(command::PGC, Some(&POSITIVE_GAMMA))?;
        self.write_command(command::NGC, Some(&NEGATIVE_GAMMA))?;
        self.write_command(command::CSCON, Some(&[0x3C]))?;
        self.write_command(command::CSCON, Some(&[0x69]))?;
        self.write_command(command::INVOFF, None)?;
        self.write_command(command::NORON, None)?;
        self.write_command(command::SLPOUT, None)?;
        Timer::after(Duration::from_millis(120)).await;
        self.write_command(command::DISPON, None)?;
        Timer::after(Duration::from_millis(20)).await;

        Ok(())
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), Error> {
        let [x0_hi, x0_lo] = x0.to_be_bytes();
        let [x1_hi, x1_lo] = x1.to_be_bytes();
        let [y0_hi, y0_lo] = y0.to_be_bytes();
        let [y1_hi, y1_lo] = y1.to_be_bytes();

        self.write_command(command::CASET, Some(&[x0_hi, x0_lo, x1_hi, x1_lo]))?;
        self.write_command(command::RASET, Some(&[y0_hi, y0_lo, y1_hi, y1_lo]))?;

        Ok(())
    }

    fn write_command(&mut self, command: u8, data: Option<&[u8]>) -> Result<(), Error> {
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[command])?;

        if let Some(data) = data {
            self.dc.set_high();
            self.spi.write(data)?;
        }

        self.cs.set_high();
        Ok(())
    }
}
