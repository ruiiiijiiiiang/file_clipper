use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};
use std::{path::PathBuf, time::SystemTime};
use strum_macros::Display;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum Operation {
    #[strum(to_string = "copy")]
    Copy,
    #[strum(to_string = "cut")]
    Cut,
    #[strum(to_string = "link")]
    Link,
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

#[derive(Debug, Clone)]
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RecordData {
    pub entries: Vec<RecordEntry>,
}

#[derive(Debug, Clone)]
pub enum Action {
    Copy(Vec<PathBuf>),
    Cut(Vec<PathBuf>),
    Link(Vec<PathBuf>),
    Paste(PathBuf),
    Clipboard,
    History,
    Clear,
}

#[derive(Debug, Clone)]
pub struct PasteContent {
    pub entries: Vec<RecordEntry>,
    pub source: RecordType,
}
