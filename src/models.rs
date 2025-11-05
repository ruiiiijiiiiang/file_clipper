use serde::{Deserialize, Serialize};
use serde_with::{TimestampSeconds, serde_as};
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CollisionResolution {
    Skip,
    Overwrite,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollisionResolutionChoice {
    Yes,
    No,
    OverwriteAll,
    SkipAll,
    Quit,
}

impl CollisionResolutionChoice {
    pub fn from_str(input: &str) -> Option<CollisionResolutionChoice> {
        match input.to_lowercase().as_str() {
            "y" => Some(CollisionResolutionChoice::Yes),
            "n" => Some(CollisionResolutionChoice::No),
            "a" => Some(CollisionResolutionChoice::OverwriteAll),
            "s" => Some(CollisionResolutionChoice::SkipAll),
            "q" => Some(CollisionResolutionChoice::Quit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_resolution_choice_from_str_valid() {
        assert_eq!(
            CollisionResolutionChoice::from_str("y"),
            Some(CollisionResolutionChoice::Yes)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("Y"),
            Some(CollisionResolutionChoice::Yes)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("n"),
            Some(CollisionResolutionChoice::No)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("N"),
            Some(CollisionResolutionChoice::No)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("a"),
            Some(CollisionResolutionChoice::OverwriteAll)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("A"),
            Some(CollisionResolutionChoice::OverwriteAll)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("s"),
            Some(CollisionResolutionChoice::SkipAll)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("S"),
            Some(CollisionResolutionChoice::SkipAll)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("q"),
            Some(CollisionResolutionChoice::Quit)
        );
        assert_eq!(
            CollisionResolutionChoice::from_str("Q"),
            Some(CollisionResolutionChoice::Quit)
        );
    }

    #[test]
    fn test_collision_resolution_choice_from_str_invalid() {
        assert_eq!(CollisionResolutionChoice::from_str(""), None);
        assert_eq!(CollisionResolutionChoice::from_str("z"), None);
        assert_eq!(CollisionResolutionChoice::from_str("yes"), None);
        assert_eq!(CollisionResolutionChoice::from_str("no"), None);
        assert_eq!(CollisionResolutionChoice::from_str("123"), None);
        assert_eq!(CollisionResolutionChoice::from_str("quit"), None);
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(Operation::Copy.to_string(), "copy");
        assert_eq!(Operation::Cut.to_string(), "cut");
        assert_eq!(Operation::Link.to_string(), "link");
    }

    #[test]
    fn test_entry_type_display() {
        assert_eq!(EntryType::File.to_string(), "File");
        assert_eq!(EntryType::Directory.to_string(), "Directory");
        assert_eq!(EntryType::Symlink.to_string(), "Symlink");
    }

    #[test]
    fn test_record_type_display() {
        assert_eq!(RecordType::Clipboard.to_string(), "clipboard");
        assert_eq!(RecordType::History.to_string(), "history");
    }

    #[test]
    fn test_operation_equality() {
        assert_eq!(Operation::Copy, Operation::Copy);
        assert_ne!(Operation::Copy, Operation::Cut);
        assert_ne!(Operation::Copy, Operation::Link);
    }

    #[test]
    fn test_entry_type_equality() {
        assert_eq!(EntryType::File, EntryType::File);
        assert_ne!(EntryType::File, EntryType::Directory);
        assert_ne!(EntryType::File, EntryType::Symlink);
    }

    #[test]
    fn test_record_entry_equality() {
        let id = Uuid::new_v4();
        let timestamp = SystemTime::now();
        let entry1 = RecordEntry {
            id,
            timestamp,
            size: Some(100),
            operation: Operation::Copy,
            entry_type: EntryType::File,
            path: PathBuf::from("/tmp/test.txt"),
        };
        let entry2 = RecordEntry {
            id,
            timestamp,
            size: Some(100),
            operation: Operation::Copy,
            entry_type: EntryType::File,
            path: PathBuf::from("/tmp/test.txt"),
        };
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_record_entry_hash() {
        use std::collections::HashSet;
        let id = Uuid::new_v4();
        let timestamp = SystemTime::now();
        let entry = RecordEntry {
            id,
            timestamp,
            size: Some(100),
            operation: Operation::Copy,
            entry_type: EntryType::File,
            path: PathBuf::from("/tmp/test.txt"),
        };

        let mut set = HashSet::new();
        set.insert(entry.clone());
        assert!(set.contains(&entry));
    }

    #[test]
    fn test_collision_resolution_equality() {
        assert_eq!(CollisionResolution::Skip, CollisionResolution::Skip);
        assert_eq!(
            CollisionResolution::Overwrite,
            CollisionResolution::Overwrite
        );
        assert_ne!(CollisionResolution::Skip, CollisionResolution::Overwrite);
    }

    #[test]
    fn test_record_data_serialization() {
        let entry = RecordEntry {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            size: Some(100),
            operation: Operation::Copy,
            entry_type: EntryType::File,
            path: PathBuf::from("/tmp/test.txt"),
        };
        let data = RecordData {
            entries: vec![entry],
        };

        let serialized = toml::to_string(&data);
        assert!(serialized.is_ok());
    }

    #[test]
    fn test_paste_content_creation() {
        let entry = RecordEntry {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            size: Some(100),
            operation: Operation::Copy,
            entry_type: EntryType::File,
            path: PathBuf::from("/tmp/test.txt"),
        };
        let paste_content = PasteContent {
            entries: vec![entry.clone()],
            source: RecordType::Clipboard,
        };

        assert_eq!(paste_content.entries.len(), 1);
        assert_eq!(paste_content.entries[0], entry);
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = Metadata {
            modified: SystemTime::now(),
            size: Some(1024),
            entry_type: EntryType::File,
            absolute_path: PathBuf::from("/tmp/test.txt"),
        };

        assert_eq!(metadata.size, Some(1024));
        assert_eq!(metadata.entry_type, EntryType::File);
    }
}
