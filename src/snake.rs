trait Game {
    fn init(&mut self);
    fn handle_input(&mut self, input: &Input);
    fn update_frame(&mut self);
    fn draw_frame(&self);
    // fn difficulty_speed(&self);
    // fn difficulty_size(&self);
}

enum Input {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq)]
enum Facing {
    Up,
    Down,
    Left,
    Right,
}

struct Snake {
    head: (i8, i8, Facing),
    tail: (i8, i8),
    frame: Vec<Vec<char>>,
}

impl Game for Snake {
    fn init(&mut self) {
        self.head = (8, 5, Facing::Right);
        self.tail = (8, 3);
        self.frame = vec![vec!['-'; 30]; 15];
        self.frame[self.head.0 as usize][self.head.1 as usize] = 'o';
        self.frame[self.head.0 as usize][self.head.1 as usize - 1] = 'o';
        self.frame[self.tail.0 as usize][self.tail.1 as usize] = 'o';
    }

    fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Up => if self.head.2 != Facing::Down { self.head.0 += 1; self.head.2 = Facing::Up } else { print!("ignored"); return },
            Input::Down => if self.head.2 != Facing::Up { self.head.0 -= 1; self.head.2 = Facing::Down } else { print!("ignored"); return },
            Input::Left => if self.head.2 != Facing::Right { self.head.1 -= 1; self.head.2 = Facing::Left } else { print!("ignored"); return },
            Input::Right => if self.head.2 != Facing::Left { self.head.1 += 1; self.head.2 = Facing::Right } else { print!("ignored"); return },
        }
    }

    fn update_frame(&mut self) {
        
    }

    fn draw_frame(&self) {
        print!("\n\n\n\n\n");
        for row in self.frame.iter() {
            for item in row.iter() {
                print!("{}", item);
            }
            print!("\n");
        }
    }
}
