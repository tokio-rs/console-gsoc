use crossterm;

use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::{InputEvent, KeyEvent};
use crossterm::{InputEvent, KeyEvent, RawScreen};

use console::ui;

enum Event {
    Input(InputEvent),
    Update,
}

fn main() -> Result<(), failure::Error> {
    let backend = CrosstermBackend::new();
    RawScreen::into_raw_mode()?.disable_drop();
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
                let reader = input.read_sync();
                for event in reader {
                    let close = InputEvent::Keyboard(KeyEvent::Char('q')) == event;
                    if let Err(_) = tx.send(Event::Input(event)) {
                        return;
                    }
                    if close {
                        return;
                    }
                }
            }
        });
    }

    // Setup 250ms tick rate
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

    // Setup instrumentation generation
    // Two threads, to apps
    {
        let store = store.clone();
        thread::Builder::new()
            .name("Server".to_string())
            .spawn(|| {
                let subscriber = storage::InProcessStore::new(store);
                tracing::subscriber::with_default(subscriber, || {
                    let kind = tracing_test::ApplicationKind::Server;
                    loop {
                        thread::sleep(Duration::from_millis(2000));
                        kind.emit();
                    }
                });
            })?;
    }
    {
        let store = store.clone();
        thread::Builder::new()
            .name("YakShave".to_string())
            .spawn(|| {
                let subscriber = storage::InProcessStore::new(store);
                tracing::subscriber::with_default(subscriber, || {
                    thread::sleep(Duration::from_millis(1000));
                    let kind = tracing_test::ApplicationKind::YakShave;
                    loop {
                        thread::sleep(Duration::from_millis(2000));
                        kind.emit();
                    }
                });
            })?;
    }

    let mut app = ui::App::new(handle);

    terminal.clear()?;
    loop {
        let draw = match rx.recv()? {
            Event::Input(event) => {
                if let Some(redraw) = app.input(event) {
                    redraw
                } else {
                    break;
                }
            }
            Event::Update => app.update(),
        };
        if draw {
            terminal.draw(|mut f| {
                app.render_to(&mut f);
            })?;
        }
    }
    terminal.clear()?;
    Ok(())
}
