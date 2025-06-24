use chrono::{DateTime, Local};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::error::Error;

use crate::models::TuiMode;

use crate::record_handler::{read_clipboard, read_history};

const PATH_WIDTH: usize = 50;
const TIMESTAMP_WIDTH: usize = 40;

pub fn enter_tui_mode(mode: TuiMode) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let entries = match mode {
        TuiMode::Clipboard => read_clipboard()?.unwrap_or(vec![]),
        TuiMode::History => read_history()?.unwrap_or(vec![]),
    };
    if entries.is_empty() {
        println!(
            "[Info]: {} is empty",
            match mode {
                TuiMode::Clipboard => "clipboard",
                TuiMode::History => "history",
            }
        );
        return Ok(());
    }
    let mut highlighted = 0;
    let mut selected = vec![false; entries.len()];

    let mut run_app = || -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| {
                let area = f.area();

                let items: Vec<ListItem> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let style = if i == highlighted {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };

                        let selected_display = if selected[i] { "[X] " } else { "[ ] " };
                        let selected_span = Span::styled(selected_display, style);

                        let path_string = entry.path.display().to_string();
                        let path_display = if path_string.len() > PATH_WIDTH {
                            format!("{}...", &path_string[0..PATH_WIDTH - 3])
                        } else {
                            format!("{: <width$}", path_string, width = PATH_WIDTH)
                        };
                        let path_span =
                            Span::styled(path_display, style.add_modifier(Modifier::BOLD));

                        let local_datetime: DateTime<Local> = entry.timestamp.into();
                        let datetime_string = local_datetime.to_rfc2822();
                        let timestamp_display = if datetime_string.len() > TIMESTAMP_WIDTH {
                            format!("{}...", &datetime_string[0..TIMESTAMP_WIDTH - 3])
                        } else {
                            format!("{: <width$}", datetime_string, width = TIMESTAMP_WIDTH)
                        };
                        let timestamp_span = Span::styled(timestamp_display, style);

                        let line = Line::from(vec![selected_span, path_span, timestamp_span]);
                        ListItem::new(line)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL))
                    .highlight_style(Style::default().bg(Color::LightGreen));

                f.render_widget(list, area);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('x') => selected[highlighted] = !selected[highlighted],
                        KeyCode::Char('j') => {
                            highlighted = (highlighted + 1).min(entries.len() - 1)
                        }
                        KeyCode::Char('k') => highlighted = highlighted.saturating_sub(1),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    };

    let res = run_app();

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}
