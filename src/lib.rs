#![no_std]

#[derive(PartialEq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Left_Shoot,
    Right_Shoot,
    Up2,
    Down2,
    Left2,
    Right2,
    Left2_Shoot,
    Right2_Shoot,
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
    Restart,
    Exit,
    Debug,
}