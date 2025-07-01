use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};
use std::{path::PathBuf, time::SystemTime};
use strum_macros::Display;
use uuid::Uuid;

use crate::{
    exceptions::{FileError, FileWarning},
    utils::get_metadata,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum Operation {
    #[strum(to_string = "copy")]
    Copy,
    #[strum(to_string = "cut")]
    Cut,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Display)]
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
    pub fn check_validity(&self) -> Result<Option<FileWarning>, FileError> {
        let Metadata {
            modified,
            size,
            entry_type,
            absolute_path,
        } = get_metadata(&self.path)?;

        if entry_type != self.entry_type {
            return Ok(Some(FileWarning::TypeMismatch {
                path: absolute_path,
                old_type: self.entry_type.to_string(),
                new_type: entry_type.to_string(),
            }));
        }

        if let (Some(expected_size), Some(self_size)) = (size, self.size) {
            if self_size != expected_size {
                return Ok(Some(FileWarning::SizeMismatch {
                    path: absolute_path,
                    old_size: self_size,
                    new_size: expected_size,
                }));
            }
        }

        if modified > self.timestamp {
            return Ok(Some(FileWarning::ModifiedMismatch {
                path: absolute_path,
            }));
        }

        Ok(None)
    }
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

pub struct PasteContent {
    pub entries: Vec<RecordEntry>,
    pub source: RecordType,
}
