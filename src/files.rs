use fs_extra::{copy_items, dir::CopyOptions, error::ErrorKind as FsErrorKind, move_items};
use glob::glob;
use std::{
    collections::VecDeque,
    env::current_dir,
    fs::{metadata, symlink_metadata},
    io::ErrorKind as IoErrorKind,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    time::SystemTime,
};
use text_io::read;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppInfo, AppWarning, FileError, FileWarning},
    models::{EntryType, Metadata, Operation, PasteContent, RecordEntry, RecordType},
    records::{read_clipboard, read_history, write_clipboard, write_history},
};

pub fn handle_transfer<P: AsRef<Path>>(
    paths: Vec<P>,
    operation: Operation,
) -> Result<(Vec<AppInfo>, Vec<AppWarning>), AppError> {
    let mut clipboard_entries = VecDeque::from(read_clipboard()?.unwrap_or(Vec::new()));
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
        infos.push(match operation {
            Operation::Copy => AppInfo::Copy { path },
            Operation::Cut => AppInfo::Cut { path },
            Operation::Link => AppInfo::Link { path },
        });
    }
    Ok((infos, warnings))
}

pub fn handle_paste<P: AsRef<Path>>(
    destination_path: P,
    paste_content: Option<PasteContent>,
) -> Result<(Vec<AppInfo>, Vec<AppWarning>), AppError> {
    let destination_path = get_absolute_path(&destination_path)?;
    let mut infos = Vec::new();
    let mut warnings = Vec::new();

    let (entries_to_paste, mut clipboard_entries, mut history_entries) = match &paste_content {
        None => (
            read_clipboard()?.unwrap_or(Vec::new()),
            Some(read_clipboard()?.unwrap_or(Vec::new())),
            Some(VecDeque::from(read_history()?.unwrap_or(Vec::new()))),
        ),
        Some(content) => match content.source {
            RecordType::Clipboard => (
                content.entries.clone(),
                Some(read_clipboard()?.unwrap_or(Vec::new())),
                Some(VecDeque::from(read_history()?.unwrap_or(Vec::new()))),
            ),
            RecordType::History => (content.entries.clone(), None, None),
        },
    };

    let mut overwrite_all = false;
    let mut skip_all = false;
    for mut entry in entries_to_paste {
        let mut options = CopyOptions::new();
        options.overwrite = overwrite_all;
        options.skip_exist = skip_all;
        let validity = check_validity(&entry);
        match validity {
            Err(_) => continue,
            Ok(Some(warning)) => warnings.push(AppWarning::File(warning)),
            Ok(_) => (),
        };

        let mut quit = false;
        if !overwrite_all && !skip_all {
            match get_metadata(&destination_path) {
                Ok(metadata) => {
                    let mut valid_input = true;
                    loop {
                        println!(
                            "[Warning]: Destination path already exists (size: {:?}). Overwrite?",
                            metadata.size
                        );
                        println!(
                        "y: yes; n: no; a: overwrite all remaining; s: skip all remaining; q: quit"
                    );
                        let choice: String = read!();
                        match choice.as_str() {
                            "y" => options.overwrite = true,
                            "n" => options.skip_exist = true,
                            "a" => overwrite_all = true,
                            "s" => skip_all = true,
                            "q" => quit = true,
                            _ => valid_input = false,
                        }
                        if valid_input {
                            break;
                        }
                        println!("Invalid input. Please try again.");
                    }
                }
                Err(FileError::PathNotFound { path: _ }) => (),
                Err(error) => return Err(AppError::File(error)),
            };
        }
        if quit {
            break;
        }

        let operation_result = match entry.operation {
            Operation::Copy => copy_items(&[&entry.path], &destination_path, &options)
                .map_err(|source| FileError::Copy {
                    from_path: entry.path.clone(),
                    to_path: destination_path.clone(),
                    source,
                })
                .map(|_| ()),
            Operation::Cut => move_items(&[&entry.path], &destination_path, &options)
                .map_err(|source| FileError::Move {
                    from_path: entry.path.clone(),
                    to_path: destination_path.clone(),
                    source,
                })
                .map(|_| ()),
            Operation::Link => {
                let file_name = entry.path.file_name().ok_or_else(|| FileError::FileName {
                    path: entry.path.clone(),
                })?;
                let mut new_path = destination_path.clone();
                new_path.push(file_name);
                symlink(&entry.path, &new_path)
                    .map_err(|source| FileError::Link {
                        from_path: entry.path.clone(),
                        to_path: destination_path.clone(),
                        source,
                    })
                    .map(|_| ())
            }
        };

        match operation_result {
            Ok(_) => {
                if let Operation::Cut = entry.operation {
                    let file_name = entry.path.file_name().ok_or_else(|| FileError::FileName {
                        path: entry.path.clone(),
                    })?;
                    let mut new_path = destination_path.clone();
                    new_path.push(file_name);
                    entry.path = new_path;
                }
                if let Some(clipboard_entries) = clipboard_entries.as_mut() {
                    clipboard_entries.retain(|clipboard_entry| clipboard_entry.id != entry.id);
                }
                if let Some(history_entries) = history_entries.as_mut() {
                    history_entries.push_front(entry.clone());
                }
                infos.push(AppInfo::Paste {
                    path: entry.path.clone(),
                });
            }
            Err(FileError::Copy {
                from_path,
                to_path,
                source,
            })
            | Err(FileError::Move {
                from_path,
                to_path,
                source,
            }) => match source.kind {
                FsErrorKind::PermissionDenied => {
                    warnings.push(AppWarning::File(FileWarning::NoPermission {
                        path: from_path,
                        destination: to_path,
                    }));
                }
                _ => {
                    return Err(AppError::File(FileError::Copy {
                        from_path,
                        to_path,
                        source,
                    }))
                }
            },
            Err(FileError::Link {
                from_path,
                to_path,
                source,
            }) => match source.kind() {
                IoErrorKind::AlreadyExists => {
                    warnings.push(AppWarning::File(FileWarning::AlreadyExists {
                        path: from_path,
                    }));
                }
                IoErrorKind::PermissionDenied => {
                    warnings.push(AppWarning::File(FileWarning::NoPermission {
                        path: from_path,
                        destination: to_path,
                    }));
                }
                _ => {
                    return Err(AppError::File(FileError::Link {
                        from_path,
                        to_path,
                        source,
                    }))
                }
            },
            Err(error) => return Err(AppError::File(error)),
        }
    }

    if let Some(clipboard_entries) = clipboard_entries {
        write_clipboard(&clipboard_entries)?
    }
    if let Some(history_entries) = history_entries {
        let history_entries: Vec<RecordEntry> = history_entries.into();
        write_history(&history_entries)?;
    }
    Ok((infos, warnings))
}

