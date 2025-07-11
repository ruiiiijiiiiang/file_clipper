use std::{
    env::home_dir,
    fs::{create_dir_all, remove_dir, remove_file, File},
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use toml::{de::from_str as toml_from_str, ser::to_string as toml_to_string};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppInfo, AppWarning, RecordError, RecordWarning},
    models::{RecordData, RecordEntry, RecordType},
};

static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());
static HISTORY_MUTEX: Mutex<()> = Mutex::new(());

const MAX_CLIPBOARD_ENTRIES: usize = 200;
const STORAGE_DIR: &str = ".local/state/file_clipper";

pub fn read_entries(mode: &RecordType) -> Result<Vec<RecordEntry>, AppError> {
    let entries = match mode {
        RecordType::Clipboard => read_clipboard()?.unwrap_or(vec![]),
        RecordType::History => read_history()?.unwrap_or(vec![]),
    };
    Ok(entries)
}

pub fn read_clipboard() -> Result<Option<Vec<RecordEntry>>, RecordError> {
    read_records(RecordType::Clipboard)
}

pub fn read_history() -> Result<Option<Vec<RecordEntry>>, RecordError> {
    read_records(RecordType::History)
}

pub fn write_clipboard(entries: &[RecordEntry]) -> Result<(), RecordError> {
    write_records(entries, RecordType::Clipboard)
}

pub fn write_history(entries: &[RecordEntry]) -> Result<(), RecordError> {
    write_records(entries, RecordType::History)
}

pub fn handle_remove(id: Uuid) -> Result<Vec<AppWarning>, AppError> {
    let mut warnings = Vec::new();
    let clipboard_entries = match read_clipboard() {
        Ok(Some(entries)) => entries,
        _ => {
            warnings.push(AppWarning::Record(RecordWarning::ClipboardUnreadable));
            return Ok(warnings);
        }
    };
    let filtered_entries: Vec<RecordEntry> = clipboard_entries
        .iter()
        .filter(|entry| entry.id != id)
        .cloned()
        .collect();
    if filtered_entries.len() == clipboard_entries.len() {
        warnings.push(AppWarning::Record(RecordWarning::EntryNotFound));
        return Ok(warnings);
    } else {
        write_clipboard(&filtered_entries)?
    }
    Ok(warnings)
}

pub fn clear_records() -> Result<Vec<AppInfo>, AppError> {
    let mut infos = Vec::new();
    for record_type in [RecordType::Clipboard, RecordType::History] {
        let record_path = get_storage_path(record_type)?;
        match remove_file(&record_path) {
            Err(source) if source.kind() != ErrorKind::NotFound => {
                return Err(AppError::Record(RecordError::ClearRecords {
                    path: record_path.clone(),
                    source,
                }))
            }
            _ => {
                infos.push(AppInfo::Clear { path: record_path });
            }
        };
    }

    let dir_path = home_dir().ok_or(RecordError::GetHomeDir)?.join(STORAGE_DIR);
    match remove_dir(&dir_path) {
        Err(source) if source.kind() != ErrorKind::NotFound => {
            return Err(AppError::Record(RecordError::ClearRecords {
                path: dir_path.clone(),
                source,
            }))
        }
        _ => {
            infos.push(AppInfo::Clear { path: dir_path });
        }
    }
    Ok(infos)
}

