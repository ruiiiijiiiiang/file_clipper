use fs_extra::error::Error as FsError;
use glob::{GlobError, PatternError};
use std::{io::Error as IoError, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Record(#[from] RecordError),

    #[error(transparent)]
    File(#[from] FileError),

    #[error(transparent)]
    Tui(#[from] TuiError),
}

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("Could not get the user's home directory. Please check your permissions.")]
    GetHomeDir,

    #[error("Could not create configuration directory at '{path}'. Please check permissions or create it manually.")]
    CreateConfigDir {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not create record file at '{path}'. Please check permissions and available disk space.")]
    CreateRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not open record file at '{path}'. Please ensure the file exists and you have permission to read it.")]
    OpenRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not read from record file at '{path}'. The file may be corrupted. Try running `clp clear` to reset it.")]
    ReadRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not parse data from record file at '{path}'. The file may be corrupted or have an invalid format. Try running `clp clear` to reset it.")]
    DeserializeRecordFile {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Could not prepare data for saving to the record file.")]
    SerializeRecordFile {
        #[source]
        source: toml::ser::Error,
    },

    #[error("Could not write to record file at '{path}'. Please check permissions and available disk space.")]
    WriteRecordFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not delete record at '{path}'. Please check permissions.")]
    ClearRecords {
        path: PathBuf,
        #[source]
        source: IoError,
    },
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error(
        "The specified path '{path}' was not found. Please ensure it exists and is accessible."
    )]
    PathNotFound { path: PathBuf },

    #[error("Could not determine the full path for '{path}'. Please check if the path is valid or if there are permission issues.")]
    AbsolutePath {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not determine the current working directory. This may be a permission issue or a problem with the file system.")]
    Cwd {
        #[source]
        source: IoError,
    },

    #[error("Could not access metadata for '{path}'. The path may be invalid or you may not have the necessary permissions.")]
    Metadata {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not determine the file name for '{path}'. The path may be invalid or you may not have the necessary permissions.")]
    FileName { path: PathBuf },

    #[error("Could not read the last modified time for '{path}'. The path may be invalid or you may not have the necessary permissions.")]
    ModifiedAccess {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("The file type for '{path}' is not supported.")]
    UnsupportedType { path: PathBuf },

    #[error("Could not copy '{from_path}' to '{to_path}'. Please check that the destination exists and you have sufficient permissions.")]
    Copy {
        from_path: PathBuf,
        to_path: PathBuf,
        #[source]
        source: FsError,
    },

    #[error("Could not move '{from_path}' to '{to_path}'. Please check that the destination exists and you have sufficient permissions.")]
    Move {
        from_path: PathBuf,
        to_path: PathBuf,
        #[source]
        source: FsError,
    },

    #[error("Could not create a symlink from '{from_path}' to '{to_path}'. Please check that the destination exists and you have sufficient permissions.")]
    Link {
        from_path: PathBuf,
        to_path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("Could not read files matching the pattern '{path}'. Please check the pattern and your file permissions.")]
    GlobUnreadable {
        path: PathBuf,
        #[source]
        source: GlobError,
    },

    #[error("The provided glob pattern '{path}' is invalid. Please check the syntax.")]
    GlobInvalidPattern {
        path: PathBuf,
        #[source]
        source: PatternError,
    },
}
#[derive(Debug, Error)]
pub enum TuiError {
    #[error("A terminal error occurred while drawing the interface.")]
    TerminalDraw {
        #[source]
        source: IoError,
    },

    #[error("A terminal error occurred while waiting for input.")]
    EventPolling {
        #[source]
        source: IoError,
    },

    #[error("A terminal error occurred while reading input.")]
    EventRead {
        #[source]
        source: IoError,
    },

    #[error("A terminal error occurred while resizing the interface.")]
    TerminalAutoresize {
        #[source]
        source: IoError,
    },
}

#[derive(Debug, Error)]
pub enum AppWarning {
    #[error(transparent)]
    File(#[from] FileWarning),

    #[error(transparent)]
    Record(#[from] RecordWarning),
}

#[derive(Debug, Error)]
pub enum FileWarning {
    #[error("File '{path}' was modified since last access. Consider reviewing recent changes.")]
    ModifiedMismatch { path: PathBuf },

    #[error("File '{path}' changed type from {old_type} to {new_type}. This might indicate an unexpected alteration.")]
    TypeMismatch {
        path: PathBuf,
        old_type: String,
        new_type: String,
    },

    #[error("File '{path}' changed size from {old_size} bytes to {new_size} bytes. Check if this change was intentional.")]
    SizeMismatch {
        path: PathBuf,
        old_size: u64,
        new_size: u64,
    },

    #[error("Glob pattern '{path}' did not match any file.")]
    GlobUnmatched { path: PathBuf },

    #[error("File '{path}' already exists at the destination.")]
    AlreadyExists { path: PathBuf },

    #[error("Permission denied for source '{path}' or destination '{destination}'.")]
    NoPermission { path: PathBuf, destination: PathBuf },
}

#[derive(Debug, Error)]
pub enum RecordWarning {
    #[error("Could not read data from the clipboard file. It may be corrupted. Try running `clp clear` to reset it.")]
    ClipboardUnreadable,

    #[error("Specified entry was not found in the clipboard.")]
    EntryNotFound,
}

#[derive(Debug, Error)]
pub enum AppInfo {
    #[error("Copied {path}")]
    Copy { path: PathBuf },

    #[error("Cut {path}")]
    Cut { path: PathBuf },

    #[error("Linked {path}")]
    Link { path: PathBuf },

    #[error("Pasted {path}")]
    Paste { path: PathBuf },

    #[error("Deleted {path}")]
    Clear { path: PathBuf },
}
