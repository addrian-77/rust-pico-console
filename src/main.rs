#![no_std]
#![no_main]

use core::{
    cell::{ RefCell, Cell} , net::Ipv4Addr, str::from_utf8
};

use cyw43_pio::{
    PioSpi,
    RM2_CLOCK_DIVIDER,
};

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_executor::{self, Spawner};
use embassy_net::{udp::UdpSocket, IpAddress, IpEndpoint};
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
use games::{
    snake::Snake, 
    spaceinvaders:: {SpaceInvaders, Enemy}, 
    sokoban:: Sokoban
};

mod irqs;
use rust_pico_console::{Input, MenuOption};

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use heapless::{Vec, Deque, spsc::Queue};

// yellow 1 orange 2 red 29 black 38
// blue black purple

static mut LAST_SELECTED: u8 = 1;
static mut CURRENT: u8 = 0;
static INPUT_SIGNAL: Signal<CriticalSectionRawMutex, Input> = Signal::new();
static mut LAST_REMOTE: Option<IpEndpoint> = None;
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
        Rectangle::new(Point::new( 0 , 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(&mut screen)
            .unwrap(); 
        unsafe {
            match CURRENT {
                0 => {
                    let mut main_menu: Menu<'_> = Menu::init("Main menu", &[MenuOption::Snake, MenuOption::SpaceInvaders, MenuOption::Sokoban, MenuOption::Debug], &mut screen);
                    let result: MenuOption = main_menu.menu_loop(&mut screen).await;
                    match result {
                        MenuOption::None => CURRENT = 0,
                        MenuOption::Snake => CURRENT = 1,
                        MenuOption::SpaceInvaders => CURRENT = 2,
                        MenuOption::Sokoban => CURRENT = 3,
                        MenuOption::Debug => CURRENT = 10,
                        _ => {}
                    }
                },
                1 => {
                    let mut frame = Vec::<u32, 32>::from_slice(&[0; 31]).unwrap();
                    let mut body_1 = Deque::<(u8, u8), 1025>::new();
                    let mut body_2 = Deque::<(u8, u8), 1025>::new();
                    let mut apples = Vec::<u32, 32>::from_slice(&[0; 31]).unwrap();
                    let mut snake: Snake = Snake::new(&mut frame, &mut body_1, &mut body_2, &mut apples);
                    snake.init(&mut screen);
                    snake.game_loop(&mut screen).await;
                },
                2 => {
                    let mut enemies = Vec::<Vec::<(Enemy, u8), 5>, 5>::from_iter(
                        [
                            Vec::from_iter([(Enemy::None, 0); 5].iter().cloned()),
                            Vec::from_iter([(Enemy::None, 0); 5].iter().cloned()),
                            Vec::from_iter([(Enemy::None, 0); 5].iter().cloned()),
                            Vec::from_iter([(Enemy::None, 0); 5].iter().cloned()),
                            Vec::from_iter([(Enemy::None, 0); 5].iter().cloned()),
                        ]
                        .iter()
                        .cloned()
                    );
                    let mut last_row = Vec::<(Enemy, u8, u8, bool), 5>::from_iter([(Enemy::None, 0, 0, false); 5].iter().cloned());
                    let mut enemy_projectiles = Vec::<(u8, u8, u8, bool), 5>::new();
                    let mut player1_projectiles = Vec::<(u8, u8, bool), 20>::new();
                    let mut player2_projectiles = Vec::<(u8, u8, bool), 20>::new();
                    let mut spaceinvaders: SpaceInvaders = SpaceInvaders::new(&mut enemies, &mut last_row, &mut enemy_projectiles, &mut player1_projectiles, &mut player2_projectiles);
                    spaceinvaders.init();
                    spaceinvaders.game_loop(&mut screen).await;
                },
                3 => {
                    let mut frame = Vec::<Vec::<u8, 15>, 15>::from_iter(
                        [
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                            Vec::from_iter([0; 15].iter().cloned()),
                        ]
                        .iter()
                        .cloned()
                    );
                    let mut destinations = Vec::<(u8, u8), 20>::new();
                    let mut sokoban: Sokoban = Sokoban::new(&mut frame, &mut destinations); 
                    sokoban.init();
                    sokoban.game_loop(&mut screen).await;
                }
                // debug, the coordinates are inverted
                10 => {
                    Rectangle::new(Point::new( 0 , 0), Size::new(10, 10))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_TURQUOISE))
                        .draw(&mut screen)
                        .unwrap();  

                        Rectangle::new(Point::new( 20 , 0), Size::new(10, 10))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                        .draw(&mut screen)
                        .unwrap();  


                        Rectangle::new(Point::new( 0 , 20), Size::new(10, 10))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_FLORAL_WHITE))
                        .draw(&mut screen)
                        .unwrap();  
                    loop {
                        match INPUT_SIGNAL.wait().await {
                            input => {
                                match input {
                                    Input::Back => {
                                        CURRENT = 0;
                                        break;
                                    }
                                    _ => {}
                                }    
                            }
                        }
                    }
                }
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
            Ok((len, meta)) => match from_utf8(&buf[..len]) {
                Ok(s) => {
                    let input: Input = match s {
                        "w" => Input::Up,
                        "a" => Input::Left,
                        "s" => Input::Down,
                        "d" => Input::Right,
                        "f" => Input::Right_Shoot,
                        "g" => Input::Left_Shoot,
                        "u" => Input::Up2,
                        "h" => Input::Left2,
                        "j" => Input::Down2,
                        "k" => Input::Right2,
                        "o" => Input::Right2_Shoot,
                        "p" => Input::Left2_Shoot,
                        "e" => Input::Select,
                        "q" => Input::Back,
                        _ => Input::Ignore,
                    };
                    if input != Input::Ignore {
                        INPUT_SIGNAL.signal(input);
                    }
                    unsafe {
                        if LAST_SELECTED != CURRENT {
                            LAST_REMOTE = Some(meta.endpoint);
                            if let Some(mut remote) = LAST_REMOTE {
                                info!("sending {}", CURRENT);
                                remote.port = 7881;
                                socket.send_to(&CURRENT.to_be_bytes(), remote).await.unwrap();
                            }
                            LAST_SELECTED = CURRENT;
                        }   
                    }
                }
                Err(_e) => warn!("received {} bytes from", len),
            },
            Err(e) => error!("error receiving packet: {:?}", e),
        }
    }
}
