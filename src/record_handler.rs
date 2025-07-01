use shellexpand::tilde;
use std::{
    fs::{create_dir_all, File},
    io::{ErrorKind, Read, Write},
    path::PathBuf,
    sync::Mutex,
};
use toml::{de::from_str as toml_from_str, ser::to_string as toml_to_string};

use crate::{
    exceptions::RecordError,
    models::{RecordData, RecordEntry, RecordType},
};

static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());
static HISTORY_MUTEX: Mutex<()> = Mutex::new(());

const MAX_CLIPBOARD_ENTRIES: usize = 200;
const STORAGE_DIR: &str = "~/.local/state/file_clipper";

pub fn read_toml_file(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
) -> Result<Option<RecordData>, RecordError> {
    let _lock = mutex.lock().unwrap();

    let mut file = match File::open(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(RecordError::OpenRecordFile {
                path: path.into(),
                source: error,
            })
        }
        Ok(file) => file,
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|error| RecordError::ReadRecordFile {
            path: path.into(),
            source: error,
        })?;

    match toml_from_str(&contents) {
        Err(error) => Err(RecordError::DeserializeRecordFile {
            path: path.into(),
            source: error,
        }),
        Ok(parsed) => Ok(Some(parsed)),
    }
}

pub fn write_toml_file(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
    data: RecordData,
) -> Result<(), RecordError> {
    let _lock = mutex.lock().unwrap();
    match toml_to_string(&data) {
        Err(error) => Err(RecordError::SerializeRecordFile { source: error }),
        Ok(toml_string) => {
            let mut file = File::create(path).map_err(|error| RecordError::CreateRecordFile {
                path: path.clone(),
                source: error,
            })?;
            file.write_all(toml_string.as_bytes()).map_err(|error| {
                RecordError::WriteRecordFile {
                    path: path.clone(),
                    source: error,
                }
            })?;
            Ok(())
        }
    }
}

pub fn read_clipboard() -> Result<Option<Vec<RecordEntry>>, RecordError> {
    let path = get_storage_path(RecordType::Clipboard)?;
    read_toml_file(&path, &CLIPBOARD_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn read_history() -> Result<Option<Vec<RecordEntry>>, RecordError> {
    let path = get_storage_path(RecordType::History)?;
    read_toml_file(&path, &HISTORY_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn write_clipboard(entries: &[RecordEntry]) -> Result<(), RecordError> {
    let path = get_storage_path(RecordType::Clipboard)?;
    let capped_entries = if entries.len() > MAX_CLIPBOARD_ENTRIES {
        &entries[..MAX_CLIPBOARD_ENTRIES]
    } else {
        entries
    };
    let record_data = RecordData {
        entries: capped_entries.to_vec(),
    };
    write_toml_file(&path, &CLIPBOARD_MUTEX, record_data)
}

pub fn write_history(entries: &[RecordEntry]) -> Result<(), RecordError> {
    let path = get_storage_path(RecordType::History)?;
    let capped_entries = if entries.len() > MAX_CLIPBOARD_ENTRIES {
        &entries[..MAX_CLIPBOARD_ENTRIES]
    } else {
        entries
    };
    let record_data = RecordData {
        entries: capped_entries.to_vec(),
    };
    write_toml_file(&path, &HISTORY_MUTEX, record_data)
}

fn get_storage_path(record_type: RecordType) -> Result<PathBuf, RecordError> {
    let path = PathBuf::from(tilde(STORAGE_DIR).as_ref());
    create_dir_all(&path).map_err(|error| RecordError::CreateConfigDir {
        path: path.clone(),
        source: error,
    })?;
    Ok(path.join(format!("{}.toml", record_type)))
}
