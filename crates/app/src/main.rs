#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use color::color;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Config;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use graphics::Screen;
use graphics::geometry::Position2;
use graphics::geometry::validity::Valid;
use hardware::lcd::Lcd;
use log::info;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let spi_config = Config::default()
        .with_mode(Mode::_0)
        .with_write_bit_order(esp_hal::spi::BitOrder::MsbFirst)
        .with_frequency(Rate::from_hz(80_000_000));
    let screen_spi = esp_hal::spi::master::Spi::new(peripherals.SPI2, spi_config)
        .unwrap()
        .with_mosi(peripherals.GPIO13)
        .with_miso(peripherals.GPIO12)
        .with_sck(peripherals.GPIO14);

    let mut lcd = Lcd::new(
        screen_spi,
        peripherals.LEDC,
        peripherals.GPIO2,
        peripherals.GPIO15,
        peripherals.GPIO27,
    )
    .unwrap();

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    lcd.init().await.unwrap();
    lcd.set_brightness(100);

    let pos = Position2::<Valid>::UPPER_LEFT;
    let size = Lcd::SIZE;

    lcd.fill(pos, size, color!(#FFFFFF)).unwrap();

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        Timer::after(Duration::from_secs(1)).await;
        let pos = Position2::<Valid>::UPPER_LEFT;
        let size = Lcd::SIZE;

        lcd.fill(pos, size, color!(#FF00FF)).unwrap();

        Timer::after(Duration::from_secs(1)).await;
        let pos = Position2::<Valid>::UPPER_LEFT;
        let size = Lcd::SIZE;

        lcd.fill(pos, size, color!(#0000FF)).unwrap();

        Timer::after(Duration::from_secs(1)).await;
        let pos = Position2::<Valid>::UPPER_LEFT;
        let size = Lcd::SIZE;

        lcd.fill(pos, size, color!(#000000)).unwrap();
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
