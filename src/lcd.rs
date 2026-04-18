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
use static_cell::StaticCell;

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

pub type ScreenSpi = Spi<'static, Blocking>;

static BACKLIGHT_TIMER: StaticCell<timer::Timer<'static, LowSpeed>> = StaticCell::new();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    Spi(SpiError),
    BacklightTimer(timer::Error),
    BacklightChannel(channel::Error),
    EmptyRect,
    CoordinatesOutOfRange,
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

pub struct Lcd {
    spi: ScreenSpi,
    dc: Output<'static>,
    cs: Output<'static>,
    _ledc: Ledc<'static>,
    backlight: channel::Channel<'static, LowSpeed>,
}

impl Lcd {
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
        })
    }

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

    pub fn set_brightness(&mut self, level: u8) {
        self.backlight.set_duty_hw(level.into());
    }

    pub fn fill_rect(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        color: u16,
    ) -> Result<(), Error> {
        if width == 0 || height == 0 {
            return Err(Error::EmptyRect);
        }

        let x1 = x
            .checked_add(width - 1)
            .ok_or(Error::CoordinatesOutOfRange)?;
        let y1 = y
            .checked_add(height - 1)
            .ok_or(Error::CoordinatesOutOfRange)?;

        self.set_window(x, y, x1, y1)?;
        self.write_repeated_color(color, width as usize * height as usize)?;

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

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), Error> {
        let [x0_hi, x0_lo] = x0.to_be_bytes();
        let [x1_hi, x1_lo] = x1.to_be_bytes();
        let [y0_hi, y0_lo] = y0.to_be_bytes();
        let [y1_hi, y1_lo] = y1.to_be_bytes();

        self.write_command(command::CASET, Some(&[x0_hi, x0_lo, x1_hi, x1_lo]))?;
        self.write_command(command::RASET, Some(&[y0_hi, y0_lo, y1_hi, y1_lo]))?;

        Ok(())
    }

    fn write_repeated_color(&mut self, color: u16, pixel_count: usize) -> Result<(), Error> {
        let color = color.to_be_bytes();
        let mut burst = [0_u8; 64];
        for pixel in burst.chunks_exact_mut(2) {
            pixel.copy_from_slice(&color);
        }

        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[command::RAMWR])?;
        self.dc.set_high();

        let mut remaining = pixel_count;
        let burst_pixels = burst.len() / 2;
        while remaining != 0 {
            let chunk_pixels = remaining.min(burst_pixels);
            self.spi.write(&burst[..chunk_pixels * 2])?;
            remaining -= chunk_pixels;
        }

        self.cs.set_high();
        Ok(())
    }
}
