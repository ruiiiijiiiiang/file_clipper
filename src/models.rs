use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};
use std::time::SystemTime;

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
    pub operation: Operation,
    pub entry_type: EntryType,
    pub path: String,
    pub id: uuid::Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct RecordData {
    pub entries: Vec<RecordEntry>,
}