pub fn get_metadata<P: AsRef<Path>>(path: P) -> Result<Metadata, FileError> {
    let path = path.as_ref();

    let absolute_path = if path.is_relative() {
        current_dir()
            .map_err(|source| FileError::Cwd { source })?
            .join(path)
    } else {
        path.to_path_buf()
    };

    let metadata = symlink_metadata(&absolute_path).map_err(|source| {
        if source.kind() == IoErrorKind::NotFound {
            FileError::PathNotFound {
                path: absolute_path.clone(),
            }
        } else {
            FileError::Metadata {
                path: absolute_path.clone(),
                source,
            }
        }
    })?;

    let canonical_path =
        absolute_path
            .canonicalize()
            .map_err(|source| FileError::AbsolutePath {
                path: absolute_path,
                source,
            })?;

    let modified = metadata
        .modified()
        .map_err(|source| FileError::ModifiedAccess {
            path: canonical_path.clone(),
            source,
        })?;

    let file_type = metadata.file_type();

    let entry_type = match () {
        () if file_type.is_symlink() => EntryType::Symlink,
        () if file_type.is_dir() => EntryType::Directory,
        () if file_type.is_file() => EntryType::File,
        _ => {
            return Err(FileError::UnsupportedType {
                path: canonical_path,
            });
        }
    };

    let size = if entry_type == EntryType::Directory {
        None
    } else {
        Some(metadata.len())
    };

    Ok(Metadata {
        modified,
        size,
        entry_type,
        absolute_path: canonical_path,
    })
}

pub fn get_absolute_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, FileError> {
    let path = path.as_ref();
    let absolute_path = if path.is_relative() {
        let cwd = current_dir().map_err(|source| FileError::Cwd { source })?;
        cwd.join(path)
    } else {
        path.to_path_buf()
    };
    let canonical_path =
        absolute_path
            .canonicalize()
            .map_err(|source| FileError::AbsolutePath {
                path: path.to_path_buf(),
                source,
            })?;
    Ok(canonical_path)
}

