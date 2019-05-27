use tui::backend::CrosstermBackend;
use tui::layout::{Layout, Rect};
use tui::style::{Modifier, Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Text, Widget};
use tui::{Frame, Terminal};

use std::io;

pub fn draw(terminal: &mut Terminal<CrosstermBackend>) -> Result<(), io::Error> {
    terminal.draw(|mut f| {
        let area = f.size();
        let text = [
            Text::raw("Hello, World! This is a first paragraph!\n"),
            Text::styled("1234\n", Style::default().fg(Color::Cyan).modifier(Modifier::UNDERLINED)),
        ];
        Paragraph::new(text.iter())
            .block(
                Block::default()
                    .title("Console")
                    .title_style(Style::default().modifier(Modifier::BOLD)),
            )
            .wrap(true)
            .render(&mut f, area);
    })
}
