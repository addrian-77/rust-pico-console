include!("snake.rs");

use std::io::{stdin, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let mut game = Arc::new(Mutex::new(Snake {
        head: (8, 5, Facing::Right),
        tail: (8, 3),
        frame: (0..15).map(|_| Vec::with_capacity(30)).collect(),
    }));
    game.lock().unwrap().init();

    let(tx, rx) = mpsc::channel();
    let game_clone = Arc::clone(&game);
    
    thread::spawn(move || {
        handle_input(tx);
    });
    loop {
        if let Ok(input) = rx.try_recv() {
            let mut game = game_clone.lock().unwrap();
            game.handle_input(&input);
            game.update_frame();
        }

        {
            let mut game = game.lock().unwrap();
            game.draw_frame();
    
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn handle_input(tx: mpsc::Sender<Input>) {
    let mut buffer = String::new();
    loop {
        print!("Write your input\n");
        std::io::stdout().flush().unwrap();

        buffer.clear();
        stdin().read_line(&mut buffer);
        
        match buffer.trim() {
            "u" => tx.send(Input::Up).unwrap(),
            "d" => tx.send(Input::Down).unwrap(),
            "l" => tx.send(Input::Left).unwrap(),
            "r" => tx.send(Input::Right).unwrap(),
            _ => print!("Invalid input\n"),
        }
    } 
}