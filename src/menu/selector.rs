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

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};
use crate::INPUT_SIGNAL;
use crate::CURRENT;

pub struct Menu<'a> {
    title: &'a str,
    options: &'a [MenuOption],
    selected: usize,
}

impl <'a> Menu<'a> {
    pub fn init(title: &'a str, options: &'a [MenuOption], screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> Menu<'a> {
        Rectangle::new(Point::new(16, 10), Size::new(96, 27 + options.len() as u32 * 16))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_AQUA))
            .draw(screen)
            .unwrap();
        
        // info!("drawing menu");
        Text::new(title, Point::new(20, 25),MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_ORANGE))
            .draw(screen)
            .unwrap();

        for i in 0..options.len() {
            Rectangle::new(Point::new(19, 36 + i as i32 * 16), Size::new(90, 14))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                .draw(screen)
                .unwrap();
        }
        Menu {
            title,
            options,
            selected: 0,
        }
    }

    pub fn draw(&self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        for (i , option) in self.options.iter().enumerate() {
            let color = if self.selected == i { Rgb565::YELLOW } else { Rgb565::CSS_ORANGE };
            Text::new(match option {
                MenuOption::Snake => "Snake",
                MenuOption::SpaceInvaders => "Space Invaders",
                MenuOption::Sokoban => "Sokoban",
                MenuOption::Debug => "Debug",
                MenuOption::Resume => "Resume",
                MenuOption::Exit => "Exit",
                _ => ""
            }, Point::new(23, 46 + i as i32 * 16),MonoTextStyle::new(&FONT_6X10, color))
                .draw(screen)
                .unwrap();
        }
    }
    
    pub fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Up | Input::Up2 => {
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = self.options.len() - 1;
                }
            }
            Input::Down | Input::Down2 => {
                if self.selected + 1 < self.options.len() {
                    self.selected += 1;
                } else {
                    self.selected = 0;
                }
            }
            _ => {}
        }
    }

    pub async fn menu_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) -> MenuOption {
        self.draw(screen);
        loop {
            Timer::after(Duration::from_millis(100)).await;
            INPUT_SIGNAL.reset();
            let input = INPUT_SIGNAL.wait().await;
            match input {
                Input::Up | Input::Down | Input::Up2 | Input::Down2 => {
                    self.handle_input(&input);
                    self.draw(screen);
                },
                Input::Select => {
                    return self.options[self.selected]
                },        
                Input::Back => {    
                    return MenuOption::None;
                }
                _ => {}
            }
            Timer::after(Duration::from_millis(100)).await;
            INPUT_SIGNAL.reset();
        }
    }
        
}