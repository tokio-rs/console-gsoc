use storage::{Store, ThreadId};
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Modifier, Style};
use tui::widgets::{Block, Borders, SelectableList, Widget};
use tui::Frame;

use crate::ui::{Hitbox, Input};
use std::cell::Cell;

pub struct ThreadSelector {
    current_thread: Option<ThreadId>,
    threads: Vec<(ThreadId, Option<String>)>,
    focused: bool,

    rect: Cell<Option<Rect>>,
}

impl ThreadSelector {
    pub(crate) fn new() -> ThreadSelector {
        ThreadSelector {
            current_thread: None,
            threads: Vec::new(),
            focused: true,

            rect: Cell::new(None),
        }
    }

    pub(crate) fn current_thread(&self) -> Option<ThreadId> {
        self.current_thread
    }

    pub(crate) fn update(&mut self, store: &Store) -> bool {
        let mut threads = store
            .threads
            .iter()
            .map(|(key, store)| (*key, store.name.clone()))
            .collect::<Vec<(ThreadId, Option<String>)>>();
        threads.sort_by_key(|(id, _)| *id);
        let rerender = self.threads == threads;
        self.threads = threads;
        if self.current_thread.is_none() {
            if self.threads.len() == 1 {
                // There is at least one element
                let thread_id = self.threads.iter().map(|(id, _)| id).next().unwrap();
                self.current_thread = Some(*thread_id);
            } else {
                if let Some(thread_id) = self.threads.iter().map(|(id, _)| id).min() {
                    self.current_thread = Some(*thread_id);
                }
            }
        }
        rerender
    }

    pub(crate) fn on_up(&mut self) -> bool {
        if let Some(current_id) = self.current_thread {
            let current_index = self
                .threads
                .iter()
                .position(|(id, _)| current_id == *id)
                .expect("BUG: Current thread id not in list");
            if let Some((id, _)) = self.threads.get(current_index.saturating_sub(1)) {
                let id = *id;
                return self.select(id);
            }
        }
        false
    }

    pub(crate) fn on_down(&mut self) -> bool {
        if let Some(current_id) = self.current_thread {
            let current_index = self
                .threads
                .iter()
                .position(|(id, _)| current_id == *id)
                .expect("BUG: Current thread id not in list");
            if let Some((id, _)) = self.threads.get(current_index.saturating_add(1)) {
                let id = *id;
                return self.select(id);
            }
        }
        false
    }

    pub(crate) fn render_to(&self, f: &mut Frame<CrosstermBackend>, r: Rect) {
        let (border_color, title_color) = self.border_color();
        self.rect.set(Some(r));
        let index =
            self.current_thread().and_then(|current_id| {
                self.threads.iter().enumerate().find_map(|(i, (id, _))| {
                    if current_id == *id {
                        Some(i)
                    } else {
                        None
                    }
                })
            });
        SelectableList::default()
            .highlight_style(Style::default().modifier(Modifier::BOLD))
            .items(
                &self
                    .threads
                    .iter()
                    .map(|(key, name)| {
                        format!(
                            "{}{} - {}",
                            // FIXME: Remove this when we render based on a paragraph
                            //
                            // If something is highlighted, tui somehow inserts a leading space
                            // We just insert one ourself, to make up when the widget is inactvive
                            if self.focused { "" } else { " " },
                            key.0,
                            if let Some(name) = name { name } else { "" }
                        )
                    })
                    .collect::<Vec<String>>(),
            )
            .select(index)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Threads")
                    .border_style(Style::default().fg(border_color))
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, r);
    }

    fn select(&mut self, id: ThreadId) -> bool {
        let new_id = Some(id);
        let rerender = self.current_thread != new_id;
        self.current_thread = new_id;
        rerender
    }
}

impl Input for ThreadSelector {
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

        if let Some((id, _)) = self.threads.get((y - rect.y - 1) as usize) {
            let id = *id;
            self.select(id)
        } else {
            false
        }
    }
}
