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

pub struct Snake<'a> {
    head: (u8, u8),
    tail: (u8, u8),
    facing: u8,
    frame: &'a mut Vec::<u32, 32>,
    body: &'a mut Deque::<(u8, u8), 1025>,
}

fn setval(value: u32, col: u8, set: bool) -> u32 {
    if set == true {
        value | 1 << (32 - col)
    } else {
        value & !(1 << (32 - col))
    }
}

fn checkval(value: u32, col: u8) -> bool {
    value & 1 << (32 - col) == 1 << (32 - col)
}

impl <'a> Snake<'a> {
    pub fn new(frame: &'a  mut Vec<u32, 32>, body: &'a mut Deque<(u8, u8), 1025>) -> Snake <'a>{
        Snake {
            head: (3, 5),
            tail: (3, 3),
            facing: 3,
            frame,
            body,
        }
    }
    pub fn init(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        info!("frame: {:?}", self.frame);
        self.frame[3] = setval(self.frame[3], 3, true);
        self.frame[3] = setval(self.frame[3], 4, true);
        self.frame[3] = setval(self.frame[3], 5, true);
        self.body.push_front((3, 5)).unwrap();
        self.body.push_front((3, 4)).unwrap();
        self.body.push_front((3, 3)).unwrap();
        Rectangle::new(Point::new(15, 15), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new(15, 20), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new(15, 25), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(screen)
            .unwrap();
    }

    pub fn update_frame(&mut self) {
        match self.facing {
            0 => self.head.1 = if self.head.1 > 0 { self.head.1 - 1 } else { self.head.1 },
            1 => self.head.1 = if self.head.1 < 20 { self.head.1 + 1 } else { self.head.1 },
            2 => self.head.0 = if self.head.0 > 0 { self.head.0 - 1 } else { self.head.0 },
            3 => self.head.0 = if self.head.0 < 20 { self.head.0 + 1 } else { self.head.0 },
            _ =>(),
        }

        info!("changed head {}", self.head);
        if checkval(self.frame[self.head.0 as usize], self.head.1) == true {
            info!("collision detected");
        }
        // apple logic?
        // if apple => don't remove tail
        self.body.push_front(self.head).unwrap();
        info!("deque len {}" , self.body.len());
        self.body.pop_back();
    }

    pub fn draw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new(self.head.0 as i32 * 5, self.head.1 as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(screen)
            .unwrap();

        Rectangle::new(Point::new(self.tail.0 as i32 * 5, self.tail.1 as i32 * 5), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        
        info!("the tail is {}", self.tail);
        match self.body.back() {
            Some(t) => self.tail = *t,
            None => (),
        };
        info!("changed tail to {}", self.tail);
    }
    
    pub fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Select => {
                
            }
            Input::Back => {
                
            }
            Input::Up => if self.facing != 1 { self.facing = 0 },
            Input::Down => if self.facing != 0 { self.facing = 1 },
            Input::Left => if self.facing != 3 { self.facing = 2 },
            Input::Right => if self.facing != 2 { self.facing = 3},
            _ => {}
        }
    }

    pub async fn snake_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        loop {
            match select(INPUT_SIGNAL.wait(), Timer::after(Duration::from_millis(10))).await {
                Either::First(input) => {
                    self.handle_input(&input);
                }
                Either::Second(_) => {
                    
                }
            }
            self.update_frame();
            self.draw(screen);
        }
    }
        
}