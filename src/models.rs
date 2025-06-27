use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};
use std::{io::Error as IoError, path::PathBuf, time::SystemTime};
use strum_macros::Display;
use thiserror::Error;
use uuid::Uuid;

use crate::utils::get_metadata;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    Copy,
    Cut,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum RecordType {
    #[strum(to_string = "clipboard")]
    Clipboard,
    #[strum(to_string = "history")]
    History,
}

pub struct Metadata {
    pub modified: SystemTime,
    pub size: Option<u64>,
    pub entry_type: EntryType,
    pub absolute_path: PathBuf,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordEntry {
    #[serde_as(as = "TimestampSeconds")]
    pub timestamp: SystemTime,
    pub size: Option<u64>,
    pub operation: Operation,
    pub entry_type: EntryType,
    pub path: PathBuf,
    pub id: Uuid,
}

impl RecordEntry {
    pub fn check_validity(&self) -> Result<Option<ValidityWarning>, ValidityError> {
        let Metadata {
            modified,
            size,
            entry_type,
            absolute_path,
        } = get_metadata(&self.path)?;

        if entry_type != self.entry_type {
            return Ok(Some(ValidityWarning::Type(absolute_path)));
        }

        if let (Some(expected_size), Some(self_size)) = (size, self.size) {
            if self_size != expected_size {
                return Ok(Some(ValidityWarning::Size(absolute_path)));
            }
        }

        if modified > self.timestamp {
            return Ok(Some(ValidityWarning::Modified(absolute_path)));
        }

        Ok(None)
    }
}

#[derive(Debug, Error)]
pub enum ValidityError {
    #[error("{0:?} is not found")]
    PathNotFound(PathBuf),
    #[error("failed to get absolute path for {0:?}: {1}")]
    AbsolutePathError(PathBuf, IoError),
    #[error("failed to get metadata path for {0:?}: {1}")]
    MetadataError(PathBuf, IoError),
    #[error("failed to get modified timestamp for {0:?}: {1}")]
    ModifiedAccessError(PathBuf, IoError),
    #[error("{0:?} is unsupported file type")]
    UnsupportedType(PathBuf),
}

#[derive(Debug, Display)]
pub enum ValidityWarning {
    #[strum(to_string = "{0:?} was modified after last access")]
    Modified(PathBuf),
    #[strum(to_string = "{0:?} has changed in type")]
    Type(PathBuf),
    #[strum(to_string = "{0:?} has changed in size")]
    Size(PathBuf),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RecordData {
    pub entries: Vec<RecordEntry>,
}

#[derive(Debug, Clone)]
pub enum Action {
    Copy(Vec<PathBuf>),
    Cut(Vec<PathBuf>),
    Paste(PathBuf),
    Clipboard,
    History,
    Help,
}

#[derive(Debug, Error)]
pub enum InputError {
    #[error("missing argument: {0}")]
    MissingArgument(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

pub struct PasteContent {
    pub entries: Vec<RecordEntry>,
    pub source: RecordType,
}
