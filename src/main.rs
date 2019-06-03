use crossterm;

use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

enum Event {
    Input(char),
    Update,
}

fn main() -> Result<(), failure::Error> {
    let backend = CrosstermBackend::new();
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let store = Arc::new(Mutex::new(storage::Store::new()));
    let handle = store.clone();

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
                tx.send(Event::Update).unwrap();
                thread::sleep(Duration::from_millis(250));
            }
        });
    }

    // Setup instrumentation beacon
    thread::spawn(|| {
        let subscriber = storage::InProcessStore::new(store);
        tokio_trace::subscriber::with_default(subscriber, || {
            let kind = tokio_trace_test::ApplicationKind::Server;
            loop {
                thread::sleep(Duration::from_millis(2000));
                kind.emit();
            }
        });
    });

    terminal.clear()?;
    loop {
        match rx.recv()? {
            Event::Input(key) => match key {
                'q' => break,
                _ => {}
            },
            Event::Update => {
                let store = handle.lock().unwrap();
                if store.updated() {
                    console::ui::draw(&mut terminal, &store)?;
                }
            }
        }
    }
    terminal.clear()?;
    Ok(())
}
