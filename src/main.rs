include!("lib.rs");

use std::io::{stdin, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let mut game = Arc::new(Mutex::new(Player {
        x: 10,
        y: 10,
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
            game.update(&input);
        }

        {
            let game = game.lock().unwrap();
            game.render();
    
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn handle_input(tx: mpsc::Sender<InputState>) {
    let mut buffer = String::new();
    loop {
        print!("Write your input\n");
        std::io::stdout().flush().unwrap();

        buffer.clear();
        stdin().read_line(&mut buffer);
        
        match buffer.trim() {
            "u" => tx.send(InputState::Up).unwrap(),
            "d" => tx.send(InputState::Down).unwrap(),
            "l" => tx.send(InputState::Left).unwrap(),
            "r" => tx.send(InputState::Right).unwrap(),
            _ => print!("Invalid input\n"),
        }
    } 
}