fn get_storage_path(record_type: RecordType) -> Result<PathBuf, RecordError> {
    let path = home_dir().ok_or(RecordError::GetHomeDir)?.join(STORAGE_DIR);
    create_dir_all(&path).map_err(|source| RecordError::CreateConfigDir {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(path.join(format!("{}.toml", record_type)))
}

fn read_records(record_type: RecordType) -> Result<Option<Vec<RecordEntry>>, RecordError> {
    let (path, mutex) = match record_type {
        RecordType::Clipboard => (get_storage_path(RecordType::Clipboard)?, &CLIPBOARD_MUTEX),
        RecordType::History => (get_storage_path(RecordType::History)?, &HISTORY_MUTEX),
    };
    read_toml_file(&path, mutex).map(|data| data.map(|d| d.entries))
}

fn write_records(entries: &[RecordEntry], record_type: RecordType) -> Result<(), RecordError> {
    let (path, mutex) = match record_type {
        RecordType::Clipboard => (get_storage_path(RecordType::Clipboard)?, &CLIPBOARD_MUTEX),
        RecordType::History => (get_storage_path(RecordType::History)?, &HISTORY_MUTEX),
    };
    let capped_entries = if entries.len() > MAX_CLIPBOARD_ENTRIES {
        &entries[..MAX_CLIPBOARD_ENTRIES]
    } else {
        entries
    };
    let record_data = RecordData {
        entries: capped_entries.to_vec(),
    };
    write_toml_file(&path, mutex, record_data)
}

fn read_toml_file<P: AsRef<Path>>(
    path: P,
    mutex: &Mutex<()>,
) -> Result<Option<RecordData>, RecordError> {
    let path = path.as_ref();
    let _lock = mutex.lock().unwrap();

    let mut file = match File::open(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(RecordError::OpenRecordFile {
                path: path.into(),
                source,
            })
        }
        Ok(file) => file,
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|source| RecordError::ReadRecordFile {
            path: path.into(),
            source,
        })?;

    match toml_from_str(&contents) {
        Err(source) => Err(RecordError::DeserializeRecordFile {
            path: path.into(),
            source,
        }),
        Ok(parsed) => Ok(Some(parsed)),
    }
}

fn write_toml_file<P: AsRef<Path>>(
    path: P,
    mutex: &Mutex<()>,
    data: RecordData,
) -> Result<(), RecordError> {
    let path = path.as_ref();
    let _lock = mutex.lock().unwrap();

    match toml_to_string(&data) {
        Err(source) => Err(RecordError::SerializeRecordFile { source }),
        Ok(toml_string) => {
            let mut file = File::create(path).map_err(|source| RecordError::CreateRecordFile {
                path: path.to_path_buf(),
                source,
            })?;
            file.write_all(toml_string.as_bytes()).map_err(|source| {
                RecordError::WriteRecordFile {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::Operation,
        test_helpers::{create_mock_record_entry, setup_test_env},
    };
    use serial_test::serial;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_then_read_toml_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let mutex = Mutex::new(());

        let entries = vec![
            create_mock_record_entry(
                Some(PathBuf::from("/tmp/file_1.txt")),
                Some(Operation::Copy),
                None,
                None,
                None,
            ),
            create_mock_record_entry(
                Some(PathBuf::from("/tmp/file_2.txt")),
                Some(Operation::Copy),
                None,
                None,
                None,
            ),
        ];
        let record_data = RecordData {
            entries: entries.clone(),
        };

        let write_result = write_toml_file(path, &mutex, record_data);
        assert!(write_result.is_ok());

        let read_result = read_toml_file(path, &mutex).unwrap();
        assert!(read_result.is_some());

        let read_data = read_result.unwrap();
        assert_eq!(read_data.entries.len(), 2);
        assert_eq!(read_data.entries[0].operation, Operation::Copy);
        assert_eq!(
            read_data.entries[1].path.to_str().unwrap(),
            "/tmp/file_2.txt"
        );
    }

    #[test]
    fn test_read_nonexistent_file() {
        let path = PathBuf::from("/tmp/this/file/does/not/exist.toml");
        let mutex = Mutex::new(());
        let result = read_toml_file(&path, &mutex).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_malformed_toml_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "this is not valid toml content").unwrap();

        let path = temp_file.path();
        let mutex = Mutex::new(());
        let result = read_toml_file(path, &mutex);

        assert!(result.is_err());
        match result.unwrap_err() {
            RecordError::DeserializeRecordFile { .. } => {}
            other_error => panic!(
                "Expected DeserializeRecordFile error, but got {:?}",
                other_error
            ),
        }
    }

    #[test]
    #[serial]
    fn test_write_records_capping() {
        let _env = setup_test_env();

        let mut entries = Vec::new();
        for _ in 0..(MAX_CLIPBOARD_ENTRIES + 50) {
            entries.push(create_mock_record_entry(None, None, None, None, None));
        }

        write_clipboard(&entries).unwrap();

        let capped_clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(capped_clipboard.len(), MAX_CLIPBOARD_ENTRIES);
    }

    #[test]
    #[serial]
    fn test_handle_remove_existing() {
        let _env = setup_test_env();
        let entry1 = create_mock_record_entry(
            Some(PathBuf::from("/tmp/file1")),
            Some(Operation::Copy),
            None,
            None,
            None,
        );
        let entry2 = create_mock_record_entry(
            Some(PathBuf::from("/tmp/file2")),
            Some(Operation::Copy),
            None,
            None,
            None,
        );
        write_clipboard(&[entry1.clone(), entry2.clone()]).unwrap();

        let result = handle_remove(entry1.id).unwrap();
        assert!(result.is_empty());

        let clipboard = read_clipboard().unwrap().unwrap();
        assert_eq!(clipboard.len(), 1);
        assert_eq!(clipboard[0].id, entry2.id);
    }

    #[test]
    #[serial]
    fn test_handle_remove_non_existing() {
        let _env = setup_test_env();
        let entry1 = create_mock_record_entry(
            Some(PathBuf::from("/tmp/file1")),
            Some(Operation::Copy),
            None,
            None,
            None,
        );
        write_clipboard(&[entry1]).unwrap();

        let random_id = Uuid::new_v4();
        let result = handle_remove(random_id).unwrap();
        assert!(!result.is_empty());
        assert!(matches!(
            result[0],
            AppWarning::Record(RecordWarning::EntryNotFound)
        ));
    }

    #[test]
    #[serial]
    fn test_clear_records_success() {
        let _env = setup_test_env();

        let clipboard_entry = create_mock_record_entry(None, None, None, None, None);
        write_clipboard(&[clipboard_entry]).unwrap();
        let history_entry = create_mock_record_entry(None, None, None, None, None);
        write_history(&[history_entry]).unwrap();

        let clipboard_path = get_storage_path(RecordType::Clipboard).unwrap();
        let history_path = get_storage_path(RecordType::History).unwrap();
        let dir_path = _env.home_dir.path().join(STORAGE_DIR);

        assert!(clipboard_path.exists());
        assert!(history_path.exists());
        assert!(dir_path.exists());

        let result = clear_records().unwrap();

        assert_eq!(result.len(), 3);
        assert!(matches!(&result[0], AppInfo::Clear { path: p } if p == &clipboard_path));
        assert!(matches!(&result[1], AppInfo::Clear { path: p } if p == &history_path));
        assert!(matches!(&result[2], AppInfo::Clear { path: p } if p == &dir_path));

        assert!(!clipboard_path.exists());
        assert!(!history_path.exists());
        assert!(!dir_path.exists());
    }
}
