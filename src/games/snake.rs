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
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};

use heapless::{
    Vec, Deque,
};

use crate::INPUT_SIGNAL;
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::Input;

static OFFSET_X: u8 = 1;
static OFFSET_Y: u8 = 7;
pub struct Snake<'a> {
    head_1: (u8, u8),
    second_1: (u8, u8),
    tail_1: (u8, u8),
    facing_1: u8,
    updated_1: bool,
    head_2: (u8, u8),
    second_2: (u8, u8),
    tail_2: (u8, u8),
    facing_2: u8,
    updated_2: bool,
    frame: &'a mut Vec::<u32, 32>,
    body_1: &'a mut Deque::<(u8, u8), 1025>,
    body_2: &'a mut Deque::<(u8, u8), 1025>,
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
    pub fn new(frame: &'a  mut Vec<u32, 32>, body_1: &'a mut Deque<(u8, u8), 1025>, body_2: &'a mut Deque<(u8, u8), 1025>) -> Snake <'a>{
        Snake {
            head_1: (10, 3),
            second_1: (9, 3),
            tail_1: (3, 3),
            facing_1: 3,
            updated_1: false,
            head_2: (10, 10),
            second_2: (9, 10),
            tail_2: (3, 10),
            facing_2: 3,
            updated_2: false,
            frame,
            body_1,
            body_2,
        }
    }
    pub fn init(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        info!("frame: {:?}", self.frame);
        self.frame[3] = setval(self.frame[3], 3, true);
        self.frame[3] = setval(self.frame[3], 4, true);
        self.frame[3] = setval(self.frame[3], 5, true);
        self.frame[3] = setval(self.frame[3], 6, true);
        self.frame[3] = setval(self.frame[3], 7, true);
        self.frame[3] = setval(self.frame[3], 8, true);
        self.frame[3] = setval(self.frame[3], 9, true);
        self.frame[3] = setval(self.frame[3], 10, true);
        self.body_1.push_front((3, 3)).unwrap();
        self.body_1.push_front((4, 3)).unwrap();
        self.body_1.push_front((5, 3)).unwrap();
        self.body_1.push_front((6, 3)).unwrap();
        self.body_1.push_front((7, 3)).unwrap();
        self.body_1.push_front((8, 3)).unwrap();
        self.body_1.push_front((9, 3)).unwrap();
        self.body_1.push_front((10, 3)).unwrap();
        self.updated_1 = true;

        self.frame[10] = setval(self.frame[10], 3, true);
        self.frame[10] = setval(self.frame[10], 4, true);
        self.frame[10] = setval(self.frame[10], 5, true);
        self.frame[10] = setval(self.frame[10], 6, true);
        self.frame[10] = setval(self.frame[10], 7, true);
        self.frame[10] = setval(self.frame[10], 8, true);
        self.frame[10] = setval(self.frame[10], 9, true);
        self.frame[10] = setval(self.frame[10], 10, true);
        self.body_2.push_front((3, 10)).unwrap();
        self.body_2.push_front((4, 10)).unwrap();
        self.body_2.push_front((5, 10)).unwrap();
        self.body_2.push_front((6, 10)).unwrap();
        self.body_2.push_front((7, 10)).unwrap();
        self.body_2.push_front((8, 10)).unwrap();
        self.body_2.push_front((9, 10)).unwrap();
        self.body_2.push_front((10, 10)).unwrap();
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
        match self.facing_1 {
            0 => self.head_1.1 = if self.head_1.1 > 0 { self.head_1.1 - 1 } else { 23 },
            1 => self.head_1.1 = if self.head_1.1 < 23 { self.head_1.1 + 1 } else { 0 },
            2 => self.head_1.0 = if self.head_1.0 > 0 { self.head_1.0 - 1 } else { 23 },
            3 => self.head_1.0 = if self.head_1.0 < 23 { self.head_1.0 + 1 } else { 0 },
            _ =>(),
        }

        match self.facing_2 {
            0 => self.head_2.1 = if self.head_2.1 > 0 { self.head_2.1 - 1 } else { 23 },
            1 => self.head_2.1 = if self.head_2.1 < 23 { self.head_2.1 + 1 } else { 0 },
            2 => self.head_2.0 = if self.head_2.0 > 0 { self.head_2.0 - 1 } else { 23 },
            3 => self.head_2.0 = if self.head_2.0 < 23 { self.head_2.0 + 1 } else { 0 },
            _ =>(),
        }

        // info!("changed head {}", self.head);
        // for (i, value) in self.frame.iter().enumerate() {
        //     info!("index {} {:#034b}", i, value);
        // }
        // for value in self.body_1.iter() {
        //     info!("part {}, {}", value.0, value.1);
        // }
        if checkval(self.frame[self.head_1.1 as usize], self.head_1.0) == true {
            info!("collision detected, caused by 1st player at {}. {}", self.head_1.0, self.head_1.1);
            return false;
        }

        if checkval(self.frame[self.head_2.1 as usize], self.head_2.0) == true {
            info!("collision detected, caused by 2nd player at {}, {}", self.head_2.0, self.head_2.1);
            return false;
        }
        // apple logic?
        // if apple => don't remove tail
        match self.body_1.front() {
            Some(t) => self.second_1 = *t,
            None => (),
        }

        match self.body_2.front() {
            Some(t) => self.second_2 = *t,
            None => (),
        }
        
        
        self.frame[self.head_1.1 as usize] = setval(self.frame[self.head_1.1 as usize], self.head_1.0, true);
        self.body_1.push_front(self.head_1).unwrap();
        self.frame[self.tail_1.1 as usize] = setval(self.frame[self.tail_1.1 as usize], self.tail_1.0, false);
        self.body_1.pop_back();
        // info!("head_1 value {}", self.head_1);
        // info!("changed frame (head_1 added) to {:#034b} at index {}", self.frame[self.head_1.1 as usize], self.head_1.0);
        // info!("tail_1 value {}", self.tail_1);
        // info!("changed frame (tail_1 removed) from {:#034b} at index {}", self.frame[self.tail_1.1 as usize], self.tail_1.0);
        match self.body_1.back() {
            Some(t) => self.tail_1 = *t,
            None => (),
        }

        self.frame[self.head_2.1 as usize] = setval(self.frame[self.head_2.1 as usize], self.head_2.0, true);
        self.body_2.push_front(self.head_2).unwrap();
        self.frame[self.tail_2.1 as usize] = setval(self.frame[self.tail_2.1 as usize], self.tail_2.0, false);
        self.body_2.pop_back();
        // info!("head_2 value{}", self.head_2);
        // info!("changed frame (head_2 added) to {:#034b} at index {}", self.frame[self.head_2.1 as usize], self.head_2.0);
        // info!("tail_2 value {}", self.tail_2);
        // info!("changed frame (tail_2 removed) from {:#034b} at index {}", self.frame[self.tail_2.1 as usize], self.tail_2.0);
        match self.body_2.back() {
            Some(t) => self.tail_2 = *t,
            None => (),
        }
        // info!("deque len {}" , self.body.len());
        return true;
    }
 

    fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new((self.head_1.0 + OFFSET_X) as i32 * 5, (self.head_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new((self.second_1.0 + OFFSET_X) as i32 * 5, (self.second_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_LIME_GREEN))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new((self.tail_1.0 + OFFSET_X) as i32 * 5, (self.tail_1.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new((self.head_2.0 + OFFSET_X) as i32 * 5, (self.head_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new((self.second_2.0 + OFFSET_X) as i32 * 5, (self.second_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
            .draw(screen)
            .unwrap();


        Rectangle::new(Point::new((self.tail_2.0 + OFFSET_X) as i32 * 5, (self.tail_2.1 + OFFSET_Y) as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        
        // info!("the tail is {}", self.tail);
        match self.body_1.back() {
            Some(t) => self.tail_1 = *t,
            None => (),
        };

        match self.body_2.back() {
            Some(t) => self.tail_2 = *t,
            None => (),
        };

        self.updated_1 = false;
        self.updated_2 = false;
        // info!("changed tail to {}", self.tail);
    }
    
    fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Select => {
                
            }
            Input::Back => {
                
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
    }

    pub async fn snake_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        loop {
            match select(INPUT_SIGNAL.wait(), Timer::after(Duration::from_millis(150))).await {
                Either::First(input) => {
                    if !(self.updated_1 || self.updated_2) {
                        self.handle_input(&input);
                    }
                }
                _ => {}
            }
            if self.update_frame() == true {
                self.draw(screen);
            } else {
                info!("game over!");
                loop {}
            }
        }
    }
        
}