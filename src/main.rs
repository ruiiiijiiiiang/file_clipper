use std::{
    env,
    error::Error,
    fs::metadata,
    path::Path,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

// Import the crates you'll be using
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fs_extra::dir::copy as copy_dir;
use fs_extra::file::copy as copy_file;
use ratatui::prelude::*;
use shellexpand::tilde;
use uuid::Uuid;

mod models;
use file_handler::{read_clipboard, read_history, write_clipboard, write_history};

mod file_handler;
use models::{ClipboardEntry, EntryType, HistoryEntry, Operation};

fn get_current_timestamp() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .expect("System time is before the Unix epoch!")
        .as_secs()
}

fn handle_copy(args: &[String]) -> Result<(), Box<dyn Error>> {
    let paths = &args[2..];
    let mut clipboard_entries = read_clipboard()?.unwrap_or(vec![]);
    for path_str in paths {
        let path = Path::new(path_str);
        let absolute_path = if path.is_relative() {
            match env::current_dir() {
                Ok(cwd) => cwd.join(path).canonicalize()?,
                Err(e) => return Err(e.into()),
            }
        } else {
            path.canonicalize()?
        };
        match metadata(&absolute_path) {
            Ok(metadata) => {
                let entry_type = if metadata.is_dir() {
                    EntryType::Directory
                // } else if metadata.is_file() {
                //     EntryType::File
                } else {
                    EntryType::File //TODO: Handle symlinks
                };
                clipboard_entries.push(ClipboardEntry {
                    operation: Operation::Copy,
                    entry_type,
                    path: absolute_path.to_string_lossy().into_owned(),
                    timestamp: get_current_timestamp(),
                });
            }
            Err(e) => return Err(e.into()),
        }
    }
    write_clipboard(&clipboard_entries)?;
    Ok(())
}

fn handle_cut(args: &[String]) -> Result<(), Box<dyn Error>> {
    // Implement cut logic here using fs_extra, write path and "CUT" to clipboard
    println!("Handling cut: {:?}", args);
    Ok(())
}

fn handle_paste(args: &[String]) -> Result<(), Box<dyn Error>> {
    let clipboard = read_clipboard()?;
    Ok(())
}

fn enter_tui_mode() -> Result<(), Box<dyn Error>> {
    // Implement Ratatui TUI here
    println!("Entering TUI mode");

    // Basic Ratatui setup (you'll expand this significantly)
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_app = || -> Result<(), Box<dyn Error>> {
        loop {
            // TODO: add TUI
            // terminal.draw(|f| {
            //     let size = f.size();
            //     let block = Block::default()
            //         .title("Kuick-Klip History")
            //         .borders(Borders::ALL);
            //     f.render_widget(block, size);
            // })?;
            //
            // if event::poll(std::time::Duration::from_millis(100))? {
            //     if let Event::Key(key) = event::read()? {
            //         if key.code == KeyCode::Char('q') || key.code == KeyCode::Ctrl('c') {
            //             break;
            //         }
            //     }
            // }
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
        eprintln!("Commands: copy <path>, cut <path>, paste, list");
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "copy" => handle_copy(&args)?,
        "cut" => handle_cut(&args)?,
        "paste" => handle_paste(&args)?,
        "list" => enter_tui_mode()?,
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: {} <command> [arguments]", args[0]);
            eprintln!("Commands: copy <path>, cut <path>, paste, list");
            process::exit(1);
        }
    }

    Ok(())
}
