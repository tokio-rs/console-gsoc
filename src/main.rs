use crossterm;

use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

enum Event<I> {
    Input(I),
    Tick,
}

fn main() -> Result<(), failure::Error> {
    let backend = CrosstermBackend::new();
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let input = crossterm::input();
            loop {
                match input.read_char() {
                    Ok(key) => {
                        if let Err(_) = tx.send(Event::Input(key)) {
                            return;
                        }
                        if key == 'q' {
                            return;
                        }
                    }
                    Err(_) => {}
                }
            }
        });
    }
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let tx = tx.clone();
            loop {
                tx.send(Event::Tick).unwrap();
                thread::sleep(Duration::from_millis(250));
            }
        });
    }

    terminal.clear();

    loop {
        console::ui::draw(&mut terminal)?;
        match rx.recv()? {
            Event::Input(key) => match key {
                'q' => break,
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}
