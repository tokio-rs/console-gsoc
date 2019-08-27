use crate::storage::*;

use crate::filter::*;
use crate::ui::{Hitbox, Input};

use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, Text, Widget};
use tui::Frame;

use std::cell::Cell;
use std::fmt::Write;

#[derive(Debug, Default)]
struct DelimittedString {
    buffer: String,
    delimiter: &'static str,

    first: bool,
}

impl DelimittedString {
    fn new(delimiter: &'static str) -> DelimittedString {
        DelimittedString {
            buffer: String::new(),
            delimiter,

            first: true,
        }
    }

    fn delimiter(&mut self) {
        if self.first {
            self.first = false;
        } else {
            self.buffer.push_str(self.delimiter);
        }
    }

    fn write_value(&mut self, value: &Option<value::Value>) {
        match value {
            Some(value::Value::Signed(i)) => write!(self.buffer, "{}", i).unwrap(),
            Some(value::Value::Unsigned(u)) => write!(self.buffer, "{}", u).unwrap(),
            Some(value::Value::Boolean(b)) => write!(self.buffer, "{}", b).unwrap(),
            Some(value::Value::Str(s)) => write!(self.buffer, "{}", s).unwrap(),
            Some(value::Value::Debug(d)) => write!(self.buffer, "{}", d.debug).unwrap(),
            None => {}
        }
    }

    fn newline(mut self) -> String {
        self.buffer.push('\n');
        self.buffer
    }
}

struct Formatter<'s> {
    store: &'s Store,
    entries: &'s Entries,

    indentation_level: usize,

    counter: usize,

    length: u16,
    offset: usize,
    selection: usize,
}

impl<'s> Formatter<'s> {
    fn entries(store: &'s Store, entries: &'s Entries) -> Formatter<'s> {
        Formatter {
            store,
            entries,

            indentation_level: 0,

            counter: 0,

            length: std::u16::MAX,
            selection: 0,
            offset: 0,
        }
    }

    fn offset(&mut self, offset: usize) {
        self.offset = offset
    }

    fn length(&mut self, length: u16) {
        self.length = length
    }

    fn selection(&mut self, selection: usize) {
        self.selection = selection;
    }

    fn style(mut self) -> Vec<Text<'static>> {
        let mut buffer = Vec::with_capacity(2 * self.length as usize);
        self.style_entries(&mut buffer, self.entries);
        buffer
    }

    fn style_entries(&mut self, buffer: &mut Vec<Text<'_>>, entries: &Entries) {
        match entries {
            Entries::Entries(entries) => {
                for &event in entries {
                    // Skip/break if event is outside viewport
                    if self.counter < self.offset {
                        continue;
                    }
                    if self.counter >= self.offset + self.length as usize {
                        break;
                    }
                    self.style_event(buffer, event);
                    self.counter += 1;
                }
            }
            Entries::Grouped { group_by, groups } => {
                self.indentation_level += 1;
                for (value, entries) in groups {
                    // Skip/break if event is outside viewport
                    if self.counter < self.offset {
                        continue;
                    }
                    if self.counter >= self.offset + self.length as usize {
                        break;
                    }
                    if let Some(value) = value {
                        buffer.push(Text::raw(format!("{} == {}\n", group_by, value)));
                    } else {
                        buffer.push(Text::raw(format!("{} == None\n", group_by)));
                    }
                    self.counter += 1;
                    self.style_entries(buffer, entries)
                }
                self.indentation_level -= 1;
            }
        }
    }

    fn style_level(level: Option<Level>, indentation: usize) -> Text<'static> {
        let mut base = " ".repeat(indentation);
        let style = match level {
            None => {
                base.push_str(" NONE ");
                Style::default().fg(Color::White)
            }
            Some(Level::Info) => {
                base.push_str(" INFO ");
                Style::default().fg(Color::White)
            }
            Some(Level::Debug) => {
                base.push_str("DEBUG ");
                Style::default().fg(Color::LightCyan)
            }
            Some(Level::Error) => {
                base.push_str("ERROR ");
                Style::default().fg(Color::Red)
            }
            Some(Level::Trace) => {
                base.push_str("TRACE ");
                Style::default().fg(Color::Green)
            }
            Some(Level::Warn) => {
                base.push_str(" WARN ");
                Style::default().fg(Color::Yellow)
            }
        };
        Text::styled(base, style)
    }

    fn style_event(&self, buffer: &mut Vec<Text<'_>>, i: usize) {
        let entry = &self.store.events()[i];
        let level = Formatter::style_level(entry.level(), self.indentation_level);
        buffer.push(level);

        let mut text = DelimittedString::new(", ");

        for value in &entry.event.values {
            text.delimiter();
            if let Some(field) = &value.field {
                write!(text.buffer, r#"{}(""#, field.name).unwrap();
                text.write_value(&value.value);
                text.buffer.push_str(r#"")"#);
            }
        }

        let text = text.newline();
        let is_selected = self.counter == self.selection - self.offset;
        let text = if is_selected {
            Text::styled(text, Style::default().modifier(Modifier::BOLD))
        } else {
            Text::raw(text)
        };
        buffer.push(text);
    }
}

pub struct EventList {
    /// Cached rows, gets populated by `EventList::update`
    logs: Entries,
    /// Cached text items, gets populated by `EventList::update`
    text: Vec<Text<'static>>,
    /// Index into logs vec, indicates which row the user selected
    selection: usize,
    /// How far the frame is offset by scrolling
    offset: usize,

    focused: bool,
    rect: Cell<Option<Rect>>,
}

impl EventList {
    pub(crate) fn new() -> EventList {
        EventList {
            focused: false,
            logs: Entries::Entries(Vec::new()),
            text: Vec::new(),

            selection: 0,
            offset: 0,
            rect: Cell::new(None),
        }
    }

    pub(crate) fn update(&mut self, store: &Store, filter: &Filter) -> bool {
        let entries = (0..store.events().len()).collect();
        let logs = filter.apply(store, Entries::Entries(entries));
        let rerender = self.logs != logs;
        // TODO: Smarter checks, this rerenders even if change is outside viewport
        if rerender {
            let mut formatter = Formatter::entries(store, &self.logs);
            formatter.selection(self.selection);
            formatter.length(
                self.rect
                    .get()
                    .as_ref()
                    .map(|rect| rect.height - 2)
                    .unwrap_or(0),
            );
            formatter.offset(self.offset);
            self.text = formatter.style();
        }
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

    pub(crate) fn render_to(&self, f: &mut Frame<CrosstermBackend>, mut r: Rect) {
        // TODO: Investigate this offset
        // Necessary because something overflows the border line, moving the corner onto the next
        r.width -= 1;
        self.rect.set(Some(r));
        // - 2: Upper and lower border of window
        let rowcount = r.height as usize - 2;

        let (border_color, title_color) = self.border_color();
        let block_title = format!(
            "Events {}-{}/{}",
            1 + self.offset,
            self.offset + std::cmp::min(rowcount, self.logs.len()),
            self.logs.len(),
        );
        Paragraph::new(self.text.iter())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(&block_title)
                    .title_style(Style::default().fg(title_color)),
            )
            .render(f, r);
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
