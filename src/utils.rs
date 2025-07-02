use std::{env, fs::metadata, io::ErrorKind, path::{Path, PathBuf}};

use crate::{
    errors::FileError,
    models::{EntryType, Metadata},
};

pub fn get_absolute_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, FileError> {
    let path = path.as_ref();
    let absolute_path = if path.is_relative() {
        let cwd = env::current_dir().map_err(|error| FileError::Cwd { source: error })?;
        cwd.join(path)
    } else {
        path.to_path_buf()
    };
    let canonical_path = absolute_path
        .canonicalize()
        .map_err(|error| FileError::AbsolutePath {
            path: path.to_path_buf(),
            source: error,
        })?;
    Ok(canonical_path)
}

pub fn get_metadata<P: AsRef<Path>>(path: P) -> Result<Metadata, FileError> {
    let path = path.as_ref();
    let absolute_path = get_absolute_path(path)?;

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