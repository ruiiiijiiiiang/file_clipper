use fs_extra::{copy_items, dir::CopyOptions, move_items};
use std::{
    collections::VecDeque,
    env,
    error::Error,
    fs::metadata,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::SystemTime,
};
use uuid::Uuid;

use crate::models::{EntryType, Operation, RecordEntry};

use crate::record_handler::{read_clipboard, read_history, write_clipboard, write_history};

pub fn handle_transfer(paths: Vec<PathBuf>, operation: Operation) -> Result<(), Box<dyn Error>> {
    let mut clipboard_entries = VecDeque::from(read_clipboard()?.unwrap_or(vec![]));
    for path in &paths {
        let absolute_path = get_absolute_path(path)?;
        println!("{}", absolute_path.display());
        let metadata = match metadata(&absolute_path) {
            Err(error) if error.kind() == ErrorKind::NotFound => {
                eprintln!(
                    "[Error]: {} does not exist; skipping",
                    absolute_path.display()
                );
                continue;
            }
            Err(error) => {
                eprintln!(
                    "[Error]: failed to get metadata for {}: {}",
                    absolute_path.display(),
                    error
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
        let size = if entry_type == EntryType::Directory {
            None
        } else {
            Some(metadata.len())
        };
        clipboard_entries.push_front(RecordEntry {
            operation: operation.clone(),
            size,
            entry_type,
            path: absolute_path,
            timestamp: SystemTime::now(),
            id: Uuid::new_v4(),
        });
    }
    let clipboard_entries: Vec<RecordEntry> = clipboard_entries.into();
    write_clipboard(&clipboard_entries)?;
    for path in paths {
        println!("[Info]: {:?} {}", operation, path.display());
    }
    Ok(())
}

pub fn handle_paste(
    destination_path: PathBuf,
    entries: Option<Vec<RecordEntry>>,
) -> Result<(), Box<dyn Error>> {
    let clipboard_entries = match entries {
        None => read_clipboard()?.unwrap_or(vec![]),
        Some(entries) => entries,
    };
    let mut history_entries = VecDeque::from(read_history()?.unwrap_or(vec![]));

    let options = CopyOptions::new();
    for mut entry in clipboard_entries {
        let metadata = match metadata(&entry.path) {
            Err(error) if error.kind() == ErrorKind::NotFound => {
                eprintln!(
                    "[Error]: {} no longer exists; skipping",
                    entry.path.display()
                );
                continue;
            }
            Err(error) => {
                eprintln!(
                    "[Error]: failed to get metadata for {}: {}; skipping",
                    entry.path.display(),
                    error
                );
                continue;
            }
            Ok(metadata) => metadata,
        };
        if !entry.entry_type.matches_metadata(&metadata) {
            eprintln!(
                "Warning: {} does not match recorded entry type",
                entry.path.display()
            );
        }
        if metadata.modified()? > entry.timestamp {
            eprintln!(
                "Warning: {} has been modified since copying",
                entry.path.display()
            );
        }
        match entry.operation {
            Operation::Copy => copy_items(&[&entry.path], &destination_path, &options)?,
            Operation::Cut => {
                move_items(&[&entry.path], &destination_path, &options)?;
                let file_name = Path::new(&entry.path).file_name().unwrap();
                let new_path = PathBuf::from(&destination_path);
                let mut absolute_path = get_absolute_path(&new_path)?;
                absolute_path.push(file_name);
                entry.path = absolute_path;
                0 // Return 0 to make branches have the same type
            }
        };
        println!("Pasted: {}", entry.path.display());
        entry.timestamp = SystemTime::now();
        history_entries.push_front(entry.clone());
    }
    write_clipboard(&[])?; // TODO: handle left over
    let history_entries: Vec<RecordEntry> = history_entries.into();
    write_history(&history_entries)?;
    Ok(())
}

pub fn handle_remove(id: Uuid) -> Result<(), Box<dyn Error>> {
    let clipboard_entries = match read_clipboard() {
        Ok(Some(entries)) => entries,
        _ => {
            println!("[Error]: failed to read clipboard");
            return Ok(());
        }
    };
    let clipboard_length = clipboard_entries.len();
    let filtered_entries: Vec<RecordEntry> = clipboard_entries
        .into_iter()
        .filter(|entry| entry.id != id)
        .collect();
    if filtered_entries.len() == clipboard_length {
        println!("[Error]: no entry found");
    } else {
        write_clipboard(&filtered_entries)?
    }
    Ok(())
}

fn get_absolute_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_relative() {
        let cwd = env::current_dir()?;
        Ok(cwd.join(path).canonicalize()?)
    } else {
        Ok(path.canonicalize()?)
    }
}
