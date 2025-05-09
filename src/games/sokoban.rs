#[allow(static_mut_refs)]

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_rp::{
    clocks::RoscRng, gpio::Output, pac::pwm::regs::En, spi::Spi
};

use embassy_usb::descriptor::descriptor_type::ENDPOINT;
use mipidsi::interface::SpiInterface;  
use mipidsi::models::ST7735s;

use embedded_graphics::{
    mono_font::{ascii::{FONT_10X20, FONT_6X10}, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, primitives::{
        PrimitiveStyle, Rectangle
    }, text::Text
};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};

use heapless::{
    Vec, spsc::Queue,
};

use rand::seq::SliceRandom;

use crate::INPUT_SIGNAL;
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::Input;

static mut OFFSET_X: i32 = 32;


pub struct Sokoban<'a> {
    player1: (u8, u8),
    player2: (u8, u8),
    level: u8,
    frame: &'a mut Vec<Vec<u8, 15>, 15>,
    destinations: &'a mut Vec<(u8, u8), 20>,
}

impl <'a> Sokoban<'a> {
    pub fn new(frame: &'a mut Vec<Vec<u8, 15>, 15>, destinations: &'a mut Vec<(u8, u8), 20>) -> Sokoban <'a> {
        Sokoban {
            player1: (0, 0),
            player2: (0, 0),
            level: 1,
            frame,
            destinations
        }
    } 
    pub fn init(&mut self) {
        match self.level {
            1 => {
                for i in 0..14 {
                    self.frame[i][0] = 1;
                    self.frame[0][i] = 1;
                    self.frame[i][13] = 1;
                    self.frame[13][i] = 1;
                }
                // 1 - wall
                // 2 - box
                self.frame[3][3] = 2;
                self.frame[3][6] = 2;
    
                self.destinations.push((8, 6)).unwrap();
                self.destinations.push((9, 6)).unwrap();
                self.player1 = (5, 3);
                self.player2 = (5, 5);
            }
            _ => {}
        }
    }
    
    async fn draw_init(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        unsafe {
            for destination in self.destinations.iter() {
                Rectangle::new(Point::new(destination.1 as i32 * 9, destination.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(screen)
                    .unwrap();
            }
            Rectangle::new(Point::new(self.player1.1 as i32 * 9, self.player1.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player2.1 as i32 * 9, self.player2.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
            for i in 0..14 {
                for j in 0..14 {
                    match self.frame[i][j] {
                        1 => {
                            // wall, yellow
                            Rectangle::new(Point::new(j as i32 * 9, i as i32 * 9 + OFFSET_X), Size::new(8, 8))
                                .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                                .draw(screen)
                                .unwrap();
                        }
                        2 => {
                            // box, brown
                            Rectangle::new(Point::new(j as i32 * 9, i as i32 * 9 + OFFSET_X), Size::new(8, 8))
                                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_BROWN))
                                .draw(screen)
                                .unwrap();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    fn handle_input(&mut self, input: &Input, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        match input {
            Input::Select => {
                
            }
            Input::Back => {
                
            }
            Input::Up => {
                self.move_player(1, -1, 0, screen);
            }
            Input::Down => {
                self.move_player(1, 1, 0, screen);
            }
            Input::Left => {
                self.move_player(1, 0, -1, screen);
            }
            Input::Right => {
                self.move_player(1, 0, 1, screen);
            } 
            Input::Up2 => {
                self.move_player(2, -1, 0, screen);
            }
            Input::Down2 => {
                self.move_player(2, 1, 0, screen);
            }
            Input::Left2 => {
                self.move_player(2, 0, -1, screen);
            } 
            Input::Right2 => {
                self.move_player(2, 0, 1, screen);
            }
            _ => {}
        }
    }
    
    fn move_player(&mut self, p: u8, x: i8, y: i8, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        let (player, color, other) = match p {
            1 => {
                (&mut self.player1, Rgb565::BLUE, self.player2)
            }
            2 => {
                (&mut self.player2, Rgb565::CSS_ORANGE, self.player1)  
            }
            _ => {
                (&mut self.player1, Rgb565::BLUE, self.player2)
            }
        };

        unsafe {
            if self.frame[(player.0 as i8 + x) as usize][(player.1 as i8 + y) as usize] == 0 && (player.0 + x as u8, player.1 + y as u8) != other {
                Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                self.frame[player.0 as usize][player.1 as usize] = 0;
                player.0 += x as u8;
                player.1 += y as u8;
                Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(screen)
                    .unwrap();
            } else if self.frame[(player.0 as i8 + x) as usize][(player.1 as i8 + y) as usize] == 2 && self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] == 0 {
                self.frame[player.0 as usize][player.1 as usize] = 0;
                self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] = 2;
                Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                player.0 += x as u8;
                player.1 += y as u8;
                Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(screen)
                    .unwrap();
                Rectangle::new(Point::new((player.1 as i32 + y as i32) * 9, (player.0 as i32 + x as i32) * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_BROWN))
                    .draw(screen)
                    .unwrap();
            }
            for destination in self.destinations.iter() {
                if self.frame[destination.0 as usize][destination.1 as usize] == 2 {
                    Rectangle::new(Point::new(destination.1 as i32 * 9, destination.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                        .draw(screen)
                        .unwrap();
                } else {
                    Rectangle::new(Point::new(destination.1 as i32 * 9, destination.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                        .draw(screen)
                        .unwrap();
                }
            }
            Rectangle::new(Point::new(self.player1.1 as i32 * 9, self.player1.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player2.1 as i32 * 9, self.player2.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
        }
    }

    pub async fn game_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        self.draw_init(screen).await;
        loop {
            let input = INPUT_SIGNAL.wait().await;
            self.handle_input(&input, screen);
            Timer::after(Duration::from_millis(100)).await;
            INPUT_SIGNAL.reset();
        }
    }
        
}