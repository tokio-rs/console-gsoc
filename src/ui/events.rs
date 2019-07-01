use storage::{Store, ThreadId};

use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Modifier, Style};
use tui::widgets::{Block, Borders, List, SelectableList, Text, Widget};
use tui::Frame;

use std::cell::Cell;

pub struct EventList {
    /// Cached rows, gets populated by `EventList::update`
    logs: Vec<String>,
    /// Index into logs vec, indicates which row the user selected
    selection: usize,
    /// How far the frame is offset by scrolling
    offset: usize,

    focused: bool,
    rowcount: Cell<usize>,

    current_thread: Option<ThreadId>,
    thread_name: Option<String>,
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
            rowcount: Cell::new(0),
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
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
        let logs = lines
            .iter()
            .enumerate()
            .map(|(index, (level, text))| {
                format!(
                    "{}{}: {:?}: {}",
                    // FIXME: Remove this when we render based on a paragraph
                    //
                    // If something is highlighted, tui somehow inserts a leading space
                    // We just insert one ourself, to make up when the widget is inactvive
                    if self.focused { "" } else { " " },
                    index + 1,
                    level,
                    text
                )
            })
            .skip(self.offset)
            .collect();
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
        let upper_limit = self.offset + self.rowcount.get();

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
        let rerender = new_offset != self.selection;
        self.selection = new_offset;

        // If the frame or the index changed, rerender for correct frame / highlighting
        // Adjust has side effects, it needs to be called first
        self.adjust_window_to_selection() || rerender
    }

    pub(crate) fn on_down(&mut self) -> bool {
        let new_offset = self.selection.saturating_add(1);
        let rerender = new_offset != self.selection;

        // Check if we are already at the last line in the buffer
        // A similar check is not necessary in `on_up` because
        // `saturating_add` already caps at 0
        if self.current_thread.is_some() {
            if self.logs.len() < new_offset {
                // We are at the end of the buffer, nothing changed
                return false;
            }
        } else {
            // There is no thread selected, so nothing to render for us
            return false;
        }

        self.selection = new_offset;
        self.adjust_window_to_selection() || rerender
    }

    pub(crate) fn render_to(
        &self,
        f: &mut Frame<CrosstermBackend>,
        r: Rect,
        current_thread: Option<ThreadId>,
    ) {
        // - 2: Upper and lower border of window
        self.rowcount.set(r.height as usize - 2);

        if let Some(current_thread) = current_thread {
            SelectableList::default()
                .highlight_style(Style::default().modifier(Modifier::BOLD))
                .items(&self.logs)
                .select(if self.focused {
                    Some(self.selection - self.offset)
                } else {
                    None
                })
                .block(Block::default().borders(Borders::ALL).title(&format!(
                    "Events(Thread {}{}) {}-{}/{}",
                    current_thread.0,
                    if let Some(name) = &self.thread_name {
                        format!(r#": "{}""#, name)
                    } else {
                        "".to_string()
                    },
                    1 + self.offset,
                    self.offset + std::cmp::min(self.rowcount.get(), self.logs.len()),
                    self.logs.len(),
                )))
                .render(f, r);
        } else {
            let logs = vec![Text::raw("--- No Messages ---")].into_iter();
            List::new(logs)
                .block(Block::default().borders(Borders::ALL).title("Messages"))
                .render(f, r);
        }
    }
}
