use std::{
    env,
    fs::metadata,
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use crate::models::{EntryType, Metadata, ValidityError};

pub fn get_absolute_path(path: &PathBuf) -> Result<PathBuf, IoError> {
    if path.is_relative() {
        let cwd = env::current_dir()?;
        Ok(cwd.join(path).canonicalize()?)
    } else {
        Ok(path.canonicalize()?)
    }
}
pub fn get_metadata(path: &PathBuf) -> Result<Metadata, ValidityError> {
    let absolute_path = match get_absolute_path(path) {
        Ok(path) => path,
        Err(error) => return Err(ValidityError::AbsolutePathError(path.clone(), error)),
    };
    let metadata = match metadata(&absolute_path) {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(ValidityError::PathNotFound(absolute_path));
        }
        Err(error) => {
            return Err(ValidityError::MetadataError(absolute_path, error));
        }
        Ok(metadata) => metadata,
    };

    let modified = match metadata.modified() {
        Ok(modified) => modified,
        Err(error) => return Err(ValidityError::ModifiedAccessError(absolute_path, error)),
    };

    let entry_type = if metadata.is_dir() {
        EntryType::Directory
    } else if metadata.is_symlink() {
        EntryType::Symlink
    } else if metadata.is_file() {
        EntryType::File
    } else {
        return Err(ValidityError::UnsupportedType(absolute_path));
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
