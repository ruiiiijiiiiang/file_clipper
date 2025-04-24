use serde::{de::DeserializeOwned, Serialize};
use shellexpand::tilde;
use std::{
    fs::{create_dir_all, File},
    io::{ErrorKind, Read, Result as IoResult, Write},
    path::PathBuf,
    sync::Mutex,
};
use toml::{de::from_str, ser::to_string};

use crate::models::{record_type_to_string, RecordData, RecordEntry, RecordType};

// Static mutex to protect file access
pub static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());
pub static HISTORY_MUTEX: Mutex<()> = Mutex::new(());

pub fn get_config_dir() -> IoResult<PathBuf> {
    let path = tilde("~/.local/state/file_clipper").into_owned();
    create_dir_all(&path)?;
    Ok(PathBuf::from(path))
}

pub fn get_storage_path(record_type: RecordType) -> IoResult<PathBuf> {
    get_config_dir().map(|dir| dir.join(format!("{}.toml", record_type_to_string(record_type))))
}

pub fn read_toml_file<T: DeserializeOwned>(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
) -> IoResult<Option<T>> {
    let _lock = mutex.lock().unwrap();

    let mut file = match File::open(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
        Ok(file) => file,
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match from_str(&contents) {
        Err(e) => {
            eprintln!(
                "Error: failed to parse TOML file '{}': {}",
                path.display(),
                e
            );
            Ok(None)
        }
        Ok(parsed) => Ok(Some(parsed)),
    }
}

pub fn write_toml_file<T: Serialize>(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
    data: T,
) -> IoResult<()> {
    let _lock = mutex.lock().unwrap(); // Acquire lock
    match to_string(&data) {
        Err(e) => {
            eprintln!("Error: failed to serialize to TOML string: {}", e);
            Err(std::io::Error::new(ErrorKind::Other, e))
        }
        Ok(toml_string) => {
            let mut file = File::create(path)?;
            file.write_all(toml_string.as_bytes())?;
            Ok(())
        }
    }
}

pub fn read_clipboard() -> IoResult<Option<Vec<RecordEntry>>> {
    let path = get_storage_path(RecordType::Clipboard)?;
    read_toml_file::<RecordData>(&path, &CLIPBOARD_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn read_history() -> IoResult<Option<Vec<RecordEntry>>> {
    let path = get_storage_path(RecordType::History)?;
    read_toml_file::<RecordData>(&path, &HISTORY_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn write_clipboard(entries: &[RecordEntry]) -> IoResult<()> {
    let path = get_storage_path(RecordType::Clipboard)?;
    let record_data = RecordData {
        entries: entries.to_vec(),
    };
    write_toml_file(&path, &CLIPBOARD_MUTEX, record_data)
}

pub fn write_history(entries: &[RecordEntry]) -> IoResult<()> {
    let path = get_storage_path(RecordType::History)?;
    let record_data = RecordData {
        entries: entries.to_vec(),
    };
    write_toml_file(&path, &HISTORY_MUTEX, record_data)
}
