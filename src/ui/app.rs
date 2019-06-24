use storage::{Store, ThreadId};

use tokio_trace::Level;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, SelectableList, Text, Widget};
use tui::Frame;

use crossterm::{InputEvent, KeyEvent};

use std::cell::Cell;
use std::sync::{Arc, Mutex};

pub struct App {
    store: Arc<Mutex<Store>>,
    current_thread: Cell<Option<ThreadId>>,
}

impl App {
    pub fn new(store: Arc<Mutex<Store>>) -> App {
        App {
            store,
            current_thread: Cell::new(None),
        }
    }

    fn on_up(&mut self) {
        // TODO: Cache UI elements and access cache
        let store = self.store.lock().unwrap();
        if let Some(current_id) = self.current_thread.get() {
            let mut threads = store
                .threads
                .iter()
                .map(|(key, store)| (key, &store.name))
                .collect::<Vec<(&ThreadId, &Option<String>)>>();
            threads.sort_by_key(|(&id, _)| id);
            let current_index = threads
                .iter()
                .position(|(id, _)| current_id == **id)
                .expect("BUG: Current thread id not in list");
            if let Some((id, _)) = threads.get(current_index.saturating_sub(1)) {
                self.current_thread.set(Some(**id))
            }
        }
    }

    fn on_down(&mut self) {
        // TODO: Cache UI elements and access cache
        let store = self.store.lock().unwrap();
        if let Some(current_id) = self.current_thread.get() {
            let mut threads = store
                .threads
                .iter()
                .map(|(key, store)| (key, &store.name))
                .collect::<Vec<(&ThreadId, &Option<String>)>>();
            threads.sort_by_key(|(&id, _)| id);
            let current_index = threads
                .iter()
                .position(|(id, _)| current_id == **id)
                .expect("BUG: Current thread id not in list");
            if let Some((id, _)) = threads.get(current_index.saturating_add(1)) {
                self.current_thread.set(Some(**id))
            }
        }
    }

    fn thread_selector(&self, store: &Store, f: &mut Frame<CrosstermBackend>, r: Rect) {
        // TODO: Cache UI elements
        let mut threads = store
            .threads
            .iter()
            .map(|(key, store)| (key, &store.name))
            .collect::<Vec<(&ThreadId, &Option<String>)>>();
        threads.sort_by_key(|(&id, _)| id);
        let index = self.current_thread.get().and_then(|current_id| {
            threads.iter().enumerate().find_map(
                |(i, (id, _))| {
                    if current_id == **id {
                        Some(i)
                    } else {
                        None
                    }
                },
            )
        });
        SelectableList::default()
            .highlight_style(Style::default().modifier(Modifier::BOLD))
            .items(
                &threads
                    .iter()
                    .map(|(key, name)| {
                        format!(
                            "{} - {}",
                            key.0,
                            if let Some(name) = name { name } else { "" }
                        )
                    })
                    .collect::<Vec<String>>(),
            )
            .select(index)
            .block(Block::default().borders(Borders::ALL).title("Threads"))
            .render(f, r);
    }

    /// Returns if the scene has to be redrawn
    pub fn input(&mut self, event: InputEvent) -> Option<bool> {
        let redraw = match event {
            InputEvent::Keyboard(key) => match key {
                KeyEvent::Char('q') => return None,
                KeyEvent::Up => {
                    self.on_up();
                    true
                }
                KeyEvent::Down => {
                    self.on_down();
                    true
                }
                _ => false,
            },
            _ => false,
        };
        Some(redraw)
    }

    pub fn render_to(&self, f: &mut Frame<CrosstermBackend>) {
        let store = self.store.lock().unwrap();
        if self.current_thread.get().is_none() {
            if store.threads.len() == 1 {
                // There is at least one element
                let thread_id = store.threads.keys().next().unwrap();
                self.current_thread.set(Some(*thread_id));
            } else {
                if let Some(thread_id) = store.threads.keys().min() {
                    self.current_thread.set(Some(*thread_id));
                }
            }
        } else {
            if !store.updated() {
                return;
            }
        }

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .direction(Direction::Horizontal)
            .split(f.size());

        self.thread_selector(&store, f, chunks[0]);
        if let Some(current_thread) = &self.current_thread.get() {
            let lines = &store
                .threads
                .get(current_thread)
                .expect("BUG: No logs for the thread")
                .lines;
            let len = lines.len();
            let logs = lines.iter().map(|(level, text)| {
                Text::styled(
                    text,
                    match *level {
                        Level::INFO => Style::default().fg(Color::White),
                        Level::ERROR => Style::default().fg(Color::Red),
                        Level::WARN => Style::default().fg(Color::LightRed),
                        Level::TRACE => Style::default().fg(Color::Cyan),
                        Level::DEBUG => Style::default().fg(Color::LightMagenta),
                    },
                )
            });
            List::new(logs)
                .block(Block::default().borders(Borders::ALL).title(&format!(
                    "Messages {}-{}/{} ",
                    1,
                    std::cmp::min((chunks[1].height - 2) as usize, len),
                    len
                )))
                .render(f, chunks[1]);
        } else {
            let logs = vec![Text::raw("--- No Messages ---")].into_iter();
            List::new(logs)
                .block(Block::default().borders(Borders::ALL).title("Messages"))
                .render(f, chunks[1]);
        }
    }
}
