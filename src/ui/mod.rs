use tokio_trace::Level;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::{Frame, Terminal};

use std::io;

pub fn draw(
    terminal: &mut Terminal<CrosstermBackend>,
    store: &storage::Store,
) -> Result<(), io::Error> {
    terminal.draw(|mut f| {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .direction(Direction::Vertical)
            .split(f.size());
        let text = [
            Text::raw("Hello, World! This is an extreeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeemly long first paragraph!\n"),
            Text::styled("1234\n", Style::default().modifier(Modifier::UNDERLINED)),
        ];
        Paragraph::new(text.iter())
            .block(
                Block::default()
                    .title("Console")
                    .title_style(Style::default().modifier(Modifier::BOLD)),
            )
            .wrap(false)
            .render(&mut f, chunks[0]);
        let first_thread_log_lines = store.lines.iter().next().expect("No logs for any thread");
        let logs = first_thread_log_lines.1.iter().map(|(level, text)| {
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
            .block(Block::default().borders(Borders::ALL).title("List"))
            .render(&mut f, chunks[1]);
    })
}
