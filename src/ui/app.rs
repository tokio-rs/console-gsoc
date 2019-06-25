use storage::{Store, ThreadId};

use tokio_trace::Level;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, SelectableList, Text, Widget};
use tui::Frame;

use crossterm::{InputEvent, KeyEvent};

use crate::ui::ThreadSelector;

use std::sync::{Arc, Mutex};

pub struct App {
    store: Arc<Mutex<Store>>,
    thread_selector: ThreadSelector,
}

impl App {
    pub fn new(store: Arc<Mutex<Store>>) -> App {
        App {
            store,
            thread_selector: ThreadSelector::new(),
        }
    }

    pub fn update(&mut self) -> bool {
        let store = self.store.lock().unwrap();
        if store.updated() {
            self.thread_selector.update(&store)
        } else {
            false
        }
    }

    fn on_up(&mut self) -> bool {
        self.thread_selector.on_up()
    }
    fn on_down(&mut self) -> bool {
        self.thread_selector.on_down()
    }

    /// Returns if the scene has to be redrawn
    pub fn input(&mut self, event: InputEvent) -> Option<bool> {
        let redraw = match event {
            InputEvent::Keyboard(key) => match key {
                KeyEvent::Char('q') => return None,
                KeyEvent::Up => self.on_up(),
                KeyEvent::Down => self.on_down(),
                _ => false,
            },
            _ => false,
        };
        Some(redraw)
    }

    pub fn render_to(&self, f: &mut Frame<CrosstermBackend>) {
        let store = self.store.lock().unwrap();
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .direction(Direction::Horizontal)
            .split(f.size());

        self.thread_selector.render_to(f, chunks[0]);
        if let Some(current_thread) = &self.thread_selector.current_thread() {
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
