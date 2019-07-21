use crate::storage::StoreHandle;

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Rect};
use tui::widgets::{Paragraph, Text, Widget};
use tui::Frame;
use tui::Terminal;

use crossterm::{InputEvent, KeyEvent, MouseEvent, RawScreen};

use crate::ui::{EventList, Hitbox, Input};

use std::cell::Cell;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(PartialEq)]
enum Focus {
    Events,
}

enum Event {
    Input(InputEvent),
    Update,
}

fn setup_input_handling() -> mpsc::Receiver<Event> {
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
    rx
}

pub struct App {
    store: StoreHandle,
    focus: Focus,

    event_list: EventList,

    rect: Cell<Option<Rect>>,
    rx: mpsc::Receiver<Event>,
}

impl App {
    pub fn new(store: StoreHandle) -> Result<App, failure::Error> {
        Ok(App {
            store,
            focus: Focus::Events,

            event_list: EventList::new(),

            rect: Cell::new(None),
            rx: setup_input_handling(),
        })
    }

    pub fn run(&mut self) -> Result<(), failure::Error> {
        let backend = CrosstermBackend::new();
        RawScreen::into_raw_mode()?.disable_drop();
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        terminal.clear()?;
        loop {
            let draw = match self.rx.recv()? {
                Event::Input(event) => {
                    if let Some(redraw) = self.input(event) {
                        redraw
                    } else {
                        break;
                    }
                }
                Event::Update => self.update(),
            };
            if draw {
                terminal.draw(|mut f| {
                    self.render_to(&mut f);
                })?;
            }
        }
        terminal.clear()?;
        Ok(())
    }

    pub fn update(&mut self) -> bool {
        let store = self.store.0.lock().unwrap();
        if store.updated() {
            let event_list = self.event_list.update(&store);

            let rerender = event_list;
            rerender
        } else {
            false
        }
    }

    fn on_up(&mut self) -> bool {
        match self.focus {
            Focus::Events => self.event_list.on_up(),
        }
    }

    fn on_down(&mut self) -> bool {
        match self.focus {
            Focus::Events => self.event_list.on_down(),
        }
    }

    fn focus_event(&mut self) -> bool {
        let rerender = self.focus != Focus::Events;
        self.focus = Focus::Events;
        self.event_list.set_focused(true);
        rerender
    }

    fn on_left(&mut self) -> bool {
        false
    }

    fn on_right(&mut self) -> bool {
        self.focus_event()
    }

    /// Returns if the scene has to be redrawn
    pub fn input(&mut self, event: InputEvent) -> Option<bool> {
        let redraw = match event {
            InputEvent::Keyboard(key) => match key {
                KeyEvent::Char('q') => return None,
                KeyEvent::Up => self.on_up(),
                KeyEvent::Down => self.on_down(),
                KeyEvent::Left => self.on_left(),
                KeyEvent::Right => self.on_right(),
                _ => false,
            },
            InputEvent::Mouse(event) => match event {
                MouseEvent::Release(x, y) => {
                    let event_rect = self.rect.get().unwrap_or_default();
                    if event_rect.hit(x, y) {
                        let rerender = self.focus_event();
                        self.event_list.on_click(x, y);
                        rerender
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        };
        Some(redraw)
    }

    pub fn render_to(&mut self, f: &mut Frame<CrosstermBackend>) {
        let mut rect = f.size();
        let mut legend_rect = rect;
        // Reserve space for legend
        rect.height -= 1;
        legend_rect.y += legend_rect.height - 1;
        legend_rect.height = 1;

        self.event_list.render_to(f, rect);
        Paragraph::new([Text::raw(" q: close, ← → ↑ ↓ click: navigate")].iter())
            .render(f, legend_rect);
        Paragraph::new([Text::raw("prerelease version ")].iter())
            .alignment(Alignment::Right)
            .render(f, legend_rect);
        self.rect.set(Some(rect));
    }
}
