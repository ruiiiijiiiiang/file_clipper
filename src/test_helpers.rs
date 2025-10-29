use ratatui::widgets::{ScrollbarState, TableState};
use std::{
    env::set_var,
    fs::{File, create_dir_all},
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

use crate::{
    files::get_metadata,
    models::{EntryType, Metadata, Operation, RecordEntry, RecordType},
    tui::Tui,
};

pub struct TestEnv {
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub home_dir: TempDir,
}

pub fn setup_test_env() -> TestEnv {
    let home_dir = tempdir().expect("Failed to create temp home dir");
    let source_dir = home_dir.path().join("source");
    let dest_dir = home_dir.path().join("dest");
    create_dir_all(&source_dir).unwrap();
    create_dir_all(&dest_dir).unwrap();
    unsafe {
        set_var("HOME", home_dir.path());
    }

    TestEnv {
        home_dir,
        source_dir,
        dest_dir,
    }
}

pub fn create_test_file(path: &Path, content: &str) {
    let mut file = File::create(path).expect("Failed to create test file");
    write!(file, "{}", content).expect("Failed to write to test file");
}

pub fn get_test_entry(path: &Path, operation: Operation) -> RecordEntry {
    let meta = get_metadata(path).expect("Failed to get metadata for test entry");
    RecordEntry {
        id: Uuid::new_v4(),
        timestamp: meta.modified,
        size: meta.size,
        operation,
        entry_type: meta.entry_type,
        path: meta.absolute_path,
    }
}

pub fn create_mock_record_entry(
    path: Option<PathBuf>,
    operation: Option<Operation>,
    entry_type: Option<EntryType>,
    timestamp: Option<SystemTime>,
    size: Option<u64>,
) -> RecordEntry {
    let id = Uuid::new_v4();
    let timestamp = timestamp.unwrap_or_else(SystemTime::now);
    let operation = operation.unwrap_or(Operation::Copy);
    let entry_type = entry_type.unwrap_or(EntryType::File);
    let path = path.unwrap_or_else(|| PathBuf::from(format!("/tmp/file_{}.txt", Uuid::new_v4())));
    let size = size.or(Some(123));

    RecordEntry {
        id,
        timestamp,
        size,
        operation,
        entry_type,
        path,
    }
}

pub fn create_file_and_get_metadata(dir: &TempDir, file_name: &str, content: &str) -> Metadata {
    let file_path = dir.path().join(file_name);
    let mut file = File::create(&file_path).expect("Failed to create test file");
    write!(file, "{}", content).expect("Failed to write to test file");
    file.sync_all().expect("Failed to sync file");
    get_metadata(&file_path).expect("Failed to get metadata for test file")
}

pub fn create_test_tui(entries_count: usize) -> Tui {
    let entries = (0..entries_count)
        .map(|i| {
            create_mock_record_entry(
                Some(PathBuf::from(format!("/test/path/{}", i))),
                Some(Operation::Copy),
                Some(EntryType::File),
                None,
                None,
            )
        })
        .collect::<Vec<RecordEntry>>();

    let mut tui = Tui {
        entries: entries.clone(),
        mode: RecordType::Clipboard,
        table_state: TableState::default(),
        scroll_state: ScrollbarState::new(entries.len().saturating_sub(1)),
        invalid: vec![false; entries.len()],
        marked: vec![false; entries.len()],
        should_exit: false,
        warnings: Vec::new(),
        infos: Vec::new(),
        paste_content: None,
    };
    if !entries.is_empty() {
        tui.table_state.select(Some(0));
    }
    tui
}
