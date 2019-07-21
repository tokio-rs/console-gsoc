use storage::{EventEntry, Store, ThreadId};

use crate::ui::{Hitbox, Input};
use tracing::Level;

use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::Frame;

use std::cell::Cell;

pub struct EventList {
    /// Cached rows, gets populated by `EventList::update`
    logs: Vec<EventEntry>,
    /// Index into logs vec, indicates which row the user selected
    selection: usize,
    /// How far the frame is offset by scrolling
    offset: usize,

    focused: bool,

    current_thread: Option<ThreadId>,
    thread_name: Option<String>,
    rect: Cell<Option<Rect>>,
}

impl EventList {
    pub(crate) fn new() -> EventList {
        EventList {
            focused: false,
            logs: Vec::new(),
            current_thread: None,
            thread_name: None,

            selection: 0,
            offset: 0,
            rect: Cell::new(None),
        }
    }

    pub(crate) fn update(&mut self, store: &Store, current_thread: Option<ThreadId>) -> bool {
        self.current_thread = current_thread;
        let current_thread = if let Some(current_thread) = current_thread {
            let name = store
                .threads
                .get(&current_thread)
                .expect("BUG: Invalid ThreadId created")
                .name
                .clone();
            self.thread_name = name;
            current_thread
        } else {
            return false;
        };

        let store = store
            .threads
            .get(&current_thread)
            .expect("BUG: No logs for the thread");
        let lines = &store.lines;
        let logs = lines.clone();
        let rerender = self.logs != logs;
        self.logs = logs;
        rerender
    }

    /// Adjusts the window if necessary, to make sure the selection is in frame
    ///
    /// In case we don't adjust ourself, `SelectableList` will do it on its own
    /// with unexpected ux effects, like the whole selection moves even though
    /// the selection has "space" to move without adjusting
    fn adjust_window_to_selection(&mut self) -> bool {
        // Calc the largest index that will be still in frame
        let rowcount = self.rect.get().unwrap_or_default().height as usize - 2;
        let upper_limit = self.offset + rowcount;

        if self.selection < self.offset {
            // The text cursor wants to move out on the upper side
            // Set the
            self.offset = self.selection;
            true
        } else if self.selection + 1 > upper_limit {
            // + 1: Upper_limit is a length, offset the index
            self.offset += (self.selection + 1) - upper_limit;
            true
        } else {
            false
        }
    }

    pub(crate) fn on_up(&mut self) -> bool {
        let new_offset = self.selection.saturating_sub(1);
        self.select(new_offset)
    }

    pub(crate) fn on_down(&mut self) -> bool {
        let new_offset = self.selection.saturating_add(1);
        self.select(new_offset)
    }

    fn select(&mut self, mut new_offset: usize) -> bool {
        if self.logs.len() < new_offset {
            new_offset = self.logs.len() - 1;
        }

        let rerender = new_offset != self.selection;
        self.selection = new_offset;

        // If the frame or the index changed, rerender for correct frame / highlighting
        // Adjust has side effects, it needs to be called first
        self.adjust_window_to_selection() || rerender
    }

    fn style_event(&self, i: usize, entry: &EventEntry) -> Vec<Text<'_>> {
        let level = match *entry.level() {
            Level::INFO => Text::styled(" INFO ", Style::default().fg(Color::White)),
            Level::DEBUG => Text::styled("DEBUG ", Style::default().fg(Color::LightCyan)),
            Level::ERROR => Text::styled("ERROR ", Style::default().fg(Color::Red)),
            Level::TRACE => Text::styled("TRACE ", Style::default().fg(Color::Green)),
            Level::WARN => Text::styled(" WARN ", Style::default().fg(Color::Yellow)),
        };
        let text = format!("{}\n", entry.display());
        if i == self.selection - self.offset {
            vec![
                level,
                Text::styled(text, Style::default().modifier(Modifier::BOLD)),
            ]
        } else {
            vec![level, Text::raw(text)]
        }
    }

    pub(crate) fn render_to(
        &self,
        f: &mut Frame<CrosstermBackend>,
        r: Rect,
        current_thread: Option<ThreadId>,
    ) {
        self.rect.set(Some(r));
        // - 2: Upper and lower border of window
        let rowcount = r.height as usize - 2;

        if let Some(current_thread) = current_thread {
            let (border_color, title_color) = self.border_color();
            let block_title = format!(
                "Events(Thread {}{}) {}-{}/{}",
                current_thread.0,
                if let Some(name) = &self.thread_name {
                    format!(r#": "{}""#, name)
                } else {
                    "".to_string()
                },
                1 + self.offset,
                self.offset + std::cmp::min(rowcount, self.logs.len()),
                self.logs.len(),
            );
            Paragraph::new(
                self.logs
                    .iter()
                    .skip(self.offset)
                    .take(rowcount)
                    .enumerate()
                    .map(|(i, e)| self.style_event(i, e))
                    .flatten()
                    .collect::<Vec<Text<'_>>>()
                    .iter(),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(&block_title)
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, r);
        } else {
            let logs = vec![Text::raw("--- No Messages ---")].into_iter();
            List::new(logs)
                .block(Block::default().borders(Borders::ALL).title("Messages"))
                .render(f, r);
        }
    }
}

impl Input for EventList {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn focused(&self) -> bool {
        self.focused
    }

    fn on_click(&mut self, x: u16, y: u16) -> bool {
        let rect = self.rect.get().unwrap_or_default().inner(1);
        if !rect.hit(x, y) {
            return false;
        }

        self.select((y - rect.y - 1) as usize)
    }
}
