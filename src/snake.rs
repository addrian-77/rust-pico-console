use std::collections::VecDeque;


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
    trail: VecDeque<(i8, i8)>,
    can_update: bool,
    frame: Vec<Vec<char>>,
}

impl Game for Snake {
    fn init(&mut self) {
        self.head = (8, 5, Facing::Right);
        self.frame = vec![vec!['-'; 30]; 15];
        self.can_update = false;
        // push 8,5 8,4 8,3              tail = trail.pop()
        self.frame[self.head.0 as usize][self.head.1 as usize] = 'o';
        self.trail.push_back((self.head.0, self.head.1));
        self.frame[self.head.0 as usize][self.head.1 as usize - 1] = 'o';
        self.trail.push_back((self.head.0, self.head.1 -1));
        self.frame[self.head.0 as usize][self.head.1 as usize - 2] = 'o';
        self.trail.push_back((self.head.0, self.head.1 - 2));
    }

    fn handle_input(&mut self, input: &Input) {
        match input {
            Input::Up => if self.head.2 != Facing::Down { self.head.0 -= 1; self.head.2 = Facing::Up; self.can_update = true} else { self.can_update = false },
            Input::Down => if self.head.2 != Facing::Up { self.head.0 += 1; self.head.2 = Facing::Down; self.can_update = true } else { self.can_update = false },
            Input::Left => if self.head.2 != Facing::Right { self.head.1 -= 1; self.head.2 = Facing::Left; self.can_update = true } else { self.can_update = false },
            Input::Right => if self.head.2 != Facing::Left { self.head.1 += 1; self.head.2 = Facing::Right; self.can_update = true } else { self.can_update = false },
        }
    }

    fn update_frame(&mut self) {
        if self.can_update == true {
            let (tail_x, tail_y) = self.trail.pop_back().unwrap_or((0, 0));
            self.trail.push_front((self.head.0, self.head.1));
            self.frame[tail_x as usize][tail_y as usize] = '-';
            self.frame[self.head.0 as usize][self.head.1 as usize] = 'o';
        }
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
