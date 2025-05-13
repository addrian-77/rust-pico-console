#[allow(static_mut_refs)]

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_rp::{
    gpio::Output, spi::Spi
};
use mipidsi::interface::SpiInterface;  
use mipidsi::models::ST7735s;

use embedded_graphics::{
    mono_font::{ascii::{FONT_10X20, FONT_6X10}, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, primitives::{
        PrimitiveStyle, Rectangle
    }, text::Text
};
use embassy_time::{Duration, Timer};

use heapless::Vec;

use crate::{menu::selector::Menu, INPUT_SIGNAL};
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};

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
    
    fn handle_input(&mut self, input: &Input, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> bool {
        match input {
            Input::Select => {
                return true;
            }
            Input::Back => {
                return false;
            }
            Input::Up => {
                self.move_player(1, -1, 0, screen);
                return true;
            }
            Input::Down => {
                self.move_player(1, 1, 0, screen);
                return true;
            }
            Input::Left => {
                self.move_player(1, 0, -1, screen);
                return true;
            }
            Input::Right => {
                self.move_player(1, 0, 1, screen);
                return true;
            } 
            Input::Up2 => {
                self.move_player(2, -1, 0, screen);
                return true;
            }
            Input::Down2 => {
                self.move_player(2, 1, 0, screen);
                return true;
            }
            Input::Left2 => {
                self.move_player(2, 0, -1, screen);
                return true;
            } 
            Input::Right2 => {
                self.move_player(2, 0, 1, screen);
                return true;
            }
            _ => { return true }
        }
    }
    
    fn move_player(&mut self, p: u8, x: i8, y: i8, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        let (player, other) = match p {
            1 => {
                (&mut self.player1, self.player2)
            }
            2 => {
                (&mut self.player2, self.player1)  
            }
            _ => {
                (&mut self.player1, self.player2)
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
            } else if self.frame[(player.0 as i8 + x) as usize][(player.1 as i8 + y) as usize] == 2 && self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] == 0 {
                self.frame[player.0 as usize][player.1 as usize] = 0;
                self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] = 2;
                Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                player.0 += x as u8;
                player.1 += y as u8;
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
            if self.handle_input(&input, screen) == false {
                // create pause menu
                let mut pause_menu: Menu<'_> = Menu::init("Pause menu", &[MenuOption::Resume, MenuOption::Exit], screen);
                let result: MenuOption = pause_menu.menu_loop(screen).await;
                info!("obtained result... somehow?");
                match result {
                    MenuOption::Resume | MenuOption::None => {
                        self.redraw(screen).await;
                        Timer::after(Duration::from_millis(100)).await;
                        INPUT_SIGNAL.reset();
                    },
                    MenuOption::Exit => {
                        unsafe { CURRENT = 0 };
                        return;
                    }
                    _ => {}
                }
            }
            Timer::after(Duration::from_millis(100)).await;
            INPUT_SIGNAL.reset();
        }
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new(0, 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
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
}