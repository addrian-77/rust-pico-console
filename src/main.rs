#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(unused_variables)]
use core::cell::RefCell;

use embassy_executor::Spawner;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};
use embassy_time::{Delay, Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::gpio::{
    Level,
    Output,
};

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use mipidsi::interface::SpiInterface;  
use mipidsi::models::ST7735s;
use mipidsi::options::{Orientation, Rotation};

use embedded_graphics::{
    mono_font::{ascii::{FONT_10X20, FONT_6X10}, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};

// Use the logging macros provided by defmt.
use defmt::*;

// Import interrupts definition module
mod irqs;


#[embassy_executor::main]
async fn main(spawner: Spawner) {

    let p = embassy_rp::init(Default::default());
    
    // let mut led = Output::new(p.PIN_2, Level::Low);


    let mut screen_config = embassy_rp::spi::Config::default();
    screen_config.frequency = 32_000_000u32;
    screen_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
    screen_config.polarity = embassy_rp::spi::Polarity::IdleHigh;

    let miso = p.PIN_12;
    let mosi = p.PIN_11;
    let clk = p.PIN_10;
    
    let screen_rst = Output::new(p.PIN_14, Level::Low);
    let screen_dc = Output::new(p.PIN_15, Level::Low);
    let screen_cs = Output::new(p.PIN_13, Level::High);

    let spi = Spi::new_blocking(p.SPI1, clk, mosi, miso, screen_config);
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
    let mut display_spi = SpiDevice::new(&spi_bus, screen_cs);
    let mut buffer = [0_u8; 512];
    let di = SpiInterface::new(&mut display_spi, screen_dc, &mut buffer);
    let mut screen = mipidsi::Builder::new(ST7735s, di)
        .reset_pin(screen_rst)
        .orientation(Orientation::new())
        .init(&mut Delay)
        .unwrap();

    screen.clear(Rgb565::BLACK).unwrap();
    
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Text::new("Hello!", Point::new(20, 20), style)
        .draw(&mut screen)
        .unwrap();
    
    loop{
        Timer::after(Duration::from_millis(100)).await;
        // info!("aaa");
        
    }
}
