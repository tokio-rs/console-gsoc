pub(crate) mod app;
pub(crate) mod events;

pub use self::app::*;
pub use self::events::*;

use tui::layout::Rect;
use tui::style::Color;

pub(crate) trait Input {
    fn set_focused(&mut self, focused: bool);
    fn focused(&self) -> bool;

    fn on_click(&mut self, _x: u16, _y: u16) -> bool {
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
