use fs_extra::{copy_items, dir::CopyOptions, move_items};
use glob::glob;
use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    time::SystemTime,
};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppInfo, AppWarning, FileError, FileWarning, RecordError, RecordWarning},
    models::{Metadata, Operation, PasteContent, RecordEntry, RecordType},
    records::{read_clipboard, read_history, write_clipboard, write_history},
    utils::{get_absolute_path, get_metadata},
};

pub fn handle_transfer(
    paths: Vec<PathBuf>,
    operation: Operation,
) -> Result<(Vec<AppInfo>, Option<Vec<AppWarning>>), AppError> {
    let mut clipboard_entries = VecDeque::from(read_clipboard()?.unwrap_or(vec![]));
    let (expanded_paths, warnings) = expand_paths(paths)?;
    let mut infos = Vec::new();

    for path in &expanded_paths {
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
    for path in expanded_paths {
        infos.push(AppInfo::Transfer {
            operation: operation.clone(),
            path,
        });
    }
    Ok((infos, warnings))
}

pub fn handle_paste(
    destination_path: PathBuf,
    paste_content: Option<PasteContent>,
) -> Result<(Vec<AppInfo>, Option<Vec<AppWarning>>), AppError> {
    let warnings: Option<Vec<AppWarning>>;
    let destination_path = get_absolute_path(&destination_path)?;
    let clipboard_entries = read_clipboard()?.unwrap_or(Vec::new());
    let mut infos = Vec::new();
    let skip_write_history = if let Some(paste_contend) = &paste_content {
        paste_contend.source == RecordType::History
    } else {
        false
    };
    let mut history_entries = if !skip_write_history {
        Some(VecDeque::from(read_history()?.unwrap_or(vec![])))
    } else {
        None
    };

    let (entries_to_paste, entries_remaining) = match &paste_content {
        None => {
            let (valid_entries, invalid_entries, validation_warnings) =
                filter_invalid_entries(&clipboard_entries);
            warnings = validation_warnings;
            (valid_entries, Some(invalid_entries))
        }
        Some(content) => {
            let PasteContent {
                entries: pasted_entries,
                source,
            } = content;
            match source {
                RecordType::History => {
                    let (mut valid_entries, _, validation_warnings) =
                        filter_invalid_entries(pasted_entries);
                    valid_entries
                        .iter_mut()
                        .for_each(|entry| entry.operation = Operation::Copy);
                    warnings = validation_warnings;
                    (valid_entries, None)
                }
                RecordType::Clipboard => {
                    let (valid_entries, invalid_entries, validation_warnings) =
                        filter_invalid_entries(pasted_entries);
                    warnings = validation_warnings;
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

    let options = CopyOptions::new();
    for mut entry in entries_to_paste {
        match entry.operation {
            Operation::Copy => {
                copy_items(&[&entry.path], &destination_path, &options).map_err(|error| {
                    FileError::Copy {
                        from_path: entry.path.clone(),
                        to_path: destination_path.clone(),
                        source: error,
                    }
                })?;
            }
            Operation::Cut => {
                move_items(&[&entry.path], &destination_path, &options).map_err(|error| {
                    FileError::Move {
                        from_path: entry.path.clone(),
                        to_path: destination_path.clone(),
                        source: error,
                    }
                })?;
                let file_name = entry.path.file_name().unwrap();
                let mut new_path = destination_path.clone();
                new_path.push(file_name);
                entry.path = new_path;
            }
        };
        infos.push(AppInfo::Paste {
            path: entry.path.clone(),
        });
        if let Some(history_entries) = history_entries.as_mut() {
            history_entries.push_front(entry.clone());
        }
    }

    if let Some(entries) = entries_remaining {
        write_clipboard(&entries)?
    }
    if let Some(history_entries) = history_entries {
        let history_entries: Vec<RecordEntry> = history_entries.into();
        write_history(&history_entries)?;
    }
    Ok((infos, warnings))
}

pub fn handle_remove(id: Uuid) -> Result<Option<RecordWarning>, RecordError> {
    let clipboard_entries = match read_clipboard() {
        Ok(Some(entries)) => entries,
        _ => return Ok(Some(RecordWarning::ClipboardUnreadable)),
    };
    let filtered_entries: Vec<RecordEntry> = clipboard_entries
        .iter()
        .filter(|entry| entry.id != id)
        .cloned()
        .collect();
    if filtered_entries.len() == clipboard_entries.len() {
        return Ok(Some(RecordWarning::EntryNotFound));
    } else {
        write_clipboard(&filtered_entries)?
    }
    Ok(None)
}

fn filter_invalid_entries(
    entries: &[RecordEntry],
) -> (Vec<RecordEntry>, Vec<RecordEntry>, Option<Vec<AppWarning>>) {
    let mut warnings: Vec<AppWarning> = Vec::new();
    let (valid_entries, invalid_entries) = entries.iter().cloned().partition(|entry| {
        let validity = entry.check_validity();
        match validity {
            Ok(warning) => {
                if let Some(warning) = warning {
                    warnings.push(warning.into());
                }
                true
            }
            Err(_) => false,
        }
    });
    (
        valid_entries,
        invalid_entries,
        if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    )
}

fn expand_paths(paths: Vec<PathBuf>) -> Result<(Vec<PathBuf>, Option<Vec<AppWarning>>), FileError> {
    let mut expanded = Vec::new();
    let mut warnings = Vec::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            match glob(&path_str) {
                Ok(entries) => {
                    let mut matched_paths = entries
                        .map(|entry| {
                            entry.map_err(|error| FileError::GlobUnreadable {
                                path: path.clone(),
                                source: error,
                            })
                        })
                        .collect::<Result<Vec<PathBuf>, FileError>>()?;

                    if matched_paths.is_empty() {
                        warnings.push(FileWarning::GlobUnmatched { path: path.clone() }.into());
                    } else {
                        matched_paths.sort();
                        expanded.extend(matched_paths);
                    }
                }
                Err(error) => {
                    return Err(FileError::GlobInvalidPattern {
                        path: path.clone(),
                        source: error,
                    });
                }
            }
        } else {
            expanded.push(path);
        }
    }

    Ok((
        expanded,
        if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    ))
}

