//! This example receives incomming udp packets and turns an led on or off depending on the payload
//! In order to connect to the wifi network please create the following two files in the `src` folder:
//! WIFI_SSID.txt and WIFI_PASSWORD.txt
//! The files above should contain the exact ssid and password to connect to the wifi network. No newline characters or quotes.
//!
//! NOTE: This targets a RP Pico2 W or PR Pico2 WH. It does not work with the RP Pico2 board (non-wifi).
//!
//! How to run with a standard usb cable (no debug probe):
//! The pico has a builtin bootloader that can be used as a replacement for a debug probe (like an ST link v2).
//! Start with the usb cable unplugged then, while holding down the BOOTSEL button, plug it in. Then you can release the button.
//! Mount the usb drive (this will be enumerated as USB mass storage) then run the following command:
//! cargo run --bin 04_receive --release
//!
//! Troubleshoot:
//! `Error: "Unable to find mounted pico"`
//! This is because the pico is not in bootloader mode. You need to press down the BOOTSEL button when you plug it in and then release the button.
//! You need to do this every time you download firmware onto the device.

#![no_std]
#![no_main]

use core::{
    str::from_utf8,
    cell::RefCell,
};

use cyw43_pio::{
    PioSpi,
    RM2_CLOCK_DIVIDER,
};

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_executor::{self, Spawner};
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embassy_time::{Delay, Duration, Timer};
use embassy_rp::{
    gpio::{Level, Output}, pio::Pio, spi::Spi
};

use mipidsi::interface::SpiInterface;  
use mipidsi::models::ST7735s;
use mipidsi::options::Orientation;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
    primitives::{
        Rectangle, PrimitiveStyle
    }
};

mod init;
use init::udp;
mod irqs;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

// yellow 1 orange 2 red 29 black 38
// blue black purple
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("main!");
    const LOCAL_PORT: u16 = 7880;

    let p = embassy_rp::init(Default::default());

    info!("started");
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
    Text::new( "Initializing \nUDP socket", Point::new(20, 20), style)
        .draw(&mut screen).unwrap();

    let cyw_pwr = Output::new(p.PIN_23, Level::Low);
    let cyw_cs = Output::new(p.PIN_25, Level::High);
    let mut cyw_pio = Pio::new(p.PIO0, irqs::Irqs);
    let cyw_spi = PioSpi::new(
        &mut cyw_pio.common,
        cyw_pio.sm0,
        RM2_CLOCK_DIVIDER,
        cyw_pio.irq0,
        cyw_cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );
    let socket = udp::udp_init(&spawner, cyw_pwr, cyw_spi, LOCAL_PORT).await;
    info!("waiting for udp packets on port {}", LOCAL_PORT);
    
    Rectangle::new(Point::new(20, 10), Size::new(80, 16))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(&mut screen)
        .unwrap();
    Rectangle::new(Point::new(20, 26), Size::new(80, 16))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(&mut screen)
        .unwrap();
    Text::new( "Done!", Point::new(20, 20), style)
        .draw(&mut screen).unwrap();

    let mut buf: [u8; 1500] = [0; 1500];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, meta)) => match from_utf8(&buf[..len]) {
                Ok(s) => {
                    info!("received '{}' from {:?}", s, meta);
                    Rectangle::new(Point::new(20, 10), Size::new(80, 16))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut screen)
                        .unwrap();
                    Text::new(s, Point::new(20, 20), style)
                        .draw(&mut screen).unwrap();
                }
                Err(_e) => warn!("received {} bytes from", len),
            },
            Err(e) => error!("error receiving packet: {:?}", e),
        }
    }
}
