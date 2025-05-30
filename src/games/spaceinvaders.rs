use core::fmt;

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
    mono_font::{ascii::{FONT_10X20, FONT_6X10}, iso_8859_14::FONT_5X8, MonoTextStyle}, pixelcolor::Rgb565, prelude::*, primitives::{
        PrimitiveStyle, Rectangle
    }, text::Text
};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};

use heapless::{
    spsc::Queue, String, Vec
};

use rand::seq::SliceRandom;

use crate::{menu::selector::Menu, INPUT_SIGNAL};
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};

static mut OFFSET_X: u8 = 0;
static mut OFFSET_Y: u8 = 50;
const ENEMY_WIDTH: u32 = 8;
const ENEMY_HEIGHT: u32 = 3;
const BOSS_WIDTH: u32 = 40;
const BOSS_HEIGHT: u32 = 10;
const PLAYER_WIDTH: u32 = 6;
const PLAYER_HEIGHT: u32 = 3;
const SPACING: u8 = 15;

#[derive(Debug)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(PartialEq)]
pub enum Enemy {
    Class1,
    Class2,
    Class3,
    Boss1,
    Boss2,
    None,
}
pub struct SpaceInvaders<'a> {
    player1_pos: u8,
    player1_pos_prev: u8,
    player2_pos: u8,
    player2_pos_prev: u8,
    player1_cooldown: u8,
    player2_cooldown: u8,
    player1_projectiles: &'a mut Vec<(u8, u8, bool), 20>,
    player2_projectiles: &'a mut Vec<(u8, u8, bool), 20>,
    player1_lives: u8,
    player2_lives: u8,
    enemies: &'a mut Vec<Vec<(Enemy, u8),5>,5>,
    last_row: &'a mut Vec<(Enemy, u8, u8, bool), 5>,
    enemy_projectiles: &'a mut Vec<(u8, u8, u8, bool), 5>,
    projectile_cooldown: u8,
    enemies_count: u8,
    level: u8,
    score: u64,
    lowest_enemy: u8,
    lowest_height: u8,
    draw_init: bool,
    step: u16,
    speed: u16,
    shift: bool,
}

