#[allow(static_mut_refs)]

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_rp::{
    clocks::RoscRng, gpio::Output, spi::Spi
};

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
    Vec, Deque,
};
use rand::seq::SliceRandom;


use crate::INPUT_SIGNAL;
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::Input;
use rust_pico_console::MenuOption;
use crate::Menu;

static OFFSET_X: u8 = 1;
static OFFSET_Y: u8 = 7;
pub struct Snake<'a> {
    head_1: (u8, u8),
    second_1: (u8, u8),
    tail_1: (u8, u8),
    facing_1: u8,
    updated_1: bool,
    active_1: bool,
    head_2: (u8, u8),
    second_2: (u8, u8),
    tail_2: (u8, u8),
    facing_2: u8,
    updated_2: bool,
    active_2: bool,
    apple: (u8, u8),
    frame: &'a mut Vec::<u32, 32>,
    body_1: &'a mut Deque::<(u8, u8), 1025>,
    body_2: &'a mut Deque::<(u8, u8), 1025>,
    apples: &'a mut Vec::<u32, 32>,
    apples_count: u16,
}

fn setval(value: u32, col: u8, set: bool) -> u32 {
    if set {
        value | (1 << (32 - col))
    } else {
        value & !(1 << (32 - col))
    }
}

fn checkval(value: u32, col: u8) -> bool {
    (value & (1 << (32 - col))) != 0
}


