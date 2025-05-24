#[allow(static_mut_refs)]

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_rp::{
    clocks::RoscRng, gpio::Output, pac::{pwm::regs::En, rosc::Rosc}, spi::Spi
};

use embassy_usb::descriptor::descriptor_type::ENDPOINT;
use mipidsi::interface::SpiInterface;  
use mipidsi::models::ST7735s;

use embedded_graphics::{
    mono_font::{ascii::{FONT_10X20, FONT_6X10}, iso_8859_16, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, primitives::{
        PrimitiveStyle, Rectangle
    }, text::Text
};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};

use heapless::{
    Vec, spsc::Queue,
};

use rand::seq::SliceRandom;
use rand::*;

use crate::{menu::selector::Menu, INPUT_SIGNAL};
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};

const BRICK_HEIGHT: i16 = 4;
const BRICK_WIDTH: i16 = 8;
const OFFSET_Y: u8 = 50;
const PLAYER_WIDTH: i8 = 20;
const MAXSPEED: f32 = 1.0;
const MINSPEED: f32 = -1.0;

#[derive(Debug)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(PartialEq)]
pub enum Block {
    None,
    Green,
    Yellow,
    Red
}
pub struct Breakout<'a> {
    bricks: &'a mut Vec<Vec<Block,16>,36>,
    bricks_count: u16,
    walls: &'a mut Vec<bool,32>,
    lastwall: i16,
    balls: &'a mut Vec<(f32, i16, f32 ,i8, bool), 50>, // posx, posy, speedx, speedy, active
    powerups: &'a mut Vec<(u8, u8, bool, bool), 20>,
    player1_started: bool,
    player2_started: bool,
    level: u8,
    drawn_init: bool,
    player1_pos: u8,
    player1_pos_prev: i8,
    player2_pos: u8,
    player2_pos_prev: i8,
}

