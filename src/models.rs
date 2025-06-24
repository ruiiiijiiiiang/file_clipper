use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};
use std::{path::PathBuf, time::SystemTime};

use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Operation {
    Copy,
    Cut,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EntryType {
    File,
    Directory,
    Symlink,
}

impl EntryType {
    pub fn matches_metadata(&self, metadata: &std::fs::Metadata) -> bool {
        match self {
            EntryType::Directory => metadata.is_dir(),
            EntryType::File => metadata.is_file(),
            EntryType::Symlink => metadata.file_type().is_symlink(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RecordType {
    Clipboard,
    History,
}

pub const fn record_type_to_string(record_type: RecordType) -> &'static str {
    match record_type {
        RecordType::Clipboard => "clipboard",
        RecordType::History => "history",
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordEntry {
    #[serde_as(as = "TimestampSeconds")]
    pub timestamp: SystemTime,
    pub size: Option<u64>,
    pub operation: Operation,
    pub entry_type: EntryType,
    pub path: PathBuf,
    pub id: uuid::Uuid,
}

#[derive(Deserialize, Serialize)]
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

#[derive(Debug, Clone)]
pub enum TuiMode {
    Clipboard,
    History,
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
    #[error("Invalid command: '{0}'. Type 'help' for available commands.")]
    InvalidCommand(String),
}
