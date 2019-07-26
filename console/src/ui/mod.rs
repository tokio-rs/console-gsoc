pub(crate) mod app;
pub(crate) mod command;
pub(crate) mod events;
pub(crate) mod query;

pub use self::app::*;
pub(crate) use self::command::*;
pub(crate) use self::events::*;
pub(crate) use self::query::*;

use tui::layout::Rect;
use tui::style::Color;

pub(crate) enum Action {
    Command(Command),
    Redraw,
    Nothing,
}

impl Action {
    fn redraw(&self) -> bool {
        match self {
            Action::Nothing => false,
            _ => true,
        }
    }
}

pub(crate) trait Input {
    fn set_focused(&mut self, focused: bool);
    fn focused(&self) -> bool;

    fn show_cursor(&self) -> Option<(u16, u16)> {
        None
    }

    fn on_click(&mut self, _x: u16, _y: u16) -> bool {
        false
    }
    fn on_char(&mut self, _c: char) -> Action {
        Action::Nothing
    }
    fn on_backspace(&mut self) -> bool {
        false
    }

    fn border_color(&self) -> (Color, Color) {
        if self.focused() {
            (Color::Rgb(50, 205, 50), Color::Rgb(0, 255, 0))
        } else {
            (Color::Reset, Color::Reset)
        }
    }
}

pub(crate) trait Hitbox {
    fn hit(&self, x: u16, y: u16) -> bool;
}

impl Hitbox for Rect {
    fn hit(&self, x: u16, y: u16) -> bool {
        (self.x..(self.x + self.width)).contains(&x)
            && (self.y..(self.y + self.height)).contains(&y)
    }
}
