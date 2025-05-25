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

use heapless::{
    Vec, String
};

use crate::{menu::selector::Menu, INPUT_SIGNAL};
use crate::CURRENT;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use rust_pico_console::{Input, MenuOption};
use core::fmt;

const OFFSET_X: i32 = 28;


pub struct Sokoban<'a> {
    player1: (u8, u8),
    player2: (u8, u8),
    level: u8,
    frame: &'a mut Vec<Vec<u8, 15>, 15>,
    destinations: &'a mut Vec<(u8, u8), 20>,
    correct_boxes: u8,
    moves: u16,
}

impl <'a> Sokoban<'a> {
    pub fn new(frame: &'a mut Vec<Vec<u8, 15>, 15>, destinations: &'a mut Vec<(u8, u8), 20>) -> Sokoban <'a> {
        Sokoban {
            player1: (0, 0),
            player2: (0, 0),
            level: 1,
            frame,
            destinations,
            correct_boxes: 0,
            moves: 0
        }
    } 
    pub fn init(&mut self) {
        self.frame.clear();
        if self.frame.is_empty() {
            info!("cleared frame");
        }
        macro_rules! row {
            ($($val:expr),*) => {{
                let mut r: Vec<u8, 15> = Vec::new();
                $(r.push($val).unwrap();)*
                self.frame.push(r).unwrap();
            }}
        }
        self.moves = 0;
        self.correct_boxes = 0;
        self.destinations.clear();
        match self.level {
            1 => {
                // 1 - wall
                // 2 - box
                row!(0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 1, 0, 1, 2, 1, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1);
                row!(0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 2, 0, 1);
                row!(0, 0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1);
                row!(0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1);
                row!(0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 2, 0, 1);
                row!(0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1);
                row!(0, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1);
                self.player1 = (5, 3);
                self.player2 = (5, 9);
                self.destinations.push((8, 5)).unwrap();
                self.destinations.push((7, 9)).unwrap();
                self.destinations.push((6, 6)).unwrap();
                self.destinations.push((4, 7)).unwrap();
            }
            2 => {
                row!(0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 2, 0, 1, 1, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 1, 0, 0, 2, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 2, 0, 0, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 0, 0);
                row!(0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0);
                self.player1 = (7, 9);
                self.player2 = (8, 5);
                self.destinations.push((5, 5)).unwrap();
                self.destinations.push((7, 7)).unwrap();
                self.destinations.push((3, 6)).unwrap();
            }
            3 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 1, 1, 0);
                row!(0, 0, 0, 1, 1, 2, 0, 0, 0, 0, 0, 1, 0);
                row!(0, 0, 0, 1, 1, 0, 0, 0, 2, 2, 0, 1, 0);
                row!(0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0);
                row!(0, 0, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (2, 5);
                self.player2 = (6, 5);
                self.destinations.push((2, 4)).unwrap();
                self.destinations.push((7, 5)).unwrap();
                self.destinations.push((6, 6)).unwrap();
                self.destinations.push((5, 8)).unwrap();
            }
            4 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0);
                row!(0, 0, 0, 1, 1, 1, 1, 1, 0, 2, 0, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 2, 0, 0, 0, 0, 0, 1, 1, 1);
                row!(0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1);
                row!(0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1);
                row!(0, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 0);
                row!(0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0);
                row!(0, 1, 0, 2, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0);
                row!(0, 1, 1, 1, 1, 1, 1, 0, 1, 2, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0);
                self.player1 = (6, 6);
                self.player2 = (6, 8);
                self.destinations.push((3, 3)).unwrap();
                self.destinations.push((2, 7)).unwrap();
                self.destinations.push((4, 11)).unwrap();
                self.destinations.push((5, 10)).unwrap();
            }
            5 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0);
                row!(0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 2, 0, 1, 0);
                row!(0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0);
                row!(0, 1, 0, 2, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0);
                row!(0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0);
                row!(0, 1, 0, 2, 2, 0, 0, 0, 1, 1, 0, 1, 0, 0);
                row!(0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 2, 0, 1, 0, 0, 1, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0);
                row!(0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0);
                self.player1 = (2, 11);
                self.player2 = (10, 11);
                self.destinations.push((6, 2)).unwrap();
                self.destinations.push((11, 4)).unwrap();
                self.destinations.push((11, 5)).unwrap();
                self.destinations.push((11, 6)).unwrap();
                self.destinations.push((11, 9)).unwrap();
            }
            6 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0);
                row!(0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0);
                row!(0, 0, 1, 1, 1, 1, 1, 0, 0, 2, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0);
                row!(0, 1, 1, 1, 1, 1, 1, 0, 2, 2, 0, 1, 0, 0);
                row!(0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0);
                row!(0, 1, 2, 1, 0, 0, 1, 0, 0, 0, 2, 0, 1, 0);
                row!(0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0);
                row!(0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (9, 11);
                self.player2 = (7, 2);
                self.destinations.push((9, 2)).unwrap();
                self.destinations.push((10, 3)).unwrap();
                self.destinations.push((8, 5)).unwrap();
                self.destinations.push((7, 9)).unwrap();
                self.destinations.push((2, 9)).unwrap();
                self.destinations.push((2, 3)).unwrap();
            }
            7 => {
                row!(0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0);
                row!(0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0);
                row!(0, 1, 0, 0, 0, 1, 2, 1, 0, 1, 1, 0, 0, 0);
                row!(0, 1, 0, 0, 0, 0, 0, 2, 0, 0, 1, 1, 1, 1);
                row!(0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 2, 0, 1);
                row!(0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 2, 0, 0, 1);
                row!(0, 1, 1, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 1);
                row!(0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0);
                row!(0, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0);
                row!(0, 0, 1, 1, 1, 0, 0, 1, 0, 2, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (3, 4);
                self.player2 = (10, 8);
                self.destinations.push((3, 2)).unwrap();
                self.destinations.push((5, 2)).unwrap();
                self.destinations.push((6, 3)).unwrap();
                self.destinations.push((7, 3)).unwrap();
                self.destinations.push((8, 3)).unwrap();
                self.destinations.push((2, 8)).unwrap();
            }
            8 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0);
                row!(0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0);
                row!(0, 0, 0, 1, 0, 0, 0, 2, 0, 1, 0, 1, 0, 0);
                row!(0, 0, 1, 1, 0, 2, 0, 1, 1, 1, 0, 1, 0, 0);
                row!(0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0);
                row!(0, 0, 1, 1, 0, 2, 0, 1, 1, 0, 0, 0, 1, 0);
                row!(0, 0, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1);
                row!(0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 1);
                row!(0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 1);
                row!(0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1);
                row!(0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (11, 10);
                self.player2 = (4, 10);
                self.destinations.push((9, 10)).unwrap();
                self.destinations.push((10, 8)).unwrap();
                self.destinations.push((6, 11)).unwrap();
                self.destinations.push((7, 3)).unwrap();
                self.destinations.push((5, 3)).unwrap();
                self.destinations.push((4, 6)).unwrap();
                self.destinations.push((2, 4)).unwrap();
            }
            9 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 1, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0);
                row!(0, 1, 0, 2, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0);
                row!(0, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 1, 0);
                row!(1, 1, 0, 2, 0, 2, 0, 1, 1, 1, 0, 0, 1, 0);
                row!(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0);
                row!(1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 1, 0);
                row!(1, 0, 2, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0);
                row!(1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0);
                row!(1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (7, 3);
                self.player2 = (7, 10);
                self.destinations.push((3, 5)).unwrap();
                self.destinations.push((6, 6)).unwrap();
                self.destinations.push((7, 4)).unwrap();
                self.destinations.push((9, 5)).unwrap();
                self.destinations.push((8, 9)).unwrap();
                self.destinations.push((6, 10)).unwrap();
                self.destinations.push((5, 11)).unwrap();
                self.destinations.push((11, 11)).unwrap();
            }
            10 => {
                row!(0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0);
                row!(0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0);
                row!(0, 1, 1, 1, 1, 0, 2, 1, 1, 2, 0, 1, 0, 0);
                row!(0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1);
                row!(0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1);
                row!(0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1);
                row!(0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1);
                row!(0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1);
                row!(0, 1, 2, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0);
                row!(0, 1, 0, 2, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0);
                row!(0, 1, 0, 0, 0, 0, 1, 1, 0, 2, 1, 1, 0, 0);
                row!(0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0);
                row!(0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 1, 0);
                row!(0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0);
                row!(0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0);
                self.player1 = (12, 11);
                self.player2 = (5, 2);
                self.destinations.push((3, 2)).unwrap();
                self.destinations.push((3, 3)).unwrap();
                self.destinations.push((3, 7)).unwrap();
                self.destinations.push((4, 5)).unwrap();
                self.destinations.push((6, 5)).unwrap();
                self.destinations.push((6, 6)).unwrap();
                self.destinations.push((6, 9)).unwrap();
                self.destinations.push((2, 10)).unwrap();
                self.destinations.push((12, 8)).unwrap();
            }
            11 => {
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                row!(1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0);
                row!(1, 0, 2, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0);
                row!(1, 0, 0, 0, 2, 0, 1, 1, 0, 0, 0, 0, 1, 0);
                row!(1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0);
                row!(0, 0, 1, 0, 2, 0, 1, 0, 1, 0, 0, 1, 1, 0);
                row!(1, 1, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0);
                row!(1, 0, 0, 0, 0, 2, 1, 0, 2, 2, 0, 0, 1, 0);
                row!(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0);
                row!(1, 1, 1, 2, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0);
                row!(0, 1, 0, 0, 0, 0, 1, 1, 2, 0, 2, 0, 1, 0);
                row!(0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0);
                row!(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                self.player1 = (4, 10);
                self.player2 = (4, 3);
                self.destinations.push((2, 4)).unwrap();
                self.destinations.push((2, 5)).unwrap();
                self.destinations.push((3, 8)).unwrap();
                self.destinations.push((7, 3)).unwrap();
                self.destinations.push((10, 2)).unwrap();
                self.destinations.push((10, 9)).unwrap();
                self.destinations.push((10, 11)).unwrap();
                self.destinations.push((9, 11)).unwrap();
                self.destinations.push((8, 11)).unwrap();
                self.destinations.push((7, 11)).unwrap();
            }
            _ => {}
        }
    }
    
    async fn draw_init(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new(0, 0), Size::new(128, 160))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
        let mut temp: String<20> = String::new();
        fmt::write(&mut temp, format_args!("Level: {}", self.level)).unwrap();
        Text::new( &temp, Point::new(10, 10), MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE))
            .draw(screen).unwrap();
        temp.clear();
        fmt::write(&mut temp, format_args!("Moves: {}", self.moves)).unwrap();
        Text::new(&temp, Point::new(10, 20), MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE))
            .draw(screen).unwrap();
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
        for (i, row) in self.frame.iter().enumerate() {
            for (j, item) in row.iter().enumerate() {
                match item {
                    1 => {
                        // wall, gray
                        Rectangle::new(Point::new(j as i32 * 9, i as i32 * 9 + OFFSET_X), Size::new(8, 8))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GRAY))
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
        if self.frame[(player.0 as i8 + x) as usize][(player.1 as i8 + y) as usize] == 0 && (player.0 + x as u8, player.1 + y as u8) != other {
            Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            self.frame[player.0 as usize][player.1 as usize] = 0;
            player.0 += x as u8;
            player.1 += y as u8;
            self.moves += 1;
        } else if self.frame[(player.0 as i8 + x) as usize][(player.1 as i8 + y) as usize] == 2 && self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] == 0 {
            self.frame[player.0 as usize][player.1 as usize] = 0;
            self.frame[(player.0 as i8 + 2 * x) as usize][(player.1 as i8 + 2 * y) as usize] = 2;
            Rectangle::new(Point::new(player.1 as i32 * 9, player.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(screen)
                .unwrap();
            player.0 += x as u8;
            player.1 += y as u8;
            self.moves += 1;
            Rectangle::new(Point::new((player.1 as i32 + y as i32) * 9, (player.0 as i32 + x as i32) * 9 + OFFSET_X), Size::new(8, 8))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_BROWN))
                .draw(screen)
                .unwrap();
        }
        self.correct_boxes = 0;
        for destination in self.destinations.iter() {
            if self.frame[destination.0 as usize][destination.1 as usize] == 2 {
                Rectangle::new(Point::new(destination.1 as i32 * 9, destination.0 as i32 * 9 + OFFSET_X), Size::new(8, 8))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                    .draw(screen)
                    .unwrap();
                self.correct_boxes += 1;
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
        Rectangle::new(Point::new(48, 13), Size::new(80, 10))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        let mut temp: String<20> = String::new();
        fmt::write(&mut temp, format_args!("{}", self.moves)).unwrap();
        Text::new( &temp, Point::new(52, 20), MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE))
            .draw(screen).unwrap();
        
    }

    pub async fn game_loop(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        self.draw_init(screen).await;
        loop {
            let input = INPUT_SIGNAL.wait().await;
            if self.handle_input(&input, screen) == false {
                // create pause menu
                let mut pause_menu: Menu<'_> = Menu::init("Pause menu", &[MenuOption::Resume, MenuOption::Restart, MenuOption::Next, MenuOption:: Previous, MenuOption::Exit], screen);
                let result: MenuOption = pause_menu.menu_loop(screen).await;
                info!("obtained result... somehow?");
                match result {
                    MenuOption::Resume | MenuOption::None => {
                        self.redraw(screen).await;
                        Timer::after(Duration::from_millis(100)).await;
                        INPUT_SIGNAL.reset();
                    },
                    MenuOption::Restart => {
                        self.init();
                        self.draw_init(screen).await;
                        Timer::after(Duration::from_millis(100)).await;
                        INPUT_SIGNAL.reset();
                    }
                    MenuOption::Next => {
                        if self.level < 11 {
                            self.level += 1;
                            self.init();
                            self.draw_init(screen).await;
                            Timer::after(Duration::from_millis(100)).await;
                            INPUT_SIGNAL.reset();
                        } else {
                            self.redraw(screen).await;
                        }
                    }
                    MenuOption::Previous => {
                        if self.level > 1 {
                            self.level -= 1;
                            self.init();
                            self.draw_init(screen).await;
                            Timer::after(Duration::from_millis(100)).await;
                            INPUT_SIGNAL.reset();
                        } else {
                            self.redraw(screen).await;
                        }
                    }
                    MenuOption::Exit => {
                        unsafe { CURRENT = 0 };
                        return;
                    }
                    _ => {}
                }
            } else {
                if self.correct_boxes == self.destinations.len() as u8 {
                    if self.level < 11 {
                        let mut pause_menu: Menu<'_> = Menu::init("Cleared!", &[MenuOption::Continue, MenuOption::Exit], screen);
                        let result: MenuOption = pause_menu.menu_loop(screen).await;
                        info!("obtained result... somehow?");
                        match result {
                            MenuOption::Continue | MenuOption::None => {
                                self.level += 1;
                                self.init();
                                self.draw_init(screen).await;
                                Timer::after(Duration::from_millis(100)).await;
                                INPUT_SIGNAL.reset();
                            },
                            MenuOption::Exit => {
                                unsafe { CURRENT = 0 };
                                return;
                            }
                            _ => {}
                        }
                    } else {
                        let mut pause_menu: Menu<'_> = Menu::init("The end!", &[MenuOption::Restart, MenuOption::Exit], screen);
                        let result: MenuOption = pause_menu.menu_loop(screen).await;
                        info!("obtained result... somehow?");
                        match result {
                            MenuOption::Restart | MenuOption::None => {
                                self.level = 1;
                                self.init();
                                self.draw_init(screen).await;
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
            Timer::after(Duration::from_millis(100)).await;
            INPUT_SIGNAL.reset();
        }
    }

    async fn redraw(&mut self, screen: &mut mipidsi::Display<SpiInterface<'_, &mut SpiDevice<'_, NoopRawMutex, Spi<'_, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_>>, Output<'_>>, ST7735s, Output<'_>>) {
        Rectangle::new(Point::new(0, 0), Size::new(128, 160))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(screen)
            .unwrap();
        let mut temp: String<20> = String::new();
        fmt::write(&mut temp, format_args!("Level: {}", self.level)).unwrap();
        Text::new( &temp, Point::new(10, 10), MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE))
            .draw(screen).unwrap();
        temp.clear();
        fmt::write(&mut temp, format_args!("Moves: {}", self.moves)).unwrap();
        Text::new(&temp, Point::new(10, 20), MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE))
            .draw(screen).unwrap();

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
        for (i, row) in self.frame.iter().enumerate() {
            for (j, item) in row.iter().enumerate() {
                match item {
                    1 => {
                        // wall, gray
                        Rectangle::new(Point::new(j as i32 * 9, i as i32 * 9 + OFFSET_X), Size::new(8, 8))
                            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GRAY))
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