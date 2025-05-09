#![no_std]

#[derive(PartialEq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Fire,
    Up2,
    Down2,
    Left2,
    Right2,
    Fire2,
    Select,
    Back,
    Ignore
}

#[derive(Clone,Copy)]
pub enum MenuOption {
    None,
    Snake,
    SpaceInvaders,
    Sokoban,
    Resume, 
    Exit,
    Debug,
}