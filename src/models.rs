use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Operation {
    Copy,
    Cut,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EntryType {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub enum StorageType {
    Clipboard,
    History,
}

pub const fn storage_type_to_string(storage_type: StorageType) -> &'static str {
    match storage_type {
        StorageType::Clipboard => "clipboard",
        StorageType::History => "history",
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipboardEntry {
    pub operation: Operation,
    pub entry_type: EntryType,
    pub path: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    entry_type: EntryType,
    path: String,
    timestamp: u64,
}

#[derive(Deserialize, Serialize)]
pub struct ClipboardData {
    pub entries: Vec<ClipboardEntry>,
}

#[derive(Deserialize, Serialize)]
pub struct HistoryData {
    pub entries: Vec<HistoryEntry>,
}
