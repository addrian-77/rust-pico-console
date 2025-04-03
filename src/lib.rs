trait Game {
    fn init(&mut self);
    fn update(&mut self, input: &InputState);
    fn render(&self);
}

enum InputState {
    Up,
    Down,
    Left,
    Right,
}

struct Player {
    x: i32,
    y: i32,
}

impl Game for Player {
    fn init(&mut self) {
        self.x = 8;
        self.y = 1;
    }

    fn update(&mut self, input: &InputState) {
        match input {
            InputState::Up => self.y = if self.y < 1 { 0 } else { self.y - 1 },
            InputState::Down => self.y = if self.y > 14 { 15 } else { self.y + 1 },
            InputState::Left => self.x = if self.x < 3 { 0 } else { self.x - 3 },
            InputState::Right => self.x = if self.x > 27 { 30 } else { self.x + 3 },
        }
    }

    fn render(&self) {
        for i in 0..16 {
            for j in 0..31 {
                if i == self.y && j == self.x {
                    print!("o");
                } else {
                    print!("-");
                }
            }
            print!("\n");
        }
        print!("\n");
    }
}

impl Copy for Player { }


impl Clone for Player {
    fn clone(&self) -> Player {
        *self
    }
}