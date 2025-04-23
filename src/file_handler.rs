use serde::{de::DeserializeOwned, Serialize};
use shellexpand::tilde;
use std::{
    fs::{create_dir_all, File},
    io::{ErrorKind, Read, Result as IoResult, Write},
    path::PathBuf,
    sync::Mutex,
};
use toml::{de::from_str, ser::to_string};

use crate::models::{
    storage_type_to_string, ClipboardData, ClipboardEntry, HistoryData, HistoryEntry, StorageType,
};

// Static mutex to protect file access
pub static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());
pub static HISTORY_MUTEX: Mutex<()> = Mutex::new(());

pub fn get_config_dir() -> IoResult<PathBuf> {
    let path = tilde("~/.local/state/file_clipper").into_owned();
    create_dir_all(&path)?;
    Ok(PathBuf::from(path))
}

pub fn get_storage_path(storage_type: StorageType) -> IoResult<PathBuf> {
    get_config_dir().map(|dir| dir.join(format!("{}.toml", storage_type_to_string(storage_type))))
}

pub fn read_toml_file<T: DeserializeOwned>(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
) -> IoResult<Option<T>> {
    let _lock = mutex.lock().unwrap();

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match from_str(&contents) {
        Ok(parsed) => Ok(Some(parsed)),
        Err(e) => {
            eprintln!("Error parsing TOML file '{}': {}", path.display(), e);
            // Consider returning a default value or a specific error type here
            Ok(None)
        }
    }
}

pub fn write_toml_file<T: Serialize>(
    path: &PathBuf,
    mutex: &'static Mutex<()>,
    data: T,
) -> IoResult<()> {
    let _lock = mutex.lock().unwrap(); // Acquire lock
    match to_string(&data) {
        Ok(toml_string) => {
            let mut file = File::create(path)?;
            file.write_all(toml_string.as_bytes())?;
            Ok(())
        }
        Err(e) => {
            eprintln!("Error serializing to TOML string: {}", e);
            Err(std::io::Error::new(ErrorKind::Other, e))
        }
    }
}

pub fn read_clipboard() -> IoResult<Option<Vec<ClipboardEntry>>> {
    let path = get_storage_path(StorageType::Clipboard)?;
    read_toml_file::<ClipboardData>(&path, &CLIPBOARD_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn read_history() -> IoResult<Option<Vec<HistoryEntry>>> {
    let path = get_storage_path(StorageType::History)?;
    read_toml_file::<HistoryData>(&path, &HISTORY_MUTEX).map(|data| data.map(|d| d.entries))
}

pub fn write_clipboard(entries: &[ClipboardEntry]) -> IoResult<()> {
    let path = get_storage_path(StorageType::Clipboard)?;
    let clipboard_data = ClipboardData {
        entries: entries.to_vec(),
    };
    write_toml_file(&path, &CLIPBOARD_MUTEX, clipboard_data)
}

pub fn write_history(entries: &[HistoryEntry]) -> IoResult<()> {
    let path = get_storage_path(StorageType::History)?;
    let history_data = HistoryData {
        entries: entries.to_vec(),
    };
    write_toml_file(&path, &HISTORY_MUTEX, history_data)
}
