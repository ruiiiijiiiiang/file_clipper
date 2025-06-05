use chrono::{DateTime, Local};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fs_extra::{copy_items, dir::CopyOptions, move_items};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::{
    collections::VecDeque,
    env,
    error::Error,
    fs::metadata,
    io::ErrorKind,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};
use uuid::Uuid;

mod models;
use file_handler::{read_clipboard, read_history, write_clipboard, write_history};

mod file_handler;
use models::{EntryType, Operation, RecordEntry, TuiMode};

fn get_absolute_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_relative() {
        let cwd = env::current_dir()?;
        Ok(cwd.join(path).canonicalize()?)
    } else {
        Ok(path.canonicalize()?)
    }
}

fn move_helper(paths: &[String], operation: Operation) -> Result<(), Box<dyn Error>> {
    let mut clipboard_entries = VecDeque::from(read_clipboard()?.unwrap_or(vec![]));
    for path_str in paths {
        let path = Path::new(path_str);
        let absolute_path = get_absolute_path(path)?;
        println!("{}", absolute_path.display());
        let metadata = match metadata(&absolute_path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                eprintln!(
                    "[Error]: {} does not exist; skipping",
                    absolute_path.display()
                );
                continue;
            }
            Err(e) => {
                eprintln!(
                    "[Error]: failed to get metadata for {}: {}",
                    absolute_path.display(),
                    e
                );
                continue;
            }
            Ok(metadata) => metadata,
        };
        let entry_type = if metadata.is_dir() {
            EntryType::Directory
        } else if metadata.is_symlink() {
            EntryType::Symlink
        } else if metadata.is_file() {
            EntryType::File
        } else {
            eprintln!(
                "[Error]: unsupported file type: {}",
                absolute_path.display()
            );
            continue;
        };
        clipboard_entries.push_front(RecordEntry {
            operation: operation.clone(),
            entry_type,
            path: absolute_path.to_string_lossy().into_owned(),
            timestamp: SystemTime::now(),
            id: Uuid::new_v4(),
        });
    }
    let clipboard_entries: Vec<RecordEntry> = clipboard_entries.into();
    write_clipboard(&clipboard_entries)?;
    for path in paths {
        println!("[Info]: {:?} {}", operation, path);
    }
    Ok(())
}

fn handle_copy(paths: &[String]) -> Result<(), Box<dyn Error>> {
    move_helper(paths, Operation::Copy)
}

fn handle_cut(paths: &[String]) -> Result<(), Box<dyn Error>> {
    move_helper(paths, Operation::Cut)
}

fn handle_paste(path: &String) -> Result<(), Box<dyn Error>> {
    let clipboard_entries = match read_clipboard() {
        Err(e) => {
            eprintln!("[Error]: failed to read clipboard: {}", e);
            return Ok(());
        }
        Ok(Some(clipboard_entries)) if !clipboard_entries.is_empty() => clipboard_entries,
        _ => {
            println!("[Info]: clipboard is empty");
            return Ok(());
        }
    };

    let mut history_entries = VecDeque::from(match read_history() {
        Err(e) => {
            eprintln!("[Error]: failed to read history: {}", e);
            return Ok(());
        }
        Ok(None) => Vec::new(),
        Ok(Some(history_entries)) => history_entries,
    });

    let options = CopyOptions::new();
    for mut entry in clipboard_entries {
        let metadata = match metadata(&entry.path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                eprintln!("[Error]: {} no longer exists; skipping", entry.path);
                continue;
            }
            Err(e) => {
                eprintln!(
                    "[Error]: failed to get metadata for {}: {}; skipping",
                    entry.path, e
                );
                continue;
            }
            Ok(metadata) => metadata,
        };
        if !entry.entry_type.matches_metadata(&metadata) {
            eprintln!("Warning: {} does not match recorded entry type", entry.path);
        }
        if metadata.modified()? > entry.timestamp {
            eprintln!("Warning: {} has been modified since copying", entry.path);
        }
        match entry.operation {
            Operation::Copy => copy_items(&[&entry.path], path, &options)?,
            Operation::Cut => {
                move_items(&[&entry.path], path, &options)?;
                let file_name = Path::new(&entry.path).file_name().unwrap();
                let new_path = PathBuf::from(path);
                let mut absolute_path = get_absolute_path(&new_path)?;
                absolute_path.push(file_name);
                entry.path = absolute_path.to_string_lossy().into_owned();
                0 // Return 0 to make branches have the same type
            }
        };
        println!("Pasted: {}", entry.path);
        entry.timestamp = SystemTime::now();
        history_entries.push_front(entry.clone());
    }
    write_clipboard(&[])?;
    let history_entries: Vec<RecordEntry> = history_entries.into();
    write_history(&history_entries)?;
    Ok(())
}

fn enter_tui_mode(mode: TuiMode) -> Result<(), Box<dyn Error>> {
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
    let mut selected = 0;

    let mut run_app = || -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| {
                let area = f.area();

                let items: Vec<ListItem> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let style = if i == selected {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };
                        let datetime: DateTime<Local> = entry.timestamp.into();
                        let line = Line::styled(
                            format!("{}    {}", entry.path, datetime.to_rfc2822()),
                            style,
                        );
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
                        KeyCode::Char('j') => selected = (selected + 1).min(entries.len() - 1),
                        KeyCode::Char('k') => selected = selected.saturating_sub(1),
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [arguments]", args[0]);
        eprintln!("Commands: copy <path>, cut <path>, paste, list, history");
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "copy" | "cp" | "y" => {
            if args.len() < 3 {
                eprintln!("[Error]: copy command requires at least one path");
                process::exit(1);
            }
            handle_copy(&args[2..])?
        }
        "cut" | "mv" | "x" => {
            if args.len() < 3 {
                eprintln!("[Error]: cut command requires at least one path");
                process::exit(1);
            }
            handle_cut(&args[2..])?
        }
        "paste" | "p" => {
            let path = if args.len() < 3 {
                env::current_dir()?.to_string_lossy().into_owned()
            } else {
                args[2].clone()
            };
            handle_paste(&path)?
        }
        "list" | "l" => enter_tui_mode(TuiMode::Clipboard)?,
        "history" | "h" => enter_tui_mode(TuiMode::History)?,
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: {} <command> [arguments]", args[0]);
            eprintln!("Commands: copy <path>, cut <path>, paste, list");
            process::exit(1);
        }
    }

    Ok(())
}