fn expand_paths<P: AsRef<Path>>(
    paths: Vec<P>,
) -> Result<(Vec<PathBuf>, Vec<AppWarning>), FileError> {
    let mut expanded = Vec::new();
    let mut warnings = Vec::new();

    for path in paths {
        let path_str = path.as_ref().to_string_lossy();

        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            match glob(&path_str) {
                Ok(entries) => {
                    let mut matched_paths = entries
                        .map(|entry| {
                            entry.map_err(|source| FileError::GlobUnreadable {
                                path: path.as_ref().to_path_buf(),
                                source,
                            })
                        })
                        .collect::<Result<Vec<PathBuf>, FileError>>()?;

                    if matched_paths.is_empty() {
                        warnings.push(
                            FileWarning::GlobUnmatched {
                                path: path.as_ref().to_path_buf(),
                            }
                            .into(),
                        );
                    } else {
                        matched_paths.sort();
                        expanded.extend(matched_paths);
                    }
                }
                Err(source) => {
                    return Err(FileError::GlobInvalidPattern {
                        path: path.as_ref().to_path_buf(),
                        source,
                    });
                }
            }
        } else {
            expanded.push(path.as_ref().to_path_buf());
        }
    }

    Ok((expanded, warnings))
}

fn check_validity(entry: &RecordEntry) -> Result<Option<FileWarning>, FileError> {
    let Metadata {
        modified,
        size,
        entry_type,
        absolute_path,
    } = get_metadata(&entry.path)?;

    if entry_type != entry.entry_type {
        return Ok(Some(FileWarning::TypeMismatch {
            path: absolute_path,
            old_type: entry.entry_type.to_string(),
            new_type: entry_type.to_string(),
        }));
    }

    if let (Some(expected_size), Some(self_size)) = (size, entry.size) {
        if self_size != expected_size {
            return Ok(Some(FileWarning::SizeMismatch {
                path: absolute_path,
                old_size: self_size,
                new_size: expected_size,
            }));
        }
    }

    if modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        > entry
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    {
        return Ok(Some(FileWarning::ModifiedMismatch {
            path: absolute_path,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::Operation,
        test_helpers::{
            create_file_and_get_metadata, create_mock_record_entry, create_test_file,
            get_test_entry, setup_test_env,
        },
    };
    use serial_test::serial;
    use std::{
        fs::{canonicalize, symlink_metadata, File, OpenOptions},
        io::Write,
        os::unix::fs::symlink,
        thread::sleep,
        time::Duration,
    };
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_handle_transfer_copy() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("a.txt");
        create_test_file(&file_path, "a");

        let (infos, warnings) = handle_transfer(vec![&file_path], Operation::Copy).unwrap();

        assert_eq!(infos.len(), 1);
        assert!(warnings.is_empty());

        let clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(clipboard.len(), 1);
        assert_eq!(clipboard[0].operation, Operation::Copy);
        assert_eq!(clipboard[0].path, get_absolute_path(&file_path).unwrap());
    }

    #[test]
    #[serial]
    fn test_handle_paste_copy() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("a.txt");
        create_test_file(&file_path, "a");
        let entry = get_test_entry(&file_path, Operation::Copy);
        write_clipboard(&[entry]).unwrap();

        let (infos, warnings) = handle_paste(&env.dest_dir, None).unwrap();

        assert_eq!(infos.len(), 1);
        assert!(warnings.is_empty());
        assert!(env.dest_dir.join("a.txt").exists());
        assert!(file_path.exists());

        let history = read_history().unwrap().unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    #[serial]
    fn test_handle_paste_cut() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("a.txt");
        create_test_file(&file_path, "a");
        let entry = get_test_entry(&file_path, Operation::Cut);
        write_clipboard(&[entry]).unwrap();

        let (infos, warnings) = handle_paste(&env.dest_dir, None).unwrap();

        assert_eq!(infos.len(), 1);
        assert!(warnings.is_empty());
        let dest_file_path = env.dest_dir.join("a.txt");
        assert!(dest_file_path.exists());
        assert!(!file_path.exists());

        let history = read_history().unwrap().unwrap();
        assert_eq!(history[0].path, get_absolute_path(&dest_file_path).unwrap());
    }

    #[test]
    #[serial]
    #[cfg(unix)]
    fn test_handle_paste_link() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("a.txt");
        create_test_file(&file_path, "a");
        let entry = get_test_entry(&file_path, Operation::Link);
        write_clipboard(&[entry]).unwrap();

        handle_paste(&env.dest_dir, None).unwrap();

        let dest_link_path = env.dest_dir.join("a.txt");
        assert!(dest_link_path.exists());
        assert!(symlink_metadata(&dest_link_path)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    #[serial]
    fn test_handle_paste_with_invalid_entry() {
        let env = setup_test_env();
        let non_existent_path = env.source_dir.join("a.txt");
        let entry = create_mock_record_entry(
            Some(non_existent_path.clone()),
            Some(Operation::Copy),
            None,
            None,
            None,
        );
        write_clipboard(&[entry.clone()]).unwrap();

        let (infos, warnings) = handle_paste(&env.dest_dir, None).unwrap();

        assert!(infos.is_empty());
        assert!(warnings.is_empty());

        let clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(clipboard.len(), 1);
        assert_eq!(clipboard[0].id, entry.id);
    }

    #[test]
    #[serial]
    fn test_handle_paste_with_existing_entry() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("a.txt");
        create_test_file(&file_path, "a");
        let entry = get_test_entry(&file_path, Operation::Copy);
        write_clipboard(&[entry.clone()]).unwrap();
        let destination_file_path = env.dest_dir.join("a.txt");
        create_test_file(&destination_file_path, "a");

        let (infos, warnings) = handle_paste(&env.dest_dir, None).unwrap();

        assert!(infos.is_empty());
        assert!(!warnings.is_empty());
        assert!(matches!(
            warnings[0],
            AppWarning::File(FileWarning::AlreadyExists { .. })
        ));

        let clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(clipboard.len(), 1);
        assert_eq!(clipboard[0].id, entry.id);
    }

    #[test]
    #[serial]
    fn test_expand_paths() {
        let env = setup_test_env();
        let file_a_path = env.source_dir.join("a.txt");
        let file_b_path = env.source_dir.join("b.txt");
        let file_c_path = env.source_dir.join("c.log");
        create_test_file(&file_a_path, "a");
        create_test_file(&file_b_path, "b");
        create_test_file(&file_c_path, "c");

        let glob_path = env.source_dir.join("*.txt");
        let (expanded, warnings) = expand_paths(vec![glob_path.clone()]).unwrap();
        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains(&file_a_path));
        assert!(expanded.contains(&file_b_path));
        assert!(warnings.is_empty());

        let unmatched_glob_path = env.source_dir.join("*.md");
        let (expanded, warnings) = expand_paths(vec![unmatched_glob_path.clone()]).unwrap();
        assert!(expanded.is_empty());
        assert!(!warnings.is_empty());
        assert!(matches!(
            warnings[0],
            AppWarning::File(FileWarning::GlobUnmatched { .. })
        ));

        let (expanded, warnings) = expand_paths(vec![file_c_path.clone()]).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], file_c_path);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_check_validity_happy_path() {
        let dir = tempdir().expect("Failed to create temp dir");
        let metadata = create_file_and_get_metadata(&dir, "valid.txt", "hello");

        let entry = create_mock_record_entry(
            Some(metadata.absolute_path),
            Some(Operation::Copy),
            Some(metadata.entry_type),
            Some(metadata.modified),
            metadata.size,
        );

        let result = check_validity(&entry).expect("check_validity failed");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_validity_type_mismatch() {
        let dir = tempdir().expect("Failed to create temp dir");
        let metadata = create_file_and_get_metadata(&dir, "type_mismatch.txt", "hello");

        let entry = create_mock_record_entry(
            Some(metadata.absolute_path),
            Some(Operation::Copy),
            Some(EntryType::Directory), // Intentionally wrong type
            Some(metadata.modified),
            metadata.size,
        );

        let warning = check_validity(&entry)
            .expect("check_validity failed")
            .expect("Expected a warning");

        matches!(warning, FileWarning::TypeMismatch { .. });
    }

    #[test]
    fn test_check_validity_size_mismatch() {
        let dir = tempdir().expect("Failed to create temp dir");
        let metadata = create_file_and_get_metadata(&dir, "size_mismatch.txt", "hello");

        let entry = create_mock_record_entry(
            Some(metadata.absolute_path),
            Some(Operation::Copy),
            Some(metadata.entry_type),
            Some(metadata.modified),
            Some(0), // Intentionally wrong size
        );

        let warning = check_validity(&entry)
            .expect("check_validity failed")
            .expect("Expected a warning");

        matches!(warning, FileWarning::SizeMismatch { .. });
    }

    #[test]
    fn test_check_validity_modified_mismatch() {
        let dir = tempdir().expect("Failed to create temp dir");
        let metadata = create_file_and_get_metadata(&dir, "modified.txt", "first");

        let entry = create_mock_record_entry(
            Some(metadata.absolute_path.clone()),
            Some(Operation::Copy),
            Some(metadata.entry_type),
            Some(metadata.modified),
            metadata.size,
        );

        // Sleep to ensure the modification time will be different
        sleep(Duration::from_secs(1));

        let mut file = OpenOptions::new()
            .write(true)
            .open(&metadata.absolute_path)
            .unwrap();
        writeln!(file, "second").unwrap();
        file.sync_all().unwrap();

        let warning = check_validity(&entry)
            .expect("check_validity failed")
            .expect("Expected a warning");

        matches!(warning, FileWarning::ModifiedMismatch { .. });
    }

    #[test]
    fn test_check_validity_path_not_found() {
        let entry = create_mock_record_entry(
            Some(PathBuf::from("/this/path/should/not/exist/ever.txt")),
            Some(Operation::Copy),
            Some(EntryType::File),
            None,
            None,
        );

        let result = check_validity(&entry);
        assert!(result.is_err());
        matches!(result.unwrap_err(), FileError::PathNotFound { .. });
    }

    #[test]
    fn test_get_absolute_path_for_existing_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        File::create(&file_path).unwrap();

        let absolute_path = get_absolute_path(&file_path).unwrap();
        assert!(absolute_path.is_absolute());
        assert!(absolute_path.exists());
        assert_eq!(absolute_path, canonicalize(&file_path).unwrap());
    }

    #[test]
    fn test_get_absolute_path_for_non_existent_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent.txt");

        let result = get_absolute_path(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_metadata_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "hello").unwrap();

        let metadata = get_metadata(&file_path).unwrap();

        assert_eq!(metadata.entry_type, EntryType::File);
        assert_eq!(metadata.size, Some(6)); // "hello\n"
        assert!(metadata.absolute_path.ends_with("test_file.txt"));
    }

    #[test]
    fn test_get_metadata_for_directory() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let metadata = get_metadata(dir_path).unwrap();

        assert_eq!(metadata.entry_type, EntryType::Directory);
        assert_eq!(metadata.size, None);
        assert!(metadata.absolute_path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_get_metadata_for_symlink() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("target.txt");
        File::create(&file_path).unwrap();

        let symlink_path = dir.path().join("link.txt");
        symlink(&file_path, &symlink_path).unwrap();

        let metadata = get_metadata(&symlink_path).unwrap();

        assert_eq!(metadata.entry_type, EntryType::Symlink);
    }

    #[test]
    fn test_get_metadata_for_non_existent_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent.txt");

        let result = get_metadata(&file_path);
        assert!(result.is_err());

        match result.unwrap_err() {
            FileError::PathNotFound { .. } => (),
            other_error => panic!("Expected PathNotFound error, got {:?}", other_error),
        }
    }

    #[test]
    #[serial]
    fn test_handle_paste_duplicate_entries() {
        let env = setup_test_env();
        let file_path = env.source_dir.join("duplicate.txt");
        create_test_file(&file_path, "content");

        let entry1 = get_test_entry(&file_path, Operation::Copy);
        let entry2 = get_test_entry(&file_path, Operation::Copy);

        write_clipboard(&[entry1, entry2]).unwrap();

        let (infos, warnings) = handle_paste(&env.dest_dir, None).unwrap();

        assert_eq!(infos.len(), 1);
        assert_eq!(warnings.len(), 1);

        assert!(matches!(infos[0], AppInfo::Paste { .. }));
        assert!(matches!(
            warnings[0],
            AppWarning::File(FileWarning::AlreadyExists { .. })
        ));
        assert!(env.dest_dir.join("duplicate.txt").exists());
        let clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(clipboard.len(), 1);
    }
}
