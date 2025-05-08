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
use embassy_time::{Duration, Timer};

use heapless::{
    Vec, spsc::Queue,
};

use tinytga::Tga;

use crate::INPUT_SIGNAL;
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::Input;

static mut OFFSET_X: u8 = 0;
static mut OFFSET_Y: u8 = 50;
static ENEMY_WIDTH: u32 = 5;
static ENEMY_HEIGHT: u32 = 3;
static BOSS_WIDTH: u32 = 40;
static BOSS_HEIGHT: u32 = 10;
static SPACING: u8 = 15;

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
    enemy_projectiles: &'a mut Vec<(u8, bool), 5>,
    enemies_count: u8,
    level: u8,
    score: u32,
    step: u16,
    speed: u16,
    shift: bool,
}

impl <'a> SpaceInvaders<'a> {
    pub fn new(enemies: &'a mut Vec<Vec<(Enemy, u8), 5>, 5> , 
    enemy_projectiles: &'a mut Vec<(u8, bool), 5>,
    player1_projectiles: &'a mut Vec<(u8, u8, bool), 20>,
    player2_projectiles: &'a mut Vec<(u8, u8, bool), 20>) -> SpaceInvaders <'a> {
        SpaceInvaders { 
            player1_pos: 54,
            player1_pos_prev: 0,
            player2_pos: 74,
            player2_pos_prev: 0,
            player1_cooldown: 40,
            player2_cooldown: 40,
            player1_projectiles,
            player2_projectiles,
            player1_lives: 3,
            player2_lives: 3,
            enemies,
            enemy_projectiles,
            enemies_count: 0,
            level: 1, 
            score: 100,
            step: 0,
            speed: 300,
            shift: true, 
        }
    }
    pub fn init(&mut self) {
        unsafe {
            OFFSET_X = 0;
            OFFSET_Y = 50;
        }
        self.speed = self.speed - self.level as u16 * 2;
        if self.level % 10 == 0 {
            self.enemies[0][0] = (Enemy::Boss2, 20);
            for i in 0..5 {
                self.enemies[3][i] = (Enemy::Class3, 3);
            }
            self.enemies_count = 6;
        } else if self.level % 5 == 0 {
            //boss level, first row boss and 1 row class3
            self.enemies[0][0] = (Enemy::Boss1, 20);
            for i in 0..5 {
                self.enemies[3][i] = (Enemy::Class3, 3);
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
                    self.enemies_count = 20;
                }
                2 => {
                    //first row category2
                    for i in 0..5 {
                        self.enemies[0][i] = (Enemy::Class2, 2);
                    }
                    
                    for i in 1..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class1, 1);
                        }
                    }
                    
                    self.enemies_count = 20;
                }
                3 => {
                    //first and second category2
                    for i in 0..5 {
                        self.enemies[0][i] = (Enemy::Class2, 2);
                        self.enemies[1][i] = (Enemy::Class2, 2);
                    }
                    
                    for i in 2..4 {
                        for j in 0..5 {
                            self.enemies[i][j] = (Enemy::Class1, 1);
                        }
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
                    self.enemies_count = 20;
                }
                _ => {},
            }
        }
    }

    fn update_frame(&mut self) -> bool {
        //check collision
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
                if projectile.1 < OFFSET_Y - 10 {
                    projectile.2 = false;
                    continue;
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
                                    if self.enemies[y][x].1 == 1 {
                                        self.enemies[y][x].0 = Enemy::None;
                                        self.enemies[y][x].1 = 0;
                                        self.enemies_count -= 1;
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
                if projectile.1 < OFFSET_Y {
                    projectile.2 = false;
                    continue;
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
                                    if self.enemies[y][x].1 == 1 {
                                        self.enemies[y][x].0 = Enemy::None;
                                        self.enemies[y][x].1 = 0;
                                        self.enemies_count -= 1;
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
        // info!("enemies: {}", self.enemies_count);
        if self.enemies_count == 0 {
            self.level += 1;
            self.init();
        }

        for projectile in self.player1_projectiles.iter_mut() {
            projectile.1 -= 1;
        }
        self.player1_projectiles
            .retain(|&(_, y, _)| y >= 60);
        
        for projectile in self.player2_projectiles.iter_mut() {
            projectile.1 -= 1;
        }
        self.player2_projectiles
            .retain(|&(_, y, _)| y >= 60);
        

        // shift enemies every X frames
        if self.step > 0 {
            self.step = self.step - 1;
            false
        } else {
            // shift enemies
            unsafe {
                match self.shift {
                    true => {
                        if OFFSET_X < 4 {
                            OFFSET_X += 1;
                        } else {
                            if OFFSET_Y < 120 {
                                OFFSET_Y += 10;
                            }
                            self.shift = false;
                        }
                    }
                    false => {
                        if OFFSET_X > 0 {
                            OFFSET_X -= 1;
                        } else {
                            if OFFSET_Y < 120 {
                                OFFSET_Y += 10;
                            }
                            self.shift = true;
                        }
                    }
                }
            }
            self.step = self.speed;
            true
        }

    }

    async fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        if self.update_frame() {
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
        Rectangle::new(Point::new(self.player1_pos_prev as i32 , 150 as i32), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        Rectangle::new(Point::new(self.player1_pos as i32 , 150 as i32), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
            .draw(screen)
            .unwrap();
        Rectangle::new(Point::new(self.player2_pos_prev as i32 , 150 as i32), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        Rectangle::new(Point::new(self.player2_pos as i32 , 150 as i32), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
            .draw(screen)
            .unwrap();
        
    }
    
    fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Select => {
                
            }
            Input::Back => {
                
            }
            Input::Left => if self.player1_pos > 0 { self.player1_pos_prev = self.player1_pos; self.player1_pos -= 1 }
            Input::Right => if self.player1_pos < 128 { self.player1_pos_prev = self.player1_pos; self.player1_pos += 1 }
            Input::Fire => if self.player1_cooldown == 0 { self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); self.player1_cooldown = 40 }
            
            Input::Left2 => if self.player2_pos > 0 { self.player2_pos_prev = self.player2_pos; self.player2_pos -= 1 }
            Input::Right2 => if self.player2_pos < 128 { self.player2_pos_prev = self.player2_pos; self.player2_pos += 1 }
            Input::Fire2 => if self.player2_cooldown == 0 { self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); self.player2_cooldown = 40 }
            _ => {}
        }
    }

    pub async fn game_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        loop {
            match select(INPUT_SIGNAL.wait(), Timer::after(Duration::from_millis(5))).await {
                Either::First(input) => {
                    self.handle_input(&input);
                }
                _ => {}
            }
            self.draw(screen).await;
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