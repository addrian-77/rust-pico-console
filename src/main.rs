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
use embassy_net::udp::UdpSocket;
use embassy_sync::{blocking_mutex::{raw::{CriticalSectionRawMutex, NoopRawMutex}, Mutex}, signal::Signal};
use embassy_time::{Delay, Duration, Timer};
use embassy_rp::{
    gpio::{Level, Output}, pio::Pio, spi::Spi
};

use mipidsi::{interface::SpiInterface, options::Rotation};  
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

mod menu;
use menu::selector::Menu;

mod games;
use games::snake::Snake;

mod irqs;
use rust_pico_console::Input;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use heapless::{Vec, Deque};

// yellow 1 orange 2 red 29 black 38
// blue black purple

static mut CURRENT: i8 = 1;
static INPUT_SIGNAL: Signal<CriticalSectionRawMutex, Input> = Signal::new();
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
    let mut screen  = mipidsi::Builder::new(ST7735s, di)
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

    Rectangle::new(Point::new(15, 10), Size::new(90, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(&mut screen)
        .unwrap();

    spawner.spawn(receive(socket)).unwrap();

    info!("waiting for udp packets on port {}", LOCAL_PORT);

    loop {
        unsafe {
            match CURRENT {
                0 => {
                    let mut main_menu: Menu<'_> = Menu::init("Main menu", &[]);
                    main_menu.menu_loop(&mut screen).await;
                },
                1 => {
                    let mut frame = Vec::<u32, 32>::from_slice(&[0; 31]).unwrap();
                    let mut body_1 = Deque::<(u8, u8), 1025>::new();
                    let mut body_2 = Deque::<(u8, u8), 1025>::new();
                    let mut snake: Snake = Snake::new(&mut frame, &mut body_1, &mut body_2);
                    snake.init(&mut screen);
                    snake.snake_loop(&mut screen).await;
                },
                _ => continue,
            }
        }
        info!("returned from loop");
    }
}

#[embassy_executor::task]
async fn receive(socket: UdpSocket<'static>) {
    let mut buf: [u8; 1500] = [0; 1500];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, _meta)) => match from_utf8(&buf[..len]) {
                Ok(s) => {
                    let input: Input = match s {
                        "w" => Input::Up,
                        "a" => Input::Left,
                        "s" => Input::Down,
                        "d" => Input::Right,
                        "u" => Input::Up2,
                        "h" => Input::Left2,
                        "j" => Input::Down2,
                        "k" => Input::Right2,
                        "e" => Input::Select,
                        "q" => Input::Back,
                        _ => Input::Ignore,
                    };
                    if input != Input::Ignore {
                        INPUT_SIGNAL.signal(input);
                    }
                }
                Err(_e) => warn!("received {} bytes from", len),
            },
            Err(e) => error!("error receiving packet: {:?}", e),
        }
    }
}