impl <'a> SpaceInvaders<'a> {
    pub fn new(enemies: &'a mut Vec<Vec<(Enemy, u8), 5>, 5> , 
    last_row: &'a mut Vec<(Enemy, u8, u8, bool), 5>,
    enemy_projectiles: &'a mut Vec<(u8, u8, u8, bool), 5>,
    player1_projectiles: &'a mut Vec<(u8, u8, bool), 20>,
    player2_projectiles: &'a mut Vec<(u8, u8, bool), 20>) -> SpaceInvaders <'a> {
        SpaceInvaders { 
            player1_pos: 54,
            player1_pos_prev: 0,
            player2_pos: 74,
            player2_pos_prev: 0,
            player1_cooldown: 0,
            player2_cooldown: 0,
            player1_projectiles,
            player2_projectiles,
            player1_lives: 3,
            player2_lives: 3,
            enemies,
            last_row,
            enemy_projectiles,
            projectile_cooldown: 50,
            enemies_count: 0,
            level: 1, 
            score: 0,
            lowest_enemy: 0,
            lowest_height: 0,
            draw_init: false,
            step: 0,
            speed: 300,
            shift: true, 
        }
    }
    pub fn init(&mut self) {
        self.draw_init = false;
        unsafe {
            OFFSET_X = 0;
            OFFSET_Y = 50;
        }
        self.speed = self.speed - self.level as u16 * 2;
        if self.level % 10 == 0 {
            self.enemies[0][1] = (Enemy::Boss2, 20);
            for i in 0..5 {
                self.enemies[2][i] = (Enemy::Class3, 3);
                self.last_row[i] = (Enemy::Class3, i as u8, 3, false);
            }
            unsafe {
                self.lowest_enemy = 2;
            }
            self.enemies_count = 6;
        } else if self.level % 5 == 0 {
            //boss level, first row boss and 1 row class3
            self.enemies[0][1] = (Enemy::Boss1, 20);
            for i in 0..5 {
                self.enemies[2][i] = (Enemy::Class3, 3);
                self.last_row[i] = (Enemy::Class3, i as u8, 3, false);
            }
            unsafe {
                self.lowest_enemy = 2;
            }
            self.enemies_count = 6;
        } else {
            match self.level % 5 {
                1 => {
                    //all easy enemies
                    for i in 0..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class1, 1); 
                        }
                    }
                    for i in 0..5 {
                        self.last_row[i] = (Enemy::Class1, i as u8, 4, false);
                    }
                    unsafe {
                        self.lowest_enemy = 3;
                    }
                    self.enemies_count = 20;
                }
                2 => {
                    //first row category2
                    for i in 0..5 {
                        self.enemies[0][i] = (Enemy::Class2, 2);
                        self.last_row[i] = (Enemy::Class1, i as u8, 4, false);
                    }
                    
                    for i in 1..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class1, 1);
                        }
                    }
                    unsafe {
                        self.lowest_enemy = 3;
                    }
                    self.enemies_count = 20;
                }
                3 => {
                    //first and second category2
                    for i in 0..5 {
                        self.enemies[0][i] = (Enemy::Class2, 2);
                        self.enemies[1][i] = (Enemy::Class2, 2);
                        self.last_row[i] = (Enemy::Class1, i as u8, 4, false);
                    }
                    
                    for i in 2..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class1, 1);
                        }
                    }
                    unsafe {
                        self.lowest_enemy = 3;
                    }
                    self.enemies_count = 20;
                }
                4 => {
                    // full class 2
                    for i in 0..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class2, 2); 
                        }
                    }
                    for i in 0..5 {
                        self.last_row[i] = (Enemy::Class2, i as u8, 4, false);
                    }
                    unsafe {
                        self.lowest_enemy = 3;
                    }
                    self.enemies_count = 20;
                }
                _ => {},
            }
        }
    }

    fn update_frame(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> (bool, bool) {
        //check collision
        // info!("lives {} {}", self.player1_lives, self.player2_lives);
        if self.player1_cooldown > 0 {
            self.player1_cooldown -= 1;
        }
        if self.player2_cooldown > 0 {
            self.player2_cooldown -= 1;
        }
        // info!("{} {} {} {} {}", self.enemies[0][0].1, self.enemies[0][1].1, self.enemies[0][2].1, self.enemies[0][3].1, self.enemies[0][4].1);
        // info!("{} {} {} {} {}", self.enemies[1][0].1, self.enemies[1][1].1, self.enemies[1][2].1, self.enemies[1][3].1, self.enemies[1][4].1);
        // info!("{} {} {} {} {}", self.enemies[2][0].1, self.enemies[2][1].1, self.enemies[2][2].1, self.enemies[2][3].1, self.enemies[2][4].1);
        // info!("{} {} {} {} {}", self.enemies[3][0].1, self.enemies[3][1].1, self.enemies[3][2].1, self.enemies[3][3].1, self.enemies[3][4].1);
        // info!("enemies left {}", self.enemies_count);
        unsafe {
            self.player1_projectiles.retain(|projectile| projectile.2);
            for projectile in self.player1_projectiles.iter_mut() {
                // info!("projectile coord {} {}", projectile.0, projectile.1);
                // info!("projectile max coords {} {}", projectile.0 + 1, projectile.1 + 3);
                // info!("first enemy coords {} {}", (OFFSET_X * SPACING as u8), OFFSET_Y);
                // info!("first enemy max coords {} {}", (OFFSET_X * SPACING as u8 + ENEMY_WIDTH as u8), (OFFSET_Y * SPACING as u8 + ENEMY_HEIGHT as u8));
                if projectile.1 > 45 + OFFSET_Y {
                    continue;
                }
                if projectile.1 < 40 {
                    projectile.2 = false;
                    continue;
                }
                if self.level % 5 == 0 {
                    if projectile.0 >= (1 + OFFSET_X) * SPACING as u8 && projectile.0 <= (1 + OFFSET_X) * SPACING as u8 + BOSS_WIDTH as u8- 1 ||
                    projectile.0 + 1 >= (1 + OFFSET_X) * SPACING as u8 && projectile.0 + 1 <= (1 + OFFSET_X) * SPACING as u8 + BOSS_WIDTH as u8 - 1 {
                        if projectile.1 >= OFFSET_Y && projectile.1 <= OFFSET_Y + BOSS_HEIGHT as u8 - 1 ||
                        projectile.1 + 3 >= OFFSET_Y && projectile.1 <= OFFSET_Y + BOSS_HEIGHT as u8 - 1 {
                            // boss hit
                            if self.enemies[0][1].0 != Enemy::None {
                                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                    .draw(screen)
                                    .unwrap();
                                if self.enemies[0][1].1 == 1 {
                                    self.enemies[0][1].0 = Enemy::None;
                                    self.score += match self.enemies[0][1].0 {
                                        Enemy::Boss1 => { 500 * self.level as u64 }
                                        Enemy::Boss2 => { 1000 * self.level as u64 }
                                        _ => { 0 }
                                    };
                                    Rectangle::new(Point::new(34, 12), Size::new(80, 8))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                    let mut temp: String<20> = String::new();
                                    fmt::write(&mut temp, format_args!("{}", self.score)).unwrap();
                                    Text::new( &temp, Point::new(35, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                        .draw(screen).unwrap();
                                    Rectangle::new(Point::new(1 + OFFSET_X as i32 * SPACING as i32, (1 as i32) * SPACING as i32 + OFFSET_Y as i32), Size::new(BOSS_WIDTH, BOSS_HEIGHT))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                } else {
                                    self.enemies[0][1].1 -= 1;
                                }
                                projectile.2 = false;
                                continue;
                            }
                        }
                    }
                }
                for x in 0..5 as usize {
                    if projectile.0 >= (x as u8 + OFFSET_X) * SPACING as u8 && projectile.0 <= (x as u8 + OFFSET_X) * SPACING as u8 + ENEMY_WIDTH as u8- 1 ||
                    projectile.0 + 1 >= (x as u8 + OFFSET_X) * SPACING as u8 && projectile.0 + 1 <= (x as u8 + OFFSET_X) * SPACING as u8 + ENEMY_WIDTH as u8 - 1 {
                        // collision with column x
                        for y in 0..4 as usize {
                            if projectile.1 >= (y as u8 * SPACING) + OFFSET_Y && projectile.1 <= (y as u8 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8 - 1 ||
                            projectile.1 + 3 >= (y as u8 * SPACING) + OFFSET_Y && projectile.1 <= (y as u8 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8 - 1 {
                                // collision with row y
                                if self.enemies[y][x].0 != Enemy::None {
                                    // info!("proj coordinates player 2 {} {}", projectile.0, projectile.1);
                                    // info!("enemy coords {} {}", (x as i32 + OFFSET_X as i32) * SPACING as i32, (y as i32) * SPACING as i32 + OFFSET_Y as i32);
                                    if self.enemies[y][x].1 == 1 {
                                        self.enemies[y][x].0 = Enemy::None;
                                        self.score += match self.enemies[y][x].0 {
                                            Enemy::Class1 => { 50 * self.level as u64 }
                                            Enemy::Class2 => { 75 * self.level as u64 }
                                            Enemy:: Class3 => { 100 * self.level as u64 }
                                            _ => { 0 }
                                        };
                                        Rectangle::new(Point::new(34, 12), Size::new(80, 8))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        let mut temp: String<20> = String::new();
                                        fmt::write(&mut temp, format_args!("{}", self.score)).unwrap();
                                        Text::new( &temp, Point::new(35, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                            .draw(screen).unwrap();
                                        Rectangle::new(Point::new((x as i32 + OFFSET_X as i32) * SPACING as i32, (y as i32) * SPACING as i32 + OFFSET_Y as i32), Size::new(ENEMY_WIDTH, ENEMY_HEIGHT))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        self.enemies[y][x].1 = 0;
                                        self.enemies_count -= 1;
                                        let mut posy = 4;
                                        loop {
                                            if self.enemies[posy][x].0 != Enemy::None {
                                                self.last_row[x] = (self.enemies[posy][x].0, x as u8, posy as u8, false);
                                                break;
                                            }
                                            if posy == 0 && self.enemies[posy][x].0 == Enemy::None {
                                                self.last_row[x] = (Enemy::None, 0, 0, false);
                                            }
                                            if posy > 0 {
                                                posy -= 1;
                                            } else {
                                                break;
                                            }
                                        }
                                    } else {
                                        self.enemies[y][x].1 -= 1;
                                    }
                                    projectile.2 = false;
                                    // exit
                                    break;
                                }
                            }
                        }
                        // exit anyway
                        break;
                    }
                }
            }

            self.player2_projectiles.retain(|projectile| projectile.2);
            for projectile in self.player2_projectiles.iter_mut() {
                // info!("projectile coord {} {}", projectile.0, projectile.1);
                // info!("projectile max coords {} {}", projectile.0 + 1, projectile.1 + 3);
                // info!("first enemy coords {} {}", (OFFSET_X * SPACING as u8), OFFSET_Y);
                // info!("first enemy max coords {} {}", (OFFSET_X * SPACING as u8 + ENEMY_WIDTH as u8), (OFFSET_Y * SPACING as u8 + ENEMY_HEIGHT as u8));
                if projectile.1 > 45 + OFFSET_Y {
                    continue;
                }
                if projectile.1 < 40 {
                    projectile.2 = false;
                    continue;
                }
                if self.level % 5 == 0 {
                    if projectile.0 >= (1 + OFFSET_X) * SPACING as u8 && projectile.0 <= (1 + OFFSET_X) * SPACING as u8 + BOSS_WIDTH as u8- 1 ||
                    projectile.0 + 1 >= (1 + OFFSET_X) * SPACING as u8 && projectile.0 + 1 <= (1 + OFFSET_X) * SPACING as u8 + BOSS_WIDTH as u8 - 1 {
                        if projectile.1 >= OFFSET_Y && projectile.1 <= OFFSET_Y + BOSS_HEIGHT as u8 - 1 ||
                        projectile.1 + 3 >= OFFSET_Y && projectile.1 <= OFFSET_Y + BOSS_HEIGHT as u8 - 1 {
                            // boss hit
                            if self.enemies[0][1].0 != Enemy::None {
                                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                    .draw(screen)
                                    .unwrap();
                                if self.enemies[0][1].1 == 1 {
                                    self.enemies[0][1].0 = Enemy::None;
                                    self.score += match self.enemies[0][1].0 {
                                        Enemy::Boss1 => { 500 * self.level as u64 }
                                        Enemy::Boss2 => { 1000 * self.level as u64 }
                                        _ => { 0 }
                                    };
                                    Rectangle::new(Point::new(34, 12), Size::new(80, 8))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                    let mut temp: String<20> = String::new();
                                    fmt::write(&mut temp, format_args!("{}", self.score)).unwrap();
                                    Text::new( &temp, Point::new(35, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                        .draw(screen).unwrap();
                                    Rectangle::new(Point::new(OFFSET_X as i32 * SPACING as i32, (1 as i32) * SPACING as i32 + OFFSET_Y as i32), Size::new(BOSS_WIDTH, BOSS_HEIGHT))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                } else {
                                    self.enemies[0][1].1 -= 1;
                                }
                                projectile.2 = false;
                                continue;
                            }
                        }
                    }
                }
                for x in 0..5 as usize {
                    if projectile.0 >= (x as u8 + OFFSET_X) * SPACING as u8 && projectile.0 <= (x as u8 + OFFSET_X) * SPACING as u8 + ENEMY_WIDTH as u8- 1 ||
                    projectile.0 + 1 >= (x as u8 + OFFSET_X) * SPACING as u8 && projectile.0 + 1 <= (x as u8 + OFFSET_X) * SPACING as u8 + ENEMY_WIDTH as u8 - 1 {
                        // collision with column x
                        for y in 0..4 as usize {
                            if projectile.1 >= (y as u8 * SPACING) + OFFSET_Y && projectile.1 <= (y as u8 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8 - 1 ||
                            projectile.1 + 3 >= (y as u8 * SPACING) + OFFSET_Y && projectile.1 <= (y as u8 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8 - 1 {
                                // collision with row y
                                if self.enemies[y][x].0 != Enemy::None {
                                    // info!("proj coordinates player 2 {} {}", projectile.0, projectile.1);
                                    Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                        .draw(screen)
                                        .unwrap();
                                    if self.enemies[y][x].1 == 1 {
                                        self.enemies[y][x].0 = Enemy::None;
                                        self.score += match self.enemies[y][x].0 {
                                            Enemy::Class1 => { 50 * self.level as u64 }
                                            Enemy::Class2 => { 75 * self.level as u64 }
                                            Enemy:: Class3 => { 100 * self.level as u64 }
                                            _ => { 0 }
                                        };
                                        Rectangle::new(Point::new(34, 12), Size::new(80, 8))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        let mut temp: String<20> = String::new();
                                        fmt::write(&mut temp, format_args!("{}", self.score)).unwrap();
                                        Text::new( &temp, Point::new(35, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                            .draw(screen).unwrap();
                                        Rectangle::new(Point::new((x as i32 + OFFSET_X as i32) * SPACING as i32, (y as i32) * SPACING as i32 + OFFSET_Y as i32), Size::new(ENEMY_WIDTH, ENEMY_HEIGHT))
                                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                            .draw(screen)
                                            .unwrap();
                                        self.enemies[y][x].1 = 0;
                                        self.enemies_count -= 1;
                                        let mut posy = 4;
                                        loop {  
                                            if self.enemies[posy][x].0 != Enemy::None {
                                                self.last_row[x] = (self.enemies[posy][x].0, x as u8, posy as u8, false);
                                                break;
                                            }
                                            if posy == 0 && self.enemies[posy][x].0 == Enemy::None {
                                                self.last_row[x] = (Enemy::None, 0, 0, false);
                                            }
                                            if posy > 0 {
                                                posy -= 1;
                                            } else {
                                                break;
                                            }
                                        }
                                    } else {
                                        self.enemies[y][x].1 -= 1;
                                    }
                                    projectile.2 = false;
                                    // exit
                                    break;
                                }
                            }
                        }
                        // exit anyway
                        break;
                    }
                }
            }

            // check enemy projectiles collision with player ships
        }
        // info!("reached");
        // info!("LAST ROW");
        // for i in 0..5 {
        //     info!("last row {}, {}", i, self.last_row[i].3);
        // }
        'lowest: for i in 0..4  {
            for j in 0..5  {
                // info!("i j {} {}",i ,j);
                if self.enemies[3 - i][4 - j].0 != Enemy::None {
                    self.lowest_height = match self.enemies[3 - i][4 - j].0 {
                        Enemy::Boss1 | Enemy::Boss2 => { BOSS_HEIGHT as u8 }
                        _ => { ENEMY_HEIGHT as u8 }
                    };
                    self.lowest_enemy = 3 - i as u8;
                    // info!("lowest {}", self.lowest_enemy);
                    break 'lowest;
                }
            }
        }
        // info!("enemies: {}", self.enemies_count);
        if self.enemies_count == 0 {
            self.level += 1;
            self.init();
        }

        if self.projectile_cooldown == 0 {
            match self.choose_enemy() as usize {
                10 => {}
                t => { 
                    self.last_row[t].3 = true;
                    self.projectile_cooldown = 100;
                }
            }
        } else {
            self.projectile_cooldown -= 1;
        }
        // info!("active projectiles {}", self.enemy_projectiles.len());
        // for item in self.last_row.iter() {
        //     match item.0 {
        //         Enemy::Class1 => {
        //             info!("Class1 {}", item.3);
        //         } 
        //         Enemy::Class2 => {
        //             info!("Class2 {}", item.3);
        //         } 
        //         Enemy::Class3 => {
        //             info!("Class3 {}", item.3);
        //         } 
        //         Enemy::None => {
        //             info!("None {}", item.3);
        //         }
        //         _ =>{}
        //     }
        // }

        for i in 0..self.enemy_projectiles.len() {
            // info!("enemy proj coords {} {}", self.enemy_projectiles[i].0, self.enemy_projectiles[i].1);
            self.enemy_projectiles[i].1 += 1;
            if self.enemy_projectiles[i].1 >= 146 && self.enemy_projectiles[i].1 <= 149 {
                if self.player1_lives > 0 {
                    if self.enemy_projectiles[i].0 + 1 >= self.player1_pos && self.enemy_projectiles[i].0 <= self.player1_pos + PLAYER_WIDTH as u8 - 1 {
                        self.last_row[self.enemy_projectiles[i].2 as usize].3 = false;
                        if self.player1_lives > 0 {
                            self.player1_lives -= 1;
                            Rectangle::new(Point::new(107, 4), Size::new(20, 6))
                                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                .draw(screen)
                                .unwrap();
                            let mut temp: String<20> = String::new();
                            fmt::write(&mut temp, format_args!("x {}", self.player1_lives)).unwrap();
                            Text::new( &temp, Point::new(110, 9), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                .draw(screen).unwrap();
                            if self.player1_lives == 0 {
                                Rectangle::new(Point::new(self.player1_pos as i32, 150), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                    .draw(screen)
                                    .unwrap();
                            }
                        }
                        self.enemy_projectiles[i].3 = false;
                        Rectangle::new(Point::new(self.enemy_projectiles[i].0 as i32, self.enemy_projectiles[i].1 as i32 - 1), Size::new(2, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                            .draw(screen)
                            .unwrap();
                    } 
                }

                if self.player2_lives > 0 {
                    if self.enemy_projectiles[i].0 + 1 >= self.player2_pos && self.enemy_projectiles[i].0 <= self.player2_pos + PLAYER_WIDTH as u8 - 1 {
                        self.last_row[self.enemy_projectiles[i].2 as usize].3 = false;
                        if self.player2_lives > 0 {
                            self.player2_lives -= 1;
                            Rectangle::new(Point::new(107, 12), Size::new(20, 6))
                                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                .draw(screen)
                                .unwrap();
                            let mut temp: String<20> = String::new();
                            fmt::write(&mut temp, format_args!("x {}", self.player2_lives)).unwrap();
                            Text::new(&temp, Point::new(110, 17), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                                .draw(screen).unwrap();
                            if self.player2_lives == 0 {
                                info!("0 lives");
                                Rectangle::new(Point::new(self.player2_pos as i32, 150), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                                    .draw(screen)
                                    .unwrap();
                            }
                        }
                        self.enemy_projectiles[i].3 = false;
                        Rectangle::new(Point::new(self.enemy_projectiles[i].0 as i32, self.enemy_projectiles[i].1 as i32 - 1), Size::new(2, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                            .draw(screen)
                            .unwrap();
                    } 
                }
            }
            if self.enemy_projectiles[i].1 > 160 {
                self.enemy_projectiles[i].3 = false;
                self.last_row[self.enemy_projectiles[i].2 as usize].3 = false;
                // info!("projectile out of scope");
            }
        }
        self.enemy_projectiles
            .retain(|active| active.3);   
        
        for projectile in self.player1_projectiles.iter_mut() {
            projectile.1 -= 1;
        }
        self.player1_projectiles
            .retain(|&(_, y, _)| y >= 60);
    
        for projectile in self.player2_projectiles.iter_mut() {
            // info!("{} {}", projectile.0, projectile.1);
            projectile.1 -= 1;
        }
        self.player2_projectiles
            .retain(|&(_, y, _)| y >= 60);
        

        unsafe {
            // info!("low {}", self.lowest_enemy * SPACING + OFFSET_Y);
            if self.lowest_enemy * SPACING + OFFSET_Y + self.lowest_height >= 150 {
                return (false, false)
            }
        }
        if self.player1_lives == 0 && self.player2_lives == 0 {
            return (false, false)
        }
        // shift enemies every X frames
        if self.step > 0 {
            self.step = self.step - 1;
            (false, true)
        } else {
            // shift enemies
            unsafe {
                match self.shift {
                    true => {
                        if OFFSET_X < 4 {
                            OFFSET_X += 1;
                        } else {
                            OFFSET_Y += 10;
                            self.shift = false;
                        }
                    }
                    false => {
                        if OFFSET_X > 0 {
                            OFFSET_X -= 1;
                        } else {
                            OFFSET_Y += 10;
                            self.shift = true;
                        }
                    }
                }
            }
            self.step = self.speed;
            (true, true)
        }

    }

    fn choose_enemy(&mut self) -> u8 {
        let available: Vec<&(Enemy, u8, u8, bool), 5> = self.last_row.iter().filter(|active_projectile| active_projectile.3 == false && active_projectile.0 != Enemy::Class1 && active_projectile.0 != Enemy::None).collect();
        // info!("AAAAAAAAAAAAAAAa");
        // info!("available enemies {}", available.len());
        let mut rng = RoscRng;
        match available.choose(&mut rng) {
            Some(t) => {
                unsafe {
                    if t.0 == Enemy::Boss1 || t.0 == Enemy::Boss2 {
                        self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 1, (t.2 * SPACING) + OFFSET_Y - 30, t.1,  true)).unwrap();
                        self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 13, (t.2 * SPACING) + OFFSET_Y - 25, t.1,  true)).unwrap();
                        self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 26, (t.2 * SPACING) + OFFSET_Y - 25, t.1,  true)).unwrap();
                        self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 38, (t.2 * SPACING) + OFFSET_Y - 30, t.1,  true)).unwrap();
                    } else {
                        self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 1, (t.2 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8, t.1,  true)).unwrap();
                    }
                }
                t.1
            },
            None => {
                10
            }
        }
    }

    async fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> bool {
        if self.draw_init == false {
             Rectangle::new(Point::new( 0 , 0), Size::new(128, 160))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            let mut temp: String<20> = String::new();
            fmt::write(&mut temp, format_args!("Level: {}", self.level)).unwrap();
            Text::new( &temp, Point::new(0, 8), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                .draw(screen).unwrap();
            temp.clear();
            fmt::write(&mut temp, format_args!("Score: {}", self.score)).unwrap();
            Text::new(&temp, Point::new(0, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                .draw(screen).unwrap();
            self.draw_init = true;

            Rectangle::new(Point::new( 100 , 5), Size::new(5, 5))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();

            Rectangle::new(Point::new( 100 , 13), Size::new(5, 5))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
            temp.clear();
            fmt::write(&mut temp, format_args!("x {}", self.player1_lives)).unwrap();
            Text::new( &temp, Point::new(110, 9), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                .draw(screen).unwrap();
            temp.clear();
            fmt::write(&mut temp, format_args!("x {}", self.player2_lives)).unwrap();
            Text::new(&temp, Point::new(110, 17), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
                .draw(screen).unwrap();
            
        }
        
        let aux = self.update_frame(screen);
        if aux.0 == true {
            Rectangle::new(Point::new( 0 , 50), Size::new(128, 150))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            for i in 0..4 as u8 {
                for j in 0..5 as u8 {
                    // info!("i j {} {}",i ,j);
                    draw_enemy(j, i, &self.enemies[i as usize][j as usize].0, screen);
                }
            }
        }
        for projectile in self.player1_projectiles.iter() {
            if projectile.2 == true {    
                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 + 1), Size::new(2, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
                    .draw(screen)
                    .unwrap();    
            }
        }
        for projectile in self.player2_projectiles.iter() {
            if projectile.2 == true {    
                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 + 1), Size::new(2, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
                Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                    .draw(screen)
                    .unwrap();    
            }
        }
        for projectile in self.enemy_projectiles.iter() {
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 - 1), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
                .draw(screen)
                .unwrap();    
        }
        if self.player1_lives > 0 {
            Rectangle::new(Point::new(self.player1_pos_prev as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player1_pos as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
        }
        if self.player2_lives > 0 {
            Rectangle::new(Point::new(self.player2_pos_prev as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player2_pos as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
        }
        aux.1
    }
    
    fn handle_input(&mut self, input: &Input) -> bool {
        match input {
            Input::Select => {
                return true
            }
            Input::Back => {
                return false
            }
            Input::Left => { 
                if self.player1_pos > 0 && self.player1_lives > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos -= 1; 
                } 
                return true
            }
            Input::Right => { 
                if self.player1_pos < 128 && self.player1_lives > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos += 1 
                }
                return true
            }
            Input::Up => {
                if self.player1_cooldown == 0 && self.player1_lives > 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                return true
            }
            Input::Right_Shoot => {
                if self.player1_cooldown == 0 && self.player1_lives > 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                if self.player1_pos < 128 && self.player1_lives > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos += 1 
                }
                return true;
            }
            Input::Left_Shoot => {
                if self.player1_cooldown == 0 && self.player1_lives > 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                if self.player1_pos > 0 && self.player1_lives > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos -= 1; 
                }
                return true;
            }
            Input::Left2 => { 
                if self.player2_pos > 0 && self.player2_lives > 0 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos -= 1 
                }
                return true
            }
            Input::Right2 => {
                if self.player2_pos < 128 && self.player2_lives > 0 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos += 1 
                }
                return true
            }
            Input::Up2 => { 
                if self.player2_cooldown == 0 && self.player2_lives > 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                return true
            }
            Input::Right2_Shoot => {
                if self.player2_cooldown == 0 && self.player2_lives > 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                if self.player2_pos < 128 && self.player2_lives > 0 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos += 1 
                }
                return true;
            }
            Input::Left2_Shoot => {
                if self.player2_cooldown == 0 && self.player2_lives > 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                if self.player2_pos > 0 && self.player2_lives > 0 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos -= 1 
                }
                return true;
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
            if self.draw(screen).await == false {
                let mut failed_menu: Menu<'_> = Menu::init("Failed!", &[MenuOption::Restart, MenuOption::Exit], screen);
                let result: MenuOption = failed_menu.menu_loop(screen).await;
                info!("obtained result... somehow?");
                match result {
                    MenuOption::Restart | MenuOption::None => {
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
                self.level = 1;
                self.player1_lives = 3;
                self.player2_lives = 3;
                self.init();
            }
        }
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new( 0 , 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        let mut temp: String<20> = String::new();
        fmt::write(&mut temp, format_args!("Level: {}", self.level)).unwrap();
        Text::new( &temp, Point::new(0, 8), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
            .draw(screen).unwrap();
        temp.clear();
        fmt::write(&mut temp, format_args!("Score: {}", self.score)).unwrap();
        Text::new(&temp, Point::new(0, 18), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
            .draw(screen).unwrap();
        self.draw_init = true;
        Rectangle::new(Point::new( 100 , 5), Size::new(5, 5))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
            .draw(screen)
            .unwrap();
        Rectangle::new(Point::new( 100 , 13), Size::new(5, 5))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
            .draw(screen)
            .unwrap();
        temp.clear();
        fmt::write(&mut temp, format_args!("x {}", self.player1_lives)).unwrap();
        Text::new( &temp, Point::new(110, 9), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
            .draw(screen).unwrap();
        temp.clear();
        fmt::write(&mut temp, format_args!("x {}", self.player2_lives)).unwrap();
        Text::new(&temp, Point::new(110, 17), MonoTextStyle::new(&FONT_5X8, Rgb565::WHITE))
            .draw(screen).unwrap();
        for i in 0..4 as u8 {
            for j in 0..5 as u8 {
                // info!("i j {} {}",i ,j);
                draw_enemy(j, i, &self.enemies[i as usize][j as usize].0, screen);
            }
        }
        for projectile in self.player1_projectiles.iter() {
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 + 1), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
                .draw(screen)
                .unwrap();    
        }
        for projectile in self.player2_projectiles.iter() {
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 + 1), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                .draw(screen)
                .unwrap();    
        }
        for projectile in self.enemy_projectiles.iter() {
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32 - 1), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(projectile.0 as i32, projectile.1 as i32), Size::new(2, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
                .draw(screen)
                .unwrap();    
        }
        if self.player1_lives > 0 {
            Rectangle::new(Point::new(self.player1_pos_prev as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player1_pos as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
        }
        if self.player2_lives > 0 {
            Rectangle::new(Point::new(self.player2_pos_prev as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(self.player2_pos as i32 , 150 as i32), Size::new(PLAYER_WIDTH, PLAYER_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();
        }
        
    }       
}

fn draw_enemy(posx: u8, posy: u8, enemy: &Enemy, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
    unsafe {
        match enemy {
            Enemy::Class1 => {
                Rectangle::new(Point::new(((posx+ OFFSET_X) * SPACING) as i32, ((posy * SPACING) + OFFSET_Y) as i32), Size::new(ENEMY_WIDTH, ENEMY_HEIGHT))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                    .draw(screen)
                    .unwrap();
            }
            Enemy::Class2 => {
                Rectangle::new(Point::new(((posx+ OFFSET_X) * SPACING) as i32, ((posy * SPACING) + OFFSET_Y) as i32), Size::new(ENEMY_WIDTH, ENEMY_HEIGHT))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                    .draw(screen)
                    .unwrap();
            }
            Enemy::Class3 => {
                Rectangle::new(Point::new(((posx+ OFFSET_X) * SPACING) as i32, ((posy * SPACING) + OFFSET_Y) as i32), Size::new(ENEMY_WIDTH, ENEMY_HEIGHT))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(screen)
                    .unwrap();
            }
            Enemy::Boss1 => {
                Rectangle::new(Point::new(((posx+ OFFSET_X) * SPACING) as i32, ((posy * SPACING) + OFFSET_Y) as i32), Size::new(BOSS_WIDTH, BOSS_HEIGHT))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                    .draw(screen)
                    .unwrap();
            }
            Enemy::Boss2 => {
                Rectangle::new(Point::new(((posx+ OFFSET_X) * SPACING) as i32, ((posy * SPACING) + OFFSET_Y) as i32), Size::new(BOSS_WIDTH, BOSS_HEIGHT))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                    .draw(screen)
                    .unwrap();
            }
            _ => {

            }
        }
    }
}