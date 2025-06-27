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
use std::{env, error::Error};

use crate::file_handler::{handle_paste, handle_remove};
use crate::models::{Operation, PasteContent, RecordType};
use crate::record_handler::{read_clipboard, read_history};
use crate::utils::get_metadata;

const PATH_WIDTH: usize = 50;
const TIMESTAMP_WIDTH: usize = 40;

pub fn enter_tui_mode(mode: RecordType) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let entries = match mode {
        RecordType::Clipboard => read_clipboard()?.unwrap_or(vec![]),
        RecordType::History => read_history()?.unwrap_or(vec![]),
    };
    if entries.is_empty() {
        println!("[Info]: {} is empty", mode);
        return Ok(());
    }
    let mut highlighted = 0;
    let mut invalid = vec![false; entries.len()];
    let mut selected = vec![false; entries.len()];

    let run_app = || -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| {
                let area = f.area();

                let items: Vec<ListItem> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let valid = get_metadata(&entry.path).is_ok();
                        invalid[i] = !valid;

                        let style = if !valid {
                            Style::default().bg(Color::Gray)
                        } else if i == highlighted {
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

                        let operation_display = match entry.operation {
                            Operation::Copy => "<Copied> ",
                            Operation::Cut => "<Cut> ",
                        };
                        let operation_span = Span::styled(operation_display, style);

                        let line = Line::from(vec![
                            selected_span,
                            path_span,
                            timestamp_span,
                            operation_span,
                        ]);
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
                        KeyCode::Char('x') => {
                            if mode == RecordType::Clipboard && !invalid[highlighted] {
                                handle_remove(entries[highlighted].id)?;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if !invalid[highlighted] {
                                selected[highlighted] = !selected[highlighted];
                            }
                        }
                        KeyCode::Char('j') => {
                            highlighted = (highlighted + 1).min(entries.len() - 1)
                        }
                        KeyCode::Char('p') => {
                            let destination_path = env::current_dir()?;
                            let selected_entries = entries
                                .into_iter()
                                .zip(selected)
                                .filter_map(
                                    |(entry, selected)| {
                                        if selected {
                                            Some(entry)
                                        } else {
                                            None
                                        }
                                    },
                                )
                                .collect();
                            let paste_content = PasteContent {
                                entries: selected_entries,
                                source: mode,
                            };
                            handle_paste(destination_path, Some(paste_content))?;
                            break;
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
