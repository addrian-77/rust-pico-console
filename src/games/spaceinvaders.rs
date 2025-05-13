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

use crate::{menu::selector::Menu, INPUT_SIGNAL};
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};

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
    last_row: &'a mut Vec<(Enemy, u8, u8, bool), 5>,
    enemy_projectiles: &'a mut Vec<(u8, u8, u8, bool), 5>,
    projectile_cooldown: u8,
    enemies_count: u8,
    level: u8,
    score: u32,
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
            level: 2, 
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
                self.last_row[i] = (Enemy::Class3, i as u8, 3, false);
            }
            self.enemies_count = 6;
        } else if self.level % 5 == 0 {
            //boss level, first row boss and 1 row class3
            self.enemies[0][0] = (Enemy::Boss1, 20);
            for i in 0..5 {
                self.enemies[3][i] = (Enemy::Class3, 3);
                self.last_row[i] = (Enemy::Class3, i as u8, 3, false);
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
                                        let mut posy = 4;
                                        loop {
                                            if self.enemies[posy][x].0 != Enemy::None {
                                                self.last_row[posy] = (self.enemies[posy][x].0, x as u8, posy as u8, false);
                                                break;
                                            }
                                            if posy == 0 && self.enemies[posy][x].0 == Enemy::None {
                                                self.last_row[posy] = (Enemy::None, 0, 0, false);
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
                                        let mut posy = 4;
                                        loop {
                                            if self.enemies[posy][x].0 != Enemy::None {
                                                self.last_row[posy] = (self.enemies[posy][x].0, x as u8, posy as u8, false);
                                                break;
                                            }
                                            if posy == 0 && self.enemies[posy][x].0 == Enemy::None {
                                                self.last_row[posy] = (Enemy::None, 0, 0, false);
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
            if self.enemy_projectiles[i].1 > 160 {
                self.enemy_projectiles[i].3 = false;
                self.last_row[self.enemy_projectiles[i].2 as usize].3 = false;
                // info!("projectile out of scope");
            }
        }
        self.enemy_projectiles.
            retain(|active| active.3);   
        
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

    fn choose_enemy(&mut self) -> u8 {
        let available: Vec<&(Enemy, u8, u8, bool), 5> = self.last_row.iter().filter(|active_projectile| active_projectile.3 == false && active_projectile.0 != Enemy::Class1).collect();
        // info!("AAAAAAAAAAAAAAAa");
        // info!("available enemies {}", available.len());
        let mut rng = RoscRng;
        match available.choose(&mut rng) {
            Some(t) => {
                unsafe {
                    self.enemy_projectiles.push(((t.1 + OFFSET_X) * SPACING + 1, (t.2 * SPACING) + OFFSET_Y + ENEMY_HEIGHT as u8, t.1,  true)).unwrap();
                }
                t.1
            },
            None => {
                10
            }
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
    
    fn handle_input(&mut self, input: &Input) -> bool {
        match input {
            Input::Select => {
                return true
            }
            Input::Back => {
                return false
            }
            Input::Left => { 
                if self.player1_pos > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos -= 1; 
                } 
                return true
            }
            Input::Right => { 
                if self.player1_pos < 128 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos += 1 
                }
                return true
            }
            Input::Up => {
                if self.player1_cooldown == 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                return true
            }
            Input::Right_Shoot => {
                if self.player1_cooldown == 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                if self.player1_pos < 128 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos += 1 
                }
                return true;
            }
            Input::Left_Shoot => {
                if self.player1_cooldown == 0 { 
                    self.player1_projectiles.push((self.player1_pos + 1, 146, true)).unwrap(); 
                    self.player1_cooldown = 60; 
                }
                if self.player1_pos > 0 { 
                    self.player1_pos_prev = self.player1_pos; 
                    self.player1_pos -= 1; 
                }
                return true;
            }
            Input::Left2 => { 
                if self.player2_pos > 0 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos -= 1 
                }
                return true
            }
            Input::Right2 => {
                if self.player2_pos < 128 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos += 1 
                }
                return true
            }
            Input::Up2 => { 
                if self.player2_cooldown == 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                return true
            }
            Input::Right2_Shoot => {
                if self.player2_cooldown == 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                if self.player2_pos < 128 { 
                    self.player2_pos_prev = self.player2_pos; 
                    self.player2_pos += 1 
                }
                return true;
            }
            Input::Left2_Shoot => {
                if self.player2_cooldown == 0 { 
                    self.player2_projectiles.push((self.player2_pos + 1, 146, true)).unwrap(); 
                    self.player2_cooldown = 60 
                }
                if self.player2_pos > 0 { 
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
            self.draw(screen).await;
        }
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new( 0 , 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
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