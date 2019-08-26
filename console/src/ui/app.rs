use crate::storage::StoreHandle;

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::widgets::{Paragraph, Text, Widget};
use tui::Frame;
use tui::Terminal;

use crossterm::{InputEvent, KeyEvent, MouseEvent, RawScreen};

use crate::filter::*;
use crate::ui::Command;
use crate::ui::{Action, EventList, Hitbox, Input, QueryView};

use std::cell::Cell;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub(crate) enum Focus {
    Events,
    Query,
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
            let _ = input.enable_mouse_mode();
            loop {
                let reader = input.read_sync();
                for event in reader {
                    let close = InputEvent::Keyboard(KeyEvent::Esc) == event;
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
    query_view: QueryView,

    filter: Filter,
    filter_updated: bool,

    rect: Cell<Option<(Rect, Rect)>>,
    rx: mpsc::Receiver<Event>,
}

impl App {
    pub fn new(store: StoreHandle) -> Result<App, failure::Error> {
        Ok(App {
            store,
            focus: Focus::Query,

            event_list: EventList::new(),
            query_view: QueryView::new(),

            filter: Filter::default(),
            filter_updated: false,

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
        let mut set_cursor = None;
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
                if let Some((x, y)) = set_cursor {
                    let _ = terminal.set_cursor(x, y);
                    let _ = terminal.show_cursor();
                } else {
                    let _ = terminal.hide_cursor();
                }
            }
            let cursor = self.show_cursor();
            if cursor != set_cursor {
                if let Some((x, y)) = cursor {
                    let _ = terminal.set_cursor(x, y);
                    let _ = terminal.show_cursor();
                    set_cursor = cursor;
                } else {
                    let _ = terminal.hide_cursor();
                }
            }
        }
        terminal.clear()?;
        Ok(())
    }

    pub fn update(&mut self) -> bool {
        let store = self.store.0.lock().unwrap();
        if store.updated() || self.filter_updated {
            let event_list = self.event_list.update(&store, &self.filter);
            self.filter_updated = false;
            let query_view = self.query_view.update(self.filter.clone());

            let rerender = event_list || query_view;
            rerender
        } else {
            false
        }
    }

    fn show_cursor(&self) -> Option<(u16, u16)> {
        match self.focus {
            Focus::Events => self.event_list.show_cursor(),
            Focus::Query => self.query_view.show_cursor(),
        }
    }

    fn on_up(&mut self) -> bool {
        match self.focus {
            Focus::Events => self.event_list.on_up(),
            Focus::Query => self.query_view.on_up(),
        }
    }

    fn on_down(&mut self) -> bool {
        match self.focus {
            Focus::Events => self.event_list.on_down(),
            Focus::Query => self.query_view.on_down(),
        }
    }
    fn on_char(&mut self, c: char) -> Action {
        match self.focus {
            Focus::Events => self.event_list.on_char(c),
            Focus::Query => self.query_view.on_char(c),
        }
    }
    fn on_backspace(&mut self) -> bool {
        match self.focus {
            Focus::Events => self.event_list.on_backspace(),
            Focus::Query => self.query_view.on_backspace(),
        }
    }

    fn focus_event(&mut self) -> bool {
        let rerender = self.focus != Focus::Events;
        self.focus = Focus::Events;
        self.query_view.set_focused(false);
        self.event_list.set_focused(true);
        rerender
    }

    fn focus_query(&mut self) -> bool {
        let rerender = self.focus != Focus::Query;
        self.focus = Focus::Query;
        self.query_view.set_focused(true);
        self.event_list.set_focused(false);
        rerender
    }

    fn on_left(&mut self) -> bool {
        self.focus_query()
    }

    fn on_right(&mut self) -> bool {
        self.focus_event()
    }

    /// Returns if the scene has to be redrawn
    pub fn input(&mut self, event: InputEvent) -> Option<bool> {
        let redraw = match event {
            InputEvent::Keyboard(key) => match key {
                KeyEvent::Esc => return None,
                KeyEvent::Char(c) => {
                    let action = self.on_char(c);
                    let redraw = action.redraw();
                    match action {
                        Action::Command(Command::Modifier(modifier)) => {
                            self.filter.insert_modifier(modifier);
                            self.filter_updated = true;
                        }
                        Action::Command(Command::GroupBy(group_by)) => {
                            self.filter.group(group_by);
                            self.filter_updated = true;
                        }
                        _ => {}
                    }
                    redraw
                }
                KeyEvent::Backspace => self.on_backspace(),
                KeyEvent::Up => self.on_up(),
                KeyEvent::Down => self.on_down(),
                KeyEvent::Left => self.on_left(),
                KeyEvent::Right => self.on_right(),
                _ => false,
            },
            InputEvent::Mouse(event) => match event {
                MouseEvent::Release(x, y) => {
                    let (query_rect, event_rect) = self.rect.get().unwrap_or_default();
                    if query_rect.hit(x, y) {
                        let rerender = self.focus_query();
                        self.query_view.on_click(x, y);
                        rerender
                    } else if event_rect.hit(x, y) {
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
        // TODO: Investigate offset
        rect.height -= 2;
        legend_rect.y += legend_rect.height - 1;
        legend_rect.height = 1;

        let chunks = Layout::default()
            .constraints([Constraint::Length(50), Constraint::Min(10)].as_ref())
            .direction(Direction::Horizontal)
            .split(rect);

        self.query_view.render_to(f, chunks[0]);
        self.event_list.render_to(f, chunks[1]);

        Paragraph::new([Text::raw(" ESC: close, ← → ↑ ↓ click: navigate")].iter())
            .render(f, legend_rect);
        Paragraph::new([Text::raw("prerelease version ")].iter())
            .alignment(Alignment::Right)
            .render(f, legend_rect);

        self.rect.set(Some((chunks[0], chunks[1])));
    }
}
