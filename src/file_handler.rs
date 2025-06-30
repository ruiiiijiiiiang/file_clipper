use fs_extra::{copy_items, dir::CopyOptions, move_items};
use glob::glob;
use std::{
    collections::{HashSet, VecDeque},
    error::Error,
    path::PathBuf,
    time::SystemTime,
};
use uuid::Uuid;

use crate::models::{Metadata, Operation, PasteContent, RecordEntry, RecordType};
use crate::record_handler::{read_clipboard, read_history, write_clipboard, write_history};
use crate::utils::{get_absolute_path, get_metadata};

pub fn handle_transfer(paths: Vec<PathBuf>, operation: Operation) -> Result<(), Box<dyn Error>> {
    let mut clipboard_entries = VecDeque::from(read_clipboard()?.unwrap_or(vec![]));
    let expand_paths = expand_paths(paths)?;
    for path in &expand_paths {
        let Metadata {
            size,
            entry_type,
            absolute_path,
            modified: _,
        } = get_metadata(path)?;

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
    for path in expand_paths {
        println!("[Info]: {:?} {}", operation, path.display());
    }
    Ok(())
}

pub fn handle_paste(
    destination_path: PathBuf,
    paste_content: Option<PasteContent>,
) -> Result<(), Box<dyn Error>> {
    let destination_path = get_absolute_path(&destination_path)?;
    let clipboard_entries = read_clipboard()?.unwrap_or(Vec::new());
    let (entries_to_paste, entries_remaining) = match paste_content {
        // If no entries are provided, use the clipboard
        None => {
            let (valid_entries, invalid_entries) = filter_invalid_entries(&clipboard_entries);
            (valid_entries, Some(invalid_entries))
        }
        // If entries are provided, remove them from the clipboard
        Some(content) => {
            let PasteContent {
                entries: pasted_entries,
                source,
            } = content;
            match source {
                RecordType::History => {
                    let (valid_entries, _) = filter_invalid_entries(&clipboard_entries);
                    (valid_entries, None)
                }
                RecordType::Clipboard => {
                    let (valid_entries, invalid_entries) = filter_invalid_entries(&pasted_entries);
                    let pasted_entry_ids: HashSet<Uuid> =
                        pasted_entries.iter().map(|entry| entry.id).collect();
                    let invalid_entry_ids: HashSet<Uuid> =
                        invalid_entries.iter().map(|entry| entry.id).collect();
                    let entries_remaining = clipboard_entries
                        .iter()
                        .filter(|entry| {
                            !pasted_entry_ids.contains(&entry.id)
                                || invalid_entry_ids.contains(&entry.id)
                        })
                        .cloned()
                        .collect();
                    (valid_entries, Some(entries_remaining))
                }
            }
        }
    };
    let mut history_entries = VecDeque::from(read_history()?.unwrap_or(vec![]));

    let options = CopyOptions::new();
    for mut entry in entries_to_paste {
        match entry.operation {
            Operation::Copy => copy_items(&[&entry.path], &destination_path, &options)?,
            Operation::Cut => {
                move_items(&[&entry.path], &destination_path, &options)?;
                let file_name = entry.path.file_name().unwrap();
                let mut new_path = destination_path.clone();
                new_path.push(file_name);
                entry.path = new_path;
                0 // Return 0 to make branches have the same type
            }
        };
        println!("[Info]: paste {}", entry.path.display());
        history_entries.push_front(entry.clone());
    }

    if let Some(entries) = entries_remaining {
        write_clipboard(&entries)?
    }
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
    let filtered_entries: Vec<RecordEntry> = clipboard_entries
        .iter()
        .filter(|entry| entry.id != id)
        .cloned()
        .collect();
    if filtered_entries.len() == clipboard_entries.len() {
        println!("[Error]: no entry found");
    } else {
        write_clipboard(&filtered_entries)?
    }
    Ok(())
}

fn filter_invalid_entries(entries: &[RecordEntry]) -> (Vec<RecordEntry>, Vec<RecordEntry>) {
    entries.iter().cloned().partition(|entry| {
        let validity = entry.check_validity();
        match validity {
            Ok(warning) => {
                if let Some(warning) = warning {
                    eprintln!("[Warning]: {}", warning);
                }
                true
            }
            Err(error) => {
                eprintln!("[Error]: {}", error);
                false
            }
        }
    })
}

fn expand_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut expanded = Vec::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            match glob(&path_str) {
                Ok(entries) => {
                    let mut matched_paths: Vec<PathBuf> = entries
                        .filter_map(|entry| match entry {
                            Ok(path) => Some(path),
                            Err(e) => {
                                eprintln!("[Warning]: Error processing glob entry: {}", e);
                                None
                            }
                        })
                        .collect();

                    if matched_paths.is_empty() {
                        eprintln!("[Warning]: No files match pattern: {}", path_str);
                    } else {
                        matched_paths.sort();
                        expanded.extend(matched_paths);
                    }
                }
                Err(e) => {
                    return Err(
                        format!("[Error]: Invalid glob pattern '{}': {}", path_str, e).into(),
                    );
                }
            }
        } else {
            expanded.push(path);
        }
    }

    Ok(expanded)
}
