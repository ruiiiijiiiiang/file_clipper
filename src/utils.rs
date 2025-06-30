use std::{
    env,
    fs::metadata,
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use crate::exceptions::FileError;
use crate::models::{EntryType, Metadata};

pub fn get_absolute_path(path: &PathBuf) -> Result<PathBuf, IoError> {
    if path.is_relative() {
        let cwd = env::current_dir()?;
        Ok(cwd.join(path).canonicalize()?)
    } else {
        Ok(path.canonicalize()?)
    }
}
pub fn get_metadata(path: &PathBuf) -> Result<Metadata, FileError> {
    let absolute_path = get_absolute_path(path).map_err(|error| FileError::AbsolutePath {
        path: path.clone(),
        source: error,
    })?;

    let metadata = metadata(&absolute_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            FileError::PathNotFound {
                path: absolute_path.clone(),
            }
        } else {
            FileError::Metadata {
                path: absolute_path.clone(),
                source: error,
            }
        }
    })?;

    let modified = metadata
        .modified()
        .map_err(|error| FileError::ModifiedAccess {
            path: absolute_path.clone(),
            source: error,
        })?;

    let entry_type = if metadata.is_dir() {
        EntryType::Directory
    } else if metadata.is_symlink() {
        EntryType::Symlink
    } else if metadata.is_file() {
        EntryType::File
    } else {
        return Err(FileError::UnsupportedType {
            path: absolute_path,
        });
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
        absolute_path,
    })
}