impl <'a> Snake<'a> {
    pub fn new(frame: &'a  mut Vec<u32, 32>, body_1: &'a mut Deque<(u8, u8), 1025>, body_2: &'a mut Deque<(u8, u8), 1025>, apples: &'a mut Vec<u32, 32>) -> Snake <'a>{
        Snake {
            head_1: (6, 3),
            second_1: (5, 3),
            tail_1: (2, 3),
            facing_1: 3,
            updated_1: false,
            active_1: true,
            head_2: (6, 10),
            second_2: (5, 10),
            tail_2: (2, 10),
            facing_2: 3,
            updated_2: false,
            active_2: true,
            apple: (10, 3),
            frame,
            body_1,
            body_2,
            apples,
            apples_count: 0,
        }
    }
    pub fn init(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        // info!("frame: {:?}", self.frame);
        self.frame[3] = setval(self.frame[3], 3, true);
        self.frame[3] = setval(self.frame[3], 4, true);
        self.frame[3] = setval(self.frame[3], 5, true);
        self.frame[3] = setval(self.frame[3], 6, true);
        self.body_1.push_front((3, 3)).unwrap();
        self.body_1.push_front((4, 3)).unwrap();
        self.body_1.push_front((5, 3)).unwrap();
        self.body_1.push_front((6, 3)).unwrap();
        self.updated_1 = true;

        self.frame[10] = setval(self.frame[10], 3, true);
        self.frame[10] = setval(self.frame[10], 4, true);
        self.frame[10] = setval(self.frame[10], 5, true);
        self.frame[10] = setval(self.frame[10], 6, true);
        self.body_2.push_front((3, 10)).unwrap();
        self.body_2.push_front((4, 10)).unwrap();
        self.body_2.push_front((5, 10)).unwrap();
        self.body_2.push_front((6, 10)).unwrap();
        self.updated_2 = true;

        for i in 0..25 {
            Rectangle::new(Point::new(OFFSET_X as i32 * 5 - 1 + 5 * i, (OFFSET_Y * 5 - 1) as i32), Size::new(1, 121))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SEA_GREEN))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(OFFSET_X as i32 * 5 - 1, (OFFSET_Y * 5 - 1) as i32 + 5 * i), Size::new(120, 1))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SEA_GREEN))
                .draw(screen)
                .unwrap();
        }
    }

    fn update_frame(&mut self) -> bool {
        if self.active_1 {
            match self.facing_1 {
                0 => self.head_1.1 = if self.head_1.1 > 0 { self.head_1.1 - 1 } else { 23 },
                1 => self.head_1.1 = if self.head_1.1 < 23 { self.head_1.1 + 1 } else { 0 },
                2 => self.head_1.0 = if self.head_1.0 > 0 { self.head_1.0 - 1 } else { 23 },
                3 => self.head_1.0 = if self.head_1.0 < 23 { self.head_1.0 + 1 } else { 0 },
                _ =>(),
            }
        }

        if self.active_2 {
            match self.facing_2 {
                0 => self.head_2.1 = if self.head_2.1 > 0 { self.head_2.1 - 1 } else { 23 },
                1 => self.head_2.1 = if self.head_2.1 < 23 { self.head_2.1 + 1 } else { 0 },
                2 => self.head_2.0 = if self.head_2.0 > 0 { self.head_2.0 - 1 } else { 23 },
                3 => self.head_2.0 = if self.head_2.0 < 23 { self.head_2.0 + 1 } else { 0 },
                _ =>(),
            }
        }
        // info!("reached part 1");
        // info!("updated 1 {}, updated 2 {}", self.updated_1, self.updated_2);

        // info!("changed head {}", self.head);
        // for (i, value) in self.frame.iter().enumerate() {
        //     info!("index {} {:#034b}", i, value);
        // }
        // for value in self.body_1.iter() {
        //     info!("part {}, {}", value.0, value.1);
        // }
        if self.active_1 {
            // info!("entered if");
            if self.head_1 != self.apple {
                if checkval(self.frame[self.head_1.1 as usize], self.head_1.0) == true {
                    info!("collision detected, caused by 1st player at {}. {}", self.head_1.0, self.head_1.1);
                    self.active_1 = false;
                    self.updated_1 = false;
                    for value in self.body_1.iter() {
                        // info!("body value {}", value);
                        self.apples[value.1 as usize] = setval(self.apples[value.1 as usize], value.0, true);
                        self.frame[value.1 as usize] = setval(self.frame[value.1 as usize], value.0, false);
                        self.apples_count += 1;
                    }
                    // info!("apples");
                    // for (i, value) in self.apples.iter().enumerate() {
                    //     info!("index {} {:#034b}", i, value);
                    // }
                    // info!("frame");
                    // for (i, value) in self.frame.iter().enumerate() {
                    //     info!("index {} {:#034b}", i, value);
                    // }
                    // return false;
                } else if checkval(self.apples[self.head_1.1 as usize], self.head_1.0) {
                    self.apples[self.head_1.1 as usize] = setval(self.apples[self.head_1.1 as usize], self.head_1.0, false);
                    self.apples_count -= 1;
                } else {
                    match self.body_1.back() {
                        Some(t) => self.tail_1 = *t,
                        None => (),
                    }
                    self.frame[self.tail_1.1 as usize] = setval(self.frame[self.tail_1.1 as usize], self.tail_1.0, false);
                    self.body_1.pop_back();
                }
            } else {
                self.generate_apple();
            }

            match self.body_1.front() {
                Some(t) => self.second_1 = *t,
                None => (),
            }
            self.frame[self.head_1.1 as usize] = setval(self.frame[self.head_1.1 as usize], self.head_1.0, true);
            self.body_1.push_front(self.head_1).unwrap();
        }
        // for (i, value) in self.apples.iter().enumerate() {
        //     info!("index {} {:#034b}", i, value);
        // }
        // info!("frame");
        // for (i, value) in self.frame.iter().enumerate() {
        //     info!("index {} {:#034b}", i, value);
        // }

        
        if self.active_2 {
            // info!("entered active 2 if");
            if self.head_2 != self.apple {
                if checkval(self.frame[self.head_2.1 as usize], self.head_2.0) == true {
                    info!("collision detected, caused by 2nd player at {}, {}", self.head_2.0, self.head_2.1);
                    self.active_2 = false;
                    self.updated_2 = false;
                    for value in self.body_2.iter() {
                        // info!("body value {}", value);
                        self.apples[value.1 as usize] = setval(self.apples[value.1 as usize], value.0, true);
                        self.frame[value.1 as usize] = setval(self.frame[value.1 as usize], value.0, false);
                        self.apples_count += 1;
                    }
                    
                    // info!("apples");
                    // for (i, value) in self.apples.iter().enumerate() {
                    //     info!("index {} {:#034b}", i, value);
                    // }
                    // info!("frame");
                    // for (i, value) in self.frame.iter().enumerate() {
                    //     info!("index {} {:#034b}", i, value);
                    // }
                    // return false;
                } else if checkval(self.apples[self.head_2.1 as usize], self.head_2.0) {
                    self.apples[self.head_2.1 as usize] = setval(self.apples[self.head_2.1 as usize], self.head_2.0, false);
                    self.apples_count -= 1;
                } else {
                    match self.body_2.back() {
                        Some(t) => self.tail_2 = *t,
                        None => (),
                    }
                    self.frame[self.tail_2.1 as usize] = setval(self.frame[self.tail_2.1 as usize], self.tail_2.0, false);
                    self.body_2.pop_back();
                }
            } else {
                self.generate_apple();
            }
            match self.body_2.front() {
                Some(t) => self.second_2 = *t,
                None => (),
            }
            self.frame[self.head_2.1 as usize] = setval(self.frame[self.head_2.1 as usize], self.head_2.0, true);
            self.body_2.push_front(self.head_2).unwrap();
        }

        // info!("frame");
        // for (i, value) in self.frame.iter().enumerate() {
        //     info!("index {} {:#034b}", i, value);
        // }
        // loop{}
        // apple logic?
        // if apple => don't remove tail


        
        // info!("head_1 value {}", self.head_1);
        // info!("changed frame (head_1 added) to {:#034b} at index {}", self.frame[self.head_1.1 as usize], self.head_1.0);
        // info!("tail_1 value {}", self.tail_1);
        // info!("changed frame (tail_1 removed) from {:#034b} at index {}", self.frame[self.tail_1.1 as usize], self.tail_1.0);

        // info!("head_2 value{}", self.head_2);
        // info!("changed frame (head_2 added) to {:#034b} at index {}", self.frame[self.head_2.1 as usize], self.head_2.0);
        // info!("tail_2 value {}", self.tail_2);
        // info!("changed frame (tail_2 removed) from {:#034b} at index {}", self.frame[self.tail_2.1 as usize], self.tail_2.0);
        // info!("deque len {}" , self.body.len());
        return self.active_1 || self.active_2;
    }
    
    fn generate_apple(&mut self) {
        let mut empty_spaces: Vec<(u8, u8), 576> = Vec::new();
        for i in 0..23 as u8 {
            for j in 0..23 as u8 {
                if checkval(self.frame[i as usize], j) == false && checkval(self.frame[i as usize], j) == false{
                    empty_spaces.push((i, j)).unwrap();
                }
            }
        }
        let mut rng = RoscRng;
        match empty_spaces.choose(&mut rng) {
            Some(t) => self.apple = *t,
            None => {} 
        }
    }

    fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        if self.active_1 == false || self.active_2 == false {
            for (index, value) in self.apples.iter().enumerate() {
                for j in 0..31 {
                    if checkval(*value, j) {
                        Rectangle::new(Point::new((j + OFFSET_X) as i32 * 5, (index as u8 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                            .draw(screen)
                            .unwrap();
                    }
                }
            }
        }
        Rectangle::new(Point::new((self.apple.0 + OFFSET_X) as i32 * 5, (self.apple.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(screen)
            .unwrap();

        if self.active_1 {
            Rectangle::new(Point::new((self.head_1.0 + OFFSET_X) as i32 * 5, (self.head_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();

            Rectangle::new(Point::new((self.second_1.0 + OFFSET_X) as i32 * 5, (self.second_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_LIME_GREEN))
                .draw(screen)
                .unwrap();
            
            if self.head_1 != self.tail_1 {
                Rectangle::new(Point::new((self.tail_1.0 + OFFSET_X) as i32 * 5, (self.tail_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            }
            match self.body_1.back() {
                Some(t) => self.tail_1 = *t,
                None => (),
            };
            self.updated_1 = false;
        }
        if self.active_2 {
            Rectangle::new(Point::new((self.head_2.0 + OFFSET_X) as i32 * 5, (self.head_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();

            Rectangle::new(Point::new((self.second_2.0 + OFFSET_X) as i32 * 5, (self.second_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                .draw(screen)
                .unwrap();

            if self.head_2 != self.tail_2 {
                Rectangle::new(Point::new((self.tail_2.0 + OFFSET_X) as i32 * 5, (self.tail_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
            }
            match self.body_2.back() {
                Some(t) => self.tail_2 = *t,
                None => (),
            };
            self.updated_2 = false;
        }
        // info!("the tail is {}", self.tail);


        // info!("changed tail to {}", self.tail);
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new(0, 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        for i in 0..25 {
            Rectangle::new(Point::new(OFFSET_X as i32 * 5 - 1 + 5 * i, (OFFSET_Y * 5 - 1) as i32), Size::new(1, 121))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SEA_GREEN))
                .draw(screen)
                .unwrap();
            Rectangle::new(Point::new(OFFSET_X as i32 * 5 - 1, (OFFSET_Y * 5 - 1) as i32 + 5 * i), Size::new(120, 1))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SEA_GREEN))
                .draw(screen)
                .unwrap();
        }
        
        if self.active_1 && self.active_2 == false {
            for (index, value) in self.apples.iter().enumerate() {
                for j in 0..31 {
                    if checkval(*value, j) {
                        Rectangle::new(Point::new((j + OFFSET_X) as i32 * 5, (index as u8 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                            .draw(screen)
                            .unwrap();
                    }
                }
            }
        }
        Rectangle::new(Point::new((self.apple.0 + OFFSET_X) as i32 * 5, (self.apple.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(screen)
            .unwrap();

        if self.active_1 {
            for part in self.body_1.iter() {
                if *part != self.head_1 && *part != self.second_1 && *part != self.tail_1 {
                    Rectangle::new(Point::new((part.0 + OFFSET_X) as i32 * 5, (part.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                        .draw(screen)
                        .unwrap();    
                }
            }
            Rectangle::new(Point::new((self.head_1.0 + OFFSET_X) as i32 * 5, (self.head_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
                .draw(screen)
                .unwrap();

            Rectangle::new(Point::new((self.second_1.0 + OFFSET_X) as i32 * 5, (self.second_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_LIME_GREEN))
                .draw(screen)
                .unwrap();

            if self.head_1 != self.tail_1 {
                Rectangle::new(Point::new((self.tail_1.0 + OFFSET_X) as i32 * 5, (self.tail_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
            }
        }

        if self.active_2 {
            for part in self.body_2.iter() {
                if *part != self.head_2 && *part != self.second_2 && *part != self.tail_2 {
                    Rectangle::new(Point::new((part.0 + OFFSET_X) as i32 * 5, (part.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                        .draw(screen)
                        .unwrap();    
                }
            }
            Rectangle::new(Point::new((self.head_2.0 + OFFSET_X) as i32 * 5, (self.head_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();

            Rectangle::new(Point::new((self.second_2.0 + OFFSET_X) as i32 * 5, (self.second_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                .draw(screen)
                .unwrap();

            if self.head_2 != self.tail_2 {
                Rectangle::new(Point::new((self.tail_2.0 + OFFSET_X) as i32 * 5, (self.tail_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(screen)
                    .unwrap();
            }
        }
    }
    
    fn handle_input(&mut self, input: &Input) -> bool {
        match input {
            Input::Select => {}
            Input::Back => {
                return false
            }
            Input::Up => if self.facing_1 != 1 && self.updated_1 == false { self.facing_1 = 0; self.updated_1 = true },
            Input::Down => if self.facing_1 != 0 && self.updated_1 == false { self.facing_1 = 1; self.updated_1 = true },
            Input::Left => if self.facing_1 != 3 && self.updated_1 == false { self.facing_1 = 2; self.updated_1 = true },
            Input::Right => if self.facing_1 != 2 && self.updated_1 == false { self.facing_1 = 3; self.updated_1 = true },
            
            Input::Up2 => if self.facing_2 != 1 && self.updated_2 == false { self.facing_2 = 0; self.updated_2 = true },
            Input::Down2 => if self.facing_2 != 0 && self.updated_2 == false { self.facing_2 = 1; self.updated_2 = true },
            Input::Left2 => if self.facing_2 != 3 && self.updated_2 == false { self.facing_2 = 2; self.updated_2 = true },
            Input::Right2 => if self.facing_2 != 2 && self.updated_2 == false { self.facing_2 = 3; self.updated_2 = true },
            _ => {}
        }
        true
    }

    pub async fn game_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        loop {
            match select(INPUT_SIGNAL.wait(), Timer::after(Duration::from_millis(250))).await {
                Either::First(input) => {
                    if !(self.updated_1 || self.updated_2) {
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
                }
                _ => {}
            }
            if self.update_frame() == true {
                self.draw(screen);
            } else {
                info!("game over!");
                // create pause menu
                let mut end_menu: Menu<'_> = Menu::init("Game over!", &[MenuOption::Restart, MenuOption::Exit], screen);
                let result: MenuOption = end_menu.menu_loop(screen).await;
                info!("obtained result... somehow?");
                match result {
                    MenuOption::Restart | MenuOption::None => {
                        unsafe { CURRENT = 1 }; 
                        Timer::after(Duration::from_millis(100)).await;
                        INPUT_SIGNAL.reset();
                        return;
                    },
                    MenuOption::Exit => {
                        unsafe { CURRENT = 0 };
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
        
}