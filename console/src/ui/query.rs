use crate::filter::*;
use crate::ui::{Action, Input};

use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Text, Widget};
use tui::Frame;

use std::borrow::Cow;
use std::cell::Cell;

pub struct QueryView {
    buffer: String,
    history: Vec<String>,
    /// 0: User didn't request a previous value
    /// 1..: Users wants the `self.history.len() - self.history_index` value
    /// Since most recent values are pushed last, this will retrieve the last value
    history_index: usize,

    filter: Option<Filter>,

    focused: bool,
    rect: Cell<Option<Rect>>,
}

impl QueryView {
    pub(crate) fn new() -> QueryView {
        QueryView {
            buffer: String::new(),
            history: Vec::new(),
            history_index: 0,

            filter: None,

            focused: true,
            rect: Cell::default(),
        }
    }

    pub(crate) fn update(&mut self, filter: Filter) -> bool {
        self.filter = Some(filter);
        false
    }

    pub(crate) fn on_up(&mut self) -> bool {
        if self.history.len() == 0 {
            // We don't have a history, ignore
            return false;
        }
        let old = self.history_index;
        self.history_index += 1;
        if self.history_index > self.history.len() {
            // Cap history_index
            // Remember, history_index is 1 based and points to the back of the vec
            self.history_index = self.history.len();
        }
        // Retrieve from history
        self.buffer.clear();
        let history_value = &self.history[self.history.len() - self.history_index];
        self.buffer.clone_from(history_value);

        let rerender = old != self.history_index;
        rerender
    }

    pub(crate) fn on_down(&mut self) -> bool {
        let old = self.history_index;
        self.history_index = self.history_index.saturating_sub(1);
        let rerender = old != self.history_index;
        if rerender {
            // Index changed, buffer will needs to be cleared anyways
            self.buffer.clear();
            // Do we need to retrieve from history or does clearing suffice
            if self.history_index != 0 {
                // Retrieve from history
                let history_value = &self.history[self.history.len() - self.history_index];
                self.buffer.clone_from(history_value);
            }
        }
        rerender
    }

    pub(crate) fn render_to(&self, f: &mut Frame<CrosstermBackend>, r: Rect) {
        let (border_color, title_color) = self.border_color();
        const HELP: [Text<'static>; 7] = [
            Text::Raw(Cow::Borrowed("Commands\n")),
            Text::Raw(Cow::Borrowed("> event.field.<name> <operator>\n")),
            Text::Raw(Cow::Borrowed("Operators\n")),
            Text::Raw(Cow::Borrowed("- == \"<string>\"\n")),
            Text::Raw(Cow::Borrowed("- contains \"<string>\"\n")),
            Text::Raw(Cow::Borrowed("- startsWith \"<string>\"\n")),
            Text::Raw(Cow::Borrowed("- matches \"<regex>\"\n")),
        ];
        let chunks = Layout::default()
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(HELP.len() as u16 + 2),
                ]
                .as_ref(),
            )
            .direction(Direction::Vertical)
            .split(r);
        let text = [
            Text::raw("> "),
            Text::styled(&self.buffer, Style::default().fg(Color::White)),
        ];
        Paragraph::new(text.into_iter())
            .block(
                Block::default()
                    .title("Query")
                    .borders(Borders::ALL & !Borders::BOTTOM)
                    .border_style(Style::default().fg(border_color))
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, chunks[0]);
        self.rect.set(Some(chunks[0]));

        let items: Vec<Text<'_>> = if let Some(filter) = self.filter.as_ref() {
            filter
                .modifier
                .values()
                .map(|m| Text::raw(format!("{}\n", m)))
                .collect::<Vec<Text<'_>>>()
        } else {
            vec![]
        };
        Paragraph::new(items.iter())
            .block(
                Block::default()
                    .title("Current Filter")
                    .borders(Borders::ALL & !Borders::BOTTOM)
                    .border_style(Style::default().fg(border_color))
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, chunks[1]);

        Paragraph::new(HELP.iter())
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, chunks[2]);
    }

    fn handle_command(&mut self) -> Action {
        let action = if let Ok(command) = self.buffer.parse() {
            Action::Command(command)
        } else {
            // Just issue redraw for cleared buffer
            Action::Redraw
        };
        self.history.push(self.buffer.clone());
        self.buffer.clear();
        action
    }

    fn last_char_is_whitespace(&self) -> bool {
        self.buffer
            .chars()
            .last()
            .map(char::is_whitespace)
            .unwrap_or(true)
    }
}

impl Input for QueryView {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn focused(&self) -> bool {
        self.focused
    }
    fn show_cursor(&self) -> Option<(u16, u16)> {
        if self.focused() {
            if let Some(rect) = self.rect.get() {
                let rect = rect.inner(1);
                let x = rect.x
                    + 2 // "> ".len()
                    + self.buffer.len() as u16;
                return Some((x, rect.y));
            }
        }
        None
    }
    fn on_char(&mut self, c: char) -> Action {
        // We don't wrap the text
        // Excess text can still be typed, just requires precision by the user
        // TODO: Decide to implement moving cursor or adjust width of frame
        match c {
            '\n' => self.handle_command(),
            _ if self.last_char_is_whitespace() && c.is_whitespace() => {
                // We are at the start of a command
                // Or the last char is already a whitespace
                // Don't insert redudant whitespace
                Action::Nothing
            }
            _ => {
                self.buffer.push(c);
                Action::Redraw
            }
        }
    }
    fn on_backspace(&mut self) -> bool {
        let rerender = self.buffer.len() != 0;
        let new_len = self.buffer.len().wrapping_sub(1);
        self.buffer.truncate(new_len);
        rerender
    }
}
