#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use color::color;
use color_core::Color;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Config;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use graphics::geometry::{Position2, Size2};
use graphics::shape::Rectangle;
use graphics::{DrawCommand, Screen};
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
        .with_frequency(Rate::from_mhz(80));
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
    lcd.set_brightness(255);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    // let color_data = (0..320).flat_map(move |row| {
    //     (0..480).map(move |col| {
    //         let red = ((row as f32 * u8::MAX as f32) / (320.0)) as u8;
    //         let green = ((col as f32 * u8::MAX as f32) / (480.0)) as u8;
    //         let blue = 128;
    //
    //         Color::new(red, green, blue)
    //     })
    // });

    // let command = DrawCommand::new(Position2::new(0, 0), Size2::new(480, 320), color_data);
    // let command = Lcd::validate_draw_command(command).unwrap();
    // lcd.draw(command).unwrap();

    lcd.fill(Position2::UPPER_LEFT, Lcd::SIZE, color!(#FFFFFF))
        .unwrap();

    let mut position = Lcd::validate_position(Position2::new(100, 100)).unwrap();
    let size = Lcd::validate_size(&position, Size2::new(35, 35)).unwrap();
    let mut bounces = 0;
    let mut dx: f32 = 3.7;
    let mut dy: f32 = 3.2;
    let colors = [
        color!(#000000),
        color!(#FF0000),
        color!(#0000FF),
        color!(#00FF00),
        color!(#FF6B6B),
        color!(#FFEAA7),
        // TODO fix this
        // color!(#55EFC4),
        color!(#0652DD),
        color!(#6C5CE7),
        color!(#00CEC9),
        color!(#E17055),
        color!(#2D3436),
    ];

    loop {
        let color = colors[bounces % colors.len()];
        let rect = Rectangle {
            position: position.clone(),
            size: size.clone(),
            corner_radius: 9999,
            fill: color,
        };
        lcd.draw_shape(rect).unwrap();

        Timer::after(Duration::from_millis(25)).await;

        // lcd.fill(position.clone(), size.clone(), color!(#FFFFFF))
        //     .unwrap();

        let new_x = (position.x as f32) + dx;
        let new_y = (position.y as f32) + dy;

        position.x = new_x as u16;
        position.y = new_y as u16;

        let mut did_bounce = false;

        if new_x <= 0.0 {
            dx = -dx;
            position.x = 0;
            did_bounce = true
        }

        if new_x + (size.width as f32) >= (Lcd::SIZE.width - 1) as f32 {
            dx = -dx;
            position.x = Lcd::SIZE.width.saturating_sub(size.width + 1);
            did_bounce = true
        }

        if new_y <= 0.0 {
            dy = -dy;
            position.y = 0;
            did_bounce = true
        }

        if new_y + (size.height as f32) >= (Lcd::SIZE.height - 1) as f32 {
            dy = -dy;
            position.y = Lcd::SIZE.height.saturating_sub(size.height + 1);
            did_bounce = true
        }

        if did_bounce {
            bounces += 1;
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
