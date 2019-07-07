use storage::Store;

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::widgets::{Paragraph, Text, Widget};
use tui::Frame;

use crossterm::{InputEvent, KeyEvent, MouseEvent};

use crate::ui::{EventList, Hitbox, Input, ThreadSelector};

use std::cell::Cell;
use std::sync::{Arc, Mutex};

#[derive(PartialEq)]
enum Focus {
    ThreadSelector,
    Events,
}

pub struct App {
    store: Arc<Mutex<Store>>,
    focus: Focus,

    event_list: EventList,
    thread_selector: ThreadSelector,

    rect: Cell<Option<(Rect, Rect)>>,
}

impl App {
    pub fn new(store: Arc<Mutex<Store>>) -> App {
        App {
            store,
            focus: Focus::ThreadSelector,

            event_list: EventList::new(),
            thread_selector: ThreadSelector::new(),

            rect: Cell::new(None),
        }
    }

    pub fn update(&mut self) -> bool {
        let store = self.store.lock().unwrap();
        if store.updated() {
            let thread_list = self.thread_selector.update(&store);
            let event_list = self
                .event_list
                .update(&store, self.thread_selector.current_thread());

            let rerender = thread_list || event_list;
            rerender
        } else {
            false
        }
    }

    fn on_up(&mut self) -> bool {
        match self.focus {
            Focus::ThreadSelector => self.thread_selector.on_up(),
            Focus::Events => self.event_list.on_up(),
        }
    }

    fn on_down(&mut self) -> bool {
        match self.focus {
            Focus::ThreadSelector => self.thread_selector.on_down(),
            Focus::Events => self.event_list.on_down(),
        }
    }

    fn focus_thread(&mut self) -> bool {
        let rerender = self.focus != Focus::ThreadSelector;
        self.focus = Focus::ThreadSelector;
        self.thread_selector.set_focused(true);
        self.event_list.set_focused(false);
        rerender
    }

    fn focus_event(&mut self) -> bool {
        let rerender = self.focus != Focus::Events;
        self.focus = Focus::Events;
        self.thread_selector.set_focused(false);
        self.event_list.set_focused(true);
        rerender
    }

    fn on_left(&mut self) -> bool {
        self.focus_thread()
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
                    let (thread_rect, event_rect) = self.rect.get().unwrap_or_default();
                    if thread_rect.hit(x, y) {
                        let rerender = self.focus_thread();
                        self.thread_selector.on_click(x, y);
                        rerender
                    } else if event_rect.hit(x, y) {
                        let rerender = self.focus_event();
                        self.event_list.on_click(x, y);
                        rerender
                    } else {
                        panic!("Event was wrongly routed")
                    }
                }
                _ => false,
            },
            _ => false,
        };
        Some(redraw)
    }

    pub fn render_to(&self, f: &mut Frame<CrosstermBackend>) {
        let mut rect = f.size();
        let mut legend_rect = rect;
        // Reserve space for legend
        rect.height -= 1;
        legend_rect.y += legend_rect.height - 1;
        legend_rect.height = 1;

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .direction(Direction::Horizontal)
            .split(rect);

        self.thread_selector.render_to(f, chunks[0]);
        self.event_list
            .render_to(f, chunks[1], self.thread_selector.current_thread());
        Paragraph::new([Text::raw(" q: close, ← → ↑ ↓ click: navigate")].iter())
            .render(f, legend_rect);
        Paragraph::new([Text::raw("prerelease version ")].iter())
            .alignment(Alignment::Right)
            .render(f, legend_rect);
        self.rect.set(Some((chunks[0], chunks[1])));
    }
}