impl <'a> Breakout<'a> {
    pub fn new(bricks: &'a mut Vec<Vec<Block, 16>, 36>, walls: &'a mut Vec<bool, 32>, balls: &'a mut Vec<(f32, i16, f32, i8, bool), 50>, powerups: &'a mut Vec<(u8, u8, bool, bool), 20>) -> Breakout <'a> {
        Breakout { 
           bricks,
           bricks_count: 0,
           walls,
           lastwall: 0,
           balls,
           powerups,
           player1_started: false,
           player2_started: false,
           level: 1,
           drawn_init: false,
           player1_pos: 20,
           player1_pos_prev: 0,
           player2_pos: 60,
           player2_pos_prev: 0,
        }
    }
    pub fn init(&mut self) {
        self.balls.clear();
        self.powerups.clear();
        self.drawn_init = false;
        self.player1_started = false;
        self.player2_started = false;
        match self.level {
            1 => {
                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Yellow;
                    self.bricks[3][i] = Block::Yellow;
                    self.bricks[4][i] = Block::Green;
                    self.bricks[5][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 90;
                self.lastwall = 0;
            }
            2 => {
                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Yellow;
                    self.bricks[3][i] = Block::Yellow;
                    self.bricks[4][i] = Block::Green;
                    self.bricks[5][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 90;
                for i in 0..5 {
                    // info!(" i {}", (32 -i -1));
                    self.walls[i] = true;
                    self.walls[32 - i - 1] = true;
                } 
                self.lastwall = 4;
            }
            3 => {
                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Yellow;
                    self.bricks[3][i] = Block::Yellow;
                    self.bricks[4][i] = Block::Green;
                    self.bricks[5][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 90;
                for i in 0..8 {
                    self.walls[i] = true;
                    self.walls[32 - i - 1] = true;
                } 
                self.lastwall = 7;
            }
            4 => {
                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Red;
                    self.bricks[3][i] = Block::Yellow;
                    self.bricks[4][i] = Block::Yellow;
                    self.bricks[5][i] = Block::Yellow;
                    self.bricks[6][i] = Block::Green;
                    self.bricks[7][i] = Block::Green;
                    self.bricks[8][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 135;

                for i in 0..12 {
                    self.walls[i] = true;
                    self.walls[32 - i - 1] = true;
                } 
                self.lastwall = 11;
            }
            5 => {
                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Red;
                    self.bricks[3][i] = Block::Red;
                    self.bricks[4][i] = Block::Yellow;
                    self.bricks[5][i] = Block::Yellow;
                    self.bricks[6][i] = Block::Yellow;
                    self.bricks[7][i] = Block::Yellow;
                    self.bricks[8][i] = Block::Green;
                    self.bricks[9][i] = Block::Green;
                    self.bricks[10][i] = Block::Green;
                    self.bricks[11][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 180;

                for i in 0..15 {
                    self.walls[i] = true;
                    self.walls[32 - i - 1] = true;
                } 
                self.lastwall = 14;
            }
            _ => {

                for i in 0..16 {
                    self.bricks[0][i] = Block::Red;
                    self.bricks[1][i] = Block::Red;
                    self.bricks[2][i] = Block::Red;
                    self.bricks[3][i] = Block::Red;
                    self.bricks[4][i] = Block::Yellow;
                    self.bricks[5][i] = Block::Yellow;
                    self.bricks[6][i] = Block::Yellow;
                    self.bricks[7][i] = Block::Yellow;
                    self.bricks[8][i] = Block::Green;
                    self.bricks[9][i] = Block::Green;
                    self.bricks[10][i] = Block::Green;
                    self.bricks[11][i] = Block::Green;
                    self.walls[i] = false;
                }
                self.bricks_count = 180;
                for i in 0..15 {
                    self.walls[i] = true;
                    self.walls[32 - i - 1] = true;
                } 
                self.lastwall = 14;
            }
        }
    }

    fn update_frame(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> bool {
        let mut rng = RoscRng;
        for powerup in self.powerups.iter_mut() {
            Rectangle::new(Point::new(powerup.0 as i32,  powerup.1 as i32), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap(); 
            powerup.1 += 1;
            if powerup.1 == 147 {
                if powerup.0 >= self.player1_pos && powerup.0 <= self.player1_pos + 19 ||
                powerup.0 + 3 >= self.player1_pos && powerup.0 + 3 <= self.player1_pos + 19 {
                    powerup.3 = false;
                    if powerup.2 == true {
                        match self.balls.choose(&mut rng) {
                            Some(&t) => {
                                match self.balls.push((t.0, t.1, if t.2 - 0.2 >= MINSPEED { t.2 - 0.2 } else { MINSPEED }, t.3, true)) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                                match self.balls.push((t.0, t.1, if t.2 + 0.2 <= MAXSPEED { t.2 + 0.2 } else { MAXSPEED }, t.3, true)) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                            None => {}
                        }
                    } else {
                        match self.balls.push((self.player1_pos as f32 + 10.0, 148, -0.2, -1, true)) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        match self.balls.push((self.player1_pos as f32 + 10.0, 148, 0.2, -1, true)) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }

                } else if powerup.0 >= self.player2_pos && powerup.0 <= self.player2_pos + 19 ||
                powerup.0 + 3 >= self.player2_pos && powerup.0 + 3 <= self.player2_pos + 19 {
                    powerup.3 = false;
                    if powerup.2 == true {
                        match self.balls.choose(&mut rng) {
                            Some(&t) => {
                                match self.balls.push((t.0, t.1, if t.2 - 0.2 >= MINSPEED { t.2 - 0.2 } else { MINSPEED }, t.3, true)) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                                match self.balls.push((t.0, t.1, if t.2 + 0.2 <= MAXSPEED { t.2 - 0.2 } else { MAXSPEED }, t.3, true)) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                            None => {}
                        }
                    } else {
                        match self.balls.push((self.player2_pos as f32 + 10.0, 148, -0.2, -1, true)) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        match self.balls.push((self.player2_pos as f32 + 10.0, 148, 0.2, -1, true)) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
            }
            if powerup.1 > 160 {
                powerup.3 = false;
            }
        }
        self.powerups.retain(|powerup| powerup.3);
        for ball in self.balls.iter_mut() {
            if ball.1 > 160 {
                ball.4 = false;
            } else {
                // info!("ball.3 is {}", ball.3);
                // info!("drawing!");
                Rectangle::new(Point::new( ball.0 as i32 , ball.1 as i32), Size::new(2, 2))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                ball.0 += ball.2;   // x + speedx
                ball.1 += ball.3 as i16;   // y + speedy
                Rectangle::new(Point::new( ball.0 as i32 , ball.1 as i32), Size::new(2, 2))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
                    .draw(screen)
                    .unwrap();

                'checker: for x in 0..32 as usize {
                    if x < 16 {
                        if ball.0 as i16 >= x as i16 * BRICK_WIDTH && ball.0 as i16 <= (x as i16 + 1) * BRICK_WIDTH - 1 ||
                        ball.0 as  i16 + 1 >= x as i16 * BRICK_WIDTH && ball.0 as i16 + 1 <= (x as i16 + 1) * BRICK_WIDTH - 1 {
                            for y in 0..32 as usize {
                                if ball.1 == (y as i16 + 1) * BRICK_HEIGHT + OFFSET_Y as i16 - 1 {
                                    if self.bricks[y][x] != Block::None {
                                        // info!("collision at ball coords{} {}, computed {} {}", ball.0, ball.1, x*4, OFFSET_Y + (y * 4) as u8);
                                        // bottom collision
                                        ball.3 = -ball.3;
                                        self.bricks[y][x] = Block::None;
                                        self.bricks_count -= 1;
                                        if rng.gen_bool(0.5) {
                                            match self.powerups.push((x as u8 * 8, y as u8 * 4 + OFFSET_Y, rng.gen_bool(0.5), true)) {
                                                Ok(_) => {}
                                                Err(_) => {}
                                            }
                                        }
                                        Rectangle::new(Point::new( (x * 8) as i32 , (OFFSET_Y as usize + (y * 4)) as i32), Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as u32))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        break 'checker;
                                    }
                                } else if ball.1 == y as i16 * BRICK_HEIGHT + OFFSET_Y as i16 {
                                    if self.bricks[y][x] != Block::None {
                                        // top collision
                                        ball.3 = -ball.3;
                                        self.bricks[y][x] = Block::None;
                                        self.bricks_count -= 1;
                                        if rng.gen_bool(0.5) {
                                            match self.powerups.push((x as u8 * 8, y as u8 * 4 + OFFSET_Y, rng.gen_bool(0.5), true)) {
                                                Ok(_) => {}
                                                Err(_) => {}
                                            }
                                        }
                                        Rectangle::new(Point::new( (x * 8) as i32 , (OFFSET_Y as usize + (y * 4)) as i32), Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as u32))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        break 'checker;
                                    }
                                }
                            }
                        }
                    } 
                    if ball.1 >= x as i16 * BRICK_HEIGHT + OFFSET_Y as i16 && ball.1 <= (x as i16 + 1) * BRICK_HEIGHT + OFFSET_Y as i16 ||
                    ball.1 + 1 >= x as i16 * BRICK_HEIGHT + OFFSET_Y as i16 && ball.1 + 1 <= (x as i16 + 1 ) * BRICK_HEIGHT + OFFSET_Y as i16 {
                        for y in 0..16 as usize {
                            if ball.0 as i16 == (y as i16 + 1) * BRICK_WIDTH - 1 {
                                 if self.bricks[x][y] != Block::None {
                                    // right collision
                                    ball.2 = -ball.2;
                                    self.bricks[x][y] = Block::None;
                                    self.bricks_count -= 1;
                                    if rng.gen_bool(0.5) {
                                        match self.powerups.push((y as u8 * 8, x as u8 * 4 + OFFSET_Y, rng.gen_bool(0.5), true)) {
                                            Ok(_) => {}
                                            Err(_) => {}
                                        }
                                    }
                                    Rectangle::new(Point::new( (y * 8) as i32 , (OFFSET_Y as usize + (x * 4)) as i32), Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as u32))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                    break 'checker;
                                 }
                            } else if ball.0 as i16 == y as i16 * BRICK_WIDTH {
                                if self.bricks[x][y] != Block::None {
                                    // left collision
                                    ball.2 = -ball.2;
                                    self.bricks[x][y] = Block::None;
                                    self.bricks_count -= 1;
                                    if rng.gen_bool(0.5) {
                                        match self.powerups.push((y as u8 * 8, x as u8 * 4 + OFFSET_Y, rng.gen_bool(0.5), true)) {
                                            Ok(_) => {}
                                            Err(_) => {}
                                        }
                                    }
                                    Rectangle::new(Point::new( (y * 8) as i32 , (OFFSET_Y as usize + (x * 4)) as i32), Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as u32))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                    break 'checker;
                                }
                            } 
                        }
                    }
                }

                if ball.1 < 40 {
                    ball.3 = -ball.3;
                }
                if ball.1 == 148 {
                    // info!("ball.0 is {} {} {}", ball.0, self.player1_pos, self.player1_pos + 19);
                    if ball.0 as i16 >= self.player1_pos as i16 && ball.0 as i16 <= self.player1_pos as i16 + 19 {
                        // info!("true!");
                        let dist = ball.0 as i16 - self.player1_pos as i16;
                        if dist < 4 {
                            ball.2 = if ball.2 - 0.2 > MINSPEED { ball.2 - 0.2 } else { MINSPEED };
                        } else if dist < 8 {
                            ball.2 = if ball.2 - 0.1 > MINSPEED { ball.2 - 0.1 } else { MINSPEED };
                        } else if dist < 12 {

                        } else if dist < 16 {
                            ball.2 = if ball.2 + 0.1 > MAXSPEED { ball.2 + 0.1 } else { MAXSPEED };
                        } else {
                            ball.2 = if ball.2 + 0.2 > MAXSPEED { ball.2 + 0.2 } else { MAXSPEED };
                        }
                        ball.3 = -ball.3;
                    } else if ball.0 as i16 >= self.player2_pos as i16 && ball.0 as i16 <= self.player2_pos as i16 + 19 {
                        let dist = ball.0 as i16 - self.player2_pos as i16;
                        if dist < 4 {
                            ball.2 = if ball.2 - 0.2 > MINSPEED { ball.2 - 0.2 } else { MINSPEED };
                        } else if dist < 8 {
                            ball.2 = if ball.2 - 0.1 > MINSPEED { ball.2 - 0.1 } else { MINSPEED };
                        } else if dist < 12 {

                        } else if dist < 16 {
                            ball.2 = if ball.2 + 0.1 > MAXSPEED { ball.2 + 0.1 } else { MAXSPEED };
                        } else {
                            ball.2 = if ball.2 + 0.2 > MAXSPEED { ball.2 + 0.2 } else { MAXSPEED };
                        }
                        ball.3 = -ball.3;
                    }
                }
                if ball.0 <= 0.0 || ball.0 >= 128.0 {
                    ball.2 = -ball.2;
                }

                if ball.1 >= 99 && ball.1 <= 104 {
                    if ball.0 > 0.0 && ball.0 as i16 - 1 <= (self.lastwall + 1) * 4 {
                        ball.3 = -ball.3;
                    } else if ball.0 <= 128.0 && ball.0 as i16 + 1 >= (32 - self.lastwall - 1) * 4 - 1 {
                        ball.3 = -ball.3;
                    }
                }

                if ball.1 >= 100 && ball.1 <= 103 {
                    if ball.0 as i16 - 1 <= (self.lastwall + 1) * 4 + 1 {
                        ball.2 = -ball.2;
                    } else if ball.0 as i16 + 1 >= (32 - self.lastwall - 1) * 4 - 1 {
                        ball.2 = -ball.2;
                    }
                }

                if ball.1 >= 160 {
                    ball.4 = false;
                }
            }
        }
        self.balls.retain(|ball| ball.4);
        info!("bricks {} {}", self.bricks_count, self.bricks_count != 0);
        self.bricks_count != 0
    }

    async fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        if self.drawn_init == false {
            Rectangle::new(Point::new(0, 0), Size::new(128, 160))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            for i in 0..32 {
                for j in 0..16 {
                    if self.bricks[i][j] != Block::None {
                        // info!("drawing to {} {}", j * 4, i * 4 + OFFSET_Y as usize);
                        Rectangle::new(Point::new( (j * 8) as i32 , (OFFSET_Y as usize + (i * 4)) as i32), Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as  u32))
                            .into_styled(PrimitiveStyle::with_fill(
                                match self.bricks[i][j] {
                                    Block::Green => if (j +i) %2 == 0 { Rgb565::GREEN } else { Rgb565::CSS_SEA_GREEN },
                                    Block::Yellow => if (j + i) % 2 == 0 { Rgb565::YELLOW } else { Rgb565::CSS_ORANGE},
                                    Block::Red => if (j + i) % 2 == 0 { Rgb565::RED } else { Rgb565::CSS_DARK_RED },
                                    _ => Rgb565::RED
                                }
                            ))
                            .draw(screen)
                            .unwrap();
                    }
                }
            }
            Rectangle::new(Point::new(self.player1_pos as i32 , 150 as i32), Size::new(20, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player2_pos as i32 , 150 as i32), Size::new(20, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
            self.drawn_init = true;
        } else {
            for powerup in self.powerups.iter() {
                match powerup.2 {
                    true => {
                        Rectangle::new(Point::new(powerup.0 as i32,  powerup.1 as i32), Size::new(4, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_ORANGE))
                            .draw(screen)
                            .unwrap(); 
                    }
                    false => {
                        Rectangle::new(Point::new(powerup.0 as i32,  powerup.1 as i32), Size::new(4, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_CYAN))
                            .draw(screen)
                            .unwrap(); 
                    }
                }
            }
            for i in 0..32 {
                if self.walls[i] == true {
                    Rectangle::new(Point::new( (i * 4) as i32 , 101), Size::new(4, 3))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GRAY))
                        .draw(screen)
                        .unwrap();
                }
            }
            if self.player1_pos_prev > 0 {
                if self.player1_pos - 1 >= self.player2_pos && self.player1_pos - 1 <= self.player2_pos + PLAYER_WIDTH as u8 - 1 {
                    Rectangle::new(Point::new(self.player1_pos as i32 - 1, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                        .draw(screen)
                        .unwrap(); 
                } else {
                    Rectangle::new(Point::new(self.player1_pos as i32 - 1, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(screen)
                        .unwrap();
                }
                Rectangle::new(Point::new(self.player1_pos as i32 + PLAYER_WIDTH as i32 - 1, 150 as i32), Size::new(1, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                    .draw(screen)
                    .unwrap();
            } else if self.player1_pos_prev < 0 {
                Rectangle::new(Point::new(self.player1_pos as i32 , 150 as i32), Size::new(1, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                    .draw(screen)
                    .unwrap();
                if self.player1_pos + PLAYER_WIDTH as u8 >= self.player2_pos && self.player1_pos + PLAYER_WIDTH as u8 <= self.player2_pos + PLAYER_WIDTH as u8 - 1 {
                    Rectangle::new(Point::new(self.player1_pos as i32 + PLAYER_WIDTH as i32, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                        .draw(screen)
                        .unwrap();
                } else {
                    Rectangle::new(Point::new(self.player1_pos as i32 + PLAYER_WIDTH as i32, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(screen)
                        .unwrap();
                }
            }
            self.player1_pos_prev = 0;
            if self.player2_pos_prev > 0 {
                if self.player2_pos - 1 >= self.player1_pos && self.player2_pos - 1 <= self.player1_pos + PLAYER_WIDTH as u8 - 1 {
                    Rectangle::new(Point::new(self.player2_pos as i32 - 1, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                        .draw(screen)
                        .unwrap();    
                } else {
                    Rectangle::new(Point::new(self.player2_pos as i32 - 1, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(screen)
                        .unwrap();
                } 
                Rectangle::new(Point::new(self.player2_pos as i32 + PLAYER_WIDTH as i32 - 1, 150 as i32), Size::new(1, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                    .draw(screen)
                    .unwrap();
            } else if self.player2_pos_prev < 0 {
                Rectangle::new(Point::new(self.player2_pos as i32 , 150 as i32), Size::new(1, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                    .draw(screen)
                    .unwrap();
                if self.player2_pos + PLAYER_WIDTH as u8 >= self.player1_pos && self.player2_pos + PLAYER_WIDTH as u8 <= self.player1_pos + PLAYER_WIDTH as u8 - 1 {
                    Rectangle::new(Point::new(self.player2_pos as i32 + PLAYER_WIDTH as i32, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                        .draw(screen)
                        .unwrap();
                } else {
                    Rectangle::new(Point::new(self.player2_pos as i32 + PLAYER_WIDTH as i32, 150 as i32), Size::new(1, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(screen)
                        .unwrap();
                }
            }
            self.player2_pos_prev = 0;
        }
        
    }
    
    fn handle_input(&mut self, input: &Input) -> bool {
        match input {
            Input::Select => {
                return true
            }
            Input::Back => {
                return false
            }
            Input::Up => {
                if self.player1_started == false {
                    self.player1_started = true;
                    match self.balls.push((self.player1_pos as f32 + 8.0, 146, 0.0, -1, true)) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                    // info!("spawned ball 1");
                }
                return true
            }
            Input::Left => { 
                if self.player1_pos > 0 { 
                    self.player1_pos_prev = -1; 
                    self.player1_pos -= 1; 
                } 
                return true
            }
            Input::Right => { 
                if self.player1_pos < 108 { 
                    self.player1_pos_prev = 1; 
                    self.player1_pos += 1 
                }
                return true
            }
            Input::Up2 => {
                if self.player2_started == false {
                    self.player2_started = true;
                    match self.balls.push((self.player2_pos as f32 + 8.0, 146, 0.0, -1, true)) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                    // info!("spawned ball 2");
                }
                return true
            }
            Input::Left2 => { 
                if self.player2_pos > 0 { 
                    self.player2_pos_prev = -1; 
                    self.player2_pos -= 1 
                }
                return true
            }
            Input::Right2 => {
                if self.player2_pos < 108 { 
                    self.player2_pos_prev = 1; 
                    self.player2_pos += 1 
                }
                return true
            }
            Input::LeftLeft => {
                if self.player1_pos > 0 { 
                    self.player1_pos_prev = -1; 
                    self.player1_pos -= 1 
                }
                if self.player2_pos > 0 { 
                    self.player2_pos_prev = -1; 
                    self.player2_pos -= 1 
                }
                return true
            }
            Input::RightLeft => {
                if self.player1_pos < 108 { 
                    self.player1_pos_prev = 1; 
                    self.player1_pos += 1 
                }
                if self.player2_pos > 0 { 
                    self.player2_pos_prev = -1; 
                    self.player2_pos -= 1 
                }
                return true
            }
            Input::LeftRight => {
                if self.player1_pos > 0 { 
                    self.player1_pos_prev = -1; 
                    self.player1_pos -= 1 
                }
                if self.player2_pos < 108 { 
                    self.player2_pos_prev = 1; 
                    self.player2_pos += 1 
                }
                return true
            }
            Input::RightRight => {
                if self.player1_pos < 108 { 
                    self.player1_pos_prev = 1; 
                    self.player1_pos += 1;
                }
                if self.player2_pos < 108 { 
                    self.player2_pos_prev = 1; 
                    self.player2_pos += 1 
                }
                return true
            }
            _ => { return true }
        }
    }

    pub async fn game_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        loop {
            INPUT_SIGNAL.reset();
            match select(INPUT_SIGNAL.wait(), Timer::after(Duration::from_millis(10))).await {
                Either::First(input) => {
                    if self.handle_input(&input) == false {
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
                }
                _ => {}
            }
            if self.update_frame(screen) == false {
                self.level += 1;
                self.init();
            }
            self.draw(screen).await;
        }
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        
        
    }       
}